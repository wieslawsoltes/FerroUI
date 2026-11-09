using ScreenKeyboard.Engine;
using ScreenKeyboard.Settings;
using Windows.Storage;

namespace ScreenKeyboard.Sample;

/// <summary>
/// Persists keyboard settings, learned words, emoji history and pinned clipboard items.
/// Demonstrates the serialization APIs of ScreenKeyboard.Core.
/// </summary>
internal static class SampleStorage
{
    private const string SettingsKey = "keyboard.settings";

    private static string? Folder
    {
        get
        {
            try
            {
                return ApplicationData.Current.LocalFolder.Path;
            }
            catch (Exception)
            {
                return null;
            }
        }
    }

    public static void Load(KeyboardEngine engine)
    {
        try
        {
            if (ApplicationData.Current.LocalSettings.Values[SettingsKey] is string json)
            {
                engine.Settings.CopyFrom(KeyboardSettings.FromJson(json));
            }
        }
        catch (Exception ex)
        {
            Console.WriteLine($"Could not load settings: {ex.Message}");
        }

        Read("emoji-history.json", engine.EmojiHistory.Import);
        Read("clipboard.json", engine.ClipboardHistory.Import);
        Read("user-dictionary-en.json", json => engine.LanguageModels.GetUserDictionary("en").Import(json));
    }

    public static void SaveSettings(KeyboardSettings settings)
    {
        try
        {
            ApplicationData.Current.LocalSettings.Values[SettingsKey] = settings.ToJson();
        }
        catch (Exception ex)
        {
            Console.WriteLine($"Could not save settings: {ex.Message}");
        }
    }

    public static void Save(KeyboardEngine engine)
    {
        SaveSettings(engine.Settings);
        Write("emoji-history.json", engine.EmojiHistory.Export());
        Write("clipboard.json", engine.ClipboardHistory.Export(pinnedOnly: true));
        foreach (var (language, dictionary) in engine.LanguageModels.UserDictionaries)
        {
            Write($"user-dictionary-{language}.json", dictionary.Export());
        }
    }

    private static void Read(string name, Action<string> apply)
    {
        try
        {
            if (Folder is { } folder && File.Exists(Path.Combine(folder, name)))
            {
                apply(File.ReadAllText(Path.Combine(folder, name)));
            }
        }
        catch (Exception ex)
        {
            Console.WriteLine($"Could not read {name}: {ex.Message}");
        }
    }

    private static void Write(string name, string content)
    {
        try
        {
            if (Folder is { } folder)
            {
                Directory.CreateDirectory(folder);
                File.WriteAllText(Path.Combine(folder, name), content);
            }
        }
        catch (Exception ex)
        {
            Console.WriteLine($"Could not write {name}: {ex.Message}");
        }
    }
}
