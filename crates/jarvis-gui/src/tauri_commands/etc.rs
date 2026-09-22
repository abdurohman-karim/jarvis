use jarvis_core::{config, APP_LOG_DIR};

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command

#[tauri::command]
pub fn get_app_version() -> String {
    if let Some(res) = config::APP_VERSION {
        res.to_string()
    } else {
        String::from("error")
    }
}

#[tauri::command]
pub fn get_log_file_path() -> String {
    APP_LOG_DIR.get()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}
// IPC token of the running assistant (same user, same config dir)
#[tauri::command]
pub fn get_ipc_token() -> Result<String, String> {
    let dir = jarvis_core::APP_CONFIG_DIR.get().ok_or("config dir not initialized")?;
    std::fs::read_to_string(dir.join(config::IPC_TOKEN_FILE))
        .map(|t| t.trim().to_string())
        .map_err(|e| e.to_string())
}
