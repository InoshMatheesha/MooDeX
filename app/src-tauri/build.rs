// build.rs - Version resource + manifest via tauri-build; build hash for Diagnostics (spec 11, 13.3).

fn main() {
    let hash = std::process::Command::new("git")
        .args(["rev-parse", "--short=10", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "dev".to_string());
    println!("cargo:rustc-env=MOODEX_BUILD_HASH={hash}");
    println!("cargo:rerun-if-changed=moodex.manifest");

    // asInvoker, PerMonitorV2, long paths, UTF-8 code page (spec 13.3).
    let windows = tauri_build::WindowsAttributes::new().app_manifest(include_str!("moodex.manifest"));
    let attrs = tauri_build::Attributes::new().windows_attributes(windows);
    if let Err(e) = tauri_build::try_build(attrs) {
        println!("cargo:warning=tauri-build failed: {e}");
        std::process::exit(1);
    }
}
