use jarvis_core::commands::{self, JCommand};

// Command packs are re-read on every call: they are a handful of small toml files and
// this way the GUI reflects a reload / newly added pack without a restart.
#[tauri::command(async)]
pub fn get_commands_count() -> usize {
    commands::parse_commands()
        .map(|list| list.iter().map(|l| l.commands.len()).sum())
        .unwrap_or(0)
}

#[tauri::command(async)]
pub fn get_commands_list() -> Vec<JCommand> {
    commands::parse_commands()
        .unwrap_or_default()
        .into_iter()
        .flat_map(|list| list.commands)
        .collect()
}
