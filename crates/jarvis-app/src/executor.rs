// Command executor: runs recognized commands (intent classification, Lua / CLI / exe,
// reaction sounds, IPC events) on its own thread so the audio pipeline never blocks.

use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;

use jarvis_core::{ai, commands, config, i18n, intent, slots, speech, voices, ipc::{self, IpcEvent}, DB};

pub enum ExecRequest {
    Command {
        text: String,
        // voice commands report back whether the assistant should keep listening (chaining)
        done: Option<Sender<bool>>,
    },
    // re-read command packs from disk and retrain the intent classifier
    ReloadCommands,
}

#[derive(Clone)]
pub struct ExecutorHandle {
    tx: Sender<ExecRequest>,
}

impl ExecutorHandle {
    // Voice command: the returned receiver yields the command's chain flag once it finished.
    pub fn submit_voice(&self, text: String) -> Receiver<bool> {
        let (done_tx, done_rx) = mpsc::channel();
        let _ = self.tx.send(ExecRequest::Command { text, done: Some(done_tx) });
        done_rx
    }

    pub fn submit_reload(&self) {
        let _ = self.tx.send(ExecRequest::ReloadCommands);
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

        let _ = self.tx.send(ExecRequest::Command { text: filtered, done: None });
    }
}

pub fn spawn(rt: Arc<tokio::runtime::Runtime>) -> ExecutorHandle {
    let (tx, rx) = mpsc::channel::<ExecRequest>();

    std::thread::Builder::new()
        .name("command-executor".into())
        .spawn(move || {
            for req in rx {
                match req {
                    ExecRequest::Command { text, done } => {
                        let chain = execute(&text, &rt);
                        if let Some(done) = done {
                            let _ = done.send(chain);
                        }
                    }
                    ExecRequest::ReloadCommands => reload_commands(&rt),
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

// Pre-generated in the voice packs, so this is spoken in the assistant's own voice
fn ai_unavailable_message(language: &str) -> String {
    match language {
        "ru" => "Нейросеть не отвечает",
        "ua" => "Нейромережа не відповідає",
        _ => "The model is not answering",
    }
    .to_string()
}

fn ai_fallback_enabled() -> bool {
    DB.get().map(|db| db.read().ai_fallback).unwrap_or(false) && ai::is_configured()
}

// Ask the language model and read the answer out loud. Returns false if it could not answer,
// so the caller can fall back to the usual "command not found" reaction.
fn answer_with_ai(question: &str) -> bool {
    let language = i18n::get_language();

    match ai::ask(question, &language) {
        Ok(answer) => {
            info!("AI answer: {}", answer);
            ipc::send(IpcEvent::AiAnswer { question: question.to_string(), answer: answer.clone() });

            if !speech::say(&answer, &language) {
                // synthesis unavailable or switched off: at least acknowledge out loud
                voices::play_ok();
            }

            ipc::send(IpcEvent::Idle);
            true
        }
        Err(e) => {
            warn!("AI could not answer: {}", e);

            // "command not found" would be misleading: the command was understood, the
            // model just could not answer (no quota, no network, bad key)
            ipc::send(IpcEvent::Error { message: format!("AI: {}", e) });
            speech::say(&ai_unavailable_message(&language), &language);
            ipc::send(IpcEvent::Idle);
            true
        }
    }
}

fn reload_commands(rt: &tokio::runtime::Runtime) {
    info!("Reloading commands...");
    match commands::reload() {
        Ok(list) => {
            if let Err(e) = rt.block_on(intent::retrain(&list)) {
                error!("Failed to retrain intent classifier: {}", e);
                ipc::send(IpcEvent::Error { message: format!("Intent classifier: {}", e) });
            }
            let count = list.iter().map(|l| l.commands.len()).sum();
            ipc::send(IpcEvent::CommandsReloaded { count });
        }
        Err(e) => {
            error!("Failed to reload commands: {}", e);
            ipc::send(IpcEvent::Error { message: format!("Reload failed: {}", e) });
        }
    }
}

// Execute a command, returns true if chaining should continue
fn execute(text: &str, rt: &tokio::runtime::Runtime) -> bool {
    let commands_list = commands::list();
    let commands_list: &[commands::JCommandsList] = &commands_list;

    let cmd_result = if let Some((intent_id, confidence)) = rt.block_on(intent::classify(text)) {
        info!("Intent recognized: {} (confidence: {:.2})", intent_id, confidence);
        intent::get_command_by_intent(commands_list, &intent_id)
    } else {
        info!("Intent not recognized, trying levenshtein fallback...");
        commands::fetch_command(text, commands_list)
    };

    let Some((cmd_path, cmd_config)) = cmd_result else {
        info!("No command found for: {}", text);

        // nothing matched: let the language model answer, if it is configured
        if ai_fallback_enabled() && answer_with_ai(text) {
            return false;
        }

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
