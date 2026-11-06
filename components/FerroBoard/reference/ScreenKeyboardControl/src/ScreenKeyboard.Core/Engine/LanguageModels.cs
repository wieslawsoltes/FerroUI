using ScreenKeyboard.Text;

namespace ScreenKeyboard.Engine;

/// <summary>
/// Registry of word dictionaries per language (two letter code, e.g. <c>"en"</c>). English is bundled;
/// register additional languages with <see cref="Register(string, Func{WordDictionary})"/>.
/// </summary>
public sealed class LanguageModels
{
    private readonly Dictionary<string, Lazy<WordDictionary>> _dictionaries = new(StringComparer.OrdinalIgnoreCase);
    private readonly Dictionary<string, UserDictionary> _userDictionaries = new(StringComparer.OrdinalIgnoreCase);
    private readonly object _gate = new();

    /// <summary>Creates a registry with the bundled English dictionary.</summary>
    public LanguageModels()
    {
        Register("en", WordDictionary.LoadBundledEnglish);
    }

    /// <summary>Raised when a user dictionary changed (for persistence).</summary>
    public event EventHandler<string>? UserDictionaryChanged;

    /// <summary>Registers a dictionary factory (invoked lazily).</summary>
    public void Register(string language, Func<WordDictionary> factory)
    {
        lock (_gate) _dictionaries[Normalize(language)] = new Lazy<WordDictionary>(factory, LazyThreadSafetyMode.ExecutionAndPublication);
    }

    /// <summary>Registers a dictionary instance.</summary>
    public void Register(string language, WordDictionary dictionary)
    {
        lock (_gate) _dictionaries[Normalize(language)] = new Lazy<WordDictionary>(() => dictionary);
    }

    /// <summary>Languages with a dictionary.</summary>
    public IReadOnlyList<string> Languages
    {
        get { lock (_gate) return _dictionaries.Keys.ToList(); }
    }

    /// <summary>Returns <c>true</c> if a dictionary exists for the language tag.</summary>
    public bool HasDictionary(string languageTag)
    {
        lock (_gate) return _dictionaries.ContainsKey(Normalize(languageTag));
    }

    /// <summary>Gets the dictionary for a language tag (e.g. <c>en-US</c>), or <c>null</c>.</summary>
    public WordDictionary? GetDictionary(string languageTag)
    {
        Lazy<WordDictionary>? lazy;
        lock (_gate)
        {
            if (!_dictionaries.TryGetValue(Normalize(languageTag), out lazy))
            {
                return null;
            }
        }
        return lazy.Value;
    }

    /// <summary>Returns <c>true</c> if the dictionary for the language was already loaded.</summary>
    public bool IsLoaded(string languageTag)
    {
        lock (_gate) return _dictionaries.TryGetValue(Normalize(languageTag), out var lazy) && lazy.IsValueCreated;
    }

    /// <summary>Gets (or creates) the user dictionary for a language.</summary>
    public UserDictionary GetUserDictionary(string languageTag)
    {
        var lang = Normalize(languageTag);
        lock (_gate)
        {
            if (!_userDictionaries.TryGetValue(lang, out var user))
            {
                user = new UserDictionary();
                user.Changed += (_, _) => UserDictionaryChanged?.Invoke(this, lang);
                _userDictionaries[lang] = user;
            }
            return user;
        }
    }

    /// <summary>All user dictionaries created so far.</summary>
    public IReadOnlyDictionary<string, UserDictionary> UserDictionaries
    {
        get { lock (_gate) return new Dictionary<string, UserDictionary>(_userDictionaries, StringComparer.OrdinalIgnoreCase); }
    }

    private static string Normalize(string tag) => tag.Split('-', '_')[0].ToLowerInvariant();
}
