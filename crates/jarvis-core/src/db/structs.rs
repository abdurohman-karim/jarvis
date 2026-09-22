use crate::config;
use serde::{Deserialize, Serialize};

use crate::config::structs::SpeechToTextEngine;
use crate::config::structs::WakeWordEngine;
use crate::config::structs::NoiseSuppressionBackend;
use crate::models::Task;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Settings {
    pub microphone: i32,
    pub voice: String,

    pub wake_word_engine: WakeWordEngine,

    // backend selections (string IDs matching model or code backend IDs)
    #[serde(default = "default_intent_backend")]
    pub intent_backend: String,
    #[serde(default = "default_slots_backend")]
    pub slots_backend: String,
    #[serde(default = "default_vad_backend")]
    pub vad_backend: String,

    pub gliner_model: String,

    pub speech_to_text_engine: SpeechToTextEngine,
    pub vosk_model: String,

    // audio processing
    pub noise_suppression: NoiseSuppressionBackend,
    pub gain_normalizer: bool,

    #[serde(default = "default_language")]
    pub language: String,

    pub api_keys: ApiKeys,

    // ask a language model when no command matched
    #[serde(default = "default_ai_fallback")]
    pub ai_fallback: bool,
    #[serde(default)]
    pub ai_model: String,
    // read AI answers out loud with the system speech synthesis
    #[serde(default = "default_speak_ai_answers")]
    pub speak_ai_answers: bool,
}

fn default_ai_fallback() -> bool { config::DEFAULT_AI_FALLBACK }
fn default_speak_ai_answers() -> bool { config::DEFAULT_SPEAK_AI_ANSWERS }

fn default_intent_backend() -> String { config::DEFAULT_INTENT_BACKEND.to_string() }
fn default_slots_backend() -> String { config::DEFAULT_SLOTS_BACKEND.to_string() }
fn default_vad_backend() -> String { config::DEFAULT_VAD_BACKEND.to_string() }
fn default_language() -> String { crate::i18n::detect_system_language().to_string() }

fn parse_bool(val: &str) -> Result<bool, String> {
    match val.trim().to_lowercase().as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("expected 'true' or 'false', got: '{}'", val)),
    }
}

// ### KEY-VALUE ACCESS

impl Settings {
    /// read a setting by key. returns None for unknown keys.
    pub fn get(&self, key: &str) -> Option<String> {
        match key {
            "selected_microphone"       => Some(self.microphone.to_string()),
            "assistant_voice"           => Some(self.voice.clone()),
            "selected_wake_word_engine" => Some(format!("{:?}", self.wake_word_engine)),
            "intent_backend"            => Some(self.intent_backend.clone()),
            "slots_backend"             => Some(self.slots_backend.clone()),
            "vad_backend"               => Some(self.vad_backend.clone()),
            "selected_gliner_model"     => Some(self.gliner_model.clone()),
            "selected_vosk_model"       => Some(self.vosk_model.clone()),
            "speech_to_text_engine"     => Some(format!("{:?}", self.speech_to_text_engine)),
            "noise_suppression"         => Some(format!("{:?}", self.noise_suppression)),
            "gain_normalizer"           => Some(self.gain_normalizer.to_string()),
            "language"                  => Some(self.language.clone()),
            "api_key__picovoice"        => Some(self.api_keys.picovoice.clone()),
            "api_key__openai"           => Some(self.api_keys.openai.clone()),
            "api_key__gemini"           => Some(self.api_keys.gemini.clone()),
            "ai_fallback"               => Some(self.ai_fallback.to_string()),
            "ai_model"                  => Some(self.ai_model.clone()),
            "speak_ai_answers"          => Some(self.speak_ai_answers.to_string()),
            _ => None,
        }
    }

    /// write a setting by key. returns Err for unknown keys or invalid values.
    pub fn set(&mut self, key: &str, val: &str) -> Result<(), String> {
        match key {
            "selected_microphone" => {
                self.microphone = val.parse::<i32>()
                    .map_err(|_| format!("invalid integer: '{}'", val))?;
            }
            "assistant_voice" => {
                self.voice = val.to_string();
            }
            "selected_wake_word_engine" => {
                self.wake_word_engine = match val.to_lowercase().as_str() {
                    "rustpotter" => WakeWordEngine::Rustpotter,
                    "vosk"       => WakeWordEngine::Vosk,
                    "porcupine" | "picovoice" => WakeWordEngine::Porcupine,
                    _ => return Err(format!("unknown wake word engine: '{}'", val)),
                };
            }
            // Backend ids are either a code backend ("none", "energy", "intent-classifier")
            // or a model id from the catalog, so the value space is open - but it is not
            // arbitrary: an unknown id used to be accepted here and then silently fall back
            // to a default at init time, which is how mis-cased values from the GUI went
            // unnoticed. Ids are lowercase by convention; validation is skipped while the
            // model registry is not initialized.
            "intent_backend" => {
                let val = val.trim().to_lowercase();
                crate::models::validate_backend(Task::Intent, &val)?;
                self.intent_backend = val;
            }
            "slots_backend" => {
                let val = val.trim().to_lowercase();
                crate::models::validate_backend(Task::Slots, &val)?;
                self.slots_backend = val;
            }
            "vad_backend" => {
                let val = val.trim().to_lowercase();
                crate::models::validate_backend(Task::Vad, &val)?;
                self.vad_backend = val;
            }
            "selected_gliner_model" => {
                self.gliner_model = val.to_string();
            }
            "selected_vosk_model" => {
                self.vosk_model = val.to_string();
            }
            "speech_to_text_engine" => {
                self.speech_to_text_engine = match val.to_lowercase().as_str() {
                    "vosk" => SpeechToTextEngine::Vosk,
                    _ => return Err(format!("unknown speech to text engine: '{}'", val)),
                };
            }
            "noise_suppression" => {
                self.noise_suppression = match val.to_lowercase().as_str() {
                    "none"        => NoiseSuppressionBackend::None,
                    "nnnoiseless" => NoiseSuppressionBackend::Nnnoiseless,
                    _ => return Err(format!("unknown noise suppression backend: '{}'", val)),
                };
            }
            "gain_normalizer" => {
                self.gain_normalizer = parse_bool(val)?;
            }
            "language" => {
                self.language = val.to_string();
            }
            "api_key__picovoice" => {
                self.api_keys.picovoice = val.to_string();
            }
            "api_key__openai" => {
                self.api_keys.openai = val.to_string();
            }
            "api_key__gemini" => {
                self.api_keys.gemini = val.trim().to_string();
            }
            "ai_model" => {
                self.ai_model = val.trim().to_string();
            }
            "ai_fallback" => {
                self.ai_fallback = parse_bool(val)?;
            }
            "speak_ai_answers" => {
                self.speak_ai_answers = parse_bool(val)?;
            }
            _ => return Err(format!("unknown setting: '{}'", key)),
        }
        Ok(())
    }

    /// all valid setting keys (for enumeration, debugging, etc.)
    pub fn keys() -> &'static [&'static str] {
        &[
            "selected_microphone",
            "assistant_voice",
            "selected_wake_word_engine",
            "intent_backend",
            "slots_backend",
            "vad_backend",
            "selected_gliner_model",
            "selected_vosk_model",
            "speech_to_text_engine",
            "noise_suppression",
            "gain_normalizer",
            "language",
            "api_key__picovoice",
            "api_key__openai",
            "api_key__gemini",
            "ai_fallback",
            "ai_model",
            "speak_ai_answers",
        ]
    }
}

// ### DEFAULT

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            microphone: -1,
            voice: String::from(""),

            wake_word_engine: config::DEFAULT_WAKE_WORD_ENGINE,

            intent_backend: config::DEFAULT_INTENT_BACKEND.to_string(),
            slots_backend: config::DEFAULT_SLOTS_BACKEND.to_string(),
            vad_backend: config::DEFAULT_VAD_BACKEND.to_string(),

            gliner_model: String::new(),
            speech_to_text_engine: config::DEFAULT_SPEECH_TO_TEXT_ENGINE,
            vosk_model: String::from(""),

            noise_suppression: config::DEFAULT_NOISE_SUPPRESSION,
            gain_normalizer: config::DEFAULT_GAIN_NORMALIZER,

            language: crate::i18n::detect_system_language().to_string(),

            api_keys: ApiKeys {
                picovoice: String::from(""),
                openai: String::from(""),
                gemini: String::from(""),
            },

            ai_fallback: config::DEFAULT_AI_FALLBACK,
            ai_model: String::new(),
            speak_ai_answers: config::DEFAULT_SPEAK_AI_ANSWERS,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApiKeys {
    pub picovoice: String,
    pub openai: String,
    #[serde(default)]
    pub gemini: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    // every key listed in keys() must be readable and writable
    #[test]
    fn all_keys_are_readable_and_writable() {
        let mut settings = Settings::default();
        for key in Settings::keys() {
            let value = settings.get(key)
                .unwrap_or_else(|| panic!("get() does not know key '{}'", key));
            settings.set(key, &value)
                .unwrap_or_else(|e| panic!("set() rejected its own value for '{}': {}", key, e));
            assert_eq!(settings.get(key).as_deref(), Some(value.as_str()), "round-trip changed '{}'", key);
        }
    }

    #[test]
    fn unknown_key_is_rejected() {
        let mut settings = Settings::default();
        assert!(settings.set("vad", "energy").is_err());
        assert!(settings.get("vad").is_none());
    }

    // the ids the GUI shows come from models::catalog and must be accepted verbatim
    #[test]
    fn code_backend_ids_are_accepted() {
        let mut settings = Settings::default();
        for (key, value) in [
            ("intent_backend", "intent-classifier"),
            ("intent_backend", "none"),
            ("slots_backend", "none"),
            ("vad_backend", "energy"),
            ("vad_backend", "nnnoiseless"),
            ("vad_backend", "none"),
            ("noise_suppression", "None"),
            ("noise_suppression", "Nnnoiseless"),
            ("selected_wake_word_engine", "Rustpotter"),
            ("selected_wake_word_engine", "Vosk"),
            ("selected_wake_word_engine", "Porcupine"),
        ] {
            settings.set(key, value).unwrap_or_else(|e| panic!("{}={}: {}", key, value, e));
        }
    }

    // the ids the settings UI offers come from the same catalog the validation uses
    #[test]
    fn unknown_backend_is_rejected_once_the_catalog_is_known() {
        // without a registry every id is accepted (settings must stay writable)
        let mut settings = Settings::default();
        assert!(settings.set("vad_backend", "whatever").is_ok());

        crate::models::init().ok();
        assert!(settings.set("vad_backend", "energy").is_ok());
        let err = settings.set("vad_backend", "enrgy").unwrap_err();
        assert!(err.contains("unknown backend"), "{}", err);
        // the rejected value must not have been stored
        assert_eq!(settings.get("vad_backend").as_deref(), Some("energy"));
    }

    #[test]
    fn backend_ids_are_normalized() {
        let mut settings = Settings::default();
        settings.set("vad_backend", " Energy ").unwrap();
        assert_eq!(settings.get("vad_backend").as_deref(), Some("energy"));
    }
}
