use vosk::{DecodingState, Recognizer};
use std::sync::Arc;
use parking_lot::{Mutex, RwLock};

use crate::{vosk_models, i18n, config, models};
use crate::models::vosk::VoskModel;
use crate::DB;

// The model Arc keeps the vosk::Model alive for the recognizers. All three are replaced
// together when the configured model or the language changes.
static LOADED_MODEL_ID: RwLock<Option<String>> = RwLock::new(None);
static VOSK_MODEL: RwLock<Option<Arc<VoskModel>>> = RwLock::new(None);
static WAKE_RECOGNIZER: Mutex<Option<Recognizer>> = Mutex::new(None);
static SPEECH_RECOGNIZER: Mutex<Option<Recognizer>> = Mutex::new(None);

pub fn init_vosk() -> Result<(), String> {
    if VOSK_MODEL.read().is_some() {
        return Ok(());
    }
    reload()
}

// (Re)load the configured model and rebuild both recognizers. The previously loaded model
// is dropped from the registry, otherwise switching models would keep both in memory.
pub fn reload() -> Result<(), String> {
    let model_path = get_configured_model_path()?;
    let model_id = format!("vosk:{}", model_path.display());

    // load through registry (shared if anything else needs the same model)
    let vosk = models::vosk::load(
        models::registry(),
        &model_id,
        model_path.to_str().unwrap(),
    )?;

    // language-specific wake grammar
    let lang = i18n::get_language();
    let wake_grammar = config::get_wake_grammar(&lang);
    info!("Wake grammar for '{}': {:?}", lang, wake_grammar);

    let mut wake_recognizer = Recognizer::new_with_grammar(&vosk.model, 16000.0, wake_grammar)
        .ok_or("Failed to create wake word recognizer")?;

    wake_recognizer.set_max_alternatives(1);

    let mut speech_recognizer = Recognizer::new(&vosk.model, 16000.0)
        .ok_or("Failed to create speech recognizer")?;

    speech_recognizer.set_max_alternatives(config::VOSK_SPEECH_RECOGNIZER_MAX_ALTERNATIVES);
    speech_recognizer.set_words(config::VOSK_SPEECH_RECOGNIZER_WORDS);
    speech_recognizer.set_partial_words(config::VOSK_SPEECH_PARTIAL_WORDS);

    // drop the recognizers of the previous model before releasing it
    *WAKE_RECOGNIZER.lock() = Some(wake_recognizer);
    *SPEECH_RECOGNIZER.lock() = Some(speech_recognizer);

    let previous = VOSK_MODEL.write().replace(vosk);
    let previous_id = LOADED_MODEL_ID.write().replace(model_id.clone());

    if let (Some(previous), Some(previous_id)) = (previous, previous_id) {
        if previous_id != model_id {
            drop(previous);
            models::registry().unload(&previous_id);
        }
    }

    Ok(())
}

// id of the model currently in use, if any
pub fn loaded_model_id() -> Option<String> {
    LOADED_MODEL_ID.read().clone()
}


pub fn recognize_wake_word(data: &[i16]) -> Option<(String, f32)> {
    let mut guard = WAKE_RECOGNIZER.lock();
    let recognizer = guard.as_mut()?;

    match recognizer.accept_waveform(data) {
        Ok(DecodingState::Running) => {
            None
        }
        Ok(DecodingState::Finalized) => {
            let result = recognizer.result();
            
            if let Some(alternatives) = result.multiple() {
                if let Some(best) = alternatives.alternatives.first() {
                    if !best.text.is_empty() {
                        return Some((best.text.to_string(), best.confidence));
                    }
                }
            }
            
            None
        }
        _ => None,
    }
}


pub fn recognize_speech(data: &[i16]) -> Option<String> {
    let mut guard = SPEECH_RECOGNIZER.lock();
    let recognizer = guard.as_mut()?;

    match recognizer.accept_waveform(data) {
        Ok(DecodingState::Finalized) => {
            recognizer.result()
                .multiple()
                .and_then(|m| m.alternatives.first().map(|a| a.text.to_string()))
        }
        _ => None,
    }
}


pub fn reset_speech_recognizer() {
    if let Some(recognizer) = SPEECH_RECOGNIZER.lock().as_mut() {
        recognizer.reset();
    }
}

pub fn reset_wake_recognizer() {
    if let Some(recognizer) = WAKE_RECOGNIZER.lock().as_mut() {
        recognizer.reset();
    }
}

fn get_configured_model_path() -> Result<std::path::PathBuf, String> {
    // try to get from settings
    if let Some(db) = DB.get() {
        let settings = db.read();
        if !settings.vosk_model.is_empty() {
            if let Some(path) = vosk_models::get_model_path(&settings.vosk_model) {
                return Ok(path);
            }
            warn!("Configured Vosk model '{}' not found, falling back to auto-detect", settings.vosk_model);
        }
    }
    
    // auto-detect: prefer model matching current language
    let available = vosk_models::scan_vosk_models();
    let language = i18n::get_language();

    let lang_code = match language.as_str() {
        "ru" => "ru",
        "en" => "us",
        "ua" => "uk",
        other => other,
    };

    if let Some(matched) = available.iter().find(|m| m.language == lang_code) {
        info!("Auto-detected Vosk model for '{}': {}", language, matched.name);
        return Ok(matched.path.clone());
    }

    if let Some(first) = available.first() {
        info!("Auto-detected Vosk model (no language match): {}", first.name);
        return Ok(first.path.clone());
    }
    
    // fallback to legacy path
    let legacy_path = std::path::Path::new(config::VOSK_MODEL_PATH);
    if legacy_path.exists() {
        return Ok(legacy_path.to_path_buf());
    }
    
    Err("No Vosk models found".into())
}
