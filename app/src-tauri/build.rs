fn main() {
    // Apple targets (macOS, iOS, iOS simulator): compile swift/MoonlockKit (EventKit glue)
    // and link it statically. Windows/Linux/Android need no Swift toolchain; the app falls
    // back to .ics export there.
    #[cfg(target_os = "macos")] // host can run `swift build`
    {
        let target = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
        if target == "macos" || target == "ios" {
            // Inside Xcode's "Build Rust Code" phase SDKROOT points at the iOS SDK, which makes
            // SwiftPM compile Package.swift (a host/macOS program) against iOS and fail.
            // swift-rs passes the right `--sdk` explicitly, so drop the inherited value.
            std::env::remove_var("SDKROOT");
            swift_rs::SwiftLinker::new("10.15")
                .with_ios("13")
                .with_package("MoonlockKit", "../../swift/MoonlockKit/")
                .link();
        }
    }
    tauri_build::build()
}
