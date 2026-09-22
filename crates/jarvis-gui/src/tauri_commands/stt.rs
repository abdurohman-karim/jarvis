use jarvis_core::{vosk_models, gliner_models, models};
use serde::Serialize;

#[derive(Serialize)]
pub struct VoskModel {
    pub name: String,
    pub language: String,
    pub size: String,
}

#[derive(Serialize)]
pub struct GlinerVariant {
    pub display_name: String,
    pub value: String,
}

#[tauri::command]
pub fn list_vosk_models() -> Vec<VoskModel> {
    vosk_models::scan_vosk_models()
        .into_iter()
        .map(|m| VoskModel {
            name: m.name,
            language: m.language,
            size: m.size,
        })
        .collect()
}

#[tauri::command]
pub fn list_gliner_models() -> Vec<GlinerVariant> {
    gliner_models::scan_gliner_variants()
        .into_iter()
        .map(|m| GlinerVariant {
            display_name: m.display_name,
            value: m.value,
        })
        .collect()
}

// Selectable backends for a task ("intent", "slots", "vad", "noise_suppression", "stt").
// The ids returned here are exactly what the matching setting key accepts.
#[tauri::command]
pub fn get_backend_options(task: models::Task) -> Vec<models::BackendOption> {
    models::get_options(task)
}
