use once_cell::sync::OnceCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

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

pub fn play_sound(filename: &PathBuf) {
    match load_cached(filename) {
        Ok(sound_data) => {
            // sound_data.duration() can be used in order to sleep, if (for some reason) blocking behaviour is required

            // play it (non-blocking)
            if let Some(manager) = MANAGER.get() {
                if let Ok(mut audio_manager) = manager.lock() {
                    if let Err(e) = audio_manager.play(sound_data) {
                        warn!("Failed to play sound: {}", e);
                    }
                }
            } else {
                warn!("Audio manager not initialized");
            }
        }
        Err(err) => {
            warn!("Cannot find sound file: {} (err: {})", filename.display(), err);
        }
    }
}