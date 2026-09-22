// Answering free-form questions with a large language model.
//
// The assistant only has pre-recorded replies, so an arbitrary answer cannot be spoken by
// a voice pack. Answers are therefore read out with the operating system's speech
// synthesis (see `speech`) and shown in the GUI.

use std::time::Duration;

use serde::Deserialize;

use crate::{config, DB};

#[derive(Debug)]
pub enum AiError {
    NotConfigured,
    Request(String),
    Api { status: u16, message: String },
    Empty,
}

impl std::fmt::Display for AiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AiError::NotConfigured => write!(f, "no Gemini API key configured"),
            AiError::Request(e) => write!(f, "request failed: {}", e),
            AiError::Api { status, message } => write!(f, "Gemini returned {}: {}", status, message),
            AiError::Empty => write!(f, "Gemini returned an empty answer"),
        }
    }
}

pub fn is_configured() -> bool {
    !api_key().is_empty()
}

fn api_key() -> String {
    DB.get().map(|db| db.read().api_keys.gemini.clone()).unwrap_or_default()
}

fn model() -> String {
    let configured = DB.get().map(|db| db.read().ai_model.clone()).unwrap_or_default();
    if configured.is_empty() {
        config::DEFAULT_AI_MODEL.to_string()
    } else {
        configured
    }
}

// System prompt: answers are spoken out loud, so they have to be short and plain.
fn system_prompt(language: &str) -> String {
    let language_name = match language {
        "ru" => "русском",
        "ua" => "українською",
        _ => "English",
    };

    if language == "en" {
        "You are a voice assistant. Answer in English, in one or two short sentences, \
         in plain words that sound natural when read aloud. No markdown, no lists, no emoji."
            .to_string()
    } else {
        format!(
            "Ты голосовой ассистент. Отвечай на {} языке, одним-двумя короткими предложениями, \
             простыми словами, которые естественно звучат вслух. Без разметки, списков и эмодзи.",
            language_name
        )
    }
}

#[derive(Deserialize)]
struct GeminiResponse {
    #[serde(default)]
    candidates: Vec<Candidate>,
    #[serde(default)]
    error: Option<GeminiError>,
}

#[derive(Deserialize)]
struct Candidate {
    #[serde(default)]
    content: Option<Content>,
    #[serde(rename = "finishReason", default)]
    finish_reason: String,
}

#[derive(Deserialize)]
struct Content {
    #[serde(default)]
    parts: Vec<Part>,
}

#[derive(Deserialize)]
struct Part {
    #[serde(default)]
    text: String,
}

#[derive(Deserialize)]
struct GeminiError {
    #[serde(default)]
    message: String,
    #[serde(default)]
    code: u16,
}

// Ask the model a question. Blocking: the executor runs off the audio thread.
pub fn ask(question: &str, language: &str) -> Result<String, AiError> {
    let key = api_key();
    if key.is_empty() {
        return Err(AiError::NotConfigured);
    }

    let model = model();
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
        model
    );

    let body = serde_json::json!({
        "system_instruction": { "parts": [{ "text": system_prompt(language) }] },
        "contents": [{ "role": "user", "parts": [{ "text": question }] }],
        "generationConfig": {
            "temperature": 0.4,
            "maxOutputTokens": config::AI_MAX_OUTPUT_TOKENS,
            // Reasoning tokens come out of the same budget, and for a one-sentence spoken
            // answer they are pure latency: without this the model spent 120-260 tokens
            // thinking and the answer itself got cut off mid-sentence.
            "thinkingConfig": { "thinkingBudget": 0 },
        }
    });

    info!("Asking {} ...", model);

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(config::AI_TIMEOUT_SECONDS))
        .build()
        .map_err(|e| AiError::Request(e.to_string()))?;

    let response = client
        .post(&url)
        .header("x-goog-api-key", key)
        .json(&body)
        .send()
        .map_err(|e| AiError::Request(e.to_string()))?;

    let status = response.status();
    let parsed: GeminiResponse = response
        .json()
        .map_err(|e| AiError::Request(format!("unreadable answer: {}", e)))?;

    if let Some(error) = parsed.error {
        return Err(AiError::Api {
            status: if error.code > 0 { error.code } else { status.as_u16() },
            message: error.message,
        });
    }

    if !status.is_success() {
        return Err(AiError::Api { status: status.as_u16(), message: String::new() });
    }

    if parsed.candidates.iter().any(|c| c.finish_reason == "MAX_TOKENS") {
        warn!("Gemini hit the token limit, the answer may be cut short");
    }

    let answer: String = parsed
        .candidates
        .into_iter()
        .filter_map(|c| c.content)
        .flat_map(|c| c.parts)
        .map(|p| p.text)
        .collect::<Vec<_>>()
        .join(" ");

    let answer = plain_text(&answer);

    if answer.is_empty() {
        return Err(AiError::Empty);
    }

    Ok(answer)
}

// Strip the markdown the model still emits now and then - asterisks and backticks are
// read out literally by speech synthesis.
fn plain_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '*' | '`' | '_' | '#' => continue,
            // collapse the whitespace left behind
            c if c.is_whitespace() => {
                if !out.ends_with(' ') && !out.is_empty() {
                    out.push(' ');
                }
                let _ = chars.peek();
            }
            c => out.push(c),
        }
    }

    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_matches_the_language() {
        assert!(system_prompt("ru").contains("русском"));
        assert!(system_prompt("en").contains("English"));
        assert!(system_prompt("ua").contains("українською"));
    }

    #[test]
    fn answer_is_extracted_from_the_response() {
        let json = r#"{"candidates":[{"content":{"parts":[{"text":" В Ташкенте сейчас солнечно. "}]}}]}"#;
        let parsed: GeminiResponse = serde_json::from_str(json).unwrap();
        let answer: String = parsed.candidates.into_iter()
            .filter_map(|c| c.content).flat_map(|c| c.parts).map(|p| p.text)
            .collect::<Vec<_>>().join(" ").trim().to_string();
        assert_eq!(answer, "В Ташкенте сейчас солнечно.");
    }

    #[test]
    fn markdown_is_stripped_for_speech() {
        assert_eq!(plain_text("Столица Франции — **Париж**."), "Столица Франции — Париж.");
        assert_eq!(plain_text("  `ls -la`  выводит   файлы \n"), "ls -la выводит файлы");
        assert_eq!(plain_text("## Заголовок"), "Заголовок");
    }

    #[test]
    fn api_errors_are_recognized() {
        let json = r#"{"error":{"code":429,"message":"Quota exceeded"}}"#;
        let parsed: GeminiResponse = serde_json::from_str(json).unwrap();
        let error = parsed.error.expect("error must parse");
        assert_eq!(error.code, 429);
        assert_eq!(error.message, "Quota exceeded");
    }
}
