import Foundation

public enum Matrix: String, CaseIterable, Sendable {
    case bayer2, bayer4, bayer8, halftone, noise
    public static let all = allCases
    public func id() -> Float { Float(Self.allCases.firstIndex(of: self)!) }
}
public enum Fill: String, CaseIterable, Sendable {
    case gradient, dotted, hatched, solid
    public static let all = allCases
    public func id() -> Float { Float(Self.allCases.firstIndex(of: self)!) }
}
public enum Bloom: String, CaseIterable, Sendable {
    case off, low, high, aura
    public static let all = allCases
    public func params() -> (Int, Float) { switch self { case .off: (0, 0); case .low: (1, 0.45); case .high: (2, 0.6); case .aura: (4, 0.7) } }
    func emit(_ p: inout Program) { if self != .off { let (r, s) = params(); p.bloom(r, s) } }
}
public enum Direction: String, CaseIterable, Sendable {
    case up, down, left, right, radial
    public func id() -> Float { Float(Self.allCases.firstIndex(of: self)!) }
}
public enum Field: String, CaseIterable, Sendable {
    case plasma, waves, ripple, noise, aurora
    public static let all = allCases
    public func id() -> Float { Float(Self.allCases.firstIndex(of: self)!) }
}
public struct Gradient: Equatable, Sendable {
    public var from: Tone, to: Tone?
    public var direction: Direction = .up
    public var matrix: Matrix = .bayer8
    public var density: Float = 1
    public init(from: Tone) { self.from = from }
    public func encode(w: Int, h: Int) -> Program {
        var p = Program()
        p.push(Op.gradient, [Float(w), Float(h), direction.id(), matrix.id(), clamp(density, 0, 1)] + tone(from) + [to == nil ? 0 : 1] + (to.map(tone) ?? [0, 0]))
        return p
    }
}
public struct Background: Equatable, Sendable {
    public var field: Field, tone: Tone, accent: Tone?
    public var matrix: Matrix = .bayer8
    public var density: Float = 0.6
    public init(field: Field, tone: Tone) { self.field = field; self.tone = tone }
    public func encode(w: Int, h: Int, time: Float) -> Program {
        var p = Program()
        p.push(Op.background, [Float(w), Float(h), field.id(), matrix.id(), clamp(density, 0, 1), time, tone.hue(), tone.chroma(), accent == nil ? 0 : 1, accent?.hue() ?? 0, accent?.chroma() ?? 0])
        return p
    }
}
public enum ButtonStyle: String, CaseIterable, Sendable {
    case solid, outline, ghost
    public func id() -> Float { Float(Self.allCases.firstIndex(of: self)!) }
    public func density(hovered: Bool, pressed: Bool) -> Float {
        let values: (Float, Float, Float) = switch self { case .solid: (0.32, 0.55, 0.85); case .outline: (0, 0.3, 0.6); case .ghost: (0, 0.25, 0.5) }
        return pressed ? values.2 : hovered ? values.1 : values.0
    }
    public func surface(tone: Tone) -> Rgba? { self == .solid ? tone.shade(0.22) : nil }
}
public func button(w: Int, h: Int, tone t: Tone, style: ButtonStyle, density: Float, time: Float) -> Program {
    var p = Program(); p.push(Op.button, [Float(w), Float(h), style.id(), density, time] + tone(t)); return p
}
public func meter(w: Int, h: Int, value: Float, tone t: Tone, fill: Fill) -> Program {
    var p = Program(); p.push(Op.meter, [Float(w), Float(h), value, fill.id()] + tone(t)); return p
}
public func swatch(tone t: Tone) -> Program { var p = Program(); p.push(Op.swatch, tone(t)); return p }
public func fnv1a(_ bytes: [UInt8]) -> UInt64 {
    var h: UInt64 = 0xcbf29ce484222325
    for b in bytes { h ^= UInt64(b); h = h &* 0x00000100000001b3 }
    return h
}
public func avatar(name: String, hue: Float? = nil) -> Program {
    let hash = fnv1a(Array(name.utf8)), bits = UInt32(truncatingIfNeeded: hash)
    let mirror = (hash >> 32) & 1 == 0
    let hue = hue ?? Float((hash >> 33) % 180) * 2
    let acc = UInt32(truncatingIfNeeded: hash >> 41) & 0x7fffff
    var p = Program()
    p.push(Op.avatar, [Float(bits & 0xffff), Float(bits >> 16), mirror ? 1 : 0, Tone.hue(hue).hue(), Float(acc & 0xffff), Float(acc >> 16)])
    return p
}
public struct Series: Equatable, Sendable {
    public var label: String, tone: Tone, fill: Fill, values: [Double], hidden: Bool
    public init(label: String, tone: Tone, values: [Double], fill: Fill = .gradient, hidden: Bool = false) {
        self.label = label; self.tone = tone; self.values = values; self.fill = fill; self.hidden = hidden
    }
}
public struct Scale: Equatable, Sendable {
    public var max: Double, ticks: [Double]
    public init(max: Double, ticks: [Double]) { self.max = max; self.ticks = ticks }
    public static func nice(max: Double, count: Int) -> Scale {
        let maximum = max.isFinite && max > 0 ? max : 1
        let raw = maximum / Double(Swift.max(count, 1))
        let mag = pow(10, floor(log10(raw)))
        let step = [1.0, 2, 2.5, 5, 10].map { $0 * mag }.first { $0 >= raw } ?? 10 * mag
        let top = ceil(maximum / step) * step
        let intervals = (top / step).rounded()
        let count = intervals.isNaN || intervals <= 0 ? 0 : Int(intervals)
        return Scale(max: top, ticks: (0...count).map { Double($0) * step })
    }
    public func y(_ value: Double, h: Int) -> Float { let span = Float(Swift.max(h - 1, 0)); return span - Float(clamp(value / max, 0, 1)) * span }
}
public func formatValue(_ v: Double) -> String {
    let a = abs(v)
    let (n, suffix): (Double, String) = a >= 1e9 ? (v / 1e9, "B") : a >= 1e6 ? (v / 1e6, "M") : a >= 1e3 ? (v / 1e3, "k") : (v, "")
    if abs(n - n.rounded()) < 0.05 {
        let rounded = n.rounded()
        let integer: Int64 = rounded >= Double(Int64.max) ? .max : rounded <= Double(Int64.min) ? .min : Int64(rounded)
        return String(integer) + suffix
    }
    if n.isNaN { return "NaN" + suffix }
    return String(format: "%.1f", locale: Locale(identifier: "en_US_POSIX"), n) + suffix
}
public func interpolate(values: [Double], f: Float) -> Double {
    let n = values.count
    if n == 0 { return 0 }; if n == 1 { return values[0] }
    let f = clamp(f, 0, Float(n - 1)), i = f.isNaN ? 0 : min(Int(floor(f)), n - 2), t = Double(f - Float(i))
    func at(_ k: Int) -> Double { values[clamp(k, 0, n - 1)] }
    let p0 = at(i - 1), p1 = at(i), p2 = at(i + 1), p3 = at(i + 2)
    let v = 0.5 * (2 * p1 + (-p0 + p2) * t + (2 * p0 - 5 * p1 + 4 * p2 - p3) * t * t + (-p0 + 3 * p1 - 3 * p2 + p3) * t * t * t)
    return clamp(v, min(p1, p2), max(p1, p2))
}
public func guide() -> Rgba { Tone.grey.shade(0.55).fade(0.45) }
public enum CartesianKind: String, CaseIterable, Sendable { case area, line, bar }
public struct Cartesian: Sendable {
    public var kind: CartesianKind, series: [Series]
    public var stacked = false
    public var progress: Float = 1
    public var hover: Int?, selected: Int?
    public var bloom: Bloom = .off
    public var yMax: Double?
    public var gridLines = true
    public var tickCount = 4
    public var baseline = true
    public init(kind: CartesianKind, series: [Series]) { self.kind = kind; self.series = series }
    public var count: Int { series.map(\.values.count).max() ?? 0 }
    public var isEmpty: Bool { count == 0 }
    public func len() -> Int { count }
    public func scale() -> Scale {
        let visible = series.filter { !$0.hidden }
        let maximum: Double
        if let yMax { maximum = yMax }
        else if stacked { maximum = (0..<count).map { i in visible.reduce(0) { $0 + ($1.values.indices.contains(i) ? $1.values[i] : 0) } }.reduce(0, max) }
        else { maximum = visible.flatMap(\.values).reduce(0, max) }
        return .nice(max: maximum, count: tickCount)
    }
    public func columnX(_ i: Int, w: Int) -> Float {
        let n = max(count, 1)
        if kind == .bar { return (Float(i) + 0.5) * Float(w) / Float(n) }
        if n == 1 { return Float(w) / 2 }
        return Float(i) * Float(max(w - 1, 0)) / Float(n - 1)
    }
    public func indexAt(_ x: Float, w: Int) -> Int? {
        let n = count
        guard n > 0 && w > 0 else { return nil }
        let i = kind == .bar ? floor(x / Float(w) * Float(n)) : (x / Float(max(w, 2) - 1) * Float(n - 1)).rounded()
        if i.isNaN || i <= 0 { return 0 }
        if i >= Float(n - 1) { return n - 1 }
        return Int(i)
    }
    private func alphaFor(_ index: Int) -> Float { selected != nil && selected != index ? 0.3 : 1 }
    private func stackBelow(_ si: Int, _ i: Int) -> Double { series[..<si].filter { !$0.hidden }.reduce(0) { $0 + ($1.values.indices.contains(i) ? $1.values[i] : 0) } }
    public func encode(w: Int, h: Int) -> Program {
        var p = Program()
        guard w >= 2 && h >= 2 else { return p }
        let scale = scale()
        if gridLines { for t in scale.ticks.dropFirst() { let y = pixel(scale.y(t, h: h).rounded()); p.line((0, y), (w - 1, y), color: guide(), dash: 3) } }
        if baseline { p.line((0, h - 1), (w - 1, h - 1), color: guide(), dash: 2) }
        if kind == .bar { encodeBars(&p, scale, w, h) } else { encodeCurves(&p, scale, w, h) }
        if kind != .bar, let hover, hover >= 0 && hover < count {
            if bloom == .off { p.bloom(0, 0) } else { bloom.emit(&p) }
            encodeHover(&p, scale, w, h)
        } else { bloom.emit(&p) }
        return p
    }
    private func encodeCurves(_ p: inout Program, _ scale: Scale, _ w: Int, _ h: Int) {
        let n = count, reveal = clamp(progress, 0, 1) * Float(w)
        let grow = 0.6 + 0.4 * Double(clamp(progress, 0, 1))
        var base = [Double](repeating: 0, count: w)
        var cols = [[Float]](repeating: [Float](repeating: NONE, count: w * 2), count: series.count)
        for x in 0..<w {
            if Float(x) > reveal { break }
            let f = n > 1 ? Float(x) / Float(w - 1) * Float(n - 1) : 0
            for (si, s) in series.enumerated() where !s.hidden {
                let v = interpolate(values: s.values, f: f) * grow, b = stacked ? base[x] : 0
                cols[si][x * 2] = scale.y(b + v, h: h); cols[si][x * 2 + 1] = scale.y(b, h: h)
                if stacked { base[x] += v }
            }
        }
        let strength: Float = kind == .area ? 1 : 0.28
        let offsets: [Float?] = series.enumerated().map { si, s in s.hidden ? nil : p.table(cols[si]) }
        for (si, s) in series.enumerated() {
            guard let off = offsets[si] else { continue }
            p.push(Op.areaBody, [off, Float(w), Float(h), s.fill.id()] + tone(s.tone) + [strength, alphaFor(si)])
        }
        for (si, s) in series.enumerated() {
            guard let off = offsets[si] else { continue }
            p.push(Op.areaStroke, [off, Float(w)] + rgba(s.tone.shade(0.78).fade(alphaFor(si))))
        }
        if kind == .line && w / max(n, 1) >= 6 {
            for (si, s) in series.enumerated() where !s.hidden {
                for (i, v) in s.values.enumerated() {
                    let x = columnX(i, w: w)
                    if x > reveal { break }
                    let b = stacked ? stackBelow(si, i) : 0
                    let y = pixel(scale.y((b + v) * grow, h: h).rounded()), c = s.tone.shade(0.78).fade(alphaFor(si))
                    p.dot(pixel(x.rounded()), y, ring: c, center: c)
                }
            }
        }
    }
    private func encodeHover(_ p: inout Program, _ scale: Scale, _ w: Int, _ h: Int) {
        guard let i = hover, i >= 0 && i < count else { return }
        let x = pixel(columnX(i, w: w).rounded())
        p.line((x, 0), (x, h - 1), color: Tone.grey.shade(0.75).fade(0.7), dash: 2)
        for (si, s) in series.enumerated() where !s.hidden && s.values.indices.contains(i) {
            let b = stacked ? stackBelow(si, i) : 0
            let y = pixel(scale.y(b + s.values[i], h: h).rounded())
            p.dot(x, y, ring: s.tone.shade(0.7), center: s.tone.shade(0.98))
        }
    }
    private func encodeBars(_ p: inout Program, _ scale: Scale, _ w: Int, _ h: Int) {
        let n = count
        guard n > 0 else { return }
        let visible = series.indices.filter { !series[$0].hidden }, k = stacked ? 1 : max(visible.count, 1)
        let band = Float(w) / Float(n), group = max(band * 0.72, 1)
        let gap: Float = group / Float(k) > 3 ? 1 : 0
        let barW = max((group - gap * (Float(k) - 1)) / Float(k), 1)
        let prog = Double(clamp(progress, 0, 1))
        for i in 0..<n {
            let x0 = Float(i) * band + (band - group) / 2
            let bandAlpha: Float = hover != nil && hover != i ? 0.45 : 1
            var base = 0.0
            for (slot, si) in visible.enumerated() {
                let s = series[si], v = (s.values.indices.contains(i) ? s.values[i] : 0) * prog
                let left = stacked ? x0 : x0 + Float(slot) * (barW + gap)
                let bx0 = pixel(left.rounded()), bx1 = pixel((left + barW).rounded()) - 1
                p.push(Op.bar, [Float(bx0), Float(bx1), scale.y(base + v, h: h), Float(pixel(scale.y(base, h: h))), Float(h), s.fill.id()] + tone(s.tone) + [alphaFor(si) * bandAlpha])
                if stacked { base += v }
            }
        }
    }
}
public func sparkline(values: [Double], tone: Tone, w: Int, h: Int, hovered: Bool, progress: Float) -> Program {
    var c = Cartesian(kind: .area, series: [Series(label: "", tone: tone, values: values)])
    c.gridLines = false; c.baseline = false; c.progress = progress; c.bloom = hovered ? .low : .off
    let lo = values.reduce(Double.infinity, min), hi = values.reduce(0, max)
    c.yMax = max(hi * 1.1, lo + 1e-9)
    return c.encode(w: w, h: h)
}
public struct Slice: Equatable, Sendable {
    public var label: String, value: Double, tone: Tone, fill: Fill
    public init(label: String, value: Double, tone: Tone, fill: Fill = .gradient) { self.label = label; self.value = value; self.tone = tone; self.fill = fill }
}
public struct PieGeometry: Sendable {
    public var center: (Float, Float), radius: Float, inner: Float, angles: [(Float, Float)]
    public func indexAt(_ x: Float, y: Float) -> Int? {
        let dx = x - center.0, dy = y - center.1, r = sqrt(dx * dx + dy * dy)
        if r < inner || r > radius + 2 { return nil }
        let a = remEuclid(atan2(dx, -dy), irTau)
        return angles.firstIndex { a >= $0.0 && a < $0.1 }
    }
}
public struct Pie: Sendable {
    public var slices: [Slice]
    public var innerRadius: Float = 0
    public var progress: Float = 1
    public var hover: Int?
    public var bloom: Bloom = .off
    public init(slices: [Slice]) { self.slices = slices }
    public func geometry(w: Int, h: Int) -> PieGeometry {
        func positive(_ value: Double) -> Double { value.isNaN ? 0 : max(value, 0) }
        let total = slices.reduce(0) { $0 + positive($1.value) }
        let sweep = irTau * clamp(progress, 0, 1)
        var acc: Float = 0
        let angles: [(Float, Float)] = slices.map { s in
            let a = total > 0 ? Float(positive(s.value) / total) * sweep : 0
            let result = (acc, acc + a); acc += a; return result
        }
        let radius = max(Float(min(w, h)) / 2 - 2.5, 1)
        return PieGeometry(center: (Float(w) / 2, Float(h) / 2), radius: radius, inner: radius * clamp(innerRadius, 0, 0.95), angles: angles)
    }
    public func encode(w: Int, h: Int) -> Program {
        let geo = geometry(w: w, h: h)
        var p = Program(), table: [Float] = []
        for (s, angles) in zip(slices, geo.angles) { table += [angles.0, angles.1] + tone(s.tone) + [s.fill.id()] }
        let offset = p.table(table)
        p.push(Op.pie, [geo.center.0, geo.center.1, geo.radius, geo.inner, slices.count > 1 ? 1 : 0, hover.map(Float.init) ?? -1, Float(slices.count), offset])
        bloom.emit(&p)
        return p
    }
}
public struct RadarGeometry: Sendable {
    public var center: (Float, Float), radius: Float, axisTips: [(Float, Float)]
}
public struct Radar: Sendable {
    public var series: [Series]
    public var max: Double?
    public var progress: Float = 1
    public var selected: Int?
    public var rings = 4
    public var bloom: Bloom = .off
    public init(series: [Series]) { self.series = series }
    public var axes: Int { series.map(\.values.count).max() ?? 0 }
    public func geometry(w: Int, h: Int) -> RadarGeometry {
        let n = Swift.max(axes, 1), center: (Float, Float) = (Float(w) / 2, Float(h) / 2)
        let radius = Swift.max(Float(Swift.min(w, h)) / 2 - 2, 1)
        let tips: [(Float, Float)] = (0..<n).map { i in
            let a = -irHalfPi + irTau * Float(i) / Float(n)
            return (center.0 + cos(a) * radius, center.1 + sin(a) * radius)
        }
        return RadarGeometry(center: center, radius: radius, axisTips: tips)
    }
    public func encode(w: Int, h: Int) -> Program {
        let geo = geometry(w: w, h: h), n = axes
        var p = Program()
        guard n >= 3 else { return p }
        let c = (pixel(geo.center.0.rounded()), pixel(geo.center.1.rounded()))
        func at(_ i: Int, _ f: Float) -> (Int, Int) {
            let (tx, ty) = geo.axisTips[i % n]
            return (pixel((geo.center.0 + (tx - geo.center.0) * f).rounded()), pixel((geo.center.1 + (ty - geo.center.1) * f).rounded()))
        }
        for ring in 1...Swift.max(rings, 1) {
            let f = Float(ring) / Float(Swift.max(rings, 1))
            for i in 0..<n { p.line(at(i, f), at(i + 1, f), color: guide(), dash: 2) }
        }
        for i in 0..<n { p.line(c, at(i, 1), color: guide(), dash: 3) }
        let maximum = max ?? Scale.nice(max: series.flatMap(\.values).reduce(0, Swift.max), count: rings).max
        let prog = clamp(progress, 0, 1)
        for (si, s) in series.enumerated() where !s.hidden {
            let alpha: Float = selected != nil && selected != si ? 0.3 : 1
            let pts: [(Float, Float)] = (0..<n).map { i in
                let f = Float(clamp((s.values.indices.contains(i) ? s.values[i] : 0) / maximum, 0, 1)) * prog
                let (tx, ty) = geo.axisTips[i]
                return (geo.center.0 + (tx - geo.center.0) * f, geo.center.1 + (ty - geo.center.1) * f)
            }
            let offset = p.table(pts.flatMap { [$0.0, $0.1] })
            p.push(Op.poly, [geo.center.0, geo.center.1, geo.radius, s.fill.id()] + tone(s.tone) + [alpha, Float(n), offset])
            let stroke = s.tone.shade(0.8).fade(alpha)
            for i in 0..<n {
                let a = (pixel(pts[i].0.rounded()), pixel(pts[i].1.rounded()))
                let b = (pixel(pts[(i + 1) % n].0.rounded()), pixel(pts[(i + 1) % n].1.rounded()))
                p.line(a, b, color: stroke, dash: 1)
            }
            for (x, y) in pts { p.dot(pixel(x.rounded()), pixel(y.rounded()), ring: stroke, center: s.tone.shade(0.97).fade(alpha)) }
        }
        bloom.emit(&p)
        return p
    }
}
