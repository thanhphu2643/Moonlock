fn main() {
    // On macOS, compile swift/MoonlockKit (EventKit glue) and link it statically.
    // Windows/Linux don't need a Swift toolchain; the app falls back to .ics export there.
    #[cfg(target_os = "macos")]
    swift_rs::SwiftLinker::new("10.15")
        .with_package("MoonlockKit", "../../swift/MoonlockKit/")
        .link();
    tauri_build::build()
}
