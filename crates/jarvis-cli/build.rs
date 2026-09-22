// The native library search path is emitted by jarvis-core/build.rs.
// Here we only make the binary look for shared libraries (libvosk.dylib / .so,
// copied next to the executable by post_build.py) in its own directory.
fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    match target_os.as_str() {
        "macos" => {
            println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path");
            println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path");
        }
        "linux" => {
            println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
        }
        _ => {}
    }

    println!("cargo:rerun-if-changed=build.rs");
}
