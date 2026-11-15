import PackagePlugin
import Foundation

@main struct CompileMetal: BuildToolPlugin {
    func createBuildCommands(context: PluginContext, target: Target) async throws -> [Command] {
        let input = URL(fileURLWithPath: target.directory.string).appendingPathComponent("Shaders/Dither.metal")
        let air = context.pluginWorkDirectoryURL.appendingPathComponent("Dither.air")
        let library = context.pluginWorkDirectoryURL.appendingPathComponent("Dither.metallib")
        // Xcode sets SDKROOT for destination builds, including iOS and simulators.
        let sdk = ProcessInfo.processInfo.environment["SDKROOT"] ?? "macosx"
        return [
            .buildCommand(displayName: "Compile Dither Metal shader", executable: URL(fileURLWithPath: "/usr/bin/xcrun"), arguments: ["--sdk", sdk, "metal", "-c", "-std=metal3.0", "-fno-fast-math", input.path, "-o", air.path], inputFiles: [input], outputFiles: [air]),
            .buildCommand(displayName: "Link Dither Metal library", executable: URL(fileURLWithPath: "/usr/bin/xcrun"), arguments: ["--sdk", sdk, "metallib", air.path, "-o", library.path], inputFiles: [air], outputFiles: [library])
        ]
    }
}
