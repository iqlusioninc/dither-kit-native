// swift-tools-version: 6.0
import PackageDescription
let package = Package(
    name: "DitherKit",
    platforms: [.macOS(.v14), .iOS(.v17)],
    products: [
        .library(name: "DitherIR", targets: ["DitherIR"]),
        .library(name: "DitherKit", targets: ["DitherKit"]),
        .executable(name: "DitherGallery", targets: ["DitherGallery"])
    ],
    targets: [
        .target(name: "DitherIR"),
        .target(name: "DitherKit", dependencies: ["DitherIR"], resources: [.copy("Shaders")], plugins: [.plugin(name: "CompileMetal")]),
        .executableTarget(name: "DitherGallery", dependencies: ["DitherKit"], path: "Examples/Gallery"),
        .testTarget(name: "DitherIRTests", dependencies: ["DitherIR"]),
        .testTarget(name: "DitherKitTests", dependencies: ["DitherKit"]),
        .plugin(name: "CompileMetal", capability: .buildTool())
    ]
)
