using System.Globalization;
namespace DitherKit.IR;

/// <summary>An 8-bit sRGB color with straight alpha.</summary>
public readonly record struct Rgba(byte R, byte G, byte B, byte A = 255)
{
    public static Rgba Transparent => new(0, 0, 0, 0);
    public static Rgba Black => new(0, 0, 0);
    public static Rgba White => new(255, 255, 255);
    public static Rgba Hex(uint hex) => new((byte)(hex >> 16), (byte)(hex >> 8), (byte)hex);
    public Rgba WithAlpha(byte alpha) => this with { A = alpha };
    public Rgba Fade(float factor) => WithAlpha((byte)Numeric.Round(A * Numeric.Clamp(factor)));
    public Rgba Mix(Rgba other, float t)
    {
        t = Numeric.Clamp(t);
        byte L(byte a, byte b) => (byte)Numeric.Round(a + (b - (float)a) * t);
        return new(L(R, other.R), L(G, other.G), L(B, other.B), L(A, other.A));
    }
    public Rgba Over(Rgba background) => background.WithAlpha(255).Mix(WithAlpha(255), A / 255f);
    public float Luminance()
    {
        static float Lin(byte channel)
        {
            var c = channel / 255f;
            return c <= .04045f ? c / 12.92f : MathF.Pow((c + .055f) / 1.055f, 2.4f);
        }
        return .2126f * Lin(R) + .7152f * Lin(G) + .0722f * Lin(B);
    }
    public static Rgba Oklch(float l, float c, float h)
    {
        // Rust max returns the non-NaN operand when only one operand is NaN.
        c = float.IsNaN(c) ? 0 : MathF.Max(c, 0);
        while (true)
        {
            if (OklchToSrgb(l, c, h) is { } rgb) return rgb;
            c -= .005f;
            if (c <= 0) return OklchToSrgb(l, 0, h) ?? Black;
        }
    }
    private static Rgba? OklchToSrgb(float l, float c, float h)
    {
        var radians = h * (MathF.PI / 180f);
        var a = c * MathF.Cos(radians); var b = c * MathF.Sin(radians);
        var ll = l + .39633778f * a + .21580376f * b;
        var mm = l - .105561346f * a - .06385417f * b;
        var ss = l - .08948418f * a - 1.2914855f * b;
        var l3 = ll * ll * ll; var m3 = mm * mm * mm; var s3 = ss * ss * ss;
        var r = 4.0767417f * l3 - 3.3077116f * m3 + .23096994f * s3;
        var g = -1.268438f * l3 + 2.6097574f * m3 - .34131938f * s3;
        var bb = -.0041960863f * l3 - .7034186f * m3 + 1.7076147f * s3;
        if (r < -1e-4f || r > 1.0001f || g < -1e-4f || g > 1.0001f || bb < -1e-4f || bb > 1.0001f) return null;
        static byte Enc(float v)
        {
            v = Numeric.Clamp(v);
            var s = v <= .0031308f ? 12.92f * v : 1.055f * MathF.Pow(v, 1f / 2.4f) - .055f;
            return (byte)Numeric.Round(s * 255f);
        }
        return new(Enc(r), Enc(g), Enc(bb));
    }
}

/// <summary>A named perceptual shade ramp, or any hue in degrees.</summary>
public readonly record struct Tone
{
    private enum Kind { Green, Blue, Purple, Pink, Orange, Red, Grey, Hue }
    private readonly Kind kind;
    private readonly float customHue;
    private Tone(Kind kind, float customHue = 0) { this.kind = kind; this.customHue = customHue; }
    public static Tone Green => new(Kind.Green);
    public static Tone Blue => new(Kind.Blue);
    public static Tone Purple => new(Kind.Purple);
    public static Tone Pink => new(Kind.Pink);
    public static Tone Orange => new(Kind.Orange);
    public static Tone Red => new(Kind.Red);
    public static Tone Grey => new(Kind.Grey);
    public static IReadOnlyList<Tone> All { get; } = [Green, Blue, Purple, Pink, Orange, Red, Grey];
    public static Tone FromHue(float hue) => new(Kind.Hue, hue);
    public string Name => kind.ToString().ToLowerInvariant();
    public float Hue => kind switch
    {
        Kind.Green => 150, Kind.Blue => 250, Kind.Purple => 300, Kind.Pink => 350,
        Kind.Orange => 55, Kind.Red => 25, Kind.Grey => 260, _ => Numeric.Mod(customHue, 360f)
    };
    public float Chroma => kind == Kind.Grey ? .012f : .19f;
    public static Tone Parse(string value) => TryParse(value, out var tone) ? tone : throw new FormatException($"Unknown tone '{value}'.");
    public static bool TryParse(string value, out Tone tone)
    {
        var text = value.Trim().ToLowerInvariant();
        Tone? named = text switch { "green" => Green, "blue" => Blue, "purple" or "violet" => Purple,
            "pink" => Pink, "orange" => Orange, "red" => Red, "grey" or "gray" => Grey, _ => null };
        if (named is { } n) { tone = n; return true; }
        if (float.TryParse(text, NumberStyles.Float, CultureInfo.InvariantCulture, out var h))
        { tone = FromHue(Numeric.Mod(h, 360)); return true; }
        tone = default; return false;
    }
    public Rgba Shade(float level) => ShadeHc(Hue, Chroma, level);
    public Rgba Base => Shade(.55f);
    public static Rgba ShadeHc(float hue, float chroma, float level)
    {
        level = Numeric.Clamp(level);
        var l = .22f + level * .72f;
        var c = chroma * Math.Clamp(1f - MathF.Abs(level - .55f) * 1.3f, .25f, 1f);
        return Rgba.Oklch(l, c, hue);
    }
}
