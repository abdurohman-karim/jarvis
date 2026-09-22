// Phrases pre-generated in the voice pack's own voice.
//
// Voice packs only contain recorded reactions, so anything else - a command result, an
// answer - has to be synthesized. Synthesis is either a different voice (the operating
// system's) or slow (cloning). Phrases that are known in advance are therefore generated
// once by scripts/voice-clone/ and shipped with the pack; at runtime they are just played.
//
// Command results are only half known in advance: "Сейчас 14 часов 5 минут" has a fixed
// shape and a number that is not. The bank therefore also holds fragments ("сейчас 14
// часов", "5 минут"), and a phrase no entry says as a whole is assembled from them.

use std::collections::HashMap;
use std::path::PathBuf;

use parking_lot::RwLock;

use crate::voices;

// voice id + language -> bank
static BANKS: RwLock<Option<HashMap<String, Bank>>> = RwLock::new(None);

const MANIFEST: &str = "phrases.toml";
const DIRECTORY: &str = "phrases";

#[derive(Default)]
struct Bank {
    clips: HashMap<String, PathBuf>,
    // words in the longest entry, so assembling does not try longer spans than exist
    longest: usize,
}

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

fn load_bank(voice_path: &std::path::Path, language: &str) -> Bank {
    let directory = voice_path.join(language).join(DIRECTORY);
    let manifest = directory.join(MANIFEST);

    let Ok(content) = std::fs::read_to_string(&manifest) else {
        return Bank::default();
    };

    let parsed: HashMap<String, String> = match toml::from_str(&content) {
        Ok(parsed) => parsed,
        Err(e) => {
            warn!("Failed to parse {}: {}", manifest.display(), e);
            return Bank::default();
        }
    };

    let clips: HashMap<String, PathBuf> = parsed
        .into_iter()
        .filter_map(|(phrase, file)| {
            let path = directory.join(&file);
            path.is_file().then(|| (normalize(&phrase), path))
        })
        .collect();

    let longest = clips.keys().map(|k| k.split_whitespace().count()).max().unwrap_or(0);

    Bank { clips, longest }
}

// Split `text` into the fewest recordings that say it, or nothing if the bank does not
// cover all of it. Partial coverage is useless: half a sentence must not be spoken.
fn assemble(bank: &Bank, text: &str) -> Option<Vec<PathBuf>> {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() || bank.longest == 0 {
        return None;
    }

    // best[i]: the fewest recordings that say words[..i], and where the last one starts
    let mut best: Vec<Option<(usize, usize)>> = vec![None; words.len() + 1];
    best[0] = Some((0, 0));

    for end in 1..=words.len() {
        for start in end.saturating_sub(bank.longest)..end {
            let Some((count, _)) = best[start] else { continue };
            if bank.clips.contains_key(&words[start..end].join(" "))
                && best[end].is_none_or(|(current, _)| count + 1 < current)
            {
                best[end] = Some((count + 1, start));
            }
        }
    }

    best[words.len()]?;

    let mut parts = Vec::new();
    let mut at = words.len();
    while at > 0 {
        let (_, start) = best[at]?;
        parts.push(bank.clips.get(&words[start..at].join(" "))?.clone());
        at = start;
    }

    parts.reverse();
    Some(parts)
}

// Recordings that say `text` in the current voice: one if it was generated as a whole,
// several if it has to be assembled from fragments, none if the bank does not cover it.
pub fn lookup(text: &str, language: &str) -> Option<Vec<PathBuf>> {
    let voice = voices::get_current_voice()?;
    let bank_key = key(&voice.voice.id, language);
    let text = normalize(text);

    if let Some(banks) = BANKS.read().as_ref() {
        if let Some(bank) = banks.get(&bank_key) {
            return assemble(bank, &text);
        }
    }

    // first use of this voice + language
    let bank = load_bank(&voice.path, language);
    if !bank.clips.is_empty() {
        info!("Voice '{}' has {} pre-generated phrase(s) for '{}'", voice.voice.id, bank.clips.len(), language);
    }
    let found = assemble(&bank, &text);

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

    fn bank(entries: &[&str]) -> Bank {
        let clips = entries.iter()
            .map(|e| (normalize(e), PathBuf::from(format!("{}.mp3", e))))
            .collect::<HashMap<_, _>>();
        let longest = clips.keys().map(|k| k.split_whitespace().count()).max().unwrap_or(0);
        Bank { clips, longest }
    }

    #[test]
    fn a_whole_phrase_wins_over_its_fragments() {
        let bank = bank(&["сейчас", "14 часов", "сейчас 14 часов"]);
        let parts = assemble(&bank, "сейчас 14 часов").unwrap();
        assert_eq!(parts, vec![PathBuf::from("сейчас 14 часов.mp3")]);
    }

    #[test]
    fn a_phrase_is_assembled_from_fragments() {
        let bank = bank(&["сейчас 14 часов", "5 минут"]);
        let parts = assemble(&bank, &normalize("Сейчас 14 часов 5 минут")).unwrap();
        assert_eq!(parts.len(), 2);
        assert!(parts[0].ends_with("сейчас 14 часов.mp3"));
        assert!(parts[1].ends_with("5 минут.mp3"));
    }

    #[test]
    fn a_phrase_the_bank_only_half_covers_is_not_spoken() {
        let bank = bank(&["сейчас 14 часов", "5 минут"]);
        assert!(assemble(&bank, "сейчас 14 часов и 5 минут").is_none(), "'и' is not in the bank");
        assert!(assemble(&bank, "сейчас 14 часов 5 минут ровно").is_none());
        assert!(assemble(&bank, "расскажи анекдот").is_none());
    }

    #[test]
    fn an_empty_bank_says_nothing() {
        assert!(assemble(&Bank::default(), "сейчас 14 часов").is_none());
    }

    // Everything the Russian command packs can say has to be in the pack, as a phrase or
    // as fragments - a result the bank only half covers is spoken by the operating system
    // instead, in a different voice. Rewording a command means regenerating the bank.
    #[test]
    fn the_russian_pack_can_say_every_result_its_commands_produce() {
        let pack = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../resources/sound/voices/jarvis-remaster");

        let bank = load_bank(&pack, "ru");
        if bank.clips.is_empty() {
            return; // the bank is generated (see scripts/voice-clone), not required to exist
        }

        let months = ["января", "февраля", "марта", "апреля", "мая", "июня",
                      "июля", "августа", "сентября", "октября", "ноября", "декабря"];

        let mut results = Vec::new();

        // time/time.lua
        for hour in 0..24 {
            for minute in 0..60 {
                results.push(format!("Сейчас {} часов {} минут", hour, minute));
            }
        }

        // time/date.lua
        for day in 1..=31 {
            for month in months {
                results.push(format!("Сегодня {} {}", day, month));
            }
        }

        // system_mac/battery.lua
        for percent in 0..=100 {
            results.push(format!("Заряд {} процентов", percent));
            results.push(format!("Заряд {} процентов, питание от сети", percent));
        }

        for result in results {
            assert!(assemble(&bank, &normalize(&result)).is_some(),
                "the voice pack cannot say {:?}", result);
        }
    }
}
