use std::path::Path;

// Tells the linker where the prebuilt native libraries (libvosk) live.
// `rustc-link-search` propagates to every binary that depends on jarvis-core.
fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let lib_root = Path::new(&manifest_dir).join("..").join("..").join("lib");

    let lib_path = match target_os.as_str() {
        "windows" => lib_root.join("windows").join("amd64"),
        "macos" => lib_root.join("macos"),
        _ => lib_root.join("linux"),
    };
    println!("cargo:rustc-link-search=native={}", lib_path.display());

    // rustc-link-arg applies to this package's own binaries only, i.e. the unit test
    // harness: it has no rpath of its own (the app / cli set theirs in their build.rs),
    // so point it straight at the checked-in library directory
    if target_os == "macos" || target_os == "linux" {
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_path.display());
    }

    println!("cargo:rerun-if-changed=build.rs");
}
