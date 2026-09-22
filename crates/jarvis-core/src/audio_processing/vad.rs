mod none;
mod energy;

use parking_lot::{Mutex, RwLock};

use crate::DB;

// the selected backend changes with the settings
static BACKEND: RwLock<Option<String>> = RwLock::new(None);
static ENERGY_STATE: Mutex<Option<energy::EnergyVad>> = Mutex::new(None);

fn backend() -> Option<String> {
    BACKEND.read().clone()
}

#[cfg(feature = "nnnoiseless")]
static NNNOISELESS_STATE: Mutex<Option<crate::models::nnnoiseless::NnnoiselessVAD>> = Mutex::new(None);

pub fn init() {
    let backend = DB.get()
        .map(|db| db.read().vad_backend.clone())
        .unwrap_or_else(|| "energy".to_string());

    *BACKEND.write() = Some(backend.clone());

    match backend.as_str() {
        "none" => {
            info!("VAD: disabled");
        }
        "energy" => {
            *ENERGY_STATE.lock() = Some(energy::EnergyVad::new());
            info!("VAD: Energy-based (adaptive)");
        }
        #[cfg(feature = "nnnoiseless")]
        "nnnoiseless" => {
            *NNNOISELESS_STATE.lock() = Some(crate::models::nnnoiseless::NnnoiselessVAD::new());
            info!("VAD: Nnnoiseless");
        }
        other => {
            warn!("Unknown VAD backend '{}', falling back to energy", other);
            *ENERGY_STATE.lock() = Some(energy::EnergyVad::new());
        }
    }
}

fn energy_detect(input: &[i16]) -> (bool, f32) {
    let mut state = ENERGY_STATE.lock();
    state.get_or_insert_with(energy::EnergyVad::new).detect(input)
}

// human readable state of the detector, for logs ("level -41.2 dB, floor -58.0 dB")
pub fn describe(input: &[i16]) -> String {
    match backend().as_deref() {
        Some("energy") | None => {
            let floor = ENERGY_STATE.lock().as_ref().map(|s| s.noise_floor_db()).unwrap_or(f32::NAN);
            format!("level {:.1} dB, floor {:.1} dB", energy::rms_dbfs(input), floor)
        }
        Some(other) => other.to_string(),
    }
}

// returns (is_voice, confidence)
pub fn detect(input: &[i16]) -> (bool, f32) {
    match backend().as_deref() {
        Some("none") | None => none::detect(input),
        Some("energy") => energy_detect(input),
        #[cfg(feature = "nnnoiseless")]
        Some("nnnoiseless") => {
            match NNNOISELESS_STATE.lock().as_mut() {
                Some(state) => state.detect(input),
                None => energy_detect(input),
            }
        }
        _ => energy_detect(input),
    }
}

pub fn reset() {
    match backend().as_deref() {
        #[cfg(feature = "nnnoiseless")]
        Some("nnnoiseless") => {
            if let Some(state) = NNNOISELESS_STATE.lock().as_mut() {
                state.reset();
            }
        }
        _ => {
            if let Some(state) = ENERGY_STATE.lock().as_mut() {
                state.reset();
            }
        }
    }
}
