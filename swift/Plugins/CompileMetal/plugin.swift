import PackagePlugin
import Foundation

@main struct CompileMetal: BuildToolPlugin {
    func createBuildCommands(context: PluginContext, target: Target) async throws -> [Command] {
        let input = URL(fileURLWithPath: target.directory.string).appendingPathComponent("Shaders/Dither.metal")
        let script = URL(fileURLWithPath: context.package.directory.string).appendingPathComponent("Plugins/CompileMetal/compile.sh")
        let library = context.pluginWorkDirectoryURL.appendingPathComponent("Dither.metallib")
        return [
            .buildCommand(
                displayName: "Compile and link Dither Metal library",
                executable: URL(fileURLWithPath: "/bin/sh"),
                arguments: [script.path, input.path, library.path],
                inputFiles: [input, script], outputFiles: [library]
            )
        ]
    }
}
