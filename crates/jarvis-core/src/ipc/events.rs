use serde::{Deserialize, Serialize};
use ts_rs::TS;

// Bumped on every incompatible change to the messages below. The assistant reports it in
// `Started`, so a client talking to an older/newer assistant can say so instead of
// silently misbehaving.
pub const PROTOCOL_VERSION: u32 = 1;

// Events sent from jarvis-app to GUI.
//
// TypeScript definitions are generated from these types into
// frontend/src/lib/ipc-types.ts (see the export test below) - the frontend used to
// re-declare the message shapes by hand and could drift from the Rust side unnoticed.
#[derive(Clone, Debug, Serialize, TS)]
#[serde(tag = "event", rename_all = "snake_case")]
#[ts(export, export_to = "../../../frontend/src/lib/ipc-types.ts")]
pub enum IpcEvent {
    // Wake word detected, starting to listen
    WakeWordDetected,
    
    // Actively listening for command
    Listening,
    
    // Speech recognized
    SpeechRecognized { text: String },
    
    // Command was executed
    CommandExecuted { id: String, success: bool },
    
    // Returned to idle state
    Idle,
    
    // Error occurred
    Error { message: String },
    
    // App started; carries the protocol version the assistant speaks
    Started { protocol: u32 },
    
    // App is shutting down
    Stopping,
    
    // Pong response
    Pong,

    // request GUI to reveal/focus window
    RevealWindow,

    // microphone muted / unmuted
    Muted { muted: bool },

    // command packs were re-read from disk
    CommandsReloaded { count: usize },

    // settings were re-read and applied to the running pipeline
    SettingsApplied { changed: Vec<String> },

    // a question was answered by the language model
    AiAnswer { question: String, answer: String },
}

// Actions sent from GUI to jarvis-app
#[derive(Clone, Debug, Deserialize, TS)]
#[serde(tag = "action", rename_all = "snake_case")]
#[ts(export, export_to = "../../../frontend/src/lib/ipc-types.ts")]
pub enum IpcAction {
    // First message of every connection: the token from ipc.token in the config dir
    Auth { token: String },

    // Request graceful shutdown
    Stop,
    
    // Reload commands from disk
    ReloadCommands,

    // Re-read settings and apply them without a restart
    ApplySettings,
    
    // Ping to check connection
    Ping,
    
    // Mute/unmute listening
    SetMuted { muted: bool },

    // Execute text command
    TextCommand { text: String },
}
#[cfg(test)]
mod tests {
    use super::*;

    // Serde and ts-rs must agree on the wire format: the tag field and the snake_case
    // variant names are what the frontend switches on.
    #[test]
    fn events_serialize_with_a_tag() {
        let json = serde_json::to_value(IpcEvent::SpeechRecognized { text: "привет".into() }).unwrap();
        assert_eq!(json["event"], "speech_recognized");
        assert_eq!(json["text"], "привет");

        let json = serde_json::to_value(IpcEvent::Started { protocol: PROTOCOL_VERSION }).unwrap();
        assert_eq!(json["event"], "started");
        assert_eq!(json["protocol"], PROTOCOL_VERSION);

        let json = serde_json::to_value(IpcEvent::Idle).unwrap();
        assert_eq!(json["event"], "idle");
    }

    #[test]
    fn actions_parse_from_the_wire_format() {
        let action: IpcAction = serde_json::from_str(r#"{"action":"text_command","text":"погода"}"#).unwrap();
        assert!(matches!(action, IpcAction::TextCommand { text } if text == "погода"));

        let action: IpcAction = serde_json::from_str(r#"{"action":"set_muted","muted":true}"#).unwrap();
        assert!(matches!(action, IpcAction::SetMuted { muted: true }));

        assert!(serde_json::from_str::<IpcAction>(r#"{"action":"nope"}"#).is_err());
    }
}
