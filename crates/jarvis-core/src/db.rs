pub mod structs;
pub mod manager;

use crate::{config, APP_CONFIG_DIR};

use log::info;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

pub use manager::SettingsManager;

fn get_db_file_path() -> PathBuf {
    PathBuf::from(format!(
        "{}/{}",
        APP_CONFIG_DIR.get().unwrap().display(),
        config::DB_FILE_NAME
    ))
}

// Read the settings file, or None if it is missing / unreadable / invalid.
pub fn load_settings_file() -> Option<structs::Settings> {
    let path = get_db_file_path();
    let file = File::open(&path).ok()?;
    match serde_json::from_reader(BufReader::new(file)) {
        Ok(settings) => Some(settings),
        Err(e) => {
            warn!("Failed to parse {}: {}", path.display(), e);
            None
        }
    }
}

pub fn init_settings() -> structs::Settings {
    let db_file_path = get_db_file_path();

    info!(
        "Loading settings db file located at: {}",
        db_file_path.display()
    );

    if db_file_path.exists() {
        if let Ok(db_file) = File::open(&db_file_path) {
            let reader = BufReader::new(db_file);
            if let Ok(settings) = serde_json::from_reader(reader) {
                info!("Settings loaded.");
                return settings;
            }
        }
    }

    warn!("No settings file found or there was an error parsing it. Creating default struct.");
    structs::Settings::default()
}

/// init settings and return a SettingsManager ready to use
pub fn init() -> SettingsManager {
    let settings = init_settings();
    SettingsManager::new(settings)
}

pub fn save_settings(settings: &structs::Settings) -> Result<(), std::io::Error> {
    let db_file_path = get_db_file_path();

    std::fs::write(
        &db_file_path,
        serde_json::to_string_pretty(&settings).unwrap(),
    )?;

    // the file holds API keys: keep it readable by this user only
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&db_file_path, std::fs::Permissions::from_mode(0o600))?;
    }

    info!("Settings saved to: {:#}", db_file_path.display());
    Ok(())
}
