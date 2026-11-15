using System.Text.Json;
using System.Diagnostics;
using DitherKit.WinUI;
using Microsoft.Graphics.Canvas;
using Windows.Graphics.Imaging;
using Xunit;
using Xunit.Abstractions;

namespace DitherKit.WinUI.Tests;

public sealed class GoldenTests(ITestOutputHelper output)
{
    [Fact]
    public async Task EveryFixtureRendersThroughDirect2DWithinGoldenTolerance()
    {
        // No XAML window or dispatcher is involved. Force software WARP so the
        // same test also runs on hosted CI machines without a physical GPU.
        using var device = new CanvasDevice(true);
        string scenesPath = Path.GetFullPath(Path.Combine(AppContext.BaseDirectory, "fixtures", "scenes.json"));
        using var scenes = JsonDocument.Parse(File.ReadAllText(scenesPath));
        int count = 0;
        foreach (var scene in scenes.RootElement.GetProperty("scenes").EnumerateArray())
        {
            string name = scene.GetProperty("name").GetString()!;
            int w = scene.GetProperty("w").GetInt32(), h = scene.GetProperty("h").GetInt32();
            float[] program = scene.GetProperty("program").EnumerateArray().Select(v => v.GetSingle()).ToArray();
            using var rendered = DitherRenderer.Render(device, program, w, h, straightAlpha: true);
            var pixels = rendered.GetPixelBytes(); // DXGI R8G8B8A8, straight alpha
            string pngPath = Path.GetFullPath(Path.Combine(AppContext.BaseDirectory, "fixtures", "png", $"{name}.png"));
            output.WriteLine($"Loading golden PNG: {pngPath}; exists={File.Exists(pngPath)}");
            Assert.True(File.Exists(pngPath), $"Golden image is missing: {pngPath}");
            using var file = File.OpenRead(pngPath);
            using var stream = file.AsRandomAccessStream();
            var decoder = await BitmapDecoder.CreateAsync(stream);
            Assert.Equal((uint)w, decoder.PixelWidth);
            Assert.Equal((uint)h, decoder.PixelHeight);
            var expected = (await decoder.GetPixelDataAsync(BitmapPixelFormat.Rgba8, BitmapAlphaMode.Straight,
                new BitmapTransform(), ExifOrientationMode.IgnoreExifOrientation, ColorManagementMode.DoNotColorManage)).DetachPixelData();
            int different = 0, maximum = 0;
            for (int y = 0; y < h; y++) for (int x = 0; x < w; x++)
            {
                int i = (y * w + x) * 4;
                int delta = Math.Max(Math.Max(Math.Abs(pixels[i] - expected[i]), Math.Abs(pixels[i + 1] - expected[i + 1])),
                    Math.Max(Math.Abs(pixels[i + 2] - expected[i + 2]), Math.Abs(pixels[i + 3] - expected[i + 3])));
                if (delta != 0) different++;
                maximum = Math.Max(maximum, delta);
            }
            Assert.True(maximum <= 2, $"{name}: maximum channel error {maximum} > 2");
            Assert.True(different <= w * h * .01, $"{name}: {different}/{w * h} differing pixels > 1%");
            count++;
        }
        Assert.Equal(39, count);
        if (Environment.GetEnvironmentVariable("DITHER_PROFILE") == "1")
            Profile(device, scenes.RootElement.GetProperty("scenes"));
    }

    // Opt-in diagnostics only: correctness above always checks all 39 scenes.
    private void Profile(CanvasDevice device, JsonElement scenes)
    {
        string[] names = ["gradient_2", "background_plasma", "area_2", "line_4", "pie_2", "radar_1"];
        foreach (string name in names)
        {
            var scene = scenes.EnumerateArray().Single(s => s.GetProperty("name").GetString() == name);
            int w = scene.GetProperty("w").GetInt32(), h = scene.GetProperty("h").GetInt32();
            var program = scene.GetProperty("program").EnumerateArray().Select(v => v.GetSingle()).ToArray();
            void RenderAndSynchronize()
            {
                using var target = DitherRenderer.Render(device, program, w, h, straightAlpha: true);
                _ = target.GetPixelBytes(); // Readback completes GPU work before stopping the clock.
            }
            for (int i = 0; i < 10; i++) RenderAndSynchronize();
            const int rendersPerSample = 25;
            double[] samples = new double[7];
            for (int sample = 0; sample < samples.Length; sample++)
            {
                var clock = Stopwatch.StartNew();
                for (int i = 0; i < rendersPerSample; i++) RenderAndSynchronize();
                clock.Stop();
                samples[sample] = clock.Elapsed.TotalMicroseconds / rendersPerSample;
            }
            double median = samples.Order().ElementAt(samples.Length / 2);
            output.WriteLine("DITHER_PROFILE " + JsonSerializer.Serialize(new
            {
                scene = name, width = w, height = h, renderer = "WARP", rendersPerSample,
                clock = "Synchronized wall time: allocation, upload, rendering and readback; not isolated GPU time",
                median_us = median, samples_us = samples,
                revision = Environment.GetEnvironmentVariable("GITHUB_SHA") ?? "local"
            }));
        }
    }
}
