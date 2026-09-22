use jarvis_core::commands::{self, JCommand};
use serde::Serialize;

// A command together with the pack it came from, which the UI groups by.
#[derive(Serialize)]
pub struct CommandEntry {
    #[serde(flatten)]
    command: JCommand,
    pack: String,
}

// Command packs are re-read on every call: they are a handful of small toml files and
// this way the GUI reflects a reload / newly added pack without a restart.
#[tauri::command(async)]
pub fn get_commands_count() -> usize {
    commands::parse_commands()
        .map(|list| list.iter().map(|l| l.commands.len()).sum())
        .unwrap_or(0)
}

#[tauri::command(async)]
pub fn get_commands_list() -> Vec<CommandEntry> {
    commands::parse_commands()
        .unwrap_or_default()
        .into_iter()
        .flat_map(|list| {
            let pack = list.path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            list.commands.into_iter().map(move |command| CommandEntry { command, pack: pack.clone() })
        })
        .collect()
}
