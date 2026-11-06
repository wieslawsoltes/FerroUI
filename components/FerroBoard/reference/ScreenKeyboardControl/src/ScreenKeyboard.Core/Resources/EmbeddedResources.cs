using System.IO.Compression;
using System.Reflection;
using System.Text;

namespace ScreenKeyboard.Resources;

/// <summary>
/// Access to the data files bundled with ScreenKeyboard.Core (FlorisBoard layouts, popup mappings,
/// subtype presets, emoji data and the English dictionary). Gzip compressed resources
/// (<c>*.gz</c>) are transparently decompressed.
/// </summary>
public static class EmbeddedResources
{
    private const string Prefix = "ScreenKeyboard.Resources.";
    private static readonly Assembly s_assembly = typeof(EmbeddedResources).Assembly;
    private static readonly Lazy<Dictionary<string, string>> s_index = new(BuildIndex);

    private static Dictionary<string, string> BuildIndex()
    {
        var map = new Dictionary<string, string>(StringComparer.Ordinal);
        foreach (var name in s_assembly.GetManifestResourceNames())
        {
            if (!name.StartsWith(Prefix, StringComparison.Ordinal))
            {
                continue;
            }
            var path = name.Substring(Prefix.Length).Replace('\\', '/');
            map[path] = name;
        }
        return map;
    }

    /// <summary>All bundled resource paths (e.g. <c>keyboard/org.florisboard.layouts/extension.json</c>).</summary>
    public static IEnumerable<string> Paths => s_index.Value.Keys;

    /// <summary>Returns <c>true</c> if a resource exists (with or without a <c>.gz</c> suffix).</summary>
    public static bool Exists(string path) => Resolve(path) is not null;

    /// <summary>Opens a bundled resource stream, or returns <c>null</c> if not found.</summary>
    public static Stream? Open(string path)
    {
        var resolved = Resolve(path);
        if (resolved is null)
        {
            return null;
        }
        var stream = s_assembly.GetManifestResourceStream(resolved.Value.Name);
        if (stream is null)
        {
            return null;
        }
        return resolved.Value.Compressed ? new GZipStream(stream, CompressionMode.Decompress) : stream;
    }

    /// <summary>Reads a bundled text resource, or returns <c>null</c> if not found.</summary>
    public static string? ReadText(string path)
    {
        using var stream = Open(path);
        if (stream is null)
        {
            return null;
        }
        using var reader = new StreamReader(stream, Encoding.UTF8);
        return reader.ReadToEnd();
    }

    private static (string Name, bool Compressed)? Resolve(string path)
    {
        path = path.Replace('\\', '/').TrimStart('/');
        if (s_index.Value.TryGetValue(path, out var name))
        {
            return (name, path.EndsWith(".gz", StringComparison.Ordinal));
        }
        if (s_index.Value.TryGetValue(path + ".gz", out name))
        {
            return (name, true);
        }
        return null;
    }
}
