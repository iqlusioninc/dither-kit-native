import SwiftUI
@_exported import DitherIR

/// A pixel-aligned display-list surface. Cell sizes are measured in logical points.
public struct DitherSurface: View {
    public var cell: CGFloat
    private let straightOutput: Bool
    private let encode: (Int, Int) -> [Float]

    public init(cell: CGFloat = 3, encode: @escaping (Int, Int) -> Program) {
        self.straightOutput = false
        self.cell = max(cell, 0.25)
        self.encode = { encode($0, $1).toArray() }
    }

    /// Draw an already encoded program, useful for deterministic exports and fixtures.
    public init(program: [Float], cell: CGFloat = 3) {
        self.straightOutput = false
        self.cell = max(cell, 0.25)
        self.encode = { _, _ in program }
    }

    init(testProgram: [Float], cell: CGFloat = 1) {
        self.cell = max(cell, 0.25)
        self.straightOutput = true
        self.encode = { _, _ in testProgram }
    }

    public var body: some View {
        GeometryReader { geometry in
            let width = max(0, Int((geometry.size.width / cell).rounded(.down)))
            let height = max(0, Int((geometry.size.height / cell).rounded(.down)))
            let data = encode(width, height)
            // Float shader arguments must preserve the integer offset exactly.
            let split = Program.bloomIndex(in: data).flatMap { Float(exactly: $0) } ?? 0
            Rectangle()
                .fill(.white)
                .colorEffect(DitherShaders.library.ditherBase(.floatArray(data), .float(Float(cell)), .float2(Float(width), Float(height)), .float(straightOutput ? 1 : 0), .float(split)))
                .layerEffect(DitherShaders.library.ditherPost(.floatArray(data), .float(Float(cell)), .float2(Float(width), Float(height)), .float(straightOutput ? 1 : 0), .float(split)), maxSampleOffset: CGSize(width: 4 * cell, height: 4 * cell))
                .frame(width: CGFloat(width) * cell, height: CGFloat(height) * cell, alignment: .topLeading)
        }
        .accessibilityLabel("Dither surface")
    }
}

enum DitherShaders {
    static let library = ShaderLibrary(url: Bundle.module.url(forResource: "Dither", withExtension: "metallib")!)
}

public extension EnvironmentValues {
    /// Disable animation for deterministic exports or a static presentation.
    /// System Reduce Motion always takes precedence when animation is enabled.
    @Entry var ditherAnimationsEnabled: Bool = true
}
