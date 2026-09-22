// Speaking arbitrary text in the assistant's voice, or failing that in the machine's.
//
// Three ways, in order: a phrase the voice pack has pre-generated (instant, its own voice,
// possibly assembled from fragments - see phrase_bank), cloning the pack's voice on the
// spot (its own voice, but seconds per phrase and an optional 4 GB component), or the
// operating system's synthesis (instant, someone else's voice).

use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::Mutex;

use crate::{audio, config, DB};

pub mod phrase_bank;
pub mod clone;

// How arbitrary text is spoken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Engine {
    // the operating system's voice: instant, offline, but not the assistant's voice
    System,
    // the voice pack's own voice, cloned locally (optional component, seconds per phrase)
    Clone,
}

fn engine() -> Engine {
    let configured = DB.get().map(|db| db.read().tts_engine.clone()).unwrap_or_default();
    match configured.as_str() {
        "clone" => Engine::Clone,
        _ => Engine::System,
    }
}

// the currently speaking process, so a new answer can interrupt the previous one
static SPEAKING: Mutex<Option<Child>> = Mutex::new(None);

// Bumped by stop(); a background synthesis that finishes after it is discarded.
static GENERATION: AtomicU64 = AtomicU64::new(0);

pub fn is_available() -> bool {
    cfg!(any(target_os = "macos", target_os = "windows", target_os = "linux"))
}

fn enabled() -> bool {
    DB.get().map(|db| db.read().speak_ai_answers).unwrap_or(true)
}

// Read `text` out loud. Returns false when synthesis is unavailable or switched off.
pub fn say(text: &str, language: &str) -> bool {
    if !enabled() || text.trim().is_empty() {
        return false;
    }

    stop();

    // a phrase generated in the voice pack's own voice beats any synthesis
    if let Some(recordings) = phrase_bank::lookup(text, language) {
        if recordings.len() == 1 {
            debug!("Speaking a pre-generated phrase: {}", recordings[0].display());
            audio::play_sound(&recordings[0]);
        } else {
            debug!("Speaking '{}' as {} pre-generated fragments", text, recordings.len());
            audio::play_sequence(recordings);
        }
        return true;
    }

    if engine() == Engine::Clone {
        // Synthesis takes seconds. Doing it inline would block whatever asked to speak
        // (a Lua command would hit its own timeout), so it happens in the background and
        // the audio plays when it is ready.
        let text = text.to_string();
        let language = language.to_string();
        let generation = GENERATION.load(Ordering::SeqCst);

        std::thread::Builder::new()
            .name("voice-clone".into())
            .spawn(move || match clone::synthesize(&text, &language) {
                Ok(path) => {
                    // a newer utterance (or stop()) happened while we were synthesizing
                    if GENERATION.load(Ordering::SeqCst) != generation {
                        debug!("Discarding a cloned phrase that is no longer current");
                        return;
                    }
                    if let Some(duration) = clone::duration_of(&path) {
                        audio::mark_output_busy_for(duration);
                    }
                    audio::play_sound(&path);
                }
                Err(e) => {
                    // the optional component may be missing or broken; say it anyway
                    warn!("Voice cloning failed ({}), falling back to the system voice", e);
                    if let Some(child) = spawn(&text, &language) {
                        audio::mark_output_busy_for(estimated_duration(&text));
                        *SPEAKING.lock() = Some(child);
                    }
                }
            })
            .ok();

        return true;
    }

    let child = spawn(text, language);

    match child {
        Some(child) => {
            // the microphone must ignore our own voice, exactly like a voice pack reply;
            // the length is unknown up front, so use a generous estimate from the text
            audio::mark_output_busy_for(estimated_duration(text));
            *SPEAKING.lock() = Some(child);
            true
        }
        None => false,
    }
}

// Which engines can actually be used right now
pub fn available_engines() -> Vec<&'static str> {
    let mut engines = vec!["system"];
    if clone::is_installed() {
        engines.push("clone");
    }
    engines
}

// Stop an answer that is still being read out (a new command interrupts it).
pub fn stop() {
    GENERATION.fetch_add(1, Ordering::SeqCst);

    if let Some(mut child) = SPEAKING.lock().take() {
        let _ = child.kill();
        let _ = child.wait();
    }
}

// Get the speech engine ready, so the first phrase does not wait for a model to load.
pub fn warm_up(language: &str) {
    if engine() == Engine::Clone && clone::is_installed() {
        clone::warm_up(language);
    }
}

fn estimated_duration(text: &str) -> std::time::Duration {
    let words = text.split_whitespace().count().max(1) as u64;
    let millis = (words * 1000 * 60) / config::TTS_WORDS_PER_MINUTE;
    std::time::Duration::from_millis(millis.clamp(1000, 60_000))
}

#[cfg(target_os = "macos")]
fn spawn(text: &str, language: &str) -> Option<Child> {
    let mut command = Command::new("say");

    // `say` picks the voice by name; the system default may not speak our language
    if let Some(voice) = macos_voice(language) {
        command.arg("-v").arg(voice);
    }

    command
        .arg("--")
        .arg(text)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| warn!("Failed to run `say`: {}", e))
        .ok()
}

// Voices shipped with macOS for the languages the assistant supports
#[cfg(target_os = "macos")]
fn macos_voice(language: &str) -> Option<&'static str> {
    match language {
        "ru" => Some("Milena"),
        "ua" => Some("Lesya"),
        "en" => Some("Samantha"),
        _ => None,
    }
}

#[cfg(target_os = "windows")]
fn spawn(text: &str, _language: &str) -> Option<Child> {
    // PowerShell ships with SAPI; the text is passed through an environment variable so
    // that quotes and newlines in it cannot break out of the script
    Command::new("powershell")
        .args(["-NoProfile", "-Command",
            "Add-Type -AssemblyName System.Speech; \
             (New-Object System.Speech.Synthesis.SpeechSynthesizer).Speak($env:JARVIS_TTS_TEXT)"])
        .env("JARVIS_TTS_TEXT", text)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| warn!("Failed to run PowerShell speech synthesis: {}", e))
        .ok()
}

#[cfg(all(unix, not(target_os = "macos")))]
fn spawn(text: &str, _language: &str) -> Option<Child> {
    Command::new("spd-say")
        .arg("--wait")
        .arg("--")
        .arg(text)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| warn!("Failed to run `spd-say` (is speech-dispatcher installed?): {}", e))
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_grows_with_the_text_and_stays_sane() {
        let short = estimated_duration("Да.");
        let long = estimated_duration(&"слово ".repeat(200));
        assert!(short >= std::time::Duration::from_secs(1), "at least a second: {:?}", short);
        assert!(long > short);
        assert!(long <= std::time::Duration::from_secs(60), "capped: {:?}", long);
    }
}
