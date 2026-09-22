// On-demand Vosk model downloads.
//
// Models are streamed into the user model directory as <name>.zip.part while
// `vosk-download-progress` events report progress, then extracted and the archive removed.

use std::io::Write;
use std::path::{Path, PathBuf};

use futures_util::StreamExt;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use jarvis_core::vosk_models::{self, VoskCatalogEntry, VoskModelInfo};

#[derive(Serialize, Clone)]
pub struct CatalogModel {
    pub name: String,
    pub language: String,
    pub description: String,
    pub size_mb: u32,
    pub installed: bool,
    pub bundled: bool,
    pub recommended: bool,
}

#[derive(Serialize, Clone, Debug)]
pub struct DownloadProgress {
    pub name: String,
    pub downloaded: u64,
    pub total: Option<u64>,
    // "downloading" | "extracting" | "done" | "error"
    pub stage: String,
    pub message: Option<String>,
}

// downloadable models with their installation state
#[tauri::command(async)]
pub fn vosk_model_catalog() -> Vec<CatalogModel> {
    let installed: Vec<VoskModelInfo> = vosk_models::scan_vosk_models();

    vosk_models::CATALOG
        .iter()
        .map(|m: &VoskCatalogEntry| {
            let found = installed.iter().find(|i| i.name == m.name);
            CatalogModel {
                name: m.name.to_string(),
                language: m.language.to_string(),
                description: m.description.to_string(),
                size_mb: m.size_mb,
                installed: found.is_some(),
                bundled: found.map(|i| i.bundled).unwrap_or(false),
                recommended: m.recommended,
            }
        })
        .collect()
}

#[tauri::command]
pub async fn download_vosk_model(app: AppHandle, name: String) -> Result<(), String> {
    let entry = vosk_models::catalog_entry(&name)
        .ok_or_else(|| format!("unknown model '{}'", name))?;

    let emitter = app.clone();
    let result = download_and_extract(entry, &vosk_models::user_models_dir(), move |p| {
        let _ = emitter.emit("vosk-download-progress", p);
    }).await;

    let progress = match &result {
        Ok(()) => DownloadProgress { name: name.clone(), downloaded: 0, total: None, stage: "done".into(), message: None },
        Err(e) => DownloadProgress { name: name.clone(), downloaded: 0, total: None, stage: "error".into(), message: Some(e.clone()) },
    };
    let _ = app.emit("vosk-download-progress", progress);

    result
}

pub async fn download_and_extract(
    entry: &VoskCatalogEntry,
    dir: &Path,
    report: impl Fn(DownloadProgress),
) -> Result<(), String> {
    let dir = dir.to_path_buf();
    tokio::fs::create_dir_all(&dir).await.map_err(|e| format!("cannot create {}: {}", dir.display(), e))?;

    let zip_path = dir.join(format!("{}.zip.part", entry.name));

    // ### download
    log::info!("Downloading Vosk model {} from {}", entry.name, entry.url);
    let response = reqwest::get(entry.url).await
        .map_err(|e| format!("download failed: {}", e))?
        .error_for_status()
        .map_err(|e| format!("download failed: {}", e))?;

    let total = response.content_length();
    let mut stream = response.bytes_stream();
    let mut file = tokio::fs::File::create(&zip_path).await
        .map_err(|e| format!("cannot create {}: {}", zip_path.display(), e))?;

    let mut downloaded: u64 = 0;
    let mut last_report = std::time::Instant::now();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("download interrupted: {}", e))?;
        tokio::io::AsyncWriteExt::write_all(&mut file, &chunk).await
            .map_err(|e| format!("write failed: {}", e))?;
        downloaded += chunk.len() as u64;

        if last_report.elapsed().as_millis() >= 200 {
            last_report = std::time::Instant::now();
            report(DownloadProgress {
                name: entry.name.to_string(), downloaded, total, stage: "downloading".into(), message: None,
            });
        }
    }
    drop(file);

    // ### extract (blocking, on a worker thread)
    report(DownloadProgress {
        name: entry.name.to_string(), downloaded, total, stage: "extracting".into(), message: None,
    });

    let name = entry.name.to_string();
    let extract_dir = dir.clone();
    let zip_for_extract = zip_path.clone();
    let extracted = tokio::task::spawn_blocking(move || extract_model(&zip_for_extract, &extract_dir, &name))
        .await
        .map_err(|e| format!("extraction task failed: {}", e))?;

    let _ = tokio::fs::remove_file(&zip_path).await;
    extracted?;

    log::info!("Vosk model {} installed into {}", entry.name, dir.display());
    Ok(())
}

// Extracts the archive into `dir`. Vosk archives contain a single top-level folder named
// after the model; anything else (or path traversal) is rejected.
fn extract_model(zip_path: &Path, dir: &Path, name: &str) -> Result<(), String> {
    let file = std::fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("bad archive: {}", e))?;

    let target = dir.join(name);
    if target.exists() {
        std::fs::remove_dir_all(&target).map_err(|e| e.to_string())?;
    }

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let Some(rel) = entry.enclosed_name().map(PathBuf::from) else {
            return Err(format!("archive entry with unsafe path: {}", entry.name()));
        };
        if !rel.starts_with(name) {
            return Err(format!("unexpected entry outside model folder: {}", rel.display()));
        }

        let out = dir.join(&rel);
        if entry.is_dir() {
            std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
            continue;
        }
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut out_file = std::fs::File::create(&out).map_err(|e| format!("{}: {}", out.display(), e))?;
        std::io::copy(&mut entry, &mut out_file).map_err(|e| e.to_string())?;
        out_file.flush().map_err(|e| e.to_string())?;
    }

    if !vosk_models::is_vosk_model(&target) {
        let _ = std::fs::remove_dir_all(&target);
        return Err("archive does not look like a Vosk model".into());
    }

    Ok(())
}

#[tauri::command(async)]
pub fn delete_vosk_model(name: String) -> Result<(), String> {
    vosk_models::delete_user_model(&name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    fn write_zip(path: &Path, entries: &[(&str, &[u8])]) {
        let file = std::fs::File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let opts = zip::write::SimpleFileOptions::default();
        for (name, data) in entries {
            if name.ends_with('/') {
                zip.add_directory(name.trim_end_matches('/'), opts).unwrap();
            } else {
                zip.start_file(*name, opts).unwrap();
                zip.write_all(data).unwrap();
            }
        }
        zip.finish().unwrap();
    }

    #[test]
    fn extracts_a_model_archive() {
        let tmp = tempfile::tempdir().unwrap();
        let zip_path = tmp.path().join("m.zip");
        write_zip(&zip_path, &[
            ("vosk-model-test/", b""),
            ("vosk-model-test/am/", b""),
            ("vosk-model-test/am/final.mdl", b"model"),
            ("vosk-model-test/conf/mfcc.conf", b"conf"),
        ]);

        extract_model(&zip_path, tmp.path(), "vosk-model-test").unwrap();
        assert_eq!(std::fs::read(tmp.path().join("vosk-model-test/am/final.mdl")).unwrap(), b"model");
        assert!(vosk_models::is_vosk_model(&tmp.path().join("vosk-model-test")));
    }

    #[test]
    fn rejects_entries_outside_the_model_folder() {
        let tmp = tempfile::tempdir().unwrap();
        let zip_path = tmp.path().join("m.zip");
        write_zip(&zip_path, &[("other/am/final.mdl", b"x")]);
        assert!(extract_model(&zip_path, tmp.path(), "vosk-model-test").is_err());
        assert!(!tmp.path().join("other").exists());
    }

    #[test]
    fn rejects_archives_that_are_not_models() {
        let tmp = tempfile::tempdir().unwrap();
        let zip_path = tmp.path().join("m.zip");
        write_zip(&zip_path, &[("vosk-model-test/readme.txt", b"x")]);
        let err = extract_model(&zip_path, tmp.path(), "vosk-model-test").unwrap_err();
        assert!(err.contains("does not look like"), "{}", err);
        assert!(!tmp.path().join("vosk-model-test").exists(), "partial extraction must be cleaned up");
    }

    // real download of the smallest catalog model (~40 MB); run manually:
    //   cargo test -p jarvis-gui --no-default-features -- --ignored downloads_real_model
    #[test]
    #[ignore]
    fn downloads_real_model() {
        let tmp = tempfile::tempdir().unwrap();
        let entry = vosk_models::catalog_entry("vosk-model-small-en-us-0.15").unwrap();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let reports = std::sync::Mutex::new(0u32);
        rt.block_on(download_and_extract(entry, tmp.path(), |p| {
            *reports.lock().unwrap() += 1;
            eprintln!("{:?}", p);
        })).unwrap();
        assert!(*reports.lock().unwrap() > 1, "progress must be reported");
        assert!(vosk_models::is_vosk_model(&tmp.path().join(entry.name)));
        assert!(!tmp.path().join(format!("{}.zip.part", entry.name)).exists());
    }
}
