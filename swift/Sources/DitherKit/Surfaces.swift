import SwiftUI
import DitherIR

extension Rgba {
    var swiftUIColor: Color { Color(.sRGB, red: Double(r) / 255, green: Double(g) / 255, blue: Double(b) / 255, opacity: Double(a) / 255) }
}

/// Interpolates a numeric input before encoding so shader geometry animates, too.
struct AnimatedDitherSurface: View, Animatable {
    nonisolated var value: Double
    var cell: CGFloat = 3
    let encode: (Int, Int, Double) -> Program
    nonisolated var animatableData: Double { get { value } set { value = newValue } }
    var body: some View { DitherSurface(cell: cell) { encode($0, $1, value) } }
}

public struct DitherGradient: View {
    public var gradient: DitherIR.Gradient
    public var cell: CGFloat
    public init(_ gradient: DitherIR.Gradient, cell: CGFloat = 3) { self.gradient = gradient; self.cell = cell }
    public init(from: Tone, to: Tone? = nil, direction: Direction = .up, matrix: Matrix = .bayer8, density: Float = 1, cell: CGFloat = 3) {
        var gradient = DitherIR.Gradient(from: from)
        gradient.to = to; gradient.direction = direction; gradient.matrix = matrix; gradient.density = density
        self.init(gradient, cell: cell)
    }
    public var body: some View {
        DitherSurface(cell: cell) { gradient.encode(w: $0, h: $1) }.accessibilityLabel("Dither gradient")
    }
}

public struct DitherBackground: View {
    public var background: Background
    public var speed: Float
    public var cell: CGFloat
    @Environment(\.accessibilityReduceMotion) private var systemReduceMotion
    @Environment(\.ditherAnimationsEnabled) private var animationsEnabled
    private var reduceMotion: Bool { systemReduceMotion || !animationsEnabled }
    @State private var epoch = Date()
    public init(_ background: Background, speed: Float = 1, cell: CGFloat = 3) {
        self.background = background; self.speed = speed; self.cell = cell
    }
    public var body: some View {
        TimelineView(.animation(paused: reduceMotion || speed == 0)) { context in
            let time = reduceMotion ? Float(0) : Float(context.date.timeIntervalSince(epoch)) * speed
            DitherSurface(cell: cell) { background.encode(w: $0, h: $1, time: time) }
        }
        .accessibilityLabel("Animated dither background")
    }
}

public struct DitherSwatch: View {
    public var tone: Tone
    public var cell: CGFloat
    public init(_ tone: Tone, cell: CGFloat = 2) { self.tone = tone; self.cell = cell }
    public var body: some View {
        DitherSurface(cell: cell) { _, _ in swatch(tone: tone) }.frame(width: 12, height: 12).accessibilityHidden(true)
    }
}

public struct DitherAvatar: View {
    public var name: String
    public var hue: Float?
    public var size: CGFloat
    public init(_ name: String, hue: Float? = nil, size: CGFloat = 48) { self.name = name; self.hue = hue; self.size = size }
    public var body: some View {
        DitherSurface(cell: max(size, 8) / 8) { _, _ in avatar(name: name, hue: hue) }
            .frame(width: max(size, 8), height: max(size, 8))
            .accessibilityLabel("Avatar for \(name)")
    }
}

public struct DitherMeter: View {
    public var value: Float
    public var tone: Tone
    public var fill: Fill
    public var label: String?
    public var cell: CGFloat
    @Environment(\.accessibilityReduceMotion) private var systemReduceMotion
    @Environment(\.ditherAnimationsEnabled) private var animationsEnabled
    private var reduceMotion: Bool { systemReduceMotion || !animationsEnabled }
    @ScaledMetric private var barHeight = 24.0
    public init(value: Float, tone: Tone = .green, fill: Fill = .gradient, label: String? = nil, cell: CGFloat = 3) {
        self.value = value; self.tone = tone; self.fill = fill; self.label = label; self.cell = cell
    }
    public var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            if let label { Text(label).font(.caption) }
            AnimatedDitherSurface(value: Double(value), cell: cell) { w, h, current in meter(w: w, h: h, value: Float(current), tone: tone, fill: fill) }
                .frame(height: barHeight)
                .animation(reduceMotion ? nil : .spring(response: 0.55, dampingFraction: 0.8), value: value)
                .accessibilityHidden(true)
        }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(label ?? "Meter")
        .accessibilityValue(Text("\(Int(min(max(value, 0), 1) * 100)) percent"))
    }
}

/// Apply to a native `Button` to retain keyboard, focus, and VoiceOver behavior.
public struct DitherButton: SwiftUI.ButtonStyle {
    public var tone: Tone
    public var style: DitherIR.ButtonStyle
    public var cell: CGFloat
    public init(tone: Tone = .green, style: DitherIR.ButtonStyle = .solid, cell: CGFloat = 3) {
        self.tone = tone; self.style = style; self.cell = cell
    }
    public func makeBody(configuration: Configuration) -> some View {
        DitherButtonBody(configuration: configuration, tone: tone, style: style, cell: cell)
    }
}

private struct DitherButtonBody: View {
    let configuration: SwiftUI.ButtonStyleConfiguration
    let tone: Tone
    let style: DitherIR.ButtonStyle
    let cell: CGFloat
    @Environment(\.accessibilityReduceMotion) private var systemReduceMotion
    @Environment(\.ditherAnimationsEnabled) private var animationsEnabled
    private var reduceMotion: Bool { systemReduceMotion || !animationsEnabled }
    @Environment(\.isEnabled) private var isEnabled
    @State private var hovered = false
    @State private var epoch = Date()
    private var density: Float { style.density(hovered: hovered && isEnabled, pressed: configuration.isPressed && isEnabled) }
    var body: some View {
        configuration.label
            .padding(.horizontal, 18).padding(.vertical, 12)
            .background {
                TimelineView(.animation(paused: reduceMotion || density <= 0.4 || !isEnabled)) { context in
                    let time = reduceMotion ? Float(0) : Float(context.date.timeIntervalSince(epoch))
                    DitherSurface(cell: cell) { w, h in button(w: w, h: h, tone: tone, style: style, density: density, time: time) }
                        .background(style.surface(tone: tone)?.swiftUIColor ?? .clear)
                }
                .accessibilityHidden(true)
            }
            .contentShape(Rectangle())
            .opacity(isEnabled ? 1 : 0.45)
            .scaleEffect(configuration.isPressed && !reduceMotion ? 0.98 : 1)
            .animation(reduceMotion ? nil : .easeOut(duration: 0.15), value: configuration.isPressed)
            .onHover { hovered = $0 }
    }
}

public struct DitherSparkline: View {
    public var values: [Double]
    public var tone: Tone
    public var cell: CGFloat
    @Environment(\.accessibilityReduceMotion) private var systemReduceMotion
    @Environment(\.ditherAnimationsEnabled) private var animationsEnabled
    private var reduceMotion: Bool { systemReduceMotion || !animationsEnabled }
    @State private var hovered = false
    @State private var appeared = false
    public init(_ values: [Double], tone: Tone = .green, cell: CGFloat = 3) { self.values = values; self.tone = tone; self.cell = cell }
    public var body: some View {
        AnimatedDitherSurface(value: (reduceMotion || appeared) ? 1 : 0, cell: cell) { w, h, progress in
            sparkline(values: values, tone: tone, w: w, h: h, hovered: hovered, progress: Float(progress))
        }
        .onHover { hovered = $0 }
        .onAppear { withAnimation(reduceMotion ? nil : .easeOut(duration: 0.9)) { appeared = true } }
        .accessibilityLabel("Sparkline")
        .accessibilityValue(values.map(formatValue).joined(separator: ", "))
    }
}
