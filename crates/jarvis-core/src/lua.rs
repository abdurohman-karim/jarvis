mod engine;
mod sandbox;
mod error;
mod api;

mod structs;
pub use structs::*;

pub use engine::LuaEngine;
pub use sandbox::SandboxLevel;
pub use error::LuaError;

use std::path::PathBuf;
use std::time::Duration;

#[cfg(test)]
mod tests;

// Execute a Lua command script
// Run a script with a hard timeout.
//
// The VM's instruction hook only fires between Lua instructions, so a script stuck in a
// blocking host call (jarvis.system.exec, jarvis.http.*) would never time out. The script
// therefore runs on its own thread and the caller waits at most `timeout` (plus a small
// grace period for the hook to fire); on expiry the thread is abandoned - it ends when the
// blocking call returns - and the command is reported as timed out.
pub fn execute(
    script_path: &PathBuf,
    context: CommandContext,
    sandbox: SandboxLevel,
    timeout: Duration,
) -> Result<CommandResult, LuaError> {
    let script_path = script_path.clone();
    let (tx, rx) = std::sync::mpsc::channel();

    let spawned = std::thread::Builder::new()
        .name(format!("lua:{}", context.command_id))
        .spawn(move || {
            let result = LuaEngine::new(sandbox)
                .and_then(|engine| engine.execute(&script_path, context, timeout));
            let _ = tx.send(result);
        });

    if let Err(e) = spawned {
        return Err(LuaError::InitError(format!("failed to spawn script thread: {}", e)));
    }

    let grace = Duration::from_millis(500);
    match rx.recv_timeout(timeout + grace) {
        Ok(result) => result,
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
            log::warn!("Lua script did not finish within {:?} (blocked in a host call?), abandoning it", timeout);
            Err(LuaError::Timeout)
        }
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
            Err(LuaError::RuntimeError("script thread terminated unexpectedly".into()))
        }
    }
}