using System.Globalization;
using ScreenKeyboard.Editor;
using ScreenKeyboard.Keys;
using ScreenKeyboard.Settings;

namespace ScreenKeyboard.Layouts;

/// <summary>An axis aligned rectangle in keyboard coordinates (device independent pixels).</summary>
public readonly record struct KeyRect(double X, double Y, double Width, double Height)
{
    public double Left => X;
    public double Top => Y;
    public double Right => X + Width;
    public double Bottom => Y + Height;
    public double CenterX => X + Width / 2;
    public double CenterY => Y + Height / 2;
    public bool IsEmpty => Width <= 0 || Height <= 0;

    /// <summary>Returns <c>true</c> if the point lies inside the rectangle.</summary>
    public bool Contains(double x, double y) => x >= X && x < Right && y >= Y && y < Bottom;

    /// <summary>Returns the rectangle shrunk by the given amounts on each side.</summary>
    public KeyRect Deflate(double left, double top, double right, double bottom) =>
        new(X + left, Y + top, Math.Max(0, Width - left - right), Math.Max(0, Height - top - bottom));

    /// <summary>Squared distance from the point to the rectangle center.</summary>
    public double CenterDistanceSquared(double x, double y)
    {
        var dx = CenterX - x;
        var dy = CenterY - y;
        return dx * dx + dy * dy;
    }
}

/// <summary>Visual spacing options applied when laying out keys.</summary>
public sealed record KeyboardGeometryOptions
{
    /// <summary>Total horizontal gap between two adjacent keys.</summary>
    public double KeySpacingHorizontal { get; init; } = 5;

    /// <summary>Total vertical gap between two adjacent rows.</summary>
    public double KeySpacingVertical { get; init; } = 10;

    /// <summary>Width of the central gap in split mode (0 disables split).</summary>
    public double SplitGap { get; init; }

    /// <summary>Number of key units a keyboard row is designed for (FlorisBoard uses 10).</summary>
    public double UnitsPerRow { get; init; } = 10;
}

/// <summary>
/// All state required to compute keys (the <see cref="IKeyEvaluator"/> values plus display related options).
/// </summary>
public sealed record KeyComputeContext : IKeyEvaluator
{
    public KeyboardMode Mode { get; init; } = KeyboardMode.Characters;
    public ShiftState ShiftState { get; init; } = ShiftState.Unshifted;
    public KeyVariation Variation { get; init; } = KeyVariation.Normal;
    public LayoutDirection Direction { get; init; } = LayoutDirection.Ltr;
    public bool IsCharHalfWidth { get; init; }
    public bool IsKanaKata { get; init; }
    public CultureInfo Culture { get; init; } = CultureInfo.InvariantCulture;

    /// <summary>The enter key action of the current field.</summary>
    public EnterAction EnterAction { get; init; } = EnterAction.Done;

    /// <summary>The currency set used for currency slots.</summary>
    public CurrencySet? CurrencySet { get; init; }

    /// <summary>Whether more than one subtype (language) is enabled.</summary>
    public bool HasMultipleSubtypes { get; init; }

    /// <summary>Behavior of the utility key.</summary>
    public UtilityKeyAction UtilityKeyAction { get; init; } = UtilityKeyAction.Dynamic;

    /// <summary>Space bar label mode.</summary>
    public SpaceBarMode SpaceBarMode { get; init; } = SpaceBarMode.CurrentLanguage;

    /// <summary>Text shown on the space bar when <see cref="SpaceBarMode.CurrentLanguage"/>.</summary>
    public string SpaceBarLabel { get; init; } = string.Empty;

    /// <summary>Hint priority.</summary>
    public KeyHintMode HintMode { get; init; } = KeyHintMode.AccentPriority;

    /// <summary>Whether symbol hints are shown on character keys.</summary>
    public bool ShowSymbolHints { get; init; } = true;

    /// <summary>Whether number hints are shown on the first character row.</summary>
    public bool ShowNumberHints { get; init; } = true;

    internal KeyComputeContext Unshifted() => this with { ShiftState = ShiftState.Unshifted };
}

/// <summary>A key of a <see cref="ComputedKeyboard"/> with its evaluated data, labels, popups and bounds.</summary>
public sealed class ComputedKey
{
    internal ComputedKey(AbstractKeyData source, int row, int column)
    {
        Source = source;
        Row = row;
        Column = column;
        Data = TextKeyData.Unspecified;
    }

    /// <summary>The key definition from the layout.</summary>
    public AbstractKeyData Source { get; }

    /// <summary>Row index.</summary>
    public int Row { get; }

    /// <summary>Column index (among all keys of the row, including hidden ones).</summary>
    public int Column { get; }

    /// <summary>The evaluated key data for the current state.</summary>
    public TextKeyData Data { get; private set; }

    /// <summary>Key code shortcut.</summary>
    public int Code => Data.Code;

    /// <summary>Key type shortcut.</summary>
    public KeyType Type => Data.Type;

    /// <summary>Main label (text) or <c>null</c> when an icon is shown.</summary>
    public string? Label { get; private set; }

    /// <summary>Icon to display (for function keys).</summary>
    public KeyIcon Icon { get; private set; }

    /// <summary>Small hint label (symbol/number) shown in the key corner.</summary>
    public string? HintLabel { get; private set; }

    /// <summary>Secondary label shown below the main label (e.g. letters on phone pad digits).</summary>
    public string? SubLabel { get; private set; }

    /// <summary>The key the hint label represents.</summary>
    public TextKeyData? HintKey { get; private set; }

    /// <summary>Keys shown in the long-press popup.</summary>
    public IReadOnlyList<TextKeyData> PopupKeys { get; private set; } = [];

    /// <summary>Index into <see cref="PopupKeys"/> of the key selected by default when the popup opens.</summary>
    public int PopupDefaultIndex { get; private set; }

    /// <summary>Whether the key is visible.</summary>
    public bool IsVisible { get; private set; } = true;

    /// <summary>Whether the key can be pressed.</summary>
    public bool IsEnabled { get; private set; } = true;

    /// <summary>Width in key units.</summary>
    public double WidthFactor { get; private set; } = 1;

    /// <summary>Grow weight when the row has free space.</summary>
    public double Grow { get; private set; }

    /// <summary>Shrink weight when the row overflows.</summary>
    public double Shrink { get; private set; } = 1;

    /// <summary>Bounds used for touch hit testing (no gaps between keys).</summary>
    public KeyRect TouchBounds { get; internal set; }

    /// <summary>Bounds used for rendering (touch bounds minus spacing).</summary>
    public KeyRect VisibleBounds { get; internal set; }

    internal AbstractKeyData? SymbolHintSource { get; set; }
    internal AbstractKeyData? NumberHintSource { get; set; }

    /// <summary>Returns <c>true</c> if this key has popup keys.</summary>
    public bool HasPopup => PopupKeys.Count > 0;

    /// <summary>Returns <c>true</c> if the key produces text (character or numeric key).</summary>
    public bool IsCharacterKey => Data.ProducesText && !Data.IsSpace && Data.Code != KeyCode.Enter && Data.Code != KeyCode.Tab;

    /// <summary>The text output of the key.</summary>
    public string Output => Data.AsString(false);

    internal void Compute(KeyComputeContext ctx, PopupMapping? mapping, PopupMapping? defaultMapping)
    {
        var data = Source.Compute(ctx);
        if (data is null)
        {
            Data = TextKeyData.Unspecified;
            IsVisible = false;
            IsEnabled = false;
            PopupKeys = [];
            Label = null;
            HintLabel = null;
            HintKey = null;
            Icon = KeyIcon.None;
            return;
        }
        data = ResolveCurrency(data, ctx);
        Data = data;
        IsVisible = EvaluateVisible(data, ctx);
        IsEnabled = data.Type != KeyType.Placeholder && !(data.Code == KeyCode.Unspecified && data.Label.Length == 0);
        ComputeFlay(data, ctx.Mode);
        (Label, Icon) = KeyLabels.GetDisplay(data, ctx);
        SubLabel = ctx.Mode is KeyboardMode.Phone or KeyboardMode.Phone2 && data.Type == KeyType.Numeric ? KeyLabels.GetPhoneSubLabel(data.Code) : null;

        // Popups & hints.
        var accents = new List<TextKeyData>();
        TextKeyData? mainAccent = null;
        var lookupData = Source.Compute(ctx.Unshifted()) ?? data;
        CollectPopups(data.Popup, ctx, accents, ref mainAccent);
        var mappingSet = FindMappingSet(lookupData.PopupLookupLabel, ctx.Variation, mapping, defaultMapping);
        if (lookupData.PopupLookupLabel != lookupData.Label && lookupData.Label.Length > 0)
        {
            CollectPopups(FindMappingSet(lookupData.Label, ctx.Variation, mapping, defaultMapping), ctx, accents, ref mainAccent);
        }
        CollectPopups(mappingSet, ctx, accents, ref mainAccent);

        TextKeyData? hint = null;
        var hintPopups = new List<TextKeyData>();
        if (ctx.HintMode != KeyHintMode.Disabled && data.Type == KeyType.Character && !data.IsSpace)
        {
            var unshifted = ctx.Unshifted();
            if (ctx.ShowNumberHints && NumberHintSource?.Compute(unshifted) is { } numberHint && numberHint.Type == KeyType.Numeric && numberHint.Code != data.Code)
            {
                hint = numberHint;
            }
            else if (ctx.ShowSymbolHints && SymbolHintSource?.Compute(unshifted) is { } symbolHint && symbolHint.Type == KeyType.Character && symbolHint.Code != data.Code)
            {
                hint = ResolveCurrency(symbolHint, ctx);
            }
            if (hint is not null)
            {
                TextKeyData? ignored = null;
                CollectPopups(hint.Popup, ctx, hintPopups, ref ignored);
                if (ignored is not null)
                {
                    hintPopups.Insert(0, ignored);
                }
                CollectPopups(FindMappingSet(hint.Label, KeyVariation.All, mapping, defaultMapping), ctx, hintPopups, ref ignored);
            }
        }
        HintKey = hint;
        HintLabel = hint?.AsString(true);

        var keys = new List<TextKeyData>();
        var defaultIndex = 0;
        if (hint is not null && ctx.HintMode == KeyHintMode.HintPriority)
        {
            keys.Add(hint);
            if (mainAccent is not null) keys.Add(mainAccent);
        }
        else
        {
            if (mainAccent is not null) keys.Add(mainAccent);
            if (hint is not null) keys.Add(hint);
        }
        keys.AddRange(accents);
        keys.AddRange(hintPopups);
        PopupKeys = Deduplicate(keys, data);
        PopupDefaultIndex = PopupKeys.Count > 0 ? defaultIndex : -1;
    }

    private static List<TextKeyData> Deduplicate(List<TextKeyData> keys, TextKeyData self)
    {
        var seen = new HashSet<string>(StringComparer.Ordinal) { KeyIdentity(self) };
        var result = new List<TextKeyData>(keys.Count);
        foreach (var k in keys)
        {
            if (k.Type == KeyType.Placeholder || k.Code == KeyCode.Unspecified && k.Label.Length == 0)
            {
                continue;
            }
            if (seen.Add(KeyIdentity(k)))
            {
                result.Add(k);
            }
        }
        return result;
    }

    private static string KeyIdentity(TextKeyData k) => k.ProducesText ? "t:" + k.AsString(false) : "c:" + k.Code.ToString(CultureInfo.InvariantCulture);

    private static PopupSet? FindMappingSet(string label, KeyVariation variation, PopupMapping? mapping, PopupMapping? defaultMapping)
    {
        if (label.Length == 0)
        {
            return null;
        }
        var set = mapping?.Get(variation, label) ?? defaultMapping?.Get(variation, label);
        if (set is null && variation is KeyVariation.EmailAddress or KeyVariation.Uri)
        {
            set = mapping?.Get(KeyVariation.Uri, label) ?? defaultMapping?.Get(KeyVariation.Uri, label);
        }
        return set ?? mapping?.Get(KeyVariation.All, label) ?? defaultMapping?.Get(KeyVariation.All, label);
    }

    private static void CollectPopups(PopupSet? set, KeyComputeContext ctx, List<TextKeyData> target, ref TextKeyData? main)
    {
        if (set is null)
        {
            return;
        }
        if (set.Main?.Compute(ctx) is { } m)
        {
            m = ResolveCurrency(m, ctx);
            if (main is null)
            {
                main = m;
            }
            else
            {
                target.Add(m);
            }
        }
        foreach (var r in set.Relevant)
        {
            if (r.Compute(ctx) is { } c)
            {
                target.Add(ResolveCurrency(c, ctx));
            }
        }
    }

    internal static TextKeyData ResolveCurrency(TextKeyData data, KeyComputeContext ctx)
    {
        if (!KeyCode.IsCurrencySlot(data.Code))
        {
            return data;
        }
        var slot = ctx.CurrencySet?.GetSlot(data.Code) ?? DefaultCurrencySlot(data.Code);
        return slot.With(popup: data.Popup ?? slot.Popup, groupId: data.GroupId);
    }

    private static TextKeyData DefaultCurrencySlot(int code)
    {
        var symbols = new[] { "$", "¢", "£", "€", "¥", "₩" };
        var index = Math.Clamp(KeyCode.CurrencySlot1 - code, 0, 5);
        return TextKeyData.Char(symbols[index]);
    }

    private static bool EvaluateVisible(TextKeyData data, KeyComputeContext ctx)
    {
        switch (data.Code)
        {
            case KeyCode.LanguageSwitch:
                return ctx.UtilityKeyAction == UtilityKeyAction.LanguageSwitch ||
                       ctx.UtilityKeyAction == UtilityKeyAction.Dynamic && ctx.HasMultipleSubtypes;
            case KeyCode.ImeUiModeMedia:
                return ctx.UtilityKeyAction == UtilityKeyAction.Emoji ||
                       ctx.UtilityKeyAction == UtilityKeyAction.Dynamic && !ctx.HasMultipleSubtypes;
        }
        return data.Type != KeyType.Placeholder;
    }

    private void ComputeFlay(TextKeyData data, KeyboardMode mode)
    {
        var code = data.Code;
        Shrink = mode switch
        {
            KeyboardMode.Numeric or KeyboardMode.NumericAdvanced or KeyboardMode.Phone or KeyboardMode.Phone2 => 1.0,
            _ => code switch
            {
                KeyCode.Shift or KeyCode.Delete => 1.5,
                KeyCode.ViewCharacters or KeyCode.ViewSymbols or KeyCode.ViewSymbols2 or KeyCode.Enter => 0.0,
                _ => 1.0,
            },
        };
        Grow = mode switch
        {
            KeyboardMode.Numeric or KeyboardMode.Phone or KeyboardMode.Phone2 => 0.0,
            KeyboardMode.NumericAdvanced => data.Type == KeyType.Numeric ? 1.0 : 0.0,
            _ => code is KeyCode.Space or KeyCode.CjkSpace ? 1.0 : 0.0,
        };
        WidthFactor = mode switch
        {
            KeyboardMode.Numeric or KeyboardMode.Phone or KeyboardMode.Phone2 => 2.68,
            KeyboardMode.NumericAdvanced => code switch
            {
                44 or 46 => 1.0,
                KeyCode.ViewSymbols or 61 => 1.26,
                _ => 1.56,
            },
            _ => code switch
            {
                KeyCode.Shift or KeyCode.Delete => 1.56,
                KeyCode.ViewCharacters or KeyCode.ViewSymbols or KeyCode.ViewSymbols2 or KeyCode.Enter => 1.56,
                _ => 1.0,
            },
        };
    }

    /// <inheritdoc />
    public override string ToString() => $"Key[{Row},{Column}] {Label ?? Icon.ToString()} ({Data})";
}

/// <summary>A keyboard computed from one or more layouts for a mode and subtype.</summary>
public sealed class ComputedKeyboard
{
    private readonly List<List<ComputedKey>> _rows;

    internal ComputedKeyboard(KeyboardMode mode, Subtype subtype, LayoutDirection direction, List<List<ComputedKey>> rows, PopupMapping? mapping, PopupMapping? defaultMapping)
    {
        Mode = mode;
        Subtype = subtype;
        Direction = direction;
        _rows = rows;
        PopupMapping = mapping;
        DefaultPopupMapping = defaultMapping;
    }

    /// <summary>Keyboard mode.</summary>
    public KeyboardMode Mode { get; }

    /// <summary>Subtype the keyboard was computed for.</summary>
    public Subtype Subtype { get; }

    /// <summary>Layout direction of the main layout.</summary>
    public LayoutDirection Direction { get; }

    /// <summary>Popup mapping of the subtype.</summary>
    public PopupMapping? PopupMapping { get; }

    /// <summary>Default popup mapping.</summary>
    public PopupMapping? DefaultPopupMapping { get; }

    /// <summary>All rows (including hidden keys).</summary>
    public IReadOnlyList<IReadOnlyList<ComputedKey>> Rows => _rows;

    /// <summary>All keys.</summary>
    public IEnumerable<ComputedKey> Keys => _rows.SelectMany(r => r);

    /// <summary>All visible keys.</summary>
    public IEnumerable<ComputedKey> VisibleKeys => Keys.Where(k => k.IsVisible);

    /// <summary>Number of rows.</summary>
    public int RowCount => _rows.Count;

    /// <summary>Incremented every time keys are recomputed or laid out.</summary>
    public int Version { get; private set; }

    /// <summary>The context used for the last computation.</summary>
    public KeyComputeContext? Context { get; private set; }

    /// <summary>Last layout width.</summary>
    public double Width { get; private set; }

    /// <summary>Last layout height.</summary>
    public double Height { get; private set; }

    /// <summary>Evaluates all keys for the given state.</summary>
    public void Compute(KeyComputeContext context)
    {
        Context = context;
        foreach (var key in Keys)
        {
            key.Compute(context, PopupMapping, DefaultPopupMapping);
        }
        Version++;
    }

    /// <summary>
    /// Lays out the keys inside a <paramref name="width"/> × <paramref name="height"/> area using the
    /// FlorisBoard flex ("flay") algorithm: keys have width factors, space grows and function keys shrink.
    /// </summary>
    public void Layout(double width, double height, KeyboardGeometryOptions? options = null)
    {
        options ??= new KeyboardGeometryOptions();
        Width = width;
        Height = height;
        var rows = _rows.Select(r => r.Where(k => k.IsVisible).ToList()).Where(r => r.Count > 0).ToList();
        if (rows.Count == 0 || width <= 0 || height <= 0)
        {
            return;
        }

        var split = options.SplitGap > 0 && options.SplitGap < width * 0.5;
        var layoutWidth = split ? width - options.SplitGap : width;
        var desiredW = layoutWidth / options.UnitsPerRow;
        var rowH = height / rows.Count;
        var sh = options.KeySpacingHorizontal / 2;
        var sv = options.KeySpacingVertical / 2;
        var mid = layoutWidth / 2;

        for (var r = 0; r < rows.Count; r++)
        {
            var row = rows[r];
            var posY = r * rowH;
            var available = layoutWidth / desiredW;
            double requested = 0, shrinkSum = 0, growSum = 0;
            foreach (var key in row)
            {
                requested += key.WidthFactor;
                shrinkSum += key.Shrink;
                growSum += key.Grow;
            }

            var posX = 0.0;
            if (requested <= available)
            {
                var additional = available - requested;
                for (var k = 0; k < row.Count; k++)
                {
                    var key = row[k];
                    var isFirst = k == 0;
                    var isLast = k == row.Count - 1;
                    double keyUnits;
                    if (growSum == 0)
                    {
                        keyUnits = row.Count == 1 ? key.WidthFactor + additional
                            : isFirst || isLast ? key.WidthFactor + additional / 2 : key.WidthFactor;
                    }
                    else
                    {
                        keyUnits = key.WidthFactor + additional * (key.Grow / growSum);
                    }
                    var keyW = desiredW * keyUnits;
                    var touch = new KeyRect(posX, posY, keyW, rowH);
                    var extraLeft = growSum == 0 && isFirst ? additional / 2 * desiredW : 0;
                    var extraRight = growSum == 0 && isLast ? additional / 2 * desiredW : 0;
                    key.TouchBounds = touch;
                    key.VisibleBounds = touch.Deflate(sh + extraLeft, sv, sh + extraRight, sv);
                    posX += keyW;
                }
            }
            else
            {
                var clipping = requested - available;
                foreach (var key in row)
                {
                    var units = key.Shrink == 0 || shrinkSum == 0 ? key.WidthFactor : key.WidthFactor - clipping * (key.Shrink / shrinkSum);
                    var keyW = desiredW * units;
                    var touch = new KeyRect(posX, posY, keyW, rowH);
                    key.TouchBounds = touch;
                    key.VisibleBounds = touch.Deflate(sh, sv, sh, sv);
                    posX += keyW;
                }
            }

            if (split)
            {
                ApplySplit(row, mid, options.SplitGap, layoutWidth, desiredW);
            }
        }
        Version++;
    }

    /// <summary>
    /// Splits a laid out row into two halves separated by <paramref name="gap"/>. Keys are assigned to the half
    /// containing their center and each half is compressed to fit; wide keys (the space bar) span the gap.
    /// </summary>
    private static void ApplySplit(List<ComputedKey> row, double mid, double gap, double layoutWidth, double unitWidth)
    {
        var left = new List<ComputedKey>();
        var right = new List<ComputedKey>();
        foreach (var key in row)
        {
            var t = key.TouchBounds;
            if (t.Width > unitWidth * 3 && t.Left < mid && t.Right > mid)
            {
                // Span the gap.
                key.TouchBounds = t with { Width = t.Width + gap };
                key.VisibleBounds = key.VisibleBounds with { Width = key.VisibleBounds.Width + gap };
                continue;
            }
            (t.CenterX <= mid + 0.01 ? left : right).Add(key);
        }
        // A spanning key splits the row: keys after it are shifted by the gap.
        if (left.Count > 0)
        {
            var leftEnd = left.Max(k => k.TouchBounds.Right);
            if (leftEnd > mid + 0.5)
            {
                var scale = mid / leftEnd;
                foreach (var key in left)
                {
                    key.TouchBounds = Scale(key.TouchBounds, 0, scale, 0);
                    key.VisibleBounds = Scale(key.VisibleBounds, 0, scale, 0);
                }
            }
        }
        if (right.Count > 0)
        {
            var rightStart = right.Min(k => k.TouchBounds.Left);
            var targetStart = Math.Max(rightStart, mid) + gap;
            var scale = rightStart < mid - 0.5 ? (layoutWidth - mid) / (layoutWidth - rightStart) : 1.0;
            foreach (var key in right)
            {
                key.TouchBounds = Scale(key.TouchBounds, rightStart, scale, targetStart);
                key.VisibleBounds = Scale(key.VisibleBounds, rightStart, scale, targetStart);
            }
        }
    }

    private static KeyRect Scale(KeyRect r, double origin, double scale, double target) =>
        r with { X = target + (r.X - origin) * scale, Width = r.Width * scale };

    /// <summary>Returns the visible, enabled key whose touch bounds contain the point.</summary>
    public ComputedKey? HitTest(double x, double y)
    {
        ComputedKey? best = null;
        var bestDistance = double.MaxValue;
        foreach (var key in _rows.SelectMany(r => r))
        {
            if (!key.IsVisible || !key.IsEnabled)
            {
                continue;
            }
            if (key.TouchBounds.Contains(x, y))
            {
                return key;
            }
            // Points in split gaps or margins snap to the nearest key of the row.
            if (y >= key.TouchBounds.Top && y < key.TouchBounds.Bottom)
            {
                var d = key.TouchBounds.CenterDistanceSquared(x, y);
                if (d < bestDistance)
                {
                    bestDistance = d;
                    best = key;
                }
            }
        }
        return best;
    }

    /// <summary>Finds the first visible key with the given code.</summary>
    public ComputedKey? FindKey(int code) => VisibleKeys.FirstOrDefault(k => k.Code == code);

    /// <summary>Returns all visible character keys (used for glide typing).</summary>
    public IReadOnlyList<ComputedKey> CharacterKeys => VisibleKeys.Where(k => k.IsCharacterKey && k.Data.Type == KeyType.Character).ToList();
}
