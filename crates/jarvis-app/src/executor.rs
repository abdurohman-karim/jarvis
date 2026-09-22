// Command executor: runs recognized commands (intent classification, Lua / CLI / exe,
// reaction sounds, IPC events) on its own thread so the audio pipeline never blocks.

use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;

use jarvis_core::{commands, config, i18n, intent, slots, voices, COMMANDS_LIST, ipc::{self, IpcEvent}};

pub struct ExecRequest {
    text: String,
    // voice commands report back whether the assistant should keep listening (chaining)
    done: Option<Sender<bool>>,
}

#[derive(Clone)]
pub struct ExecutorHandle {
    tx: Sender<ExecRequest>,
}

impl ExecutorHandle {
    // Voice command: the returned receiver yields the command's chain flag once it finished.
    pub fn submit_voice(&self, text: String) -> Receiver<bool> {
        let (done_tx, done_rx) = mpsc::channel();
        let _ = self.tx.send(ExecRequest { text, done: Some(done_tx) });
        done_rx
    }

    // Text command (typed in the GUI): never chains, result is not awaited.
    pub fn submit_text(&self, text: String) {
        info!("Processing text command: {}", text);
        ipc::send(IpcEvent::SpeechRecognized { text: text.clone() });

        let filtered = strip_assistant_phrases(&text);
        if filtered.is_empty() {
            ipc::send(IpcEvent::Idle);
            return;
        }

        let _ = self.tx.send(ExecRequest { text: filtered, done: None });
    }
}

pub fn spawn(rt: Arc<tokio::runtime::Runtime>) -> ExecutorHandle {
    let (tx, rx) = mpsc::channel::<ExecRequest>();

    std::thread::Builder::new()
        .name("command-executor".into())
        .spawn(move || {
            for req in rx {
                let chain = execute(&req.text, &rt);
                if let Some(done) = req.done {
                    let _ = done.send(chain);
                }
            }
        })
        .expect("failed to spawn command executor thread");

    ExecutorHandle { tx }
}

// lowercases and removes activation words ("jarvis", ...) from a phrase
pub fn strip_assistant_phrases(text: &str) -> String {
    let mut filtered = text.to_lowercase();
    for tbr in config::get_phrases_to_remove(&i18n::get_language()) {
        filtered = filtered.replace(tbr, "");
    }
    filtered.trim().to_string()
}

// Execute a command, returns true if chaining should continue
fn execute(text: &str, rt: &tokio::runtime::Runtime) -> bool {
    let commands_list = match COMMANDS_LIST.get() {
        Some(c) => c,
        None => {
            ipc::send(IpcEvent::Error { message: "Commands not loaded".to_string() });
            ipc::send(IpcEvent::Idle);
            return false;
        }
    };

    let cmd_result = if let Some((intent_id, confidence)) = rt.block_on(intent::classify(text)) {
        info!("Intent recognized: {} (confidence: {:.2})", intent_id, confidence);
        intent::get_command_by_intent(commands_list, &intent_id)
    } else {
        info!("Intent not recognized, trying levenshtein fallback...");
        commands::fetch_command(text, commands_list)
    };

    let Some((cmd_path, cmd_config)) = cmd_result else {
        info!("No command found for: {}", text);
        voices::play_not_found();
        ipc::send(IpcEvent::Error { message: format!("Command not found: {}", text) });
        ipc::send(IpcEvent::Idle);
        return false;
    };

    info!("Command found: {:?}", cmd_path);

    // extract slots if the command declares any
    let extracted_slots = if !cmd_config.slots.is_empty() {
        let s = slots::extract(text, &cmd_config.slots);
        if !s.is_empty() {
            info!("Extracted slots: {:?}", s);
        }
        Some(s)
    } else {
        None
    };

    let chain = match commands::execute_command(cmd_path, cmd_config, Some(text), extracted_slots.as_ref()) {
        Ok(chain) => {
            info!("Command executed successfully");
            voices::play_random_from(cmd_config.get_sounds(&i18n::get_language()).as_slice());
            ipc::send(IpcEvent::CommandExecuted { id: cmd_config.id.clone(), success: true });
            chain
        }
        Err(msg) => {
            error!("Error executing command: {}", msg);
            voices::play_error();
            ipc::send(IpcEvent::CommandExecuted { id: cmd_config.id.clone(), success: false });
            ipc::send(IpcEvent::Error { message: msg.to_string() });
            false
        }
    };

    ipc::send(IpcEvent::Idle);
    chain
}
