using System.ComponentModel;
using System.Globalization;
using System.Text.RegularExpressions;
using ScreenKeyboard.Clipboard;
using ScreenKeyboard.Editor;
using ScreenKeyboard.Emoji;
using ScreenKeyboard.Glide;
using ScreenKeyboard.Keys;
using ScreenKeyboard.Layouts;
using ScreenKeyboard.Settings;
using ScreenKeyboard.Text;

namespace ScreenKeyboard.Engine;

/// <summary>
/// The platform independent keyboard brain. It owns the keyboard state (mode, shift, language, panel),
/// turns key events into edits on the attached <see cref="ITextInputTarget"/> and provides suggestions,
/// glide typing, clipboard and emoji support. UI layers render <see cref="Keyboard"/> and forward input.
/// </summary>
public sealed partial class KeyboardEngine : IDisposable
{
    private readonly LayoutComputer _layoutComputer;
    private readonly Dictionary<(KeyboardMode, string, bool), ComputedKeyboard> _keyboardCache = new();
    private readonly List<Subtype> _subtypes = new();
    private ITextInputTarget? _target;
    private EditorController? _editor;
    private ComputedKeyboard? _keyboard;
    private Subtype _activeSubtype = Subtype.Default;
    private KeyboardMode _mode = KeyboardMode.Characters;
    private ShiftState _shiftState = ShiftState.Unshifted;
    private KeyboardUiMode _uiMode = KeyboardUiMode.Text;
    private bool _isCharHalfWidth;
    private bool _isKanaKata;
    private IReadOnlyList<Suggestion> _suggestions = [];
    private bool _suppressTargetChanged;
    private double _layoutWidth;
    private double _layoutHeight;
    private KeyboardGeometryOptions _geometry = new();

    // Shift state machine
    private bool _shiftHeld;
    private bool _keyPressedWhileShiftHeld;
    private DateTime _lastShiftUp = DateTime.MinValue;

    // Typing state
    private DateTime _lastSpaceTime = DateTime.MinValue;
    private bool _lastInsertWasAutoSpace;
    private AutoCorrection? _lastAutoCorrection;
    private readonly HashSet<string> _rejectedCorrections = new(StringComparer.OrdinalIgnoreCase);
    private string? _lastClipboardSuggestion;
    private bool _glideActive;
    private string? _lastGlideWord;
    private int _glidePointsSincePreview;
    private int? _preciseDeleteOrigin;

    /// <summary>Creates an engine.</summary>
    public KeyboardEngine(KeyboardSettings? settings = null, KeyboardResources? resources = null, LanguageModels? languageModels = null)
    {
        Settings = settings ?? new KeyboardSettings();
        Resources = resources ?? KeyboardResources.Default;
        LanguageModels = languageModels ?? new LanguageModels();
        _layoutComputer = new LayoutComputer(Resources);
        Resources.Changed += OnResourcesChanged;
        Settings.PropertyChanged += OnSettingsChanged;
        ClipboardHistory.Changed += (_, _) => UpdateSuggestions();
        LoadSubtypesFromSettings();
    }

    // ================================================================== services & configuration

    /// <summary>Keyboard settings.</summary>
    public KeyboardSettings Settings { get; }

    /// <summary>Layouts and localization resources.</summary>
    public KeyboardResources Resources { get; }

    /// <summary>Dictionaries per language.</summary>
    public LanguageModels LanguageModels { get; }

    /// <summary>System clipboard access (replace with a platform implementation).</summary>
    public IClipboardService ClipboardService { get; set; } = new InMemoryClipboardService();

    /// <summary>Clipboard history.</summary>
    public ClipboardHistory ClipboardHistory { get; } = new();

    /// <summary>Recently used emojis.</summary>
    public EmojiHistory EmojiHistory { get; } = new();

    /// <summary>Glide typing classifier.</summary>
    public GlideTypingClassifier GlideClassifier { get; } = new();

    /// <summary>
    /// Optional factory for a custom suggestion provider per subtype. When it returns <c>null</c> the built-in
    /// dictionary provider is used.
    /// </summary>
    public Func<Subtype, ISuggestionProvider?>? SuggestionProviderFactory { get; set; }

    /// <summary>Clock used for timing decisions (overridable for tests).</summary>
    public Func<DateTime> Clock { get; set; } = () => DateTime.UtcNow;

    /// <summary>Time window for "double space inserts a period".</summary>
    public TimeSpan DoubleSpaceWindow { get; set; } = TimeSpan.FromMilliseconds(800);

    // ================================================================== events

    /// <summary>Raised when <see cref="Keyboard"/> changed or was recomputed.</summary>
    public event EventHandler? KeyboardChanged;

    /// <summary>Raised when any state property changed (mode, shift, panel, language...).</summary>
    public event EventHandler? StateChanged;

    /// <summary>Raised when <see cref="Suggestions"/> changed.</summary>
    public event EventHandler? SuggestionsChanged;

    /// <summary>Raised when the UI should give haptic/audio feedback.</summary>
    public event EventHandler<FeedbackKind>? FeedbackRequested;

    /// <summary>Raised when the keyboard should be hidden.</summary>
    public event EventHandler? HideRequested;

    /// <summary>Raised when the language (subtype) picker should be shown.</summary>
    public event EventHandler? SubtypePickerRequested;

    /// <summary>Raised when the settings key was pressed.</summary>
    public event EventHandler? SettingsRequested;

    /// <summary>Raised when the keyboard resize mode was requested.</summary>
    public event EventHandler? ResizeModeRequested;

    /// <summary>Raised when the voice input key was pressed.</summary>
    public event EventHandler? VoiceInputRequested;

    /// <summary>Raised when text was committed to the target.</summary>
    public event EventHandler<string>? TextCommitted;

    // ================================================================== state

    /// <summary>The attached input target, if any.</summary>
    public ITextInputTarget? Target => _target;

    /// <summary>Editing helper for the attached target.</summary>
    public EditorController? Editor => _editor;

    /// <summary>Attributes of the attached field.</summary>
    public InputAttributes Attributes => _target?.Attributes ?? InputAttributes.Default;

    /// <summary>The current computed keyboard.</summary>
    public ComputedKeyboard Keyboard => _keyboard ??= BuildKeyboard();

    /// <summary>Current keyboard mode.</summary>
    public KeyboardMode Mode => _mode;

    /// <summary>Current shift state.</summary>
    public ShiftState ShiftState => _shiftState;

    /// <summary>Current panel.</summary>
    public KeyboardUiMode UiMode => _uiMode;

    /// <summary>Half-width characters (CJK).</summary>
    public bool IsCharHalfWidth => _isCharHalfWidth;

    /// <summary>Katakana mode (Japanese).</summary>
    public bool IsKanaKata => _isKanaKata;

    /// <summary>Whether glide typing is in progress.</summary>
    public bool IsGliding => _glideActive;

    /// <summary>Whether incognito is active (setting or private field).</summary>
    public bool IsIncognito => Settings.IncognitoMode || Attributes.IsPrivate || Attributes.IsPassword;

    /// <summary>Enabled subtypes.</summary>
    public IReadOnlyList<Subtype> Subtypes => _subtypes;

    /// <summary>The active subtype.</summary>
    public Subtype ActiveSubtype => _activeSubtype;

    /// <summary>Culture of the active subtype.</summary>
    public CultureInfo Culture { get; private set; } = CultureInfo.InvariantCulture;

    /// <summary>Current suggestions.</summary>
    public IReadOnlyList<Suggestion> Suggestions => _suggestions;

    /// <summary>The composer of the active subtype.</summary>
    public IComposer Composer => Resources.GetComposer(_activeSubtype.Composer);

    /// <summary>The punctuation rule of the active subtype.</summary>
    public PunctuationRule PunctuationRule => Resources.GetPunctuationRule(_activeSubtype.PunctuationRule);

    /// <summary>The emoji catalog for the active language.</summary>
    public EmojiCatalog EmojiCatalog => EmojiCatalog.Load(_activeSubtype.LanguageTag);

    // ================================================================== attach / detach

    /// <summary>Attaches the keyboard to a text target.</summary>
    public void Attach(ITextInputTarget target)
    {
        if (ReferenceEquals(_target, target))
        {
            return;
        }
        Detach();
        _target = target;
        _editor = new EditorController(target) { Clock = Clock };
        _target.Changed += OnTargetChanged;
        _lastAutoCorrection = null;
        _lastInsertWasAutoSpace = false;
        var initialMode = target.Attributes.InitialMode;
        _uiMode = KeyboardUiMode.Text;
        _mode = initialMode;
        _shiftState = ShiftState.Unshifted;
        InvalidateKeyboard();
        UpdateAutoCapitalization();
        UpdateSuggestions();
        OnStateChanged();
    }

    /// <summary>Detaches from the current target.</summary>
    public void Detach()
    {
        if (_target is null)
        {
            return;
        }
        FinishGlide(commit: false);
        _target.Changed -= OnTargetChanged;
        _target = null;
        _editor = null;
        SetSuggestions([]);
        OnStateChanged();
    }

    private void OnTargetChanged(object? sender, EventArgs e)
    {
        if (_suppressTargetChanged)
        {
            return;
        }
        // External edit or caret move: reset transient typing state.
        _lastAutoCorrection = null;
        _lastInsertWasAutoSpace = false;
        _editor?.EndSelectionMode();
        _editor?.BreakUndoGroup();
        UpdateAutoCapitalization();
        UpdateSuggestions();
    }

    // ================================================================== subtypes

    /// <summary>Sets the enabled subtypes and activates the first one (or keeps the active one).</summary>
    public void SetSubtypes(IEnumerable<Subtype> subtypes)
    {
        _subtypes.Clear();
        _subtypes.AddRange(subtypes);
        if (_subtypes.Count == 0)
        {
            _subtypes.Add(Subtype.Default);
        }
        var active = _subtypes.FirstOrDefault(s => s.Id == _activeSubtype.Id) ?? _subtypes[0];
        ActivateSubtype(active, force: true);
    }

    /// <summary>Activates a subtype (it is added to the enabled list if missing).</summary>
    public void SetActiveSubtype(Subtype subtype)
    {
        if (!_subtypes.Any(s => s.Id == subtype.Id))
        {
            _subtypes.Add(subtype);
        }
        ActivateSubtype(subtype, force: false);
    }

    /// <summary>Switches to the next enabled subtype.</summary>
    public void NextSubtype() => CycleSubtype(1);

    /// <summary>Switches to the previous enabled subtype.</summary>
    public void PreviousSubtype() => CycleSubtype(-1);

    private void CycleSubtype(int delta)
    {
        if (_subtypes.Count < 2)
        {
            return;
        }
        var index = _subtypes.FindIndex(s => s.Id == _activeSubtype.Id);
        index = ((index + delta) % _subtypes.Count + _subtypes.Count) % _subtypes.Count;
        ActivateSubtype(_subtypes[index], force: false);
    }

    private void ActivateSubtype(Subtype subtype, bool force)
    {
        if (!force && subtype.Id == _activeSubtype.Id)
        {
            return;
        }
        _activeSubtype = subtype;
        Culture = TryGetCulture(subtype.LanguageTag);
        _keyboardCache.Clear();
        InvalidateKeyboard();
        UpdateGlideWords();
        UpdateAutoCapitalization();
        UpdateSuggestions();
        OnStateChanged();
    }

    /// <summary>Finds a subtype by id among presets (<c>"de-DE/qwertz"</c>) or language tag.</summary>
    public Subtype? FindSubtype(string id)
    {
        var presets = Resources.SubtypePresets;
        var match = presets.FirstOrDefault(s => string.Equals(s.Id, id, StringComparison.OrdinalIgnoreCase));
        if (match is not null)
        {
            return match;
        }
        var parts = id.Split('/');
        var preset = Resources.FindSubtypePreset(parts[0]);
        if (preset is not null && parts.Length > 1)
        {
            return preset with { Layouts = preset.Layouts with { Characters = ComponentName.Parse(parts[1], SubtypeLayoutMap.CoreLayouts) } };
        }
        return preset;
    }

    private void LoadSubtypesFromSettings()
    {
        var subtypes = Settings.Subtypes.Select(FindSubtype).Where(s => s is not null).Cast<Subtype>().ToList();
        SetSubtypes(subtypes.Count > 0 ? subtypes : [Subtype.Default]);
    }

    /// <summary>Human readable name of a subtype (e.g. "English (United States)").</summary>
    public static string GetSubtypeDisplayName(Subtype subtype)
    {
        if (!string.IsNullOrEmpty(subtype.DisplayName))
        {
            return subtype.DisplayName!;
        }
        try
        {
            var culture = CultureInfo.GetCultureInfo(subtype.LanguageTag);
            var name = culture.NativeName;
            if (string.IsNullOrEmpty(name) || name == subtype.LanguageTag || culture.Name.Length == 0)
            {
                return subtype.LanguageTag;
            }
            return TextUtils.Capitalize(name, culture);
        }
        catch (CultureNotFoundException)
        {
            return subtype.LanguageTag;
        }
    }

    private static CultureInfo TryGetCulture(string tag)
    {
        try
        {
            return CultureInfo.GetCultureInfo(tag);
        }
        catch (CultureNotFoundException)
        {
            return CultureInfo.InvariantCulture;
        }
    }

    // ================================================================== modes

    /// <summary>Switches the keyboard mode.</summary>
    public void SetMode(KeyboardMode mode)
    {
        if (_mode == mode && _uiMode == KeyboardUiMode.Text)
        {
            return;
        }
        _mode = mode;
        _uiMode = KeyboardUiMode.Text;
        if (mode != KeyboardMode.Characters)
        {
            _shiftState = ShiftState.Unshifted;
        }
        InvalidateKeyboard();
        UpdateAutoCapitalization();
        OnStateChanged();
    }

    /// <summary>Switches the panel (text keyboard, emoji, clipboard, editing).</summary>
    public void SetUiMode(KeyboardUiMode mode)
    {
        if (_uiMode == mode)
        {
            return;
        }
        _uiMode = mode;
        _editor?.EndSelectionMode();
        UpdateSuggestions();
        OnStateChanged();
    }

    /// <summary>Sets the shift state explicitly.</summary>
    public void SetShiftState(ShiftState state)
    {
        if (_shiftState == state)
        {
            return;
        }
        _shiftState = state;
        RecomputeKeyboard();
        OnStateChanged();
    }

    // ================================================================== layout

    /// <summary>
    /// Informs the engine about the size of the keys area so it can lay out keys and prepare glide typing.
    /// </summary>
    public void UpdateLayout(double width, double height, KeyboardGeometryOptions? geometry = null)
    {
        _layoutWidth = width;
        _layoutHeight = height;
        _geometry = geometry ?? _geometry;
        var keyboard = Keyboard;
        keyboard.Layout(width, height, _geometry);
        if (_mode == KeyboardMode.Characters)
        {
            GlideClassifier.SetLayout(keyboard.CharacterKeys);
        }
    }

    /// <summary>Builds the key compute context for the current state.</summary>
    public KeyComputeContext CreateComputeContext() => new()
    {
        Mode = _mode,
        ShiftState = _shiftState,
        Variation = Attributes.Variation,
        Direction = _keyboard?.Direction ?? LayoutDirection.Ltr,
        IsCharHalfWidth = _isCharHalfWidth,
        IsKanaKata = _isKanaKata,
        Culture = Culture,
        EnterAction = Attributes.EffectiveEnterAction,
        CurrencySet = Resources.GetCurrencySet(_activeSubtype.CurrencySet),
        HasMultipleSubtypes = _subtypes.Count > 1,
        UtilityKeyAction = Settings.UtilityKeyAction,
        SpaceBarMode = Settings.SpaceBarMode,
        SpaceBarLabel = GetSubtypeDisplayName(_activeSubtype),
        HintMode = Settings.HintMode,
        ShowNumberHints = Settings.HintedNumberRow && !Settings.NumberRow,
        ShowSymbolHints = Settings.HintedSymbols,
    };

    private ComputedKeyboard BuildKeyboard()
    {
        var numberRow = Settings.NumberRow && _mode == KeyboardMode.Characters;
        var key = (_mode, _activeSubtype.Id, numberRow);
        if (!_keyboardCache.TryGetValue(key, out var keyboard))
        {
            keyboard = _layoutComputer.Compute(_mode, _activeSubtype, numberRow);
            _keyboardCache[key] = keyboard;
        }
        _keyboard = keyboard;
        keyboard.Compute(CreateComputeContext() with { Direction = keyboard.Direction });
        if (_layoutWidth > 0 && _layoutHeight > 0)
        {
            keyboard.Layout(_layoutWidth, _layoutHeight, _geometry);
            if (_mode == KeyboardMode.Characters)
            {
                GlideClassifier.SetLayout(keyboard.CharacterKeys);
            }
        }
        return keyboard;
    }

    private void InvalidateKeyboard()
    {
        _keyboard = null;
        _ = Keyboard;
        KeyboardChanged?.Invoke(this, EventArgs.Empty);
    }

    /// <summary>Re-evaluates keys for the current state (e.g. after shift changes).</summary>
    public void RecomputeKeyboard()
    {
        if (_keyboard is null)
        {
            InvalidateKeyboard();
            return;
        }
        _keyboard.Compute(CreateComputeContext() with { Direction = _keyboard.Direction });
        if (_layoutWidth > 0 && _layoutHeight > 0)
        {
            _keyboard.Layout(_layoutWidth, _layoutHeight, _geometry);
        }
        KeyboardChanged?.Invoke(this, EventArgs.Empty);
    }

    private void OnResourcesChanged(object? sender, EventArgs e)
    {
        _keyboardCache.Clear();
        InvalidateKeyboard();
    }

    private void OnSettingsChanged(object? sender, PropertyChangedEventArgs e)
    {
        switch (e.PropertyName)
        {
            case nameof(KeyboardSettings.Subtypes):
                LoadSubtypesFromSettings();
                break;
            case nameof(KeyboardSettings.NumberRow):
                InvalidateKeyboard();
                break;
            case nameof(KeyboardSettings.HintedNumberRow):
            case nameof(KeyboardSettings.HintedSymbols):
            case nameof(KeyboardSettings.HintMode):
            case nameof(KeyboardSettings.UtilityKeyAction):
            case nameof(KeyboardSettings.SpaceBarMode):
                RecomputeKeyboard();
                break;
            case nameof(KeyboardSettings.ShowSuggestions):
            case nameof(KeyboardSettings.IncognitoMode):
            case nameof(KeyboardSettings.SuggestionCount):
            case nameof(KeyboardSettings.NextWordPrediction):
            case nameof(KeyboardSettings.EmojiSuggestions):
                UpdateSuggestions();
                OnStateChanged();
                break;
            case nameof(KeyboardSettings.AutoCapitalization):
                UpdateAutoCapitalization();
                break;
            case nameof(KeyboardSettings.GlideTyping):
                UpdateGlideWords();
                break;
        }
    }

    private void OnStateChanged() => StateChanged?.Invoke(this, EventArgs.Empty);

    private void Feedback(FeedbackKind kind) => FeedbackRequested?.Invoke(this, kind);

    /// <summary>Requests long-press feedback (used by the touch processor).</summary>
    public void FeedbackLongPress() => Feedback(FeedbackKind.LongPress);

    /// <summary>Requests gesture step feedback (used by the touch processor).</summary>
    public void FeedbackGestureStep() => Feedback(FeedbackKind.GestureStep);

    // ================================================================== dispose

    /// <inheritdoc />
    public void Dispose()
    {
        Detach();
        Resources.Changed -= OnResourcesChanged;
        Settings.PropertyChanged -= OnSettingsChanged;
    }

    private sealed record AutoCorrection(string Original, string Corrected, string Separator, int End);

    [GeneratedRegex(@"(?:^|\s):([\p{L}\p{N}_+\-]{2,})$")]
    private static partial Regex EmojiShortcodeRegex();
}
