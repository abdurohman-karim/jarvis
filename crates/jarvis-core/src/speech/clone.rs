// Speaking in the voice pack's own voice.
//
// Cloning runs in a Python process (PyTorch, ~4 GB) that is deliberately not part of the
// app bundle - see scripts/voice-clone/. The process is started on first use and kept
// alive, because loading the model takes seconds and doing that per phrase would be
// unusable. It talks one JSON request per line over stdin/stdout.

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::time::Duration;

use parking_lot::Mutex;

use crate::{voices, APP_DIR};

struct Server {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<std::process::ChildStdout>,
    // the voice the loaded reference belongs to; changing voices restarts the process
    voice_id: String,
    language: String,
}

static SERVER: Mutex<Option<Server>> = Mutex::new(None);

// Where the optional component lives, next to the executable or in the repository.
fn component_dir() -> Option<PathBuf> {
    let candidates = [
        APP_DIR.join("scripts").join("voice-clone"),
        APP_DIR.join("../../scripts/voice-clone"),
        PathBuf::from("scripts/voice-clone"),
    ];

    candidates.into_iter().find(|p| p.join("synth.py").is_file())
}

// Load the model ahead of the first phrase: it takes seconds, and a command that speaks
// would otherwise hit its own timeout waiting for it.
pub fn warm_up(language: &str) {
    let language = language.to_string();
    std::thread::Builder::new()
        .name("voice-clone-warmup".into())
        .spawn(move || {
            let mut guard = SERVER.lock();
            if guard.is_none() {
                match start(&language) {
                    Ok(server) => *guard = Some(server),
                    Err(e) => warn!("Voice cloning could not be started: {}", e),
                }
            }
        })
        .ok();
}

pub fn is_installed() -> bool {
    component_dir()
        .map(|dir| dir.join("venv/bin/python").is_file() && dir.join("model/model.safetensors").exists())
        .unwrap_or(false)
}

// Read lines until one parses as our JSON protocol and matches `accept`.
fn read_reply(
    stdout: &mut BufReader<std::process::ChildStdout>,
    accept: impl Fn(&serde_json::Value) -> bool,
) -> Result<serde_json::Value, String> {
    for _ in 0..200 {
        let mut line = String::new();
        let read = stdout.read_line(&mut line).map_err(|e| e.to_string())?;
        if read == 0 {
            return Err("the process exited".into());
        }

        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        match serde_json::from_str::<serde_json::Value>(line) {
            Ok(value) if accept(&value) => return Ok(value),
            Ok(_) => continue,
            // progress output from the library, not ours
            Err(_) => debug!("voice cloning: {}", line),
        }
    }

    Err("no protocol answer".into())
}

fn start(language: &str) -> Result<Server, String> {
    let dir = component_dir().ok_or("voice cloning component not found")?;
    let python = dir.join("venv/bin/python");

    if !python.is_file() {
        return Err("voice cloning is not installed (run scripts/voice-clone/install.sh)".into());
    }

    let voice = voices::get_current_voice().ok_or("no voice selected")?;
    let reference = voice.clone_reference(language)
        .ok_or_else(|| format!("voice '{}' has no reference clip for '{}'", voice.voice.id, language))?;

    let reference_path = voice.path.join(&reference.reference);
    if !reference_path.is_file() {
        return Err(format!("reference clip {} is missing", reference_path.display()));
    }

    info!("Starting the voice cloning process for '{}' ...", voice.voice.id);

    let mut child = Command::new(&python)
        .arg(dir.join("synth.py"))
        .arg("--serve")
        .arg("--reference").arg(&reference_path)
        .arg("--reference-text").arg(&reference.text)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("cannot start {}: {}", python.display(), e))?;

    let stdin = child.stdin.take().ok_or("no stdin")?;
    let mut stdout = BufReader::new(child.stdout.take().ok_or("no stdout")?);

    // The process answers once the model is loaded. Loading can print progress of its own
    // (model downloads, warnings), so skip anything that is not our protocol.
    let ready = read_reply(&mut stdout, |value| value.get("ready").is_some());
    if let Err(e) = ready {
        let _ = child.kill();
        return Err(format!("the cloning process did not start: {}", e));
    }

    info!("Voice cloning ready.");
    Ok(Server { child, stdin, stdout, voice_id: voice.voice.id.clone(), language: language.to_string() })
}

// Synthesize `text` into a wav file. Blocking: expect seconds, not milliseconds.
pub fn synthesize(text: &str, language: &str) -> Result<PathBuf, String> {
    let mut guard = SERVER.lock();

    // the voice or language changed: the reference no longer matches
    if let Some(server) = guard.as_ref() {
        let stale = voices::get_current_voice().map(|v| v.voice.id != server.voice_id).unwrap_or(true)
            || server.language != language;
        if stale {
            info!("Voice changed, restarting the cloning process.");
            if let Some(mut server) = guard.take() {
                let _ = server.child.kill();
            }
        }
    }

    if guard.is_none() {
        *guard = Some(start(language)?);
    }

    let server = guard.as_mut().ok_or("cloning process is not running")?;

    let out = std::env::temp_dir().join(format!("jarvis-tts-{}.wav", std::process::id()));
    let request = serde_json::json!({ "text": text, "out": out.to_string_lossy() });

    writeln!(server.stdin, "{}", request).map_err(|e| format!("cannot send the request: {}", e))?;
    server.stdin.flush().map_err(|e| format!("cannot send the request: {}", e))?;

    let answer = read_reply(&mut server.stdout, |value| {
        value.get("out").is_some() || value.get("error").is_some()
    })?;

    if let Some(error) = answer.get("error").and_then(|e| e.as_str()) {
        return Err(error.to_string());
    }

    debug!("Cloned speech in {:.1}s", answer.get("took").and_then(|t| t.as_f64()).unwrap_or(0.0));
    Ok(out)
}

// How long the produced audio is, so the microphone can ignore it while it plays.
pub fn duration_of(path: &PathBuf) -> Option<Duration> {
    let reader = hound::WavReader::open(path).ok()?;
    let spec = reader.spec();
    let seconds = reader.duration() as f64 / spec.sample_rate as f64;
    Some(Duration::from_secs_f64(seconds))
}

// Stop the process (settings changed, or the assistant is shutting down)
pub fn shutdown() {
    if let Some(mut server) = SERVER.lock().take() {
        let _ = server.child.kill();
        let _ = server.child.wait();
        info!("Voice cloning process stopped.");
    }
}
