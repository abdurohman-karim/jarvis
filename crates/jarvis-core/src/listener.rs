mod rustpotter;
mod vosk;

use parking_lot::RwLock;

use crate::config::structs::WakeWordEngine;

use crate::DB;

// the engine changes with the settings
static WAKE_WORD_ENGINE: RwLock<Option<WakeWordEngine>> = RwLock::new(None);

pub fn init() -> Result<(), String> {
    if WAKE_WORD_ENGINE.read().is_some() {
        return Ok(());
    }
    reinit()
}

// (Re)build the wake word engine from the current settings.
pub fn reinit() -> Result<(), String> {
    let engine = DB.get().map(|db| db.read().wake_word_engine)
        .unwrap_or(crate::config::DEFAULT_WAKE_WORD_ENGINE);

    *WAKE_WORD_ENGINE.write() = Some(engine);

    match engine {
        WakeWordEngine::Porcupine => {
            Err("Porcupine wake-word engine is not supported".to_string())
        }
        WakeWordEngine::Rustpotter => {
            info!("Initializing Rustpotter wake-word engine.");
            rustpotter::init()
                .map_err(|_| "Failed to init Rustpotter".to_string())
        }
        WakeWordEngine::Vosk => {
            info!("Initializing Vosk as wake-word engine.");
            warn!("Using Vosk as wake-word engine is highly not recommended, because it's very slow for this task.");
            vosk::init()
                .map_err(|_| "Failed to init Vosk wake-word".to_string())
        }
    }
}

pub fn data_callback(frame_buffer: &[i16]) -> Option<i32> {
    match (*WAKE_WORD_ENGINE.read())? {
        WakeWordEngine::Porcupine => None,
        WakeWordEngine::Rustpotter => rustpotter::data_callback(frame_buffer),
        WakeWordEngine::Vosk => vosk::data_callback(frame_buffer),
    }
}
