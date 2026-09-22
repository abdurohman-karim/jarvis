pub mod structs;
use structs::AudioType;
use structs::SpeechToTextEngine;
use structs::WakeWordEngine;

use once_cell::sync::Lazy;
use std::env;
use std::fs;
use std::path::PathBuf;

use platform_dirs::AppDirs;

#[cfg(feature="jarvis_app")]
use rustpotter::{
    AudioFmt, BandPassConfig, DetectorConfig, FiltersConfig, GainNormalizationConfig,
    RustpotterConfig, ScoreMode,
};

use crate::config::structs::NoiseSuppressionBackend;
use crate::{APP_CONFIG_DIR, APP_DIRS, APP_LOG_DIR};

#[allow(dead_code)]
pub fn init_dirs() -> Result<(), String> {
    // infer app dirs
    if APP_DIRS.get().is_some() {
        return Ok(());
    }

    // cache_dir, config_dir, data_dir, state_dir
    APP_DIRS
        .set(AppDirs::new(Some(BUNDLE_IDENTIFIER), false).unwrap())
        .unwrap();

    // setup directories
    let mut config_dir = PathBuf::from(&APP_DIRS.get().unwrap().config_dir);
    let mut log_dir = PathBuf::from(&APP_DIRS.get().unwrap().config_dir);

    // create dirs, if required
    if !config_dir.exists() {
        if fs::create_dir_all(&config_dir).is_err() {
            config_dir = env::current_dir().expect("Cannot infer the config directory");
            fs::create_dir_all(&config_dir)
                .expect("Cannot create config directory, access denied?");
        }
    }

    if !log_dir.exists() {
        if fs::create_dir_all(&log_dir).is_err() {
            log_dir = env::current_dir().expect("Cannot infer the log directory");
            fs::create_dir_all(&log_dir).expect("Cannot create log directory, access denied?");
        }
    }

    // store inferred paths
    APP_CONFIG_DIR.set(config_dir).unwrap();
    APP_LOG_DIR.set(log_dir).unwrap();

    Ok(())
}

/*
   Defaults.
*/
pub const DEFAULT_AUDIO_TYPE: AudioType = AudioType::Kira;

// while the assistant speaks its own voice reaches the microphone; input is dropped for the
// duration of the sound plus this tail (room reverb, speaker latency)
pub const AUDIO_OUTPUT_TAIL: std::time::Duration = std::time::Duration::from_millis(350);
// assumed length when the backend cannot report one
pub const AUDIO_OUTPUT_UNKNOWN_DURATION: std::time::Duration = std::time::Duration::from_millis(1500);
// @TODO. Switch to Rustpotter once its detection quality is verified on real voices
// (it was broken until the frame re-chunking fix in listener/rustpotter.rs, so nobody could).
pub const DEFAULT_WAKE_WORD_ENGINE: WakeWordEngine = WakeWordEngine::Vosk;
pub const DEFAULT_SPEECH_TO_TEXT_ENGINE: SpeechToTextEngine = SpeechToTextEngine::Vosk;

// backend defaults (string IDs)
pub const DEFAULT_INTENT_BACKEND: &str = "intent-classifier";
pub const DEFAULT_SLOTS_BACKEND: &str = "none";
pub const DEFAULT_VAD_BACKEND: &str = "energy";

pub const DEFAULT_VOICE: &str = "jarvis-remaster";
pub const SOUND_PATH: &str = "resources/sound"; // extended from SOUND_DIR (resources/sound)
pub const VOICES_PATH: &str = "voices"; // extended from SOUND_PATH (resources/sound)

pub const BUNDLE_IDENTIFIER: &str = "com.jarvis.assistant";
pub const DB_FILE_NAME: &str = "app.db";
// session token the IPC server expects from clients (written by jarvis-app on startup)
pub const IPC_TOKEN_FILE: &str = "ipc.token";
pub const LOG_FILE_NAME: &str = "log.txt";
pub const APP_VERSION: Option<&str> = option_env!("CARGO_PKG_VERSION");

/*
   Tray.
*/
pub const TRAY_ICON: &str = "32x32.png";
pub const TRAY_TOOLTIP: &str = "Jarvis Voice Assistant";

// RUSPOTTER
pub const RUSPOTTER_MIN_SCORE: f32 = 0.62;

#[cfg(feature="jarvis_app")]
pub const RUSTPOTTER_DEFAULT_CONFIG: Lazy<RustpotterConfig> = Lazy::new(|| {
    RustpotterConfig {
        fmt: AudioFmt::default(),
        detector: DetectorConfig {
            avg_threshold: 0.,
            threshold: 0.5,
            min_scores: 15,
            score_ref: 0.22,
            band_size: 5,
            vad_mode: None,
            score_mode: ScoreMode::Max,
            eager: false,
            // comparator_band_size: 5,
            // comparator_ref: 0.22
        },
        filters: FiltersConfig {
            gain_normalizer: GainNormalizationConfig {
                // enabled: true,
                // gain_ref: None,
                // min_gain: 0.7,
                // max_gain: 1.0,
                enabled: false, // disable, now we have separate gain normalizer implementation
                gain_ref: None,
                min_gain: 0.7,
                max_gain: 1.0,
            },
            band_pass: BandPassConfig {
                enabled: true,
                low_cutoff: 80.,
                high_cutoff: 400.,
            },
        },
    }
});

// PICOVOICE
pub const COMMANDS_PATH: &str = "resources/commands/";
pub const KEYWORDS_PATH: &str = "resources/picovoice/keywords/";
pub const DEFAULT_KEYWORD: &str = "jarvis_windows.ppn";
pub const DEFAULT_SENSITIVITY: f32 = 1.0;

// VOSK
// pub const VOSK_MODEL_PATH: &str = const_concat!(PUBLIC_PATH, "/vosk/model_small");
pub const VOSK_MODELS_PATH: &str = "resources/vosk";
pub const VOSK_MODEL_PATH: &str = "resources/vosk/model_small";
pub const VOSK_FETCH_PHRASE: &str = "джарвис";
pub const VOSK_MIN_RATIO: f64 = 70.0;

// 0.7 lenient, expect false positives
// 0.8 balanced
// 0.9 strict
// etc
pub const VOSK_WAKE_CONFIDENCE: f32 = 0.9;

pub const VOSK_SPEECH_RECOGNIZER_MAX_ALTERNATIVES: u16 = 3;
pub const VOSK_SPEECH_RECOGNIZER_WORDS: bool = false;
pub const VOSK_SPEECH_PARTIAL_WORDS: bool = false;

// IRE (intents recognition)
pub const INTENT_CLASSIFIER_MIN_CONFIDENCE: f64 = 0.75;


// embedding classifier
pub const EMBEDDING_MIN_CONFIDENCE: f64 = 0.70;

// AUDIO PROCESSING DEFAULTS
pub const DEFAULT_NOISE_SUPPRESSION: NoiseSuppressionBackend = NoiseSuppressionBackend::None;
pub const DEFAULT_GAIN_NORMALIZER: bool = false;

// VAD settings
// energy VAD (adaptive, levels in dBFS; a plain 512-sample frame of room noise on a laptop
// mic is typically around -55..-45 dBFS, speech -35..-15 dBFS)
pub const VAD_ENERGY_INITIAL_FLOOR_DB: f32 = -60.0;  // assumed room level before calibration
pub const VAD_ENERGY_CALIBRATION_FRAMES: u32 = 16;   // ~0.5 s of fast floor adaptation at startup
pub const VAD_ENERGY_ON_MARGIN_DB: f32 = 9.0;        // voice starts this far above the noise floor
pub const VAD_ENERGY_OFF_MARGIN_DB: f32 = 5.0;       // ...and ends this far above it (hysteresis)
pub const VAD_ENERGY_MIN_DB: f32 = -52.0;            // never trigger below this level (≈ RMS 82)
pub const VAD_ENERGY_ONSET_FRAMES: u32 = 3;          // frames (~100 ms) above threshold to start; keyboard clicks are shorter
pub const VAD_ENERGY_HANGOVER_FRAMES: u32 = 4;       // frames below threshold to end (~130 ms)
pub const VAD_ENERGY_FLOOR_DOWN_ALPHA: f32 = 0.10;   // floor follows quieter frames quickly (~0.3 s)
pub const VAD_ENERGY_FLOOR_UP_ALPHA: f32 = 0.02;     // ...louder (sub-threshold) frames slower (~1.5 s)
pub const VAD_ENERGY_FLOOR_UP_ALPHA_DURING_VOICE: f32 = 0.004; // ...and speech very slowly (~8 s)
pub const VAD_NNNOISELESS_THRESHOLD: f32 = 0.8;  // probability threshold for nnnoiseless
pub const VAD_SILENCE_FRAMES: u32 = 15;  // frames of silence before speech end (~480ms)

// gain normalizer settings
pub const GAIN_TARGET_RMS: f32 = 3000.0;  // target RMS level
pub const GAIN_MIN: f32 = 0.5;  // minimum gain multiplier
pub const GAIN_MAX: f32 = 3.0;  // maximum gain multiplier

// nnnoiseless frame size (fixed by library)
pub const NNNOISELESS_FRAME_SIZE: usize = 480;

// LUA
pub const DEFAULT_LUA_SANDBOX: &str = "standard";
pub const DEFAULT_LUA_TIMEOUT: u64 = 10000; // ms

// ETC
pub const CMD_RATIO_THRESHOLD: f64 = 75f64;
pub const CMS_WAIT_DELAY: std::time::Duration = std::time::Duration::from_secs(15);

// pub const ASSISTANT_GREET_PHRASES: [&str; 3] = ["greet1", "greet2", "greet3"];
// pub const ASSISTANT_PHRASES_TBR: [&str; 17] = [
//     "джарвис",
//     "сэр",
//     "слушаю сэр",
//     "всегда к услугам",
//     "произнеси",
//     "ответь",
//     "покажи",
//     "скажи",
//     "давай",
//     "да сэр",
//     "к вашим услугам сэр",
//     "всегда к вашим услугам сэр",
//     "запрос выполнен сэр",
//     "выполнен сэр",
//     "есть",
//     "загружаю сэр",
//     "очень тонкое замечание сэр",
// ];



pub fn get_wake_phrases(lang: &str) -> &'static [&'static str] {
    match lang {
        "ru" => &["джарвис", "джервис", "гарвис", "джарви", "гарви"],
        "ua" => &["джарвіс", "джервіс"],
        "en" => &["jarvis", "jervis"],
        _ => &["jarvis"],
    }
}

pub fn get_phrases_to_remove(lang: &str) -> &'static [&'static str] {
    match lang {
        "ru" => &[
            "джарвис", "джервис", "гарвис", "джарви", "гарви",
            "сэр", "слушаю сэр", "всегда к услугам",
            "произнеси", "ответь", "покажи", "скажи", "давай",
            "да сэр", "к вашим услугам сэр", "загружаю сэр",
        ],
        "ua" => &[
            "джарвіс", "джервіс", "сер", "слухаю сер", "завжди до послуг",
            "скажи", "покажи", "відповідай", "давай",
            "так сер", "до ваших послуг сер",
        ],
        "en" => &[
            "jarvis", "jervis", "sir", "yes sir", "at your service",
            "please", "say", "show", "tell", "hey",
        ],
        _ => &["jarvis"],
    }
}

pub fn get_wake_grammar(lang: &str) -> &'static [&'static str] {
    match lang {
        "ru" => &[
            "джарвис", "[unk]", "джон", "джони", "джей",
            "джонстон", "привет", "давай",
        ],
        "ua" => &[
            "джарвіс", "[unk]", "джон", "джоні", "джей",
            "привіт", "давай",
        ],
        "en" => &[
            "jarvis", "[unk]", "john", "johnny", "jay",
            "hello", "hey", "hi",
        ],
        _ => &["jarvis", "[unk]"],
    }
}