using System.Globalization;
using System.Text.Json;
using Microsoft.UI;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Controls.Primitives;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Windows.Foundation;
using Windows.System;

namespace DitherKit.WinUI;

public record DitherSeries(string Label, IReadOnlyList<double> Values, string Tone = "blue", string Fill = "gradient");
public record DitherSlice(string Label, double Value, string Tone = "blue", string Fill = "gradient");

public class DitherView : UserControl
{
    public DitherSurface Surface { get; } = new();
    public double Cell { get => Surface.Cell; set { Surface.Cell = value; RefreshChrome(); } }
    public Dictionary<string, object?> Properties { get => Surface.Properties; set { Surface.Properties = value; RefreshChrome(); } }
    public string AccessibleLabel { get => AutomationProperties.GetName(this); set => AutomationProperties.SetName(this, value); }
    protected Grid Root { get; } = new();
    protected DitherView(string component, string name)
    {
        Surface.Component = component;
        Root.Children.Add(Surface);
        Content = Root;
        AccessibleLabel = name;
        MinWidth = 24; MinHeight = 24;
    }
    public void SetOption(string name, object? value) { Surface.SetOption(name, value); RefreshChrome(); }
    protected virtual void RefreshChrome() { }
}

public sealed class DitherGradient : DitherView { public DitherGradient() : base("gradient", "Dither gradient") { } }
public sealed class DitherBackground : DitherView
{
    public double Speed { get => Surface.Speed; set => Surface.Speed = value; }
    public DitherBackground() : base("background", "Animated dither background") { Surface.Animate = true; }
}
public sealed class DitherAvatar : DitherView
{
    public string Text { get => Properties.GetValueOrDefault("text")?.ToString() ?? "Dither"; set { SetOption("text", value); AccessibleLabel = $"Avatar for {value}"; } }
    public DitherAvatar() : base("avatar", "Dither avatar") { Width = Height = 64; Text = "Dither"; }
}
public sealed class DitherSwatch : DitherView { public DitherSwatch() : base("swatch", "Color swatch") { Width = Height = 15; } }

public sealed class DitherButton : Button
{
    public DitherSurface Surface { get; } = new() { Component = "button", Animate = true };
    private readonly TextBlock label = new() { HorizontalAlignment = HorizontalAlignment.Center, VerticalAlignment = VerticalAlignment.Center };
    private bool hovered;
    private string ditherStyle = "solid";
    public string Text { get => label.Text; set { label.Text = value; AutomationProperties.SetName(this, value); } }
    public string DitherStyle { get => ditherStyle; set { ditherStyle = value; Surface.SetOption("style", value); UpdateDensity(); } }
    public string Tone { get => Surface.Properties.GetValueOrDefault("tone")?.ToString() ?? "green"; set { Surface.SetOption("tone", value); UpdateDensity(); } }
    public DitherButton()
    {
        var grid = new Grid(); grid.Children.Add(Surface); grid.Children.Add(label); Content = grid;
        HorizontalContentAlignment = HorizontalAlignment.Stretch; VerticalContentAlignment = VerticalAlignment.Stretch;
        Padding = new Thickness(0); MinWidth = 120; MinHeight = 42; Text = "Continue";
        PointerEntered += (_, _) => { hovered = true; UpdateDensity(); };
        PointerExited += (_, _) => { hovered = false; UpdateDensity(); };
        RegisterPropertyChangedCallback(IsPressedProperty, (_, _) => UpdateDensity());
        UpdateDensity();
    }
    private void UpdateDensity()
    {
        var style = Enum.Parse<DitherKit.IR.ButtonStyle>(DitherStyle, true);
        double density = DitherKit.IR.OptionExtensions.Density(style, hovered, IsPressed);
        Surface.SetOption("density", density);
        Surface.Animate = density > .4;
        var tint = DitherKit.IR.Tone.Parse(Tone).Shade(.22f);
        Background = new SolidColorBrush(DitherStyle == "solid" ? Windows.UI.Color.FromArgb(tint.A, tint.R, tint.G, tint.B) : Colors.Transparent);
    }
}

public sealed class DitherMeter : DitherView
{
    private readonly TextBlock label = new() { HorizontalAlignment = HorizontalAlignment.Right, VerticalAlignment = VerticalAlignment.Center };
    private readonly DispatcherTimer timer = new() { Interval = TimeSpan.FromMilliseconds(16) };
    private double value, displayed, velocity;
    public double Value
    {
        get => value;
        set { this.value = Math.Clamp(value, 0, 1); label.Text = this.value.ToString("P0", CultureInfo.CurrentCulture); AccessibleLabel = $"Meter {label.Text}";
            if (new Windows.UI.ViewManagement.UISettings().AnimationsEnabled) timer.Start(); else { displayed = this.value; Surface.SetOption("value", displayed); } }
    }
    public DitherMeter() : base("meter", "Meter")
    {
        MinHeight = 24; Surface.Margin = new Thickness(0, 0, 52, 0); Root.Children.Add(label);
        timer.Tick += (_, _) => { if (!new Windows.UI.ViewManagement.UISettings().AnimationsEnabled) { displayed = value; velocity = 0; Surface.SetOption("value", value); timer.Stop(); return; } velocity += (value - displayed) * .12; velocity *= .72; displayed += velocity; Surface.SetOption("value", displayed); if (Math.Abs(value - displayed) + Math.Abs(velocity) < .001) { displayed = value; Surface.SetOption("value", value); timer.Stop(); } };
        Unloaded += (_, _) => timer.Stop();
    }
}

public sealed class DitherSparkline : DitherView
{
    public DitherSparkline() : base("sparkline", "Sparkline")
    {
        Surface.AnimateEntrance = true;
        Background = new SolidColorBrush(Colors.Transparent);
        PointerEntered += (_, _) => SetOption("hovered", true);
        PointerExited += (_, _) => SetOption("hovered", false);
        GotFocus += (_, _) => SetOption("hovered", true);
        LostFocus += (_, _) => SetOption("hovered", false);
        IsTabStop = true;
    }
}

public sealed class DitherLegend : StackPanel
{
    public event EventHandler<int>? Toggled;
    public event EventHandler<int?>? Selected;
    private (string Label, string Tone, bool Visible)[] current = [];
    public DitherLegend() { Orientation = Orientation.Horizontal; Spacing = 12; }
    public void SetItems(IEnumerable<(string Label, string Tone, bool Visible)> items)
    {
        var next = items.ToArray();
        if (next.Length == current.Length && next.Select((x, i) => x.Label == current[i].Label && x.Tone == current[i].Tone).All(x => x))
        {
            for (int i = 0; i < next.Length; i++) ((ToggleButton)Children[i]).IsChecked = next[i].Visible;
            current = next; return;
        }
        current = next;
        Children.Clear(); int index = 0;
        foreach (var item in next)
        {
            int current = index++;
            var content = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 6 };
            var swatch = new DitherSwatch(); swatch.SetOption("tone", item.Tone); content.Children.Add(swatch);
            content.Children.Add(new TextBlock { Text = item.Label });
            var toggle = new ToggleButton { Content = content, IsChecked = item.Visible };
            AutomationProperties.SetName(toggle, $"Show {item.Label}");
            toggle.Click += (_, _) => Toggled?.Invoke(this, current);
            toggle.PointerEntered += (_, _) => Selected?.Invoke(this, current);
            toggle.PointerExited += (_, _) => Selected?.Invoke(this, null);
            toggle.GotFocus += (_, _) => Selected?.Invoke(this, current);
            toggle.LostFocus += (_, _) => Selected?.Invoke(this, null);
            Children.Add(toggle);
        }
    }
}

internal static class ChromeData
{
    private static readonly JsonSerializerOptions options = new() { PropertyNamingPolicy = JsonNamingPolicy.CamelCase };
    public static JsonElement Json(object? value) => JsonSerializer.SerializeToElement(value, options);
    public static bool IsHidden(JsonElement row) => row.TryGetProperty("hidden", out var hidden) && hidden.ValueKind == JsonValueKind.True;
    public static Dictionary<string, object?>[] ToggleHidden(Dictionary<string, object?> props, int index) => Rows(props, "series").Select((r, i) => {
        var row = r.EnumerateObject().ToDictionary(p => p.Name, p => (object?)p.Value.Clone());
        if (i == index) row["hidden"] = !IsHidden(r);
        return row;
    }).ToArray();
    public static bool Bool(Dictionary<string, object?> props, string key) => props.TryGetValue(key, out var v) && Json(v).ValueKind == JsonValueKind.True;
    public static double? Number(Dictionary<string, object?> props, string key) => props.TryGetValue(key, out var v) && Json(v).ValueKind == JsonValueKind.Number ? Json(v).GetDouble() : null;
    public static string Text(JsonElement element, string key, string fallback = "") => element.TryGetProperty(key, out var p) ? p.ToString() : fallback;
    public static List<JsonElement> Rows(Dictionary<string, object?> props, string key)
    {
        if (!props.TryGetValue(key, out var value)) return [];
        var element = Json(value);
        return element.ValueKind == JsonValueKind.Array ? element.EnumerateArray().Select(x => x.Clone()).ToList() : [];
    }
}
