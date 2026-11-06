using System.Globalization;
using Microsoft.UI.Xaml.Media;
using Path = Microsoft.UI.Xaml.Shapes.Path;
using ScreenKeyboard.Keys;
using Windows.Foundation;

namespace ScreenKeyboard.Controls.Icons;

/// <summary>
/// Creates vector icons for <see cref="KeyIcon"/> values. Icons are Material Symbols (Apache-2.0) converted to
/// resolution independent path data, so they render identically on every Uno Platform target without icon fonts.
/// </summary>
public static class IconFactory
{
    private static readonly Dictionary<KeyIcon, List<Command>> s_parsed = new();
    private static readonly Dictionary<KeyIcon, string> s_custom = new();

    /// <summary>
    /// Registers custom path data for an icon. The path must use absolute <c>M</c>, <c>L</c>, <c>C</c> and <c>Z</c>
    /// commands in a 0..1 coordinate box.
    /// </summary>
    public static void Register(KeyIcon icon, string normalizedPath)
    {
        lock (s_parsed)
        {
            s_custom[icon] = normalizedPath;
            s_parsed.Remove(icon);
        }
    }

    /// <summary>Returns <c>true</c> if path data exists for the icon.</summary>
    public static bool HasIcon(KeyIcon icon) => icon != KeyIcon.None && (s_custom.ContainsKey(icon) || IconData.Paths.ContainsKey(icon));

    /// <summary>Creates a <see cref="Path"/> of <paramref name="size"/> × <paramref name="size"/> DIPs.</summary>
    public static Path Create(KeyIcon icon, double size, Brush? fill)
    {
        return new Path
        {
            Data = CreateGeometry(icon, size),
            Fill = fill,
            Width = size,
            Height = size,
            Stretch = Stretch.None,
            IsHitTestVisible = false,
        };
    }

    /// <summary>Creates the geometry of an icon scaled to <paramref name="size"/>.</summary>
    public static Geometry CreateGeometry(KeyIcon icon, double size)
    {
        var commands = GetCommands(icon);
        var geometry = new PathGeometry { FillRule = FillRule.Nonzero };
        PathFigure? figure = null;
        foreach (var c in commands)
        {
            switch (c.Kind)
            {
                case 'M':
                    figure = new PathFigure { StartPoint = P(c.X, c.Y, size), IsClosed = false, IsFilled = true };
                    geometry.Figures.Add(figure);
                    break;
                case 'L':
                    figure?.Segments.Add(new LineSegment { Point = P(c.X, c.Y, size) });
                    break;
                case 'C':
                    figure?.Segments.Add(new BezierSegment
                    {
                        Point1 = P(c.X1, c.Y1, size),
                        Point2 = P(c.X2, c.Y2, size),
                        Point3 = P(c.X, c.Y, size),
                    });
                    break;
                case 'Z':
                    if (figure is not null)
                    {
                        figure.IsClosed = true;
                    }
                    break;
            }
        }
        return geometry;
    }

    private static Point P(double x, double y, double size) => new(x * size, y * size);

    private static List<Command> GetCommands(KeyIcon icon)
    {
        lock (s_parsed)
        {
            if (s_parsed.TryGetValue(icon, out var cached))
            {
                return cached;
            }
            var data = s_custom.TryGetValue(icon, out var custom) ? custom : IconData.Paths.TryGetValue(icon, out var builtIn) ? builtIn : string.Empty;
            var parsed = Parse(data);
            s_parsed[icon] = parsed;
            return parsed;
        }
    }

    private static List<Command> Parse(string data)
    {
        var result = new List<Command>();
        var tokens = data.Split(' ', StringSplitOptions.RemoveEmptyEntries);
        var i = 0;
        double N() => double.Parse(tokens[i++], CultureInfo.InvariantCulture);
        while (i < tokens.Length)
        {
            var t = tokens[i++];
            switch (t)
            {
                case "M":
                case "L":
                    result.Add(new Command(t[0], 0, 0, 0, 0, N(), N()));
                    break;
                case "C":
                {
                    var x1 = N(); var y1 = N(); var x2 = N(); var y2 = N(); var x = N(); var y = N();
                    result.Add(new Command('C', x1, y1, x2, y2, x, y));
                    break;
                }
                case "Z":
                    result.Add(new Command('Z', 0, 0, 0, 0, 0, 0));
                    break;
            }
        }
        return result;
    }

    private readonly record struct Command(char Kind, double X1, double Y1, double X2, double Y2, double X, double Y);
}
