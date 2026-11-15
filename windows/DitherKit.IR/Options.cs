namespace DitherKit.IR;
public enum Matrix { Bayer2, Bayer4, Bayer8, Halftone, Noise }
public enum Fill { Gradient, Dotted, Hatched, Solid }
public enum Bloom { Off, Low, High, Aura }
public enum Direction { Up, Down, Left, Right, Radial }
public enum Field { Plasma, Waves, Ripple, Noise, Aurora }
public enum ButtonStyle { Solid, Outline, Ghost }
public enum CartesianKind { Area, Line, Bar }

public static class OptionExtensions
{
    public static (int Radius, float Strength) Parameters(this Bloom bloom) => bloom switch
    { Bloom.Low => (1, .45f), Bloom.High => (2, .6f), Bloom.Aura => (4, .7f), _ => (0, 0) };
    internal static void Emit(this Bloom bloom, DitherProgram program)
    {
        if (bloom == Bloom.Off) return;
        var (radius, strength) = bloom.Parameters(); program.Bloom(radius, strength);
    }
    public static float Density(this ButtonStyle style, bool hovered, bool pressed)
    {
        var (rest, hover, press) = style switch
        { ButtonStyle.Solid => (.32f, .55f, .85f), ButtonStyle.Outline => (0f, .3f, .6f), _ => (0f, .25f, .5f) };
        return pressed ? press : hovered ? hover : rest;
    }
    public static Rgba? Surface(this ButtonStyle style, Tone tone) => style == ButtonStyle.Solid ? tone.Shade(.22f) : null;
}
public sealed record Gradient(Tone From)
{
    public Tone? To { get; set; }
    public Direction Direction { get; set; } = Direction.Up;
    public Matrix Matrix { get; set; } = Matrix.Bayer8;
    public float Density { get; set; } = 1;
    public DitherProgram Encode(int w, int h)
    {
        var p = new DitherProgram();
        p.Push(Op.Gradient, w, h, (float)Direction, (float)Matrix, Numeric.Clamp(Density), From.Hue, From.Chroma,
            To.HasValue ? 1 : 0, To?.Hue ?? 0, To?.Chroma ?? 0);
        return p;
    }
}
public sealed record Background(Field Field, Tone Tone)
{
    public Tone? Accent { get; set; }
    public Matrix Matrix { get; set; } = Matrix.Bayer8;
    public float Density { get; set; } = .6f;
    public DitherProgram Encode(int w, int h, float time)
    {
        var p = new DitherProgram();
        p.Push(Op.Background, w, h, (float)Field, (float)Matrix, Numeric.Clamp(Density), time, Tone.Hue, Tone.Chroma,
            Accent.HasValue ? 1 : 0, Accent?.Hue ?? 0, Accent?.Chroma ?? 0);
        return p;
    }
}
public sealed record Series(string Label, Tone Tone, IReadOnlyList<double> Values)
{
    public Fill Fill { get; set; } = Fill.Gradient;
    public bool Hidden { get; set; }
}
public sealed record Slice(string Label, double Value, Tone Tone)
{
    public Fill Fill { get; set; } = Fill.Gradient;
}
public sealed record Scale(double Max, IReadOnlyList<double> Ticks)
{
    public static Scale Nice(double max, int count)
    {
        max = double.IsFinite(max) && max > 0 ? max : 1;
        var raw = max / Math.Max(count, 1);
        var mag = Math.Pow(10, Math.Floor(Math.Log10(raw)));
        var step = new[] { 1d, 2d, 2.5, 5d, 10d }.Select(m => m * mag).FirstOrDefault(s => s >= raw, 10d * mag);
        var top = Math.Ceiling(max / step) * step;
        return new(top, Enumerable.Range(0, (int)Math.Round(top / step, MidpointRounding.AwayFromZero) + 1).Select(i => i * step).ToArray());
    }
    public float Y(double value, int h)
    {
        var span = (float)Math.Max(h - 1, 0);
        return span - (float)Math.Clamp(value / Max, 0d, 1d) * span;
    }
}
