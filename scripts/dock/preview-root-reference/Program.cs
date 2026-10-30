using System;
using System.Globalization;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Layout;
using Avalonia.Themes.Simple;
using Avalonia.VisualTree;

sealed class ProbeApp : Application
{
    public override void Initialize() => Styles.Add(new SimpleTheme());
}
partial class Program
{
    [STAThread]
    static void Main()
    {
        CultureInfo.CurrentCulture = CultureInfo.InvariantCulture;
        AppBuilder.Configure<ProbeApp>()
            .UseHeadless(new AvaloniaHeadlessPlatformOptions { UseHeadlessDrawing = true })
            .SetupWithoutStarting();
        Console.WriteLine("case\tscale\troot\tx\ty\texpected_x\texpected_y");
        Border layer = new()
        {
            Width = 300, Height = 200, Margin = new Thickness(17, 23, 0, 0),
            HorizontalAlignment = HorizontalAlignment.Left, VerticalAlignment = VerticalAlignment.Top
        };
        Point detached = GetManagedPosition(layer, new PixelPoint(44, 55), new PixelPoint(3, -5));
        if (detached != default) throw new InvalidOperationException("Detached source guard changed.");
        Console.WriteLine("detached\t1\tnull\t0\t0\t0\t0");
        Window window = new() { Width = 400, Height = 300, Position = new PixelPoint(100, 200), Content = layer };
        window.Show();
        try
        {
            foreach (double scale in new[] { 1.0, 2.0 })
            {
                window.SetRenderScaling(scale);
                window.UpdateLayout();
                Capture("attached", window, layer);
            }
            window.Hide();
            Capture("hidden", window, layer);
        }
        finally { window.Close(); }
    }
    static void Capture(string name, Window window, Visual layer)
    {
        PixelPoint pointer = layer.PointToScreen(new Point(41, 52));
        PixelPoint offset = new(3, -5);
        Point actual = GetManagedPosition(layer, pointer, offset);
        var root = layer.GetVisualRoot();
        Point expected = root is TopLevel
            ? layer.PointToClient(new PixelPoint(pointer.X + offset.X, pointer.Y + offset.Y))
            : default;
        if (actual != expected) throw new InvalidOperationException($"Source coordinates changed: {actual} != {expected}");
        string rootName = root is null ? "null" : root.GetType().Name;
        Console.WriteLine($"{name}\t{window.RenderScaling:R}\t{rootName}\t{actual.X:R}\t{actual.Y:R}\t{expected.X:R}\t{expected.Y:R}");
    }
}
