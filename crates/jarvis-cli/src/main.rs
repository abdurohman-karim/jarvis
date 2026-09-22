// Command line client for Jarvis.
//
// This used to be a second, cut-down assistant: it loaded the command packs and the intent
// classifier into its own process and matched phrases there, answering from a different
// state than the assistant the user is actually talking to. It is now a thin client - it
// talks to the running assistant over IPC - plus a few offline commands that only read files.

use std::io::{self, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

use jarvis_core::{commands, config, db, ipc};
use tungstenite::{stream::MaybeTlsStream, Message, WebSocket};

const USAGE: &str = "\
Jarvis CLI

Talking to a running assistant:
  say <text>        send a command, as if it had been spoken
  status            connection and protocol check
  watch             print events until interrupted
  stop              shut the assistant down
  reload            re-read the command packs
  apply-settings    re-read and apply the settings
  mute / unmute     stop / resume listening

Offline (no running assistant needed):
  commands          list the loaded command packs
  phrases           list the phrases commands are matched against
  settings          dump the stored settings
  devices           list the audio input devices
  models            list the installed speech recognition models
";

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (command, rest) = match args.split_first() {
        Some((c, rest)) => (c.as_str(), rest),
        None => {
            print!("{}", USAGE);
            return Ok(());
        }
    };

    config::init_dirs()?;

    match command {
        "help" | "-h" | "--help" => print!("{}", USAGE),

        // ### talking to the assistant
        "say" => {
            let text = rest.join(" ");
            if text.is_empty() {
                return Err("usage: say <text>".into());
            }
            let (mut ws, _) = connect()?;
            send(&mut ws, serde_json::json!({ "action": "text_command", "text": text }))?;
            follow(&mut ws, Some(Duration::from_secs(15)), true)?;
        }
        "status" => status()?,
        "watch" => {
            let (mut ws, _) = connect()?;
            eprintln!("Listening for events, press Ctrl-C to stop.");
            follow(&mut ws, None, false)?;
        }
        "stop" => simple_action("stop")?,
        "reload" => simple_action("reload_commands")?,
        "apply-settings" => simple_action("apply_settings")?,
        "mute" => set_muted(true)?,
        "unmute" => set_muted(false)?,

        // ### offline
        "commands" => {
            for pack in commands::parse_commands()?.iter() {
                println!("{}", pack.path.display());
                for cmd in &pack.commands {
                    println!("  {} ({})", cmd.id, cmd.cmd_type);
                }
            }
        }
        "phrases" => {
            let language = current_language();
            for pack in commands::parse_commands()?.iter() {
                for cmd in &pack.commands {
                    println!("[{}]", cmd.id);
                    for phrase in cmd.get_phrases(&language).iter() {
                        println!("  {}", phrase);
                    }
                }
            }
        }
        "settings" => {
            for (key, value) in db::init().dump() {
                println!("{} = {}", key, value);
            }
        }
        "devices" => {
            println!(" -1  (system default)");
            for (index, name) in jarvis_core::recorder::get_audio_devices().iter().enumerate() {
                println!("{:>3}  {}", index, name);
            }
        }
        "models" => {
            for model in jarvis_core::vosk_models::scan_vosk_models() {
                println!("{:<40} {:<4} {}", model.name, model.language,
                    if model.bundled { "bundled" } else { "downloaded" });
            }
        }

        other => return Err(format!("unknown command '{}', try `jarvis-cli help`", other)),
    }

    Ok(())
}

fn current_language() -> String {
    db::init().lock().language.clone()
}

// ### IPC CLIENT

type Socket = WebSocket<MaybeTlsStream<TcpStream>>;

fn token() -> Result<String, String> {
    let dir = jarvis_core::APP_CONFIG_DIR.get().ok_or("config dir not initialized")?;
    std::fs::read_to_string(dir.join(config::IPC_TOKEN_FILE))
        .map(|t| t.trim().to_string())
        .map_err(|_| "the assistant does not seem to be running (no ipc.token)".to_string())
}

// Returns the socket and the protocol version the assistant reported.
fn connect() -> Result<(Socket, u32), String> {
    let token = token()?;
    let url = format!("ws://{}:{}", ipc::IPC_ADDR, ipc::IPC_PORT);

    let (mut ws, _) = tungstenite::connect(&url)
        .map_err(|e| format!("cannot reach the assistant at {}: {}", url, e))?;

    send(&mut ws, serde_json::json!({ "action": "auth", "token": token }))?;

    // the assistant answers a valid token with `started`
    match read_event(&mut ws, Duration::from_secs(5))? {
        Some(event) if event["event"] == "started" => {
            let protocol = event["protocol"].as_u64().unwrap_or(0) as u32;
            if protocol != ipc::PROTOCOL_VERSION {
                eprintln!("warning: assistant speaks protocol v{}, this CLI v{} - update both",
                    protocol, ipc::PROTOCOL_VERSION);
            }
            Ok((ws, protocol))
        }
        _ => Err("the assistant rejected the connection (stale token?)".into()),
    }
}

fn send(ws: &mut Socket, action: serde_json::Value) -> Result<(), String> {
    ws.send(Message::Text(action.to_string().into()))
        .map_err(|e| format!("failed to send: {}", e))
}

fn read_event(ws: &mut Socket, timeout: Duration) -> Result<Option<serde_json::Value>, String> {
    set_timeout(ws, Some(timeout))?;

    match ws.read() {
        Ok(Message::Text(text)) => Ok(serde_json::from_str(&text).ok()),
        Ok(_) => Ok(None),
        Err(tungstenite::Error::Io(e))
            if matches!(e.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut) => Ok(None),
        Err(e) => Err(format!("connection lost: {}", e)),
    }
}

fn set_timeout(ws: &mut Socket, timeout: Option<Duration>) -> Result<(), String> {
    match ws.get_ref() {
        MaybeTlsStream::Plain(stream) => stream.set_read_timeout(timeout).map_err(|e| e.to_string()),
        _ => Ok(()),
    }
}

// Print events as they arrive. With `until_idle`, stop once the assistant is done with what
// we just asked for; with a timeout, stop when it expires.
fn follow(ws: &mut Socket, timeout: Option<Duration>, until_idle: bool) -> Result<(), String> {
    let started = Instant::now();
    let mut saw_activity = false;

    loop {
        if let Some(timeout) = timeout {
            if started.elapsed() >= timeout {
                return Ok(());
            }
        }

        let Some(event) = read_event(ws, Duration::from_millis(500))? else {
            continue;
        };

        match event["event"].as_str().unwrap_or("?") {
            "pong" | "started" | "listening" | "wake_word_detected" => continue,
            "speech_recognized" => {
                println!("heard: {}", event["text"].as_str().unwrap_or(""));
                saw_activity = true;
            }
            "command_executed" => {
                let ok = event["success"].as_bool().unwrap_or(false);
                println!("{}: {}", if ok { "executed" } else { "failed" },
                    event["id"].as_str().unwrap_or("?"));
                saw_activity = true;
            }
            "error" => {
                println!("error: {}", event["message"].as_str().unwrap_or(""));
                saw_activity = true;
            }
            "ai_answer" => {
                println!("answer: {}", event["answer"].as_str().unwrap_or(""));
                saw_activity = true;
            }
            "settings_applied" => {
                println!("settings applied: {}", event["changed"]);
                saw_activity = true;
            }
            "commands_reloaded" => {
                println!("commands reloaded: {}", event["count"]);
                saw_activity = true;
            }
            "muted" => {
                println!("muted: {}", event["muted"]);
                saw_activity = true;
            }
            "idle" if until_idle && saw_activity => return Ok(()),
            "idle" => continue,
            "stopping" => {
                println!("assistant is shutting down");
                return Ok(());
            }
            other => println!("{}", other),
        }

        io::stdout().flush().ok();
    }
}

fn simple_action(action: &str) -> Result<(), String> {
    let (mut ws, _) = connect()?;
    send(&mut ws, serde_json::json!({ "action": action }))?;
    follow(&mut ws, Some(Duration::from_secs(10)), true)
}

fn set_muted(muted: bool) -> Result<(), String> {
    let (mut ws, _) = connect()?;
    send(&mut ws, serde_json::json!({ "action": "set_muted", "muted": muted }))?;
    follow(&mut ws, Some(Duration::from_secs(3)), true)
}

fn status() -> Result<(), String> {
    match connect() {
        Ok((mut ws, protocol)) => {
            send(&mut ws, serde_json::json!({ "action": "ping" }))?;
            match read_event(&mut ws, Duration::from_secs(3))? {
                Some(event) if event["event"] == "pong" => {
                    println!("assistant: running (protocol v{}, CLI v{})", protocol, ipc::PROTOCOL_VERSION);
                    Ok(())
                }
                _ => Err("assistant: connected but not answering".into()),
            }
        }
        Err(e) => {
            println!("assistant: not running ({})", e);
            std::process::exit(1);
        }
    }
}
