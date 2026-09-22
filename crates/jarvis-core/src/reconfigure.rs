// Applying settings to a running assistant.
//
// Components used to be write-once (OnceCell), so any settings change needed a restart.
// They can now be rebuilt, and this module decides what a given change actually requires:
// switching the microphone restarts capture, switching the VAD only rebuilds the audio
// chain, and the speech model is only reloaded when it really differs.

use crate::{audio_processing, db, i18n, listener, recorder, stt, voices, DB};

// What the caller has to do after settings were applied
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Applied {
    // the microphone changed: capture has to be torn down and started again
    pub restart_capture: bool,
}

// A snapshot of the settings that affect the running pipeline
#[derive(Debug, Clone, PartialEq)]
pub struct PipelineSettings {
    pub microphone: i32,
    pub wake_word_engine: String,
    pub vosk_model: String,
    pub vad_backend: String,
    pub noise_suppression: String,
    pub gain_normalizer: bool,
    pub language: String,
    pub voice: String,
}

// The GUI runs in its own process and writes the settings file directly, so the
// assistant's in-memory copy has to be refreshed before anything is compared.
pub fn reload_from_disk() -> bool {
    let Some(fresh) = db::load_settings_file() else {
        return false;
    };
    let Some(db) = DB.get() else {
        return false;
    };

    *db.write() = fresh;
    info!("Settings re-read from disk");
    true
}

impl PipelineSettings {
    pub fn current() -> Option<Self> {
        let db = DB.get()?;
        let s = db.read();
        Some(Self {
            microphone: s.microphone,
            wake_word_engine: format!("{:?}", s.wake_word_engine),
            vosk_model: s.vosk_model.clone(),
            vad_backend: s.vad_backend.clone(),
            noise_suppression: format!("{:?}", s.noise_suppression),
            gain_normalizer: s.gain_normalizer,
            language: s.language.clone(),
            voice: s.voice.clone(),
        })
    }
}

// What a settings change requires. Deciding this is separate from doing it, so the
// decision can be tested without a microphone or a loaded model.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Plan {
    pub set_language: bool,
    pub set_voice: bool,
    pub rebuild_audio_chain: bool,
    pub reload_speech_model: bool,
    pub rebuild_wake_word: bool,
    pub restart_capture: bool,
}

impl Plan {
    pub fn is_empty(&self) -> bool {
        *self == Plan::default()
    }

    // names of what changed, for logs and the GUI
    pub fn changed(&self) -> Vec<String> {
        let mut changed = Vec::new();
        if self.set_language { changed.push("language".into()); }
        if self.set_voice { changed.push("voice".into()); }
        if self.rebuild_audio_chain { changed.push("audio_processing".into()); }
        if self.reload_speech_model { changed.push("vosk_model".into()); }
        if self.rebuild_wake_word { changed.push("wake_word_engine".into()); }
        if self.restart_capture { changed.push("microphone".into()); }
        changed
    }
}

// Work out what has to be rebuilt for `new` to take effect.
pub fn plan(previous: &PipelineSettings, new: &PipelineSettings) -> Plan {
    let language_changed = previous.language != new.language;

    Plan {
        set_language: language_changed,
        set_voice: previous.voice != new.voice,

        rebuild_audio_chain: previous.vad_backend != new.vad_backend
            || previous.noise_suppression != new.noise_suppression
            || previous.gain_normalizer != new.gain_normalizer,

        // the wake grammar is built from the language, so both depend on it
        reload_speech_model: previous.vosk_model != new.vosk_model || language_changed,
        rebuild_wake_word: previous.wake_word_engine != new.wake_word_engine || language_changed,

        restart_capture: previous.microphone != new.microphone,
    }
}

// Execute a plan. Everything except capture is rebuilt here; restarting capture is left
// to the caller, which owns the capture thread.
pub fn apply(plan: Plan, new: &PipelineSettings) {
    if plan.set_language {
        i18n::set_language(&new.language);
    }

    if plan.set_voice || plan.set_language {
        voices::set_current_voice(&new.voice);
    }

    if plan.rebuild_audio_chain {
        if let Err(e) = audio_processing::reinit() {
            error!("Failed to rebuild audio processing: {}", e);
        }
    }

    if plan.reload_speech_model {
        match stt::reload_model() {
            Ok(()) => info!("Speech recognition model reloaded"),
            Err(e) => error!("Failed to reload the speech model: {}", e),
        }
    }

    if plan.rebuild_wake_word {
        if let Err(e) = listener::reinit() {
            error!("Failed to rebuild the wake word engine: {}", e);
        }
    }

    if plan.restart_capture {
        // release the device; the caller starts capture again
        recorder::shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> PipelineSettings {
        PipelineSettings {
            microphone: -1,
            wake_word_engine: "Vosk".into(),
            vosk_model: "vosk-model-small-ru-0.22".into(),
            vad_backend: "energy".into(),
            noise_suppression: "None".into(),
            gain_normalizer: false,
            language: "ru".into(),
            voice: "jarvis-remaster".into(),
        }
    }

    #[test]
    fn nothing_changed_means_no_work() {
        assert!(plan(&settings(), &settings()).is_empty());
    }

    #[test]
    fn microphone_change_only_restarts_capture() {
        let mut new = settings();
        new.microphone = 2;
        let p = plan(&settings(), &new);
        assert_eq!(p, Plan { restart_capture: true, ..Plan::default() });
        assert_eq!(p.changed(), vec!["microphone"]);
    }

    #[test]
    fn audio_settings_only_rebuild_the_chain() {
        for mutate in [
            (|s: &mut PipelineSettings| s.vad_backend = "none".into()) as fn(&mut PipelineSettings),
            |s: &mut PipelineSettings| s.noise_suppression = "Nnnoiseless".into(),
            |s: &mut PipelineSettings| s.gain_normalizer = true,
        ] {
            let mut new = settings();
            mutate(&mut new);
            assert_eq!(plan(&settings(), &new), Plan { rebuild_audio_chain: true, ..Plan::default() });
        }
    }

    // the wake grammar comes from the speech model, which is built per language
    #[test]
    fn language_change_rebuilds_recognition() {
        let mut new = settings();
        new.language = "en".into();
        let p = plan(&settings(), &new);
        assert!(p.set_language && p.reload_speech_model && p.rebuild_wake_word);
        assert!(!p.restart_capture, "the microphone is unaffected by the language");
    }

    #[test]
    fn model_change_does_not_touch_the_wake_engine_choice() {
        let mut new = settings();
        new.vosk_model = "vosk-model-ru-0.42".into();
        assert_eq!(plan(&settings(), &new), Plan { reload_speech_model: true, ..Plan::default() });
    }
}
