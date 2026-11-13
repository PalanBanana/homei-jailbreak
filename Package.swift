// swift-tools-version:5.9
// HomeiKit: Foundation-only logic shared by the HOMEI app. It mirrors the Rust core and is checked
// against the same data. It has not been compiled in the project's Linux development environment.
import PackageDescription

let package = Package(
    name: "HomeiKit",
    platforms: [.iOS(.v17), .macOS(.v14)],
    products: [.library(name: "HomeiKit", targets: ["HomeiKit"])],
    targets: [
        .target(name: "HomeiKit", path: "App/Sources/Kit"),
        .testTarget(name: "HomeiKitTests", dependencies: ["HomeiKit"], path: "Tests/SwiftTests"),
    ]
)
