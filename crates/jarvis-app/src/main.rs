use jarvis_core::slots;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

// include core
use jarvis_core::{
    audio, audio_processing, commands, config, db, listener, recorder, stt, intent,
    ipc::{self, IpcAction},
    i18n, voices, models,
    APP_CONFIG_DIR, APP_LOG_DIR, DB,
};

// include log
#[macro_use]
extern crate simple_log;
mod log;

// include app
mod app;
mod executor;

// include tray
mod tray;

static SHOULD_STOP: AtomicBool = AtomicBool::new(false);

fn main() -> Result<(), String> {
    // initialize directories
    config::init_dirs()?;

    // initialize logging
    log::init_logging()?;

    // panics would otherwise go to stderr, which nobody sees when launched from the GUI
    std::panic::set_hook(Box::new(|info| {
        let thread = std::thread::current();
        error!("PANIC in thread '{}': {}", thread.name().unwrap_or("?"), info);
    }));

    // log some base info
    info!("Starting Jarvis v{} ...", config::APP_VERSION.unwrap());
    info!("Config directory is: {}", APP_CONFIG_DIR.get().unwrap().display());
    info!("Log directory is: {}", APP_LOG_DIR.get().unwrap().display());

    // single instance: the IPC port doubles as the instance lock
    let ipc_listener = match ipc::bind() {
        Ok(l) => l,
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
            error!("Another Jarvis instance is already running (IPC port {} is busy). Exiting.", ipc::IPC_PORT);
            return Err("already running".into());
        }
        Err(e) => {
            error!("Failed to bind IPC port {}: {}", ipc::IPC_PORT, e);
            return Err(e.to_string());
        }
    };

    // initialize settings
    let settings = db::init();

    // set global DB (for core modules that read settings at init time)
    DB.set(settings.arc().clone())
            .expect("DB already initialized");

    // init voices
    let voice_id = settings.lock().voice.clone();
    let language = settings.lock().language.clone();
    if let Err(e) = voices::init(&voice_id, &language) {
        warn!("Failed to init voices: {}", e);
    }

    // init i18n
    i18n::init(&settings.lock().language);

    // init recorder
    if recorder::init().is_err() {
        app::close(1);
    }

    // init models registry (scans available AI models)
    if let Err(e) = models::init() {
        warn!("Models registry init failed: {}", e);
    }

    // init stt engine
    if let Err(e) = stt::init() {
        // @TODO. Allow continuing even without STT, if commands is using keywords or smthng?
        error!("Speech recognition could not be initialized: {}. Download a Vosk model in the GUI settings.", e);
        app::close(1); // cannot continue without stt
    }

    // init commands
    info!("Initializing commands.");
    let cmds = match commands::parse_commands() {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to parse commands: {}. Starting with empty command list.", e);
            Vec::new()
        }
    };
    info!("Commands initialized. Count: {}, List: {:?}", cmds.len(), commands::list_paths(&cmds));
    commands::set_list(cmds);

    // get the speech engine ready (the cloning model takes seconds to load)
    jarvis_core::speech::warm_up(&i18n::get_language());

    // init audio
    if audio::init().is_err() {
        // @TODO. Allow continuing even without audio?
        app::close(1); // cannot continue without audio
    }

    // init wake-word engine
    if let Err(e) = listener::init() {
        error!("Wake-word engine init failed: {}", e);
        app::close(1);
    }

    // shared async runtime for intent classification, IPC, etc.
    let rt = Arc::new(
        tokio::runtime::Runtime::new().expect("Failed to create tokio runtime")
    );

    // init intent-recognition engine
    rt.block_on(async {
        if let Err(e) = intent::init(&commands::list()).await {
            error!("Failed to initialize intent classifier: {}", e);
            app::close(1);
        }
    });

    // init slots parsing engine
    slots::init().map_err(|e| error!("Slot extraction init failed: {}", e)).ok();

    // init audio processing
    info!("Initializing audio processing...");
    if let Err(e) = audio_processing::init() {
        warn!("Audio processing init failed: {}", e);
    }

    // init IPC
    info!("Initializing IPC...");
    ipc::init();
    if let Err(e) = ipc::init_token() {
        error!("Failed to write IPC token: {}. GUI will not be able to connect.", e);
    }

    // command executor: runs commands off the audio thread
    let executor = executor::spawn(Arc::clone(&rt));

    let ipc_executor = executor.clone();
    ipc::set_action_handler(move |action| {
        match action {
            IpcAction::Stop => {
                info!("Received stop command from GUI");
                SHOULD_STOP.store(true, Ordering::SeqCst);
            }
            IpcAction::ReloadCommands => {
                info!("Received reload commands request");
                ipc_executor.submit_reload();
            }
            IpcAction::SetMuted { muted } => {
                app::set_muted(muted);
            }
            IpcAction::ApplySettings => {
                info!("Received apply settings request");
                app::request_apply_settings();
            }
            IpcAction::TextCommand { text } => {
                info!("Received text command: {}", text);
                ipc_executor.submit_text(text);
            }
            IpcAction::Ping | IpcAction::Auth { .. } => {
                // handled internally by server
            }
        }
    });

    // start WebSocket server on the shared runtime
    let ipc_rt = Arc::clone(&rt);
    std::thread::spawn(move || {
        ipc_rt.block_on(ipc::start_server_on(ipc_listener));
    });
    
    // start the audio pipeline (in the background thread)
    std::thread::Builder::new().name("audio-processing".into()).spawn(move || {
        // the main thread is blocked in the tray event loop, so once the assistant loop
        // ends (stop from the GUI, or a crash) end the process here instead of leaving
        // a tray icon without an assistant behind it
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| app::start(executor))) {
            Ok(_) => {
                info!("Assistant loop finished, exiting.");
                std::process::exit(0);
            }
            Err(_) => {
                error!("Assistant pipeline crashed, exiting.");
                ipc::send(ipc::IpcEvent::Error { message: "Assistant crashed, see log.txt".into() });
                std::thread::sleep(std::time::Duration::from_millis(200)); // let the event flush
                std::process::exit(2);
            }
        }
    }).expect("failed to spawn audio processing thread");

    tray::init_blocking(settings);

    Ok(())
}

pub fn should_stop() -> bool {
    SHOULD_STOP.load(Ordering::SeqCst)
}
