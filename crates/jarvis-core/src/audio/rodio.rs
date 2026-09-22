/*
    Abandoned temporary.
    Problems with blocking behaviour.
    Possible fixes are running rodio in a separate thread or smthng.
*/

use once_cell::sync::OnceCell;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

// use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use rodio::{Decoder, Sink, Source};

// NOTE: OutputStream is not Send/Sync on macOS (cpal CoreAudio), so it cannot live
// in a static. Instead it is leaked with mem::forget to keep the device open for the
// whole process lifetime; only the (Send + Sync) sink is stored.
static INITIALIZED: AtomicBool = AtomicBool::new(false);
static SINK: OnceCell<Sink> = OnceCell::new();

pub fn init() -> Result<(), ()> {
    if INITIALIZED.load(Ordering::SeqCst) {
        return Ok(());
    } // already initialized

    // get output stream handle to the default physical sound device
    match rodio::OutputStreamBuilder::open_default_stream() {
        Ok(stream_handle) => {
            // create sink
            let sink = Sink::connect_new(&stream_handle.mixer());
            info!("Sink initialized.");

            // store the sink, keep the stream alive for the whole process
            let _ = SINK.set(sink);
            std::mem::forget(stream_handle);
            INITIALIZED.store(true, Ordering::SeqCst);

            // success
            Ok(())
        }
        Err(msg) => {
            error!("Failed to initialize audio stream.\nError details: {}", msg);

            // failed
            Err(())
        }
    }
}

pub fn play_sound(filename: &PathBuf, sleep: bool) -> Option<Duration> {
    let file = match File::open(filename) {
        Ok(f) => BufReader::new(f),
        Err(e) => {
            warn!("Cannot open sound file {}: {}", filename.display(), e);
            return None;
        }
    };

    let source = match Decoder::new(file) {
        Ok(s) => s,
        Err(e) => {
            warn!("Cannot decode sound file {}: {}", filename.display(), e);
            return None;
        }
    };

    let duration = source.total_duration();

    let Some(sink) = SINK.get() else {
        warn!("Audio sink not initialized");
        return None;
    };

    sink.append(source);

    if sleep {
        // The sound plays in a separate thread. This call will block the current thread until the sink
        // has finished playing all its queued sounds.
        sink.sleep_until_end();
    }

    duration
}
