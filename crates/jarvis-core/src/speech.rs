// Speaking arbitrary text with the operating system's speech synthesis.
//
// Voice packs are pre-recorded files and can only say what was recorded; answers from the
// language model are not known in advance, so they go through the OS instead.

use std::process::{Child, Command, Stdio};

use parking_lot::Mutex;

use crate::{audio, config, DB};

// the currently speaking process, so a new answer can interrupt the previous one
static SPEAKING: Mutex<Option<Child>> = Mutex::new(None);

pub fn is_available() -> bool {
    cfg!(any(target_os = "macos", target_os = "windows", target_os = "linux"))
}

fn enabled() -> bool {
    DB.get().map(|db| db.read().speak_ai_answers).unwrap_or(true)
}

// Read `text` out loud. Returns false when synthesis is unavailable or switched off.
pub fn say(text: &str, language: &str) -> bool {
    if !enabled() || text.trim().is_empty() {
        return false;
    }

    stop();

    let child = spawn(text, language);

    match child {
        Some(child) => {
            // the microphone must ignore our own voice, exactly like a voice pack reply;
            // the length is unknown up front, so use a generous estimate from the text
            audio::mark_output_busy_for(estimated_duration(text));
            *SPEAKING.lock() = Some(child);
            true
        }
        None => false,
    }
}

// Stop an answer that is still being read out (a new command interrupts it).
pub fn stop() {
    if let Some(mut child) = SPEAKING.lock().take() {
        let _ = child.kill();
        let _ = child.wait();
    }
}

fn estimated_duration(text: &str) -> std::time::Duration {
    let words = text.split_whitespace().count().max(1) as u64;
    let millis = (words * 1000 * 60) / config::TTS_WORDS_PER_MINUTE;
    std::time::Duration::from_millis(millis.clamp(1000, 60_000))
}

#[cfg(target_os = "macos")]
fn spawn(text: &str, language: &str) -> Option<Child> {
    let mut command = Command::new("say");

    // `say` picks the voice by name; the system default may not speak our language
    if let Some(voice) = macos_voice(language) {
        command.arg("-v").arg(voice);
    }

    command
        .arg("--")
        .arg(text)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| warn!("Failed to run `say`: {}", e))
        .ok()
}

// Voices shipped with macOS for the languages the assistant supports
#[cfg(target_os = "macos")]
fn macos_voice(language: &str) -> Option<&'static str> {
    match language {
        "ru" => Some("Milena"),
        "ua" => Some("Lesya"),
        "en" => Some("Samantha"),
        _ => None,
    }
}

#[cfg(target_os = "windows")]
fn spawn(text: &str, _language: &str) -> Option<Child> {
    // PowerShell ships with SAPI; the text is passed through an environment variable so
    // that quotes and newlines in it cannot break out of the script
    Command::new("powershell")
        .args(["-NoProfile", "-Command",
            "Add-Type -AssemblyName System.Speech; \
             (New-Object System.Speech.Synthesis.SpeechSynthesizer).Speak($env:JARVIS_TTS_TEXT)"])
        .env("JARVIS_TTS_TEXT", text)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| warn!("Failed to run PowerShell speech synthesis: {}", e))
        .ok()
}

#[cfg(all(unix, not(target_os = "macos")))]
fn spawn(text: &str, _language: &str) -> Option<Child> {
    Command::new("spd-say")
        .arg("--wait")
        .arg("--")
        .arg(text)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| warn!("Failed to run `spd-say` (is speech-dispatcher installed?): {}", e))
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_grows_with_the_text_and_stays_sane() {
        let short = estimated_duration("Да.");
        let long = estimated_duration(&"слово ".repeat(200));
        assert!(short >= std::time::Duration::from_secs(1), "at least a second: {:?}", short);
        assert!(long > short);
        assert!(long <= std::time::Duration::from_secs(60), "capped: {:?}", long);
    }
}
