fn main() {
    // let the binary find shared libraries (e.g. libvosk.dylib) next to itself
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    match target_os.as_str() {
        "macos" => {
            println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path");
            println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path");
        }
        "linux" => println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN"),
        _ => {}
    }

    // Tauri build
    tauri_build::build()
}
