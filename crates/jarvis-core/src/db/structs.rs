use crate::config;
use serde::{Deserialize, Serialize};

use crate::config::structs::SpeechToTextEngine;
use crate::config::structs::WakeWordEngine;
use crate::config::structs::NoiseSuppressionBackend;

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
}

fn default_intent_backend() -> String { config::DEFAULT_INTENT_BACKEND.to_string() }
fn default_slots_backend() -> String { config::DEFAULT_SLOTS_BACKEND.to_string() }
fn default_vad_backend() -> String { config::DEFAULT_VAD_BACKEND.to_string() }
fn default_language() -> String { crate::i18n::detect_system_language().to_string() }

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
            // backend ids are lowercase (code backends: "none", "energy", "intent-classifier", ...;
            // model ids come from model.toml and are lowercase by convention)
            "intent_backend" => {
                self.intent_backend = val.trim().to_lowercase();
            }
            "slots_backend" => {
                self.slots_backend = val.trim().to_lowercase();
            }
            "vad_backend" => {
                self.vad_backend = val.trim().to_lowercase();
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
                self.gain_normalizer = match val.to_lowercase().as_str() {
                    "true"  => true,
                    "false" => false,
                    _ => return Err(format!("expected 'true' or 'false', got: '{}'", val)),
                };
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
            },
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApiKeys {
    pub picovoice: String,
    pub openai: String,
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

    #[test]
    fn backend_ids_are_normalized() {
        let mut settings = Settings::default();
        settings.set("vad_backend", " Energy ").unwrap();
        assert_eq!(settings.get("vad_backend").as_deref(), Some("energy"));
    }
}
