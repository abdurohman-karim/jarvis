// Cleaning up recognized phrases.

use seqdiff::ratio;

use crate::config;

// The wake word detector runs a recognizer with a tiny grammar, so it hears "джарвис"
// reliably. The speech recognizer uses the full model, which does not really know the
// name and renders it as whatever sounds close - "тебя рис", "жарвис", "дарвин" - so the
// command text starts with garbage that no exact-match list can cover.
//
// Measured on the shipped Russian model: such fragments score 67-92% similarity against
// the wake phrases, while the first words of real commands stay below 47% ("установи"
// 46%, "громкость" 40%, "погода" 36%), so the threshold below separates them safely.
//
// Everything up to and including the last wake-word-like word within the first few words
// is dropped; if nothing would remain, the phrase is returned untouched.
pub fn strip_wake_word_prefix(text: &str, lang: &str) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return text.to_string();
    }

    let wake_phrases: Vec<Vec<char>> = config::get_wake_phrases(lang)
        .iter()
        .map(|p| p.chars().collect())
        .collect();

    let window = words.len().min(config::WAKE_PREFIX_MAX_WORDS);
    let mut cut_after: Option<usize> = None;

    for (i, word) in words.iter().take(window).enumerate() {
        let word_chars: Vec<char> = word.chars().collect();
        let best = wake_phrases
            .iter()
            .map(|wake| ratio(wake, &word_chars))
            .fold(0.0_f64, f64::max);

        if best >= config::WAKE_PREFIX_MIN_RATIO {
            cut_after = Some(i);
        }
    }

    match cut_after {
        Some(i) if i + 1 < words.len() => {
            let cleaned = words[i + 1..].join(" ");
            debug!("Stripped wake word prefix: '{}' -> '{}'", text, cleaned);
            cleaned
        }
        _ => text.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_misheard_wake_word() {
        // what the full Vosk model actually produced for "джарвис какая погода в ташкенте"
        assert_eq!(strip_wake_word_prefix("тебя рис какая погода в ташкенте", "ru"), "какая погода в ташкенте");
        assert_eq!(strip_wake_word_prefix("жарвис открой браузер", "ru"), "открой браузер");
        assert_eq!(strip_wake_word_prefix("чарвис включи музыку", "ru"), "включи музыку");
        assert_eq!(strip_wake_word_prefix("джарвис погода", "ru"), "погода");
    }

    #[test]
    fn keeps_real_commands_intact() {
        for phrase in [
            "какая погода в ташкенте",
            "установи город ташкент",
            "открой браузер",
            "громкость на максимум",
            "запусти счётчик",
            "включи музыку",
            "город ташкент",
        ] {
            assert_eq!(strip_wake_word_prefix(phrase, "ru"), phrase, "must not touch '{}'", phrase);
        }
    }

    #[test]
    fn never_empties_the_phrase() {
        // the whole phrase is the wake word: leave it, the caller decides what it means
        assert_eq!(strip_wake_word_prefix("джарвис", "ru"), "джарвис");
        assert_eq!(strip_wake_word_prefix("", "ru"), "");
    }

    #[test]
    fn only_looks_at_the_beginning() {
        // a late word that happens to be similar is not a wake word leftover
        let phrase = "какая погода в городе рис";
        assert_eq!(strip_wake_word_prefix(phrase, "ru"), phrase);
    }

    #[test]
    fn works_for_other_languages() {
        assert_eq!(strip_wake_word_prefix("jervis open browser", "en"), "open browser");
        assert_eq!(strip_wake_word_prefix("open browser", "en"), "open browser");
    }
}
