using System.Runtime.InteropServices;
using ComputeSharp.D2D1;
using ComputeSharp.D2D1.Interop;
using ComputeSharp.D2D1.WinUI;
using DitherKit.WinUI.Shaders;
using Microsoft.Graphics.Canvas;
using Windows.Graphics.DirectX;
using Windows.UI;

namespace DitherKit.WinUI.Tests;

/// <summary>
/// Profiling-only copy of DitherRenderer's allocation/upload/draw pipeline.
/// The legacy constructor leaves programSplit at zero, selecting the GPU scan
/// in the CURRENT shader. All other current shader optimizations remain shared.
/// </summary>
internal static class LegacyProfileRenderer
{
    public static CanvasRenderTarget Render(CanvasDevice device, float[] program, int width, int height)
    {
        ArgumentNullException.ThrowIfNull(program);
        if (program.Length == 0 || width < 1 || height < 1) throw new ArgumentOutOfRangeException(nameof(program));
        var manager = new D2D1ResourceTextureManager(
            extents: [(uint)program.Length],
            bufferPrecision: D2D1BufferPrecision.Float32,
            channelDepth: D2D1ChannelDepth.One,
            filter: D2D1Filter.MinMagMipPoint,
            extendModes: [D2D1ExtendMode.Clamp],
            data: MemoryMarshal.AsBytes(program.AsSpan()), strides: []);
        using var effect = new PixelShaderEffect<DitherShader>
        {
            ConstantBuffer = new DitherShader(width, height, true),
            ResourceTextureManagers = { [0] = manager }
        };
        var target = new CanvasRenderTarget(device, width, height, 96,
            DirectXPixelFormat.R8G8B8A8UIntNormalized, CanvasAlphaMode.Premultiplied);
        try
        {
            using var draw = target.CreateDrawingSession();
            draw.Clear(Color.FromArgb(0, 0, 0, 0));
            draw.Blend = CanvasBlend.Copy;
            draw.DrawImage(effect, new Windows.Foundation.Rect(0, 0, width, height), new Windows.Foundation.Rect(0, 0, width, height));
            return target;
        }
        catch { target.Dispose(); throw; }
    }
}
