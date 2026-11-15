import Foundation

// Rust f32 constants round to nearest; Swift Float.pi rounds downward.
let irPi = Float(bitPattern: 0x40490fdb)
let irTau = Float(bitPattern: 0x40c90fdb)
let irHalfPi = Float(bitPattern: 0x3fc90fdb)

func clamp<T: Comparable>(_ x: T, _ lo: T, _ hi: T) -> T { min(max(x, lo), hi) }
func remEuclid(_ x: Float, _ modulus: Float) -> Float {
    let r = x.truncatingRemainder(dividingBy: modulus)
    return r < 0 ? r + abs(modulus) : r
}
// Rust float-to-integer casts saturate and convert NaN to zero.
func pixel(_ value: Float) -> Int {
    if value.isNaN { return 0 }
    if value >= Float(Int32.max) { return Int(Int32.max) }
    if value <= Float(Int32.min) { return Int(Int32.min) }
    return Int(value)
}
func byte(_ value: Float) -> UInt8 {
    if value.isNaN { return 0 }
    return UInt8(clamp(value.rounded(), 0, 255))
}

/// Eight bit, straight-alpha sRGB.
public struct Rgba: Equatable, Hashable, Sendable {
    public var r: UInt8, g: UInt8, b: UInt8, a: UInt8
    public init() { self.r = 0; self.g = 0; self.b = 0; self.a = 0 }
    public init(r: UInt8, g: UInt8, b: UInt8, a: UInt8 = 255) { self.r = r; self.g = g; self.b = b; self.a = a }
    public init(_ r: UInt8, _ g: UInt8, _ b: UInt8, _ a: UInt8 = 255) { self.init(r: r, g: g, b: b, a: a) }
    public static let transparent = Rgba(), black = Rgba(0, 0, 0), white = Rgba(255, 255, 255)
    public static func rgb(_ r: UInt8, _ g: UInt8, _ b: UInt8) -> Rgba { Rgba(r, g, b) }
    public static func hex(_ hex: UInt32) -> Rgba { Rgba(UInt8(truncatingIfNeeded: hex >> 16), UInt8(truncatingIfNeeded: hex >> 8), UInt8(truncatingIfNeeded: hex)) }
    public func withAlpha(_ a: UInt8) -> Rgba { Rgba(r, g, b, a) }
    public func fade(_ factor: Float) -> Rgba { withAlpha(byte(Float(a) * clamp(factor, 0, 1))) }
    public func mix(_ other: Rgba, _ t: Float) -> Rgba {
        let t = clamp(t, 0, 1)
        func lerp(_ a: UInt8, _ b: UInt8) -> UInt8 { byte(Float(a) + (Float(b) - Float(a)) * t) }
        return Rgba(lerp(r, other.r), lerp(g, other.g), lerp(b, other.b), lerp(a, other.a))
    }
    public func over(_ background: Rgba) -> Rgba { background.withAlpha(255).mix(withAlpha(255), Float(a) / 255) }
    public func luminance() -> Float {
        func lin(_ channel: UInt8) -> Float { let c = Float(channel) / 255; return c <= 0.04045 ? c / 12.92 : pow((c + 0.055) / 1.055, 2.4) }
        return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b)
    }
    public static func oklch(_ l: Float, _ c: Float, _ h: Float) -> Rgba {
        var c = c.isNaN ? 0 : max(c, 0)
        while true {
            if let rgb = oklchToSrgb(l, c, h) { return rgb }
            c -= 0.005
            if c <= 0 { return oklchToSrgb(l, 0, h) ?? .black }
        }
    }
}

private func oklchToSrgb(_ l: Float, _ c: Float, _ h: Float) -> Rgba? {
    // Keep every intermediate in Float, matching the reference f32 arithmetic.
    let radians = h * Float(0.017453292519943295)
    let a = c * cos(radians), b = c * sin(radians)
    let ll = l + 0.39633778 * a + 0.21580376 * b
    let mm = l - 0.105561346 * a - 0.06385417 * b
    let ss = l - 0.08948418 * a - 1.2914855 * b
    let l3 = ll * ll * ll, m3 = mm * mm * mm, s3 = ss * ss * ss
    let r = 4.0767417 * l3 - 3.3077116 * m3 + 0.23096994 * s3
    let g = -1.268438 * l3 + 2.6097574 * m3 - 0.34131938 * s3
    let b2 = -0.0041960863 * l3 - 0.7034186 * m3 + 1.7076147 * s3
    if [r, g, b2].contains(where: { $0 < -1e-4 || $0 > 1.0001 }) { return nil }
    func enc(_ value: Float) -> UInt8 {
        let v = clamp(value, 0, 1)
        let s: Float = v <= 0.0031308 ? 12.92 * v : 1.055 * pow(v, 1 / 2.4) - 0.055
        return byte(s * 255)
    }
    return .rgb(enc(r), enc(g), enc(b2))
}

public enum Tone: Equatable, Hashable, Sendable {
    case green, blue, purple, pink, orange, red, grey, hue(Float)
    public static let all: [Tone] = [.green, .blue, .purple, .pink, .orange, .red, .grey]
    public func name() -> String {
        switch self { case .green: "green"; case .blue: "blue"; case .purple: "purple"; case .pink: "pink"; case .orange: "orange"; case .red: "red"; case .grey: "grey"; case .hue: "hue" }
    }
    public static func parse(_ string: String) -> Tone? {
        let s = string.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        switch s {
        case "green": return .green
        case "blue": return .blue
        case "purple", "violet": return .purple
        case "pink": return .pink
        case "orange": return .orange
        case "red": return .red
        case "grey", "gray": return .grey
        default: return Float(s).map { .hue(remEuclid($0, 360)) }
        }
    }
    public func hue() -> Float {
        switch self { case .green: 150; case .blue: 250; case .purple: 300; case .pink: 350; case .orange: 55; case .red: 25; case .grey: 260; case let .hue(h): remEuclid(h, 360) }
    }
    public func chroma() -> Float { self == .grey ? 0.012 : 0.19 }
    public func shade(_ level: Float) -> Rgba { shadeHC(hue: hue(), chroma: chroma(), level: level) }
    public func base() -> Rgba { shade(0.55) }
}
public func shadeHC(hue: Float, chroma: Float, level: Float) -> Rgba {
    let level = clamp(level, 0, 1)
    let l = 0.22 + level * 0.72
    let c = chroma * clamp(1 - abs(level - 0.55) * 1.3, 0.25, 1)
    return .oklch(l, c, hue)
}
