// Phrases pre-generated in the voice pack's own voice.
//
// Voice packs only contain recorded reactions, so anything else - a command result, an
// answer - has to be synthesized. Synthesis is either a different voice (the operating
// system's) or slow (cloning). Phrases that are known in advance are therefore generated
// once by scripts/voice-clone/ and shipped with the pack; at runtime they are just played.

use std::collections::HashMap;
use std::path::PathBuf;

use parking_lot::RwLock;

use crate::voices;

// voice id + language -> (phrase -> file)
static BANKS: RwLock<Option<HashMap<String, HashMap<String, PathBuf>>>> = RwLock::new(None);

const MANIFEST: &str = "phrases.toml";
const DIRECTORY: &str = "phrases";

fn key(voice_id: &str, language: &str) -> String {
    format!("{}/{}", voice_id, language)
}

// Phrases are matched on their text, so differences that do not change how a phrase sounds
// (case, punctuation, extra spaces) must not prevent a match.
fn normalize(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn load_bank(voice_path: &std::path::Path, language: &str) -> HashMap<String, PathBuf> {
    let directory = voice_path.join(language).join(DIRECTORY);
    let manifest = directory.join(MANIFEST);

    let Ok(content) = std::fs::read_to_string(&manifest) else {
        return HashMap::new();
    };

    let parsed: HashMap<String, String> = match toml::from_str(&content) {
        Ok(parsed) => parsed,
        Err(e) => {
            warn!("Failed to parse {}: {}", manifest.display(), e);
            return HashMap::new();
        }
    };

    parsed
        .into_iter()
        .filter_map(|(phrase, file)| {
            let path = directory.join(&file);
            path.is_file().then(|| (normalize(&phrase), path))
        })
        .collect()
}

// Recording of `text` in the current voice, if one was generated for it.
pub fn lookup(text: &str, language: &str) -> Option<PathBuf> {
    let voice = voices::get_current_voice()?;
    let bank_key = key(&voice.voice.id, language);

    if let Some(banks) = BANKS.read().as_ref() {
        if let Some(bank) = banks.get(&bank_key) {
            return bank.get(&normalize(text)).cloned();
        }
    }

    // first use of this voice + language
    let bank = load_bank(&voice.path, language);
    if !bank.is_empty() {
        info!("Voice '{}' has {} pre-generated phrase(s) for '{}'", voice.voice.id, bank.len(), language);
    }
    let found = bank.get(&normalize(text)).cloned();

    BANKS.write().get_or_insert_with(HashMap::new).insert(bank_key, bank);
    found
}

// Forget the loaded banks (the voice pack changed on disk)
pub fn reset() {
    *BANKS.write() = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalization_ignores_case_and_punctuation() {
        assert_eq!(normalize("Скриншот на рабочем столе"), normalize("скриншот  на рабочем столе."));
        assert_eq!(normalize("О чём спросить?"), "о чём спросить");
        assert_eq!(normalize("  Готово,   сэр!  "), "готово сэр");
    }

    #[test]
    fn different_phrases_stay_different() {
        assert_ne!(normalize("Вайфай не подключен"), normalize("Вайфай подключен"));
    }
}
