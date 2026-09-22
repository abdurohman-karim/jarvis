use sysinfo::{System, Pid, ProcessRefreshKind, ProcessesToUpdate, RefreshKind};
use std::sync::Mutex;
use once_cell::sync::Lazy;

static SYS: Lazy<Mutex<System>> = Lazy::new(|| {
    Mutex::new(System::new_with_specifics(
        RefreshKind::nothing()
            .with_processes(ProcessRefreshKind::nothing().with_memory().with_cpu())
    ))
});

// PID of the assistant process we last saw (spawned by us or found by a scan), so the
// periodic stats poll refreshes a single process instead of the whole process table
static KNOWN_PID: Mutex<Option<Pid>> = Mutex::new(None);

#[cfg(target_os = "windows")]
const JARVIS_APP_NAME: &str = "jarvis-app.exe";
#[cfg(not(target_os = "windows"))]
const JARVIS_APP_NAME: &str = "jarvis-app";

/// Full scan of the process table for the assistant (used when the PID is unknown, e.g. the
/// assistant was started from the tray or a previous GUI session)
fn scan_for_jarvis_app(sys: &mut System) -> Option<Pid> {
    sys.refresh_processes(ProcessesToUpdate::All, true);
    sys.processes()
        .iter()
        .find(|(_, p)| p.name().eq_ignore_ascii_case(JARVIS_APP_NAME))
        .map(|(pid, _)| *pid)
}

fn find_jarvis_app_pid(sys: &mut System) -> Option<Pid> {
    let mut known = KNOWN_PID.lock().unwrap();

    if let Some(pid) = *known {
        sys.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);
        match sys.process(pid) {
            Some(p) if p.name().eq_ignore_ascii_case(JARVIS_APP_NAME) => return Some(pid),
            _ => *known = None, // exited, or the pid was reused by another process
        }
    }

    let found = scan_for_jarvis_app(sys);
    *known = found;
    found
}

#[derive(serde::Serialize)]
pub struct JarvisAppStats {
    pub running: bool,
    pub ram_mb: u64,
    pub cpu_usage: f32,
}

#[tauri::command(async)]
pub fn get_jarvis_app_stats() -> JarvisAppStats {
    let mut sys = SYS.lock().unwrap();

    if let Some(pid) = find_jarvis_app_pid(&mut sys) {
        if let Some(proc) = sys.process(pid) {
            return JarvisAppStats {
                running: true,
                ram_mb: proc.memory() / 1024 / 1024,
                cpu_usage: proc.cpu_usage(),
            };
        }
    }

    JarvisAppStats {
        running: false,
        ram_mb: 0,
        cpu_usage: 0.0,
    }
}

#[tauri::command]
pub fn run_jarvis_app() -> Result<(), String> {
    let exe_dir = std::env::current_exe()
        .map_err(|e| format!("Failed to get exe path: {}", e))?
        .parent()
        .ok_or("Failed to get exe directory")?
        .to_path_buf();

    let jarvis_app_path = exe_dir.join(JARVIS_APP_NAME);

    if !jarvis_app_path.exists() {
        return Err(format!("jarvis-app not found at: {}", jarvis_app_path.display()));
    }

    let child = std::process::Command::new(&jarvis_app_path)
        .spawn()
        .map_err(|e| format!("Failed to start jarvis-app: {}", e))?;

    *KNOWN_PID.lock().unwrap() = Some(Pid::from_u32(child.id()));

    Ok(())
}
