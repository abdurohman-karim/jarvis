// Audio pipeline.
//
//   capture thread ──frames──▶ processing thread ──voice command──▶ executor thread
//   (recorder)                 (VAD / wake word / STT)               (intent, Lua/CLI, sounds)
//
// The capture thread only reads the microphone, so a slow consumer never makes the
// recorder overflow. The processing thread runs the state machine below and hands
// recognized commands to the executor; while a command runs it keeps listening for the
// wake word, and the executor reports back whether the assistant should keep listening
// (command chaining).

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TrySendError};
use std::time::{Duration, Instant};

use jarvis_core::{audio, audio_buffer::AudioRingBuffer, audio_processing, config, listener, recorder, stt, text, voices, ipc::{self, IpcEvent}, i18n};

use crate::executor::{self, ExecutorHandle};
use crate::should_stop;

use std::sync::atomic::{AtomicBool, Ordering};

const FRAME_LENGTH: usize = recorder::frame_length();

// while muted, captured audio is discarded (nothing reaches VAD / wake word / STT)
static MUTED: AtomicBool = AtomicBool::new(false);

pub fn set_muted(muted: bool) {
    MUTED.store(muted, Ordering::SeqCst);
    info!("Microphone {}", if muted { "muted" } else { "unmuted" });
    ipc::send(IpcEvent::Muted { muted });
}

pub fn is_muted() -> bool {
    MUTED.load(Ordering::SeqCst)
}
const SAMPLE_RATE: usize = 16000;

// ~2 seconds of audio may queue up before frames get dropped
const FRAME_QUEUE_CAPACITY: usize = 64;

pub type Frame = [i16; FRAME_LENGTH];

// VAD state machine
#[derive(Debug, Clone, Copy, PartialEq)]
enum VadState {
    WaitingForVoice,
    VoiceActive,
}

// why the command listening phase ended
enum CommandOutcome {
    // a command was recognized and handed to the executor
    Submitted(Receiver<bool>),
    // silence / timeout / nothing usable
    Abandoned,
    Stop,
}

pub fn start(executor: ExecutorHandle) -> Result<(), ()> {
    voices::play_greet();

    if recorder::start_recording().is_err() {
        error!("Cannot start recording.");
        return Err(());
    }
    info!(
        "Recording started. Microphone: {}",
        recorder::get_audio_device_name(recorder::get_selected_microphone_index())
    );

    let frames = spawn_capture_thread();

    ipc::send(IpcEvent::Idle);
    let result = processing_loop(frames, executor);

    recorder::stop_recording().ok();
    ipc::send(IpcEvent::Stopping);
    result
}

// ### CAPTURE

fn spawn_capture_thread() -> Receiver<Frame> {
    let (tx, rx) = mpsc::sync_channel::<Frame>(FRAME_QUEUE_CAPACITY);

    std::thread::Builder::new()
        .name("audio-capture".into())
        .spawn(move || capture_loop(tx))
        .expect("failed to spawn audio capture thread");

    rx
}

fn capture_loop(tx: SyncSender<Frame>) {
    let mut frame: Frame = [0; FRAME_LENGTH];
    let mut dropped: u64 = 0;

    while !should_stop() {
        if !recorder::read_microphone(&mut frame) {
            // recorder error: don't spin
            std::thread::sleep(Duration::from_millis(50));
            continue;
        }

        match tx.try_send(frame) {
            Ok(()) => {}
            Err(TrySendError::Full(_)) => {
                dropped += 1;
                if dropped == 1 || dropped % 100 == 0 {
                    warn!("Audio processing is falling behind, dropped {} frame(s)", dropped);
                }
            }
            Err(TrySendError::Disconnected(_)) => break,
        }
    }

    debug!("Audio capture thread finished.");
}

// Input is ignored while the assistant's own voice is audible, otherwise the recognizer
// turns the reply into a "command" (and an error sound, which is heard again...).
// Returns true if the frame was swallowed; recognizer state is reset so the tail of our
// own voice cannot merge with what the user says next.
fn drop_while_speaking(vad_state: &mut VadState, silence_frames: &mut u32, audio_buffer: &mut AudioRingBuffer) -> bool {
    if !audio::output_busy() {
        return false;
    }

    if *vad_state != VadState::WaitingForVoice {
        *vad_state = VadState::WaitingForVoice;
        stt::reset_wake_recognizer();
        stt::reset_speech_recognizer();
    }
    *silence_frames = 0;
    audio_buffer.clear();
    audio_processing::reset();
    true
}

// Next frame from the capture thread. None when stopping.
fn next_frame(frames: &Receiver<Frame>) -> Option<Frame> {
    loop {
        if should_stop() {
            return None;
        }
        match frames.recv_timeout(Duration::from_millis(200)) {
            Ok(frame) => return Some(frame),
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => return None,
        }
    }
}

// ### PROCESSING

fn processing_loop(frames: Receiver<Frame>, executor: ExecutorHandle) -> Result<(), ()> {
    // ring buffer: keeps last 5 seconds of audio (pre-roll)
    let mut audio_buffer = AudioRingBuffer::new(5.0, FRAME_LENGTH, SAMPLE_RATE);

    let mut vad_state = VadState::WaitingForVoice;
    let mut silence_frames: u32 = 0;

    // how many frames of silence before we consider speech ended (1.5 s)
    let silence_threshold = frames_for_seconds(1.5);

    // command currently running in the executor (if any); its result tells us whether
    // to go straight back to listening for a command (chaining)
    let mut running_command: Option<Receiver<bool>> = None;

    // ### WAKE WORD DETECTION LOOP
    'wake_word: loop {
        // executor finished a voice command?
        if let Some(done) = &running_command {
            match done.try_recv() {
                Ok(true) => {
                    running_command = None;
                    info!("Chaining enabled, continuing to listen...");
                    match listen_for_command(&frames, &executor, false) {
                        CommandOutcome::Submitted(rx) => running_command = Some(rx),
                        CommandOutcome::Abandoned => {}
                        CommandOutcome::Stop => break,
                    }
                    reset_after_command(&mut vad_state, &mut silence_frames, &mut audio_buffer);
                    continue 'wake_word;
                }
                Ok(false) | Err(mpsc::TryRecvError::Disconnected) => {
                    running_command = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }

        let Some(frame) = next_frame(&frames) else {
            break;
        };

        if drop_while_speaking(&mut vad_state, &mut silence_frames, &mut audio_buffer) {
            continue;
        }

        if is_muted() {
            if vad_state == VadState::VoiceActive {
                reset_after_command(&mut vad_state, &mut silence_frames, &mut audio_buffer);
            }
            continue;
        }

        let processed = audio_processing::process(&frame);

        match vad_state {
            VadState::WaitingForVoice => {
                // always buffer (processed) audio
                audio_buffer.push(&processed.samples);

                if processed.is_voice {
                    // voice started! flush buffer to the wake word engine
                    info!("VAD: Voice started ({}), flushing {} buffered frames",
                        audio_processing::vad::describe(&processed.samples), audio_buffer.len());

                    let buffered = audio_buffer.drain_all();
                    for buffered_frame in &buffered {
                        listener::data_callback(buffered_frame);
                    }
                    audio_buffer.recycle(buffered);
                    // the current frame was never buffered
                    listener::data_callback(&processed.samples);

                    vad_state = VadState::VoiceActive;
                    silence_frames = 0;
                }
            }

            VadState::VoiceActive => {
                // dual-feed: speech recognizer gets frames in parallel with wake word detector
                let _ = stt::recognize(&processed.samples, false);

                // feed to wake word detector
                if listener::data_callback(&processed.samples).is_some() {
                    // WAKE WORD DETECTED!
                    info!("Wake word activated!");
                    ipc::send(IpcEvent::WakeWordDetected);

                    // a command still running in the executor keeps running, but its
                    // chain result is no longer relevant
                    running_command = None;

                    stt::reset_wake_recognizer();
                    audio_processing::reset();

                    // brief sniff to keep feeding STT while transitioning
                    for _ in 0..frames_for_seconds(0.3) {
                        let Some(frame) = next_frame(&frames) else { break 'wake_word };
                        let sniffed = audio_processing::process(&frame);
                        stt::recognize(&sniffed.samples, false);
                    }

                    ipc::send(IpcEvent::Listening);
                    match listen_for_command(&frames, &executor, true) {
                        CommandOutcome::Submitted(rx) => running_command = Some(rx),
                        CommandOutcome::Abandoned => {}
                        CommandOutcome::Stop => break,
                    }

                    reset_after_command(&mut vad_state, &mut silence_frames, &mut audio_buffer);
                    continue 'wake_word;
                }

                // track silence
                if processed.is_voice {
                    silence_frames = 0;
                } else {
                    silence_frames += 1;

                    if silence_frames > silence_threshold {
                        debug!("VAD: Silence timeout, returning to wait state");
                        vad_state = VadState::WaitingForVoice;
                        silence_frames = 0;
                        stt::reset_wake_recognizer();
                        stt::reset_speech_recognizer(); // reset since we were dual-feeding
                    }
                }
            }
        }
    }

    info!("Stop signal received, shutting down...");
    voices::play_goodbye();
    Ok(())
}

fn reset_after_command(vad_state: &mut VadState, silence_frames: &mut u32, audio_buffer: &mut AudioRingBuffer) {
    *vad_state = VadState::WaitingForVoice;
    *silence_frames = 0;
    audio_buffer.clear();
    stt::reset_wake_recognizer();
    stt::reset_speech_recognizer();
    audio_processing::reset();
    ipc::send(IpcEvent::Idle);
}

// Listen for a spoken command after the wake word (or after a chained command).
// Returns once a command was handed to the executor or listening was abandoned.
fn listen_for_command(frames: &Receiver<Frame>, executor: &ExecutorHandle, prefed_audio: bool) -> CommandOutcome {
    let mut audio_buffer = AudioRingBuffer::new(2.0, FRAME_LENGTH, SAMPLE_RATE);
    let mut vad_state = if prefed_audio { VadState::VoiceActive } else { VadState::WaitingForVoice };
    let mut silence_frames: u32 = 0;
    let mut start = Instant::now();
    let mut first_recognition = prefed_audio;

    // longer silence threshold for commands (user might pause to think): 5 seconds
    let silence_threshold = frames_for_seconds(5.0);

    let language = i18n::get_language();
    let wake_phrases = config::get_wake_phrases(&language);

    loop {
        let Some(frame) = next_frame(frames) else {
            return CommandOutcome::Stop;
        };

        // our own reply ("yes, sir") must not be mistaken for the command, and the time
        // spent speaking must not count towards the command timeout
        if drop_while_speaking(&mut vad_state, &mut silence_frames, &mut audio_buffer) {
            start = Instant::now();
            continue;
        }

        if is_muted() {
            info!("Muted while listening for a command, returning to wake word mode.");
            return CommandOutcome::Abandoned;
        }
        let processed = audio_processing::process(&frame);

        match vad_state {
            VadState::WaitingForVoice => {
                audio_buffer.push(&processed.samples);

                if processed.is_voice {
                    // flush buffer to STT
                    let buffered = audio_buffer.drain_all();
                    for buffered_frame in &buffered {
                        stt::recognize(buffered_frame, false);
                    }
                    audio_buffer.recycle(buffered);
                    vad_state = VadState::VoiceActive;
                    silence_frames = 0;
                } else {
                    silence_frames += 1;

                    if silence_frames > silence_threshold {
                        info!("Long silence detected, returning to wake word mode.");
                        return CommandOutcome::Abandoned;
                    }
                }
            }

            VadState::VoiceActive => {
                // feed to STT
                if let Some(raw) = stt::recognize(&processed.samples, false) {
                    info!("Recognized voice: {}", raw);

                    // the full recognizer renders the wake word as whatever sounds close
                    // ("тебя рис"), so drop that leftover before reporting or matching
                    let mut recognized_voice = text::strip_wake_word_prefix(&raw.to_lowercase(), &language);

                    ipc::send(IpcEvent::SpeechRecognized { text: recognized_voice.clone() });

                    // check if wake word repeated (reactivate)
                    if wake_phrases.iter().any(|wp| recognized_voice.contains(wp)) {
                        // strip the wake word
                        let mut remaining = recognized_voice.clone();
                        for wp in wake_phrases {
                            remaining = remaining.replace(wp, "");
                        }
                        let remaining = remaining.trim();

                        if remaining.is_empty() {
                            if first_recognition {
                                // leftover wake word from dual-feed, just discard it
                                info!("Discarding initial wake word from prefed audio");
                                first_recognition = false;
                            } else {
                                // just wake word, no command - reactivate
                                info!("Wake word repeated during chaining, reactivating...");
                                ipc::send(IpcEvent::Listening);
                            }

                            voices::play_reply();
                            stt::reset_speech_recognizer();
                            vad_state = VadState::WaitingForVoice;
                            silence_frames = 0;
                            start = Instant::now();
                            audio_buffer.clear();
                            continue;
                        }

                        // wake word + command in one phrase - execute the command part
                        info!("Wake word + command during chaining: '{}'", remaining);
                        recognized_voice = remaining.to_string();
                    }

                    first_recognition = false;

                    let command = executor::strip_assistant_phrases(&recognized_voice);

                    if command.len() < 5 {
                        debug!("Ignoring too short recognition: '{}'", command);
                        continue;
                    }

                    return CommandOutcome::Submitted(executor.submit_voice(command));
                }

                // track silence
                if processed.is_voice {
                    silence_frames = 0;
                } else {
                    silence_frames += 1;

                    if silence_frames > silence_threshold {
                        info!("Long silence detected, returning to wake word mode.");
                        return CommandOutcome::Abandoned;
                    }
                }
            }
        }

        // timeout
        if start.elapsed() > config::CMS_WAIT_DELAY {
            info!("Command timeout, returning to wake word mode.");
            return CommandOutcome::Abandoned;
        }
    }
}

// number of frames in `seconds` of audio
fn frames_for_seconds(seconds: f32) -> u32 {
    ((seconds * SAMPLE_RATE as f32) / FRAME_LENGTH as f32) as u32
}

pub fn close(code: i32) {
    info!("Closing application.");
    voices::play_goodbye();
    ipc::send(IpcEvent::Stopping);
    std::process::exit(code);
}
