use once_cell::sync::OnceCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

// use kira::{
//     manager::{backend::DefaultBackend, AudioManager, AudioManagerSettings},
//     sound::static_sound::{StaticSoundData, StaticSoundSettings},
// };

use kira::{
    AudioManager, AudioManagerSettings, DefaultBackend,
    sound::static_sound::StaticSoundData,
};

static MANAGER: OnceCell<Mutex<AudioManager>> = OnceCell::new();

// Decoded sounds by path. Reaction sounds are short mp3/wav files played over and over;
// decoding them from disk on every play was the cost. StaticSoundData is Arc-backed, so
// cloning a cached entry is cheap.
static CACHE: Mutex<Option<HashMap<PathBuf, StaticSoundData>>> = Mutex::new(None);

pub fn init() -> Result<(), ()> {
    if MANAGER.get().is_some() {
        return Ok(());
    }  // already initialized

    // Create an audio manager. This plays sounds and manages resources.
    match AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()) {
        Ok(manager) => {
            // store
            MANAGER.set(Mutex::new(manager)).ok();

            // success
            Ok(())
        }
        Err(msg) => {
            error!("Failed to initialize audio stream.\nError details: {}", msg);

            // failed
            Err(())
        }
    }
}

fn load_cached(filename: &PathBuf) -> Result<StaticSoundData, kira::sound::FromFileError> {
    if let Ok(cache) = CACHE.lock() {
        if let Some(hit) = cache.as_ref().and_then(|c| c.get(filename)) {
            return Ok(hit.clone());
        }
    }

    let sound = StaticSoundData::from_file(filename)?;

    if let Ok(mut cache) = CACHE.lock() {
        cache.get_or_insert_with(HashMap::new).insert(filename.clone(), sound.clone());
    }

    Ok(sound)
}

// Plays the file and returns its duration, so the caller can tell how long the
// assistant's own voice will be audible.
pub fn play_sound(filename: &PathBuf) -> Option<Duration> {
    match load_cached(filename) {
        Ok(sound_data) => {
            let duration = sound_data.duration();

            // play it (non-blocking)
            let Some(manager) = MANAGER.get() else {
                warn!("Audio manager not initialized");
                return None;
            };

            match manager.lock() {
                Ok(mut audio_manager) => match audio_manager.play(sound_data) {
                    Ok(_) => Some(duration),
                    Err(e) => {
                        warn!("Failed to play sound: {}", e);
                        None
                    }
                },
                Err(e) => {
                    warn!("Audio manager lock poisoned: {}", e);
                    None
                }
            }
        }
        Err(err) => {
            warn!("Cannot find sound file: {} (err: {})", filename.display(), err);
            None
        }
    }
}

// How long the recording is, without playing it (it lands in the cache on the way).
pub fn duration_of(filename: &PathBuf) -> Option<Duration> {
    load_cached(filename).ok().map(|sound| sound.duration())
}

// Play the recordings back to back. Blocking, so the caller runs it on its own thread.
pub fn play_sequence(files: &[PathBuf]) {
    for file in files {
        let started = Instant::now();

        let Some(duration) = play_sound(file) else { continue };

        // The fragments are trimmed, so waiting out the full recording would leave an
        // audible gap between them; the next one starts a hair early instead.
        let wait = duration
            .saturating_sub(FRAGMENT_OVERLAP)
            .saturating_sub(started.elapsed());

        std::thread::sleep(wait);
    }
}

// how much of a fragment the next one starts under, to hide the join
const FRAGMENT_OVERLAP: Duration = Duration::from_millis(30);
