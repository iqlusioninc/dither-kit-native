import SwiftUI
import DitherKit
#if os(macOS)
import AppKit
#endif

@main struct DitherGallery: App {
    init() {
        #if os(macOS)
        if let index = CommandLine.arguments.firstIndex(of: "--snapshot"), CommandLine.arguments.indices.contains(index + 1) {
            let renderer = ImageRenderer(content: GalleryView().environment(\.ditherAnimationsEnabled, false).frame(width: 1100).environment(\.colorScheme, .dark))
            renderer.scale = 1
            guard let image = renderer.cgImage else { fatalError("Could not render gallery") }
            let bitmap = NSBitmapImageRep(cgImage: image)
            do { try bitmap.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: CommandLine.arguments[index + 1])) }
            catch { fatalError("Could not write gallery: \(error)") }
            exit(0)
        }
        #endif
    }
    var body: some Scene {
        WindowGroup {
            ScrollView { GalleryView().frame(maxWidth: 1200) }.background(Color(white: 0.06)).preferredColorScheme(.dark)
                .frame(minWidth: 760, minHeight: 700)
        }
        .defaultSize(width: 1100, height: 1000)
    }
}

struct GalleryView: View {
    @State private var meter: Float = 0.67
    private let series = [
        Series(label: "Inference", tone: .green, values: [12, 18, 13, 29, 24, 36, 30, 44, 41]),
        Series(label: "Training", tone: .purple, values: [8, 11, 10, 17, 14, 22, 18, 27, 25])
    ]
    private let slices = [DitherIR.Slice(label: "Compute", value: 56, tone: .green), DitherIR.Slice(label: "Storage", value: 28, tone: .purple), DitherIR.Slice(label: "Network", value: 16, tone: .orange)]
    var body: some View {
        VStack(alignment: .leading, spacing: 22) {
            header
            HStack(spacing: 18) {
                panel("Area / hover for values") { DitherChart(kind: .area, series: series, labels: ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun", "Mon", "Tue"]).frame(height: 190) }
                panel("Line / toggle the legend") { DitherChart(kind: .line, series: series).frame(height: 190) }
            }
            HStack(spacing: 18) {
                panel("Grouped bars") { DitherChart(kind: .bar, series: series).frame(height: 175) }
                panel("Stacked bars") {
                    DitherChart(stackedChart).frame(height: 175)
                }
            }
            HStack(alignment: .top, spacing: 18) {
                panel("Allocation / donut") { DitherPie(slices: slices, innerRadius: 0.55).frame(height: 225) }
                panel("Radar / axis geometry") { DitherRadar(series: [Series(label: "Current", tone: .blue, values: [8, 6, 9, 5, 7]), Series(label: "Target", tone: .pink, values: [9, 8, 7, 9, 8])], labels: ["Speed", "Quality", "Cost", "Scale", "Safety"]).frame(height: 225) }
            }
            HStack(spacing: 18) {
                panel("Identity / micro charts") {
                    HStack(spacing: 18) {
                        DitherAvatar("DitherKit", size: 64)
                        DitherAvatar("Metal", size: 64)
                        DitherSparkline([5, 8, 6, 13, 9, 17, 15, 23], tone: .green).frame(height: 64)
                    }
                }
                panel("Native controls") {
                    DitherMeter(value: meter, tone: .green, label: "GPU utilization")
                    HStack {
                        Button("Refresh") { meter = meter > 0.8 ? 0.35 : 0.92 }.buttonStyle(DitherButton())
                        Button("Outline") { meter = 0.52 }.buttonStyle(DitherButton(tone: .purple, style: .outline))
                        Button("Ghost") { meter = 0.75 }.buttonStyle(DitherButton(tone: .orange, style: .ghost))
                    }
                }
            }
            HStack(spacing: 18) {
                panel("Gradient / two tones") { DitherGradient(from: .green, to: .blue, direction: .right).frame(height: 80) }
                panel("Animated field / respects Reduce Motion") { DitherBackground(Background(field: .aurora, tone: .purple)).frame(height: 80) }
            }
            Text("Native SwiftUI chrome · Metal display-list shaders · 3-point cells · keyboard and VoiceOver controls")
                .font(.caption).foregroundStyle(.secondary)
        }
        .padding(28).background(Color(white: 0.06))
    }
    private var stackedChart: Cartesian { var chart = Cartesian(kind: .bar, series: series); chart.stacked = true; return chart }
    private var header: some View {
        HStack(alignment: .bottom) {
            VStack(alignment: .leading, spacing: 5) {
                Text("DitherKit").font(.system(size: 36, weight: .semibold, design: .rounded))
                Text("Small pixels. Native surfaces.").foregroundStyle(.secondary)
            }
            Spacer()
            DitherAvatar("SwiftUI", size: 48)
        }
    }
    private func panel<Content: View>(_ title: String, @ViewBuilder content: () -> Content) -> some View {
        VStack(alignment: .leading, spacing: 14) {
            Text(title).font(.subheadline.weight(.semibold)).foregroundStyle(.secondary)
            content()
        }
        .padding(16).frame(maxWidth: .infinity, alignment: .topLeading)
        .background(Color(white: 0.095), in: .rect(cornerRadius: 12))
    }
}
