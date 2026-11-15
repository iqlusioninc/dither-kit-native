using System.Text.Json;
using DitherKit.WinUI;
using Microsoft.UI;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;

namespace DitherKit.Gallery;

public partial class App : Application
{
    private Window? window;
    public App() { InitializeComponent(); }
    protected override void OnLaunched(LaunchActivatedEventArgs args)
    {
        window = new Window { Title = "Dither Kit · Native WinUI 3" };
        var page = new StackPanel { Spacing = 24, Padding = new Thickness(32), MaxWidth = 1040, HorizontalAlignment = HorizontalAlignment.Left };
        page.Children.Add(new TextBlock { Text = "Dither Kit", FontSize = 40, FontWeight = Microsoft.UI.Text.FontWeights.SemiBold });
        page.Children.Add(new TextBlock { Text = "Native WinUI controls. Pixel textures, live charts, and small moments of motion.", FontSize = 16, TextWrapping = TextWrapping.Wrap });
        var buttons = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 16 };
        foreach (string style in new[] { "solid", "outline", "ghost" })
        {
            var button = new DitherButton { Text = style, DitherStyle = style, Width = 160 };
            button.Click += (_, _) => button.Text = "Clicked"; buttons.Children.Add(button);
        }
        page.Children.Add(buttons);
        var meter = new DitherMeter { Width = 460, Height = 28, Value = .62 };
        var slider = new Slider { Minimum = 0, Maximum = 1, Value = .62, Width = 460, Header = "Meter value", StepFrequency = .01 };
        slider.ValueChanged += (_, e) => meter.Value = e.NewValue;
        page.Children.Add(meter); page.Children.Add(slider);
        using var document = JsonDocument.Parse(File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "scenes.json")));
        var selected = new HashSet<string> { "gradient_0", "background_aurora", "avatar_ada_lovelace", "swatch_blue", "area_0", "line_3", "bar_5", "sparkline_1", "pie_1", "radar_0" };
        foreach (var scene in document.RootElement.GetProperty("scenes").EnumerateArray())
        {
            string name = scene.GetProperty("name").GetString()!;
            if (!selected.Contains(name)) continue;
            string kind = scene.GetProperty("kind").GetString()!;
            DitherView control = kind switch
            {
                "gradient" => new DitherGradient(), "background" => new DitherBackground(), "avatar" => new DitherAvatar(),
                "swatch" => new DitherSwatch(), "cartesian" => new DitherChart { Labels = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug"] },
                "sparkline" => new DitherSparkline(), "pie" => new DitherPie(), "radar" => new DitherRadar { Labels = ["Speed", "Quality", "Range", "Value", "Power", "Ease"] },
                _ => throw new InvalidOperationException(kind)
            };
            // Encode from scene PARAMETERS, never the stored expected bytecode.
            control.Properties = scene.EnumerateObject().Where(p => p.Name != "program").ToDictionary(p => p.Name, p => (object?)p.Value.Clone());
            control.Width = kind == "avatar" ? 80 : Math.Max(240, scene.GetProperty("w").GetInt32() * 4);
            control.Height = kind is "cartesian" or "pie" or "radar" ? 260 : Math.Max(48, scene.GetProperty("h").GetInt32() * 3);
            control.AccessibleLabel = name.Replace('_', ' ');
            var card = new StackPanel { Spacing = 12 };
            card.Children.Add(new TextBlock { Text = name.Replace('_', ' '), FontSize = 20, FontWeight = Microsoft.UI.Text.FontWeights.SemiBold }); card.Children.Add(control);
            page.Children.Add(new Border { Child = card, Padding = new Thickness(20), CornerRadius = new CornerRadius(10), BorderThickness = new Thickness(1), BorderBrush = new SolidColorBrush(Windows.UI.Color.FromArgb(36, 128, 128, 128)) });
        }
        window.Content = new ScrollViewer { Content = page, HorizontalScrollBarVisibility = ScrollBarVisibility.Auto };
        window.Activate();
    }
}
