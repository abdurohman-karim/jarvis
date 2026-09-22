# PvRecorder Binding for Rust

PvRecorder is an easy-to-use, cross-platform audio recorder designed for real-time speech audio processing. It allows developers access to an audio device's input stream, broken up into data frames of a given size.

## This repo is unofficial
Keep in mind this is reverted from original repo (commit [#142](https://github.com/Picovoice/pvrecorder/pull/142)).  
The reason is to keep a good library to continue working on Rust.  
I'll also try to continue the development of this binding (fix some bugs, etc).  
But that's all unofficial and based on my enthusiasm.

## Requirements

- Rust 1.54+ (tested on 1.92)

## Compatibility

- Linux (x86_64)
- macOS (x86_64 and arm64)
- Windows (x86_64 and arm64)
- Raspberry Pi:
    - Zero
    - 3 (32 and 64 bit)
    - 4 (32 and 64 bit)
    - 5 (32 and 64 bit)

## Installation

Because this fork is unnofficial, I don't think it would be a good idea to publish it on Crates, since I'm not a developer (although official versions was yanked bruh).  
So in order to install it, you need to link it as a Git repository inside your `Cargo.toml` manifest:
```toml
[dependencies]
pv_recorder = { path = "vendor/pvrecorder" }
```

## Usage

Getting the list of input devices does not require an instance:

```rust
use pv_recorder::PvRecorderBuilder

let audio_devices = PvRecorderBuilder::default().get_audio_devices()?;
```

To start recording, initialize an instance using the builder and call `start()`:

```rust
use pv_recorder::PvRecorderBuilder;

let frame_length = 512;
let recorder = PvRecorderBuilder::new(frame_length).init()?;
recorder.start()?
```

Read frames of audio:

```rust
while recorder.is_recording() {
    let frame = recorder.read()?;
    // process audio frame
}
```

To stop recording, call `stop()` on the instance:

```rust
recorder.stop()?;
```

Make sure to also check the source code inside `src/` and read thoroughly through documentation strings, as it can help you to understand how this crate works.