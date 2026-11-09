using ScreenKeyboard.Layouts;

namespace ScreenKeyboard.Sample;

/// <summary>
/// Shows how to add custom layouts and languages. Layouts use the FlorisBoard JSON format, so any FlorisBoard
/// layout can be dropped in as well.
/// </summary>
internal static class CustomLayouts
{
    /// <summary>An alphabetical layout, useful for kiosks where users are not familiar with QWERTY.</summary>
    private const string Alphabetical = """
    {
      "type": "characters",
      "id": "alphabetical",
      "label": "Alphabetical (ABC)",
      "authors": [ "ScreenKeyboard sample" ],
      "direction": "ltr",
      "arrangement": [
        [ "a", "b", "c", "d", "e", "f", "g", "h", "i", "j" ],
        [ "k", "l", "m", "n", "o", "p", "q", "r", "s" ],
        [ "t", "u", "v", "w", "x", "y", "z" ]
      ]
    }
    """;

    public static void Register()
    {
        var resources = KeyboardResources.Default;
        var layout = resources.AddLayoutJson(Alphabetical, extensionId: "sample");

        // A subtype combines a language with layouts, popups, currency and composer.
        resources.AddSubtypePreset(new Subtype
        {
            LanguageTag = "en-US",
            DisplayName = "English (ABC)",
            PopupMapping = new ComponentName(Subtype.CoreLocalization, "en"),
            Layouts = new SubtypeLayoutMap { Characters = layout.Name },
        });
    }
}
