use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::collections::HashMap;

use crate::{config, stt, i18n};

// Wake phrases are compared char by char against every recognition candidate; keep the
// decomposed form per language instead of rebuilding it on every candidate.
static WAKE_CHARS: Lazy<RwLock<HashMap<String, Vec<Vec<char>>>>> = Lazy::new(|| RwLock::new(HashMap::new()));

fn wake_phrase_chars(lang: &str) -> Vec<Vec<char>> {
    if let Some(cached) = WAKE_CHARS.read().get(lang) {
        return cached.clone();
    }

    let chars: Vec<Vec<char>> = config::get_wake_phrases(lang)
        .iter()
        .map(|p| p.chars().collect())
        .collect();
    WAKE_CHARS.write().insert(lang.to_string(), chars.clone());
    chars
}

pub fn init() -> Result<(), ()> {
    Ok(()) // nothing to init for Vosk
}

pub fn data_callback(frame_buffer: &[i16]) -> Option<i32> {
    if let Some((recognized, _confidence)) = stt::recognize_wake_word(frame_buffer) {
        let recognized = recognized.trim().to_lowercase();
        
        // skip unknown/empty
        if recognized.is_empty() || recognized == "[unk]" {
            return None;
        }
        
        info!("Wake word candidate: '{}'", recognized);
        
        // language-specific wake phrase
        let wake_phrases = wake_phrase_chars(&i18n::get_language());

        // verify with seqdiff ratio
        for word in recognized.split_whitespace() {
            if word == "[unk]" {
                continue;
            }
            
            let word_chars: Vec<char> = word.chars().collect();

            for wake_chars in &wake_phrases {
                let similarity = seqdiff::ratio(wake_chars, &word_chars);

                if similarity >= config::VOSK_MIN_RATIO {
                    info!("Wake word match: '{}' ~ '{}' ({:.1}%)",
                        word, wake_chars.iter().collect::<String>(), similarity);
                    return Some(0);
                }
            }
        }
        
        // info!("Similarity: {:.1}% ('{}' vs '{}')", similarity, recognized, config::VOSK_FETCH_PHRASE);
    }
    
    None
}

// @TODO. Make it better somehow (more accurate or with higher sensitivity).
// pub fn data_callback(frame_buffer: &[i16]) -> Option<i32> {
//     // recognize & convert to sequence
//     let recognized_phrase = stt::recognize(&frame_buffer, true).unwrap_or("".into());

//     if !recognized_phrase.trim().is_empty() {
//         info!("Vosk wake-word debug info:");
//         info!("rec: {}", recognized_phrase);
//         let recognized_phrases = recognized_phrase.split_whitespace();
//         for phrase in recognized_phrases {
//             let recognized_phrase_chars = phrase.trim().to_lowercase().chars().collect::<Vec<_>>();

//             // compare
//             let compare_ratio = seqdiff::ratio(
//                 &config::VOSK_FETCH_PHRASE.chars().collect::<Vec<_>>(),
//                 &recognized_phrase_chars,
//             );
//             info!("og phrase: {:?}", &config::VOSK_FETCH_PHRASE);
//             info!("recognized phrase: {:?}", &recognized_phrase_chars);
//             info!("compare ratio: {}", compare_ratio);

//             if compare_ratio >= config::VOSK_MIN_RATIO {
//                 info!("Phrase activated.");
//                 return Some(0);
//             }
//         }
//     }

//     None
// }
