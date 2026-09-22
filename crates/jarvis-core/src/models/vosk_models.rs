// Vosk speech recognition models.
//
// Models are looked up in two places:
//   - the bundled directory (APP_DIR/resources/vosk), read-only inside an app bundle;
//   - the user directory (data dir/models/vosk), where models downloaded from the
//     catalog below are stored.
// A user model shadows a bundled one with the same name.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::{APP_DIR, APP_DIRS, APP_CONFIG_DIR, config};

#[derive(Debug, Clone, Serialize)]
pub struct VoskModelInfo {
    pub name: String,       // folder name: "vosk-model-small-ru-0.22"
    pub path: PathBuf,      // full path
    pub language: String,   // extracted from name: "ru"
    pub size: String,       // "small", "large", etc.
    pub bundled: bool,      // shipped with the app (not deletable) vs. downloaded by the user
}

// A model the user can download. Sizes are the zip sizes published on alphacephei.com.
#[derive(Debug, Clone, Serialize)]
pub struct VoskCatalogEntry {
    pub name: &'static str,
    // our language code ("ru", "en", "ua")
    pub language: &'static str,
    pub description: &'static str,
    pub size_mb: u32,
    pub url: &'static str,
}

macro_rules! model {
    ($name:literal, $lang:literal, $desc:literal, $mb:literal) => {
        VoskCatalogEntry {
            name: $name,
            language: $lang,
            description: $desc,
            size_mb: $mb,
            url: concat!("https://alphacephei.com/vosk/models/", $name, ".zip"),
        }
    };
}

pub const CATALOG: &[VoskCatalogEntry] = &[
    model!("vosk-model-small-ru-0.22", "ru", "Small, fast, good for commands", 45),
    model!("vosk-model-ru-0.42", "ru", "Large, best accuracy", 1800),
    model!("vosk-model-small-en-us-0.15", "en", "Small, fast, good for commands", 40),
    model!("vosk-model-en-us-0.22-lgraph", "en", "Medium, dynamic grammar", 128),
    model!("vosk-model-en-us-0.22", "en", "Large, best accuracy", 1800),
    model!("vosk-model-small-uk-v3-nano", "ua", "Nano, fastest", 73),
    model!("vosk-model-small-uk-v3-small", "ua", "Small", 133),
    model!("vosk-model-uk-v3", "ua", "Large, best accuracy", 343),
];

pub fn catalog_entry(name: &str) -> Option<&'static VoskCatalogEntry> {
    CATALOG.iter().find(|m| m.name == name)
}

// directory with models shipped in the app / repository
pub fn bundled_models_dir() -> PathBuf {
    APP_DIR.join(config::VOSK_MODELS_PATH)
}

// directory for models downloaded by the user (always writable)
pub fn user_models_dir() -> PathBuf {
    let base = APP_DIRS.get()
        .map(|d| d.data_dir.clone())
        .or_else(|| APP_CONFIG_DIR.get().cloned())
        .unwrap_or_else(|| APP_DIR.clone());
    base.join("models").join("vosk")
}

// Scan for available Vosk models (user models first, they shadow bundled ones)
pub fn scan_vosk_models() -> Vec<VoskModelInfo> {
    let mut models: Vec<VoskModelInfo> = Vec::new();

    for (dir, bundled) in [(user_models_dir(), false), (bundled_models_dir(), true)] {
        for model in scan_dir(&dir, bundled) {
            if !models.iter().any(|m| m.name == model.name) {
                models.push(model);
            }
        }
    }

    models.sort_by(|a, b| a.name.cmp(&b.name));
    models
}

fn scan_dir(models_dir: &Path, bundled: bool) -> Vec<VoskModelInfo> {
    let mut models = Vec::new();

    debug!("Scanning Vosk models in {}", models_dir.display());

    let entries = match fs::read_dir(models_dir) {
        Ok(e) => e,
        Err(_) => return models, // missing dir is normal (nothing bundled / nothing downloaded yet)
    };

    for entry in entries.flatten() {
        let path = entry.path();

        if !path.is_dir() || !is_vosk_model(&path) {
            continue;
        }

        let name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        let (language, size) = parse_model_name(&name);

        models.push(VoskModelInfo { name, path, language, size, bundled });
    }

    models
}

// Check if directory looks like a Vosk model
pub fn is_vosk_model(path: &Path) -> bool {
    // vosk models typically have these subdirectories
    path.join("am").exists() ||
    path.join("conf").exists() ||
    path.join("graph").exists() ||
    path.join("ivector").exists()
}

// Extract language and size from model name
// e.g., "vosk-model-small-ru-0.22" -> ("ru", "small")
fn parse_model_name(name: &str) -> (String, String) {
    let parts: Vec<&str> = name.split('-').collect();

    let mut language = String::from("unknown");
    let mut size = String::from("unknown");

    // look for common size indicators
    for part in &parts {
        match *part {
            "small" | "big" | "large" | "lgraph" => size = part.to_string(),
            // language codes are usually 2 letters
            s if s.len() == 2 && s.chars().all(|c| c.is_alphabetic()) => {
                language = s.to_string();
            }
            _ => {}
        }
    }

    (language, size)
}

// Get model path by name (user dir first, then bundled)
pub fn get_model_path(model_name: &str) -> Option<PathBuf> {
    scan_vosk_models()
        .into_iter()
        .find(|m| m.name == model_name)
        .map(|m| m.path)
}

// Remove a downloaded model. Bundled models cannot be deleted.
pub fn delete_user_model(model_name: &str) -> Result<(), String> {
    if model_name.is_empty() || model_name.contains(['/', '\\']) || model_name.starts_with('.') {
        return Err("invalid model name".into());
    }

    let path = user_models_dir().join(model_name);
    if !path.is_dir() {
        return Err(format!("model '{}' is not installed in the user directory", model_name));
    }

    fs::remove_dir_all(&path).map_err(|e| format!("failed to delete {}: {}", path.display(), e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_name_parsing() {
        assert_eq!(parse_model_name("vosk-model-small-ru-0.22"), ("ru".into(), "small".into()));
        assert_eq!(parse_model_name("vosk-model-en-us-0.22-lgraph"), ("us".into(), "lgraph".into()));
    }

    #[test]
    fn catalog_urls_are_well_formed() {
        for m in CATALOG {
            assert!(m.url.ends_with(&format!("{}.zip", m.name)), "{}", m.url);
            assert!(["ru", "en", "ua"].contains(&m.language), "{}", m.name);
        }
    }

    #[test]
    fn delete_rejects_paths() {
        assert!(delete_user_model("../x").is_err());
        assert!(delete_user_model("").is_err());
    }
}
