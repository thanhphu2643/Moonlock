// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "MoonlockKit",
    platforms: [.macOS(.v10_15), .iOS(.v13)],
    products: [
        // Static lib linked into the Tauri binary by swift-rs (app/src-tauri/build.rs).
        .library(name: "MoonlockKit", type: .static, targets: ["MoonlockKit"]),
    ],
    targets: [
        .target(name: "MoonlockKit", path: "Sources/MoonlockKit"),
    ]
)
