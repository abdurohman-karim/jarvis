use crate::AppState;

#[tauri::command]
pub fn db_read(state: tauri::State<'_, AppState>, key: &str) -> String {
    state.settings.read(key).unwrap_or_default()
}

// write several settings with a single save; fails as a whole on any invalid entry
#[tauri::command]
pub fn db_write_many(state: tauri::State<'_, AppState>, entries: Vec<(String, String)>) -> Result<(), String> {
    let pairs: Vec<(&str, &str)> = entries.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    state.settings.write_many(&pairs).map_err(|e| {
        log::warn!("db_write_many: {}", e);
        e
    })
}

#[tauri::command]
pub fn db_write(state: tauri::State<'_, AppState>, key: &str, val: &str) -> bool {
    match state.settings.write(key, val) {
        Ok(()) => true,
        Err(e) => {
            log::warn!("db_write('{}', '{}'): {}", key, val, e);
            false
        }
    }
}
