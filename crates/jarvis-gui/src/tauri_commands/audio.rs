use jarvis_core::recorder;
// use rodio::{Decoder, OutputStream, Sink};
use std::path::PathBuf;
use jarvis_core::audio;

#[tauri::command(async)]
pub fn pv_get_audio_devices(refresh: Option<bool>) -> Vec<String> {
    if refresh.unwrap_or(false) {
        recorder::refresh_audio_devices()
    } else {
        recorder::get_audio_devices()
    }
}

#[tauri::command(async)]
pub fn pv_get_audio_device_name(idx: i32) -> String {
     recorder::get_audio_device_name(idx)
}

#[tauri::command(async)]
pub fn play_sound(filename: &str) {
    let path = PathBuf::from(filename);
    audio::play_sound(&path);
}
