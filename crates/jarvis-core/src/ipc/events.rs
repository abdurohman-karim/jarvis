use serde::{Deserialize, Serialize};

// Events sent from jarvis-app to GUI
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
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
    
    // App started
    Started,
    
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
}

// Actions sent from GUI to jarvis-app
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
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