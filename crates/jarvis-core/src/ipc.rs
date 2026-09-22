mod events;
mod server;

pub use events::{IpcAction, IpcEvent};
pub use server::{init, send, set_action_handler, bind, start_server, start_server_on, has_clients, IPC_ADDR, IPC_PORT};