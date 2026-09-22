mod kira;
mod rodio;

use once_cell::sync::OnceCell;
use parking_lot::Mutex;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::config::structs::AudioType;
use crate::{config, DB, SOUND_DIR};

static AUDIO_TYPE: OnceCell<AudioType> = OnceCell::new();

// When the assistant speaks, the microphone picks its own voice up and the recognizer
// happily turns it into a "command". Playback marks the output busy until the sound has
// finished (plus a tail for room reverb), and the audio pipeline drops input meanwhile.
static OUTPUT_BUSY_UNTIL: Mutex<Option<Instant>> = Mutex::new(None);

// true while the assistant's own voice may still be audible
pub fn output_busy() -> bool {
    match *OUTPUT_BUSY_UNTIL.lock() {
        Some(until) => Instant::now() < until,
        None => false,
    }
}

// how long the assistant's own voice may still be audible
pub fn output_busy_for() -> Option<Duration> {
    let until = (*OUTPUT_BUSY_UNTIL.lock())?;
    until.checked_duration_since(Instant::now())
}

// Mark our own output audible for `duration` (speech synthesis knows no exact length)
pub fn mark_output_busy_for(duration: Duration) {
    mark_output_busy(Some(duration));
}

fn mark_output_busy(duration: Option<Duration>) {
    let duration = duration.unwrap_or(config::AUDIO_OUTPUT_UNKNOWN_DURATION) + config::AUDIO_OUTPUT_TAIL;
    let until = Instant::now() + duration;

    let mut busy = OUTPUT_BUSY_UNTIL.lock();
    // several sounds may overlap: keep the latest end
    if busy.map_or(true, |current| current < until) {
        *busy = Some(until);
    }
}

pub fn init() -> Result<(), ()> {
    if AUDIO_TYPE.get().is_some() {
        return Ok(());
    } // already initialized

    // set default audio type
    // @TODO. Make it configurable?
    AUDIO_TYPE.set(config::DEFAULT_AUDIO_TYPE).unwrap();

    // load given audio backend
    match AUDIO_TYPE.get().unwrap() {
        AudioType::Rodio => {
            // Init Rodio
            info!("Initializing Rodio audio backend.");

            match rodio::init() {
                Ok(_) => {
                    info!("Successfully initialized Rodio audio backend.");
                }
                Err(()) => {
                    error!("Failed to initialize Rodio audio backend.");

                    return Err(());
                }
            }
        }
        AudioType::Kira => {
            // Init Kira
            info!("Initializing Kira audio backend.");

            match kira::init() {
                Ok(_) => {
                    info!("Successfully initialized Kira audio backend.");
                }
                Err(_msg) => {
                    error!("Failed to initialize Kira audio backend.");

                    return Err(());
                }
            }
        }
    }

    Ok(())
}

pub fn play_sound(filename: &PathBuf) {
    let audio_type = match AUDIO_TYPE.get() {
        Some(t) => t,
        None => {
            warn!("Audio not initialized, cannot play: {}", filename.display());
            return;
        }
    };
    
    info!("Playing {}", filename.display());

    let duration = match audio_type {
        AudioType::Rodio => rodio::play_sound(filename, true),
        AudioType::Kira => kira::play_sound(filename),
    };

    mark_output_busy(duration);
}

pub fn get_sound_directory() -> Option<PathBuf> {
    let db = DB.get()?;

    let voice_path = {
        let s = db.read();
        SOUND_DIR.join(&s.voice)
    };

    match voice_path.exists() {
        true => Some(voice_path),
        _ => {
            error!("No sounds folder found. Search path - {:?}", voice_path);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // one test: OUTPUT_BUSY_UNTIL is global state and tests run in parallel
    #[test]
    fn output_busy_window() {
        *OUTPUT_BUSY_UNTIL.lock() = None;
        assert!(!output_busy(), "nothing played yet");

        mark_output_busy(Some(Duration::from_millis(400)));
        assert!(output_busy());
        let after_short = output_busy_for().unwrap();

        // a longer sound started while the first one is still playing
        mark_output_busy(Some(Duration::from_millis(2000)));
        assert!(output_busy_for().unwrap() > after_short, "the later end must win");

        // a shorter one must not cut the window short
        let before = output_busy_for().unwrap();
        mark_output_busy(Some(Duration::from_millis(10)));
        assert!(output_busy_for().unwrap() + Duration::from_millis(50) >= before);

        // an unknown duration still guards for a while
        *OUTPUT_BUSY_UNTIL.lock() = None;
        mark_output_busy(None);
        assert!(output_busy());
        assert!(output_busy_for().unwrap() >= config::AUDIO_OUTPUT_UNKNOWN_DURATION);

        *OUTPUT_BUSY_UNTIL.lock() = None;
        assert!(!output_busy());
    }
}
