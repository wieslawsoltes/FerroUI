using ScreenKeyboard.Settings;

namespace ScreenKeyboard.Sample;

/// <summary>
/// Optional start-up options, e.g. <c>http://localhost:5080/?theme=dark&amp;onehanded=right&amp;langs=en-US/qwerty,de-DE/qwertz</c>
/// (WebAssembly query string) or <c>--theme=dark</c> (desktop command line). Handy for demos and screenshots.
/// </summary>
internal static class SampleOptions
{
    public static Dictionary<string, string> Values { get; } = new(StringComparer.OrdinalIgnoreCase);

    public static void Parse(string? arguments)
    {
        if (string.IsNullOrWhiteSpace(arguments))
        {
            return;
        }
        foreach (var part in arguments.TrimStart('?').Split(['&', ' '], StringSplitOptions.RemoveEmptyEntries))
        {
            var kv = part.TrimStart('-').Split('=', 2);
            Values[Uri.UnescapeDataString(kv[0])] = kv.Length > 1 ? Uri.UnescapeDataString(kv[1]) : "true";
        }
    }

    public static string? Get(string key) => Values.TryGetValue(key, out var value) ? value : null;

    public static void Apply(KeyboardSettings settings)
    {
        if (Get("theme") is { } theme && Enum.TryParse<ThemeMode>(theme, true, out var mode)) settings.ThemeMode = mode;
        if (Get("lighttheme") is { } light) settings.LightTheme = light;
        if (Get("darktheme") is { } dark) settings.DarkTheme = dark;
        if (Get("onehanded") is { } oneHanded && Enum.TryParse<OneHandedMode>(oneHanded, true, out var ohm)) settings.OneHandedMode = ohm;
        if (Get("split") is { } split) settings.SplitKeyboard = bool.Parse(split);
        if (Get("floating") is { } floating) settings.Floating = bool.Parse(floating);
        if (Get("numberrow") is { } numberRow) settings.NumberRow = bool.Parse(numberRow);
        if (Get("glide") is { } glide) settings.GlideTyping = bool.Parse(glide);
        if (Get("keyheight") is { } keyHeight) settings.KeyHeight = double.Parse(keyHeight, System.Globalization.CultureInfo.InvariantCulture);
        if (Get("langs") is { } langs) settings.Subtypes = langs.Split(',', StringSplitOptions.RemoveEmptyEntries).ToList();
        if (Get("longpress") is { } longPress) settings.LongPressDelay = int.Parse(longPress, System.Globalization.CultureInfo.InvariantCulture);
    }
}
