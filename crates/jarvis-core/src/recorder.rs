mod pvrecorder;

use std::time::{Duration, Instant};

use once_cell::sync::OnceCell;
use parking_lot::Mutex;

use crate::DB;

// pvrecorder requires a frame buffer of 512 samples
const FRAME_LENGTH: u32 = 512;

static INITIALIZED: OnceCell<()> = OnceCell::new();

// Enumerating audio devices goes through the OS audio stack (CoreAudio / WASAPI) and takes
// seconds on some machines; startup used to do it 3-4 times. Cache the list for a while.
static DEVICES_CACHE: Mutex<Option<(Instant, Vec<String>)>> = Mutex::new(None);
const DEVICES_CACHE_TTL: Duration = Duration::from_secs(30);

pub fn init() -> Result<(), ()> {
    if INITIALIZED.get().is_some() {
        return Ok(());
    }

    info!("Loading recorder ...");
    debug!("Available audio_devices are:\n{:?}", get_audio_devices());

    info!("Initializing PvRecorder recording backend.");
    let selected_microphone = get_selected_microphone_index();
    if !pvrecorder::init_microphone(selected_microphone, FRAME_LENGTH) {
        error!("Recorder initialization failed.");
        return Err(());
    }

    info!(
        "Recorder initialization success. Listening to microphone ({}): {}",
        selected_microphone,
        get_audio_device_name(selected_microphone)
    );

    let _ = INITIALIZED.set(());
    Ok(())
}

// Blocks until a full frame is available; false on error (buffer untouched)
pub fn read_microphone(frame_buffer: &mut [i16]) -> bool {
    pvrecorder::read_microphone(frame_buffer)
}

pub const fn frame_length() -> usize {
    FRAME_LENGTH as usize
}

pub fn start_recording() -> Result<(), ()> {
    pvrecorder::start_recording(get_selected_microphone_index(), FRAME_LENGTH)
}

pub fn stop_recording() -> Result<(), ()> {
    pvrecorder::stop_recording()
}

pub fn get_selected_microphone_index() -> i32 {
    let idx = DB.get().map(|db| db.read().microphone).unwrap_or(-1);

    if idx > 0 {
        // validate that this microphone is actually in the list
        let devices = get_audio_devices();
        if (idx as usize) >= devices.len() {
            warn!("Microphone index {} not found ({} available), falling back to default", 
                idx, devices.len());
            return -1;
        }
    }
    
    idx
}

pub fn get_audio_devices() -> Vec<String> {
    {
        let cache = DEVICES_CACHE.lock();
        if let Some((at, devices)) = cache.as_ref() {
            if at.elapsed() < DEVICES_CACHE_TTL {
                return devices.clone();
            }
        }
    }

    refresh_audio_devices()
}

// Re-enumerate audio devices, bypassing the cache (e.g. after plugging in a microphone).
pub fn refresh_audio_devices() -> Vec<String> {
    let devices = pvrecorder::list_audio_devices();
    *DEVICES_CACHE.lock() = Some((Instant::now(), devices.clone()));
    devices
}

pub fn get_audio_device_name(idx: i32) -> String {
    pvrecorder::get_audio_device_name(idx)
}
