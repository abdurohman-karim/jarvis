// IPC between the assistant and its clients (GUI, CLI).
//
// The message types are always available; the server half needs the assistant's features.
pub mod events;

#[cfg(feature = "jarvis_app")]
mod server;

pub use events::{IpcAction, IpcEvent, PROTOCOL_VERSION};

#[cfg(feature = "jarvis_app")]
pub use server::{init, init_token, read_token, send, set_action_handler, bind, start_server,
    start_server_on, has_clients, IPC_ADDR, IPC_PORT, IPC_TOKEN_FILE};

// address / port are useful to clients too
#[cfg(not(feature = "jarvis_app"))]
pub const IPC_PORT: u16 = 9712;
#[cfg(not(feature = "jarvis_app"))]
pub const IPC_ADDR: &str = "127.0.0.1";
#[cfg(not(feature = "jarvis_app"))]
pub use crate::config::IPC_TOKEN_FILE;
