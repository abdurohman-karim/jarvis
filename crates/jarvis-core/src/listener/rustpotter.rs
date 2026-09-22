use std::sync::Mutex;

use once_cell::sync::OnceCell;
use rustpotter::Rustpotter;

use crate::{config, APP_DIR};

const RUSTPOTTER_PATH: &str = "resources/rustpotter";

// store rustpotter instance
static RUSTPOTTER: OnceCell<Mutex<Rustpotter>> = OnceCell::new();

pub fn init() -> Result<(), ()> {
    let rustpotter_config = config::RUSTPOTTER_DEFAULT_CONFIG;

    // create rustpotter instance
    match Rustpotter::new(&rustpotter_config) {
        Ok(mut rinstance) => {
            // success
            // wake word files list
            // @TODO. Make it configurable via GUI for custom user voice.
            let rustpotter_wake_word_files: [&str; 1] = [
                "jarvis-default.rpw",
                // "jarvis-community-1.rpw",
                // "jarvis-community-2.rpw",
                // "jarvis-community-3.rpw",
                // "jarvis-community-4.rpw",
                // "jarvis-community-5.rpw",
            ];

            // load wake word files (resolved against the app directory, cwd is arbitrary
            // when launched from a bundle or the GUI)
            let mut loaded = 0;
            for rpw in rustpotter_wake_word_files {
                let path = APP_DIR.join(RUSTPOTTER_PATH).join(rpw);
                let path_str = path.to_string_lossy();
                match rinstance.add_wakeword_from_file(rpw, &path_str) {
                    Ok(_) => loaded += 1,
                    Err(e) => error!("Failed to load wakeword file '{}': {}", path_str, e),
                }
            }

            if loaded == 0 {
                error!("Rustpotter: no wakeword files loaded, wake word detection will not work.");
                return Err(());
            }

            // store
            let _ = RUSTPOTTER.set(Mutex::new(rinstance));
        }
        Err(msg) => {
            error!("Rustpotter failed to initialize.\nError details: {}", msg);

            return Err(());
        }
    }

    Ok(())
}

pub fn data_callback(frame_buffer: &[i16]) -> Option<i32> {
    let mut lock = RUSTPOTTER.get().unwrap().lock();
    let rustpotter = lock.as_mut().unwrap();
    // let detection = rustpotter.process_samples(frame_buffer.to_vec()); // @TODO. Temp crutch. Fix optimization issue, frame_buffer should not be copied to a new vector!
    let detection = rustpotter.process_samples(frame_buffer);

    // info!("Ruspotter data callback");

    if let Some(detection) = detection {
        if detection.score > config::RUSPOTTER_MIN_SCORE {
            info!("Rustpotter detection info:\n{:?}", detection);

            return Some(0);
        } else {
            info!("Rustpotter detection info:\n{:?}", detection)
        }
    }

    None
}
