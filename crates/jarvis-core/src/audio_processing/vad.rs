mod none;
mod energy;

use once_cell::sync::OnceCell;
use parking_lot::Mutex;

use crate::DB;

static BACKEND: OnceCell<String> = OnceCell::new();
static ENERGY_STATE: Mutex<Option<energy::EnergyVad>> = Mutex::new(None);

#[cfg(feature = "nnnoiseless")]
static NNNOISELESS_STATE: OnceCell<Mutex<crate::models::nnnoiseless::NnnoiselessVAD>> = OnceCell::new();

pub fn init() {
    if BACKEND.get().is_some() {
        return;
    }

    let backend = DB.get()
        .map(|db| db.read().vad_backend.clone())
        .unwrap_or_else(|| "energy".to_string());

    BACKEND.set(backend.clone()).ok();

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
            NNNOISELESS_STATE.set(Mutex::new(crate::models::nnnoiseless::NnnoiselessVAD::new())).ok();
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
    match BACKEND.get().map(|s| s.as_str()) {
        Some("energy") | None => {
            let floor = ENERGY_STATE.lock().as_ref().map(|s| s.noise_floor_db()).unwrap_or(f32::NAN);
            format!("level {:.1} dB, floor {:.1} dB", energy::rms_dbfs(input), floor)
        }
        Some(other) => other.to_string(),
    }
}

// returns (is_voice, confidence)
pub fn detect(input: &[i16]) -> (bool, f32) {
    match BACKEND.get().map(|s| s.as_str()) {
        Some("none") | None => none::detect(input),
        Some("energy") => energy_detect(input),
        #[cfg(feature = "nnnoiseless")]
        Some("nnnoiseless") => {
            if let Some(state) = NNNOISELESS_STATE.get() {
                state.lock().detect(input)
            } else {
                energy_detect(input)
            }
        }
        _ => energy_detect(input),
    }
}

pub fn reset() {
    match BACKEND.get().map(|s| s.as_str()) {
        #[cfg(feature = "nnnoiseless")]
        Some("nnnoiseless") => {
            if let Some(state) = NNNOISELESS_STATE.get() {
                state.lock().reset();
            }
        }
        _ => {
            if let Some(state) = ENERGY_STATE.lock().as_mut() {
                state.reset();
            }
        }
    }
}
