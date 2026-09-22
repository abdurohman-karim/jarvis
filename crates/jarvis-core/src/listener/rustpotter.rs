use std::sync::Mutex;

use once_cell::sync::OnceCell;
use rustpotter::Rustpotter;

use crate::{config, APP_DIR};

const RUSTPOTTER_PATH: &str = "resources/rustpotter";

// Rustpotter only accepts frames of exactly `get_samples_per_frame()` samples (30 ms = 480 at
// 16 kHz) and silently returns None otherwise, while the recorder delivers 512-sample frames.
// Incoming audio is therefore re-chunked through this accumulator.
struct Detector {
    rustpotter: Rustpotter,
    pending: Vec<i16>,
    samples_per_frame: usize,
}

static DETECTOR: OnceCell<Mutex<Detector>> = OnceCell::new();

pub fn init() -> Result<(), ()> {
    let rustpotter_config = config::RUSTPOTTER_DEFAULT_CONFIG;

    // create rustpotter instance
    match Rustpotter::new(&rustpotter_config) {
        Ok(mut rinstance) => {
            // success
            // wake word files list: the default recording plus community recordings of
            // other voices, which improves recall for speakers unlike the default one
            // @TODO. Make it configurable via GUI for custom user voice.
            let rustpotter_wake_word_files: [&str; 6] = [
                "jarvis-default.rpw",
                "jarvis-community-1.rpw",
                "jarvis-community-2.rpw",
                "jarvis-community-3.rpw",
                "jarvis-community-4.rpw",
                "jarvis-community-5.rpw",
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

            let samples_per_frame = rinstance.get_samples_per_frame();
            info!("Rustpotter: {} wakeword file(s) loaded, {} samples per frame", loaded, samples_per_frame);

            // store
            let _ = DETECTOR.set(Mutex::new(Detector {
                rustpotter: rinstance,
                pending: Vec::with_capacity(samples_per_frame * 2),
                samples_per_frame,
            }));
        }
        Err(msg) => {
            error!("Rustpotter failed to initialize.\nError details: {}", msg);

            return Err(());
        }
    }

    Ok(())
}

pub fn data_callback(frame_buffer: &[i16]) -> Option<i32> {
    let mut lock = DETECTOR.get()?.lock().ok()?;
    let det = &mut *lock;

    det.pending.extend_from_slice(frame_buffer);

    let mut result = None;
    while det.pending.len() >= det.samples_per_frame {
        let chunk: Vec<i16> = det.pending.drain(..det.samples_per_frame).collect();
        if let Some(detection) = det.rustpotter.process_samples(&chunk) {
            if handle_detection(&detection) {
                result = Some(0);
            }
        }
    }

    result
}

// logs the detection and tells whether it passes our score threshold
fn handle_detection(detection: &rustpotter::RustpotterDetection) -> bool {
    let accepted = detection.score > config::RUSPOTTER_MIN_SCORE;
    info!(
        "Rustpotter: '{}' score {:.2} avg {:.2} (min {:.2}) - {}",
        detection.name, detection.score, detection.avg_score, config::RUSPOTTER_MIN_SCORE,
        if accepted { "ACCEPTED" } else { "rejected" }
    );
    accepted
}
