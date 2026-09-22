use parking_lot::RwLock;
use pv_recorder::{PvRecorder, PvRecorderBuilder};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::NATIVE_LIB_DIR;

#[cfg(target_os = "macos")]
const PV_LIBRARY_NAME: &str = "libpv_recorder.dylib";
#[cfg(target_os = "windows")]
const PV_LIBRARY_NAME: &str = "libpv_recorder.dll";
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
const PV_LIBRARY_NAME: &str = "libpv_recorder.so";

// The pv_recorder crate defaults to a library path baked in at compile time (OUT_DIR),
// which does not exist on end-user machines. Packaged builds ship the library in
// NATIVE_LIB_DIR, so use that when present.
fn builder(frame_length: i32) -> PvRecorderBuilder {
    let mut builder = PvRecorderBuilder::new(frame_length);
    let bundled = NATIVE_LIB_DIR.join(PV_LIBRARY_NAME);
    if bundled.is_file() {
        builder = builder.library_path(&bundled);
    }
    builder
}

// The recorder is replaceable: switching the microphone in the settings tears the old one
// down and builds a new one, so this is a lock rather than a write-once cell.
static RECORDER: RwLock<Option<PvRecorder>> = RwLock::new(None);
static IS_RECORDING: AtomicBool = AtomicBool::new(false);

pub fn init_microphone(device_index: i32, frame_length: u32) -> bool {
    if RECORDER.read().is_some() {
        return true; // already initialized
    }

    match builder(frame_length as i32).device_index(device_index).init() {
        Ok(pv) => {
            *RECORDER.write() = Some(pv);
            true
        }
        Err(msg) => {
            error!("Failed to initialize pvrecorder.\nError details: {:?}", msg);
            false
        }
    }
}

// Drop the current recorder (stopping it first). A blocked read_microphone returns an
// error once the device goes away, which is how the capture thread learns to stop.
pub fn shutdown() {
    let recorder = RECORDER.write().take();
    if let Some(recorder) = recorder {
        if IS_RECORDING.swap(false, Ordering::SeqCst) {
            if let Err(e) = recorder.stop() {
                warn!("Failed to stop the recorder while shutting it down: {}", e);
            }
        }
        info!("Recorder released.");
    }
}

// Blocks until a full frame is available. Returns false if nothing was read
// (recorder not initialized or read error) - the buffer is left untouched then.
pub fn read_microphone(frame_buffer: &mut [i16]) -> bool {
    let guard = RECORDER.read();
    let Some(recorder) = guard.as_ref() else {
        return false;
    };

    match recorder.read() {
        Ok(f) => {
            frame_buffer.copy_from_slice(f.as_slice());
            true
        }
        Err(msg) => {
            error!("Failed to read audio frame. {:?}", msg);
            false
        }
    }
}

pub fn start_recording(device_index: i32, frame_length: u32) -> Result<(), ()> {
    // ensure microphone is initialized
    if !init_microphone(device_index, frame_length) {
        return Err(());
    }

    let guard = RECORDER.read();
    let Some(recorder) = guard.as_ref() else {
        return Err(());
    };

    match recorder.start() {
        Ok(_) => {
            info!("START recording from microphone ...");
            IS_RECORDING.store(true, Ordering::SeqCst);
            Ok(())
        }
        Err(msg) => {
            error!("Failed to START audio recording: {}", msg);
            Err(())
        }
    }
}

pub fn stop_recording() -> Result<(), ()> {
    if !IS_RECORDING.load(Ordering::SeqCst) {
        return Ok(()); // already stopped or not yet initialized
    }

    let guard = RECORDER.read();
    let Some(recorder) = guard.as_ref() else {
        return Ok(());
    };

    match recorder.stop() {
        Ok(_) => {
            info!("STOP recording from microphone ...");
            IS_RECORDING.store(false, Ordering::SeqCst);
            Ok(())
        }
        Err(msg) => {
            error!("Failed to STOP audio recording: {}", msg);
            Err(())
        }
    }
}

pub fn list_audio_devices() -> Vec<String> {
    let audio_devices = builder(512).get_available_devices();
    match audio_devices {
        Ok(audio_devices) => audio_devices,
        Err(err) => {
            error!("Failed to get audio devices: {}", err);
            Vec::new()
        },
    }
}

pub fn get_audio_device_name(idx: i32) -> String {
    if idx == -1 {
        return String::from("System Default");
    }

    let audio_devices = list_audio_devices();
    let mut first_device: String = String::new();

    for (_idx, device) in audio_devices.iter().enumerate() {
        if idx as usize == _idx {
            return device.to_string();
        }

        if _idx == 0 {
            first_device = device.to_string()
        }
    }

    // return first device as default, if none were matched
    first_device
}
