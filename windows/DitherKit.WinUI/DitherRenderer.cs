using System.Runtime.InteropServices;
using ComputeSharp.D2D1;
using ComputeSharp.D2D1.Interop;
using ComputeSharp.D2D1.WinUI;
using DitherKit.WinUI.Shaders;
using Microsoft.Graphics.Canvas;
using Windows.Graphics.DirectX;
using Windows.UI;

namespace DitherKit.WinUI;

/// <summary>Grid-resolution renderer shared by controls and windowless GPU tests.</summary>
public static class DitherRenderer
{
    /// <remarks>
    /// straightAlpha is a readback-only path: the target's raw bytes contain
    /// straight alpha although its D2D container is marked premultiplied, avoiding
    /// lossy un-premultiplication of eight-bit golden pixels. Do not composite it.
    /// </remarks>
    public static CanvasRenderTarget Render(CanvasDevice device, float[] program, int width, int height, bool straightAlpha = false)
    {
        ArgumentNullException.ThrowIfNull(program);
        if (program.Length == 0 || width < 1 || height < 1) throw new ArgumentOutOfRangeException(nameof(program));
        // ComputeSharp 3.2.0 API verified against the upstream v3.2.0 source:
        // https://github.com/Sergio0694/ComputeSharp/blob/v3.2.0/src/ComputeSharp.D2D1/Shaders/Interop/D2D1ResourceTextureManager.cs
        var manager = new D2D1ResourceTextureManager(
            extents: [(uint)program.Length],
            bufferPrecision: D2D1BufferPrecision.Float32,
            channelDepth: D2D1ChannelDepth.One,
            filter: D2D1Filter.MinMagMipPoint,
            extendModes: [D2D1ExtendMode.Clamp],
            data: MemoryMarshal.AsBytes(program.AsSpan()), strides: []);
        using var effect = new PixelShaderEffect<DitherShader>
        {
            ConstantBuffer = new DitherShader(width, height, straightAlpha),
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
