#if HAS_UNO
using Microsoft.UI.Xaml.Documents.TextFormatting;
using Windows.UI.Text;
#endif

namespace ScreenKeyboard.Controls;

/// <summary>
/// Makes color emoji render on Skia targets without a system emoji font (WebAssembly, Linux framebuffer...).
/// It plugs an emoji font into Uno Platform's font fallback chain; other code points keep using the
/// previously configured fallback service.
/// </summary>
/// <example>
/// <code language="csharp"><![CDATA[
/// // App constructor, before any text is rendered:
/// EmojiFontFallback.UseNotoColorEmoji();                       // download from a CDN on first use
/// EmojiFontFallback.Use(() => OpenBundledFontAsync());          // or ship the font with the app
/// ]]></code>
/// </example>
public static class EmojiFontFallback
{
    /// <summary>Family name used for the emoji font.</summary>
    public const string FamilyName = "Noto Color Emoji";

    /// <summary>Default download location of Noto Color Emoji (SIL Open Font License 1.1).</summary>
    public const string DefaultNotoColorEmojiUrl = "https://cdn.jsdelivr.net/gh/googlefonts/noto-emoji@v2.051/fonts/NotoColorEmoji.ttf";

    /// <summary>Returns <c>true</c> when the emoji fallback was installed.</summary>
    public static bool IsEnabled { get; private set; }

    /// <summary>Downloads Noto Color Emoji from <paramref name="url"/> the first time an emoji is rendered.</summary>
    public static void UseNotoColorEmoji(string url = DefaultNotoColorEmojiUrl)
    {
        Use(async () =>
        {
            using var client = new HttpClient();
            var bytes = await client.GetByteArrayAsync(url).ConfigureAwait(false);
            return new MemoryStream(bytes);
        });
    }

    /// <summary>
    /// Uses a custom font source (for example a font file bundled with the application). The provider is called
    /// once; the font is cached in memory.
    /// </summary>
    public static void Use(Func<Task<Stream>> fontProvider)
    {
#if HAS_UNO
        var previous = Uno.UI.FeatureConfiguration.Font.FallbackService;
        Uno.UI.FeatureConfiguration.Font.FallbackService = new EmojiFallbackService(fontProvider, previous);
        IsEnabled = true;
#endif
    }

    /// <summary>Returns <c>true</c> for code points that should be rendered with the emoji font.</summary>
    public static bool IsEmojiCodePoint(int cp) =>
        cp is >= 0x1F000 and <= 0x1FAFF   // pictographs, emoticons, transport, symbols & pictographs extended
            or >= 0x2600 and <= 0x27BF    // misc symbols, dingbats
            or >= 0x2300 and <= 0x23FF    // misc technical (⌚ ⏰ ...)
            or >= 0x2B00 and <= 0x2BFF    // arrows (⭐ ⬛ ...)
            or >= 0x1F1E6 and <= 0x1F1FF  // regional indicators (flags)
            or >= 0xE0020 and <= 0xE007F  // tags (subdivision flags)
            or 0x200D or 0xFE0F or 0x20E3 // joiners, variation selector, keycap
            or 0x00A9 or 0x00AE or 0x203C or 0x2049 or 0x2122 or 0x2139
            or >= 0x2194 and <= 0x21AA
            or 0x24C2 or 0x3030 or 0x303D or 0x3297 or 0x3299;

#if HAS_UNO
    private sealed class EmojiFallbackService(Func<Task<Stream>> provider, IFontFallbackService? previous) : IFontFallbackService
    {
        private Task<byte[]>? _font;

        public async Task<string?> GetFontFamilyForCodepoint(int codepoint)
        {
            if (IsEmojiCodePoint(codepoint))
            {
                return FamilyName;
            }
            return previous is null ? null : await previous.GetFontFamilyForCodepoint(codepoint).ConfigureAwait(false);
        }

        public async Task<Stream?> GetFontStreamForFontFamily(string fontFamily, FontWeight weight, FontStretch stretch, FontStyle style)
        {
            if (fontFamily == FamilyName)
            {
                try
                {
                    _font ??= LoadAsync();
                    var bytes = await _font.ConfigureAwait(false);
                    return new MemoryStream(bytes, writable: false);
                }
                catch (Exception)
                {
                    _font = null;
                    return null;
                }
            }
            return previous is null ? null : await previous.GetFontStreamForFontFamily(fontFamily, weight, stretch, style).ConfigureAwait(false);
        }

        private async Task<byte[]> LoadAsync()
        {
            using var stream = await provider().ConfigureAwait(false);
            using var memory = new MemoryStream();
            await stream.CopyToAsync(memory).ConfigureAwait(false);
            return memory.ToArray();
        }
    }
#endif
}
