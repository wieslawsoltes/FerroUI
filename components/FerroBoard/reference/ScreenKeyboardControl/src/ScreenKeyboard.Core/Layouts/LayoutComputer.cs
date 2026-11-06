using ScreenKeyboard.Keys;

namespace ScreenKeyboard.Layouts;

/// <summary>
/// Builds <see cref="ComputedKeyboard"/> instances by merging a subtype's layouts the same way
/// FlorisBoard does: an optional extension row (number row), the main layout and a modifier layout
/// (shift/delete/space row) whose placeholder key is replaced by the last main row.
/// </summary>
public sealed class LayoutComputer
{
    private readonly KeyboardResources _resources;

    /// <summary>Creates a computer for the given resources.</summary>
    public LayoutComputer(KeyboardResources resources)
    {
        _resources = resources;
    }

    /// <summary>The resources used.</summary>
    public KeyboardResources Resources => _resources;

    /// <summary>Id of the default modifier layouts.</summary>
    public ComponentName DefaultModifier { get; set; } = new(KeyboardResources.CoreLayoutsExtension, "default");

    /// <summary>
    /// Computes the key arrangement for a mode. The returned keyboard still needs <see cref="ComputedKeyboard.Compute"/>
    /// (state evaluation) and <see cref="ComputedKeyboard.Layout"/> (geometry).
    /// </summary>
    /// <param name="mode">The keyboard mode.</param>
    /// <param name="subtype">The subtype.</param>
    /// <param name="numberRow">Whether a number row is added on top of the characters layout.</param>
    public ComputedKeyboard Compute(KeyboardMode mode, Subtype subtype, bool numberRow = false)
    {
        KeyboardLayout? main;
        KeyboardLayout? modifier = null;
        KeyboardLayout? extension = null;
        var map = subtype.Layouts;

        switch (mode)
        {
            case KeyboardMode.Characters:
                if (numberRow)
                {
                    extension = _resources.GetLayout(LayoutType.NumericRow, map.NumericRow);
                }
                main = _resources.GetLayout(LayoutType.Characters, map.Characters)
                       ?? _resources.GetLayout(LayoutType.Characters, new ComponentName(KeyboardResources.CoreLayoutsExtension, "qwerty"));
                modifier = ResolveModifier(main, LayoutType.CharactersMod);
                break;
            case KeyboardMode.Symbols:
                extension = _resources.GetLayout(LayoutType.NumericRow, map.NumericRow);
                main = _resources.GetLayout(LayoutType.Symbols, map.Symbols);
                modifier = ResolveModifier(main, LayoutType.SymbolsMod);
                break;
            case KeyboardMode.Symbols2:
                main = _resources.GetLayout(LayoutType.Symbols2, map.Symbols2);
                modifier = ResolveModifier(main, LayoutType.Symbols2Mod);
                break;
            case KeyboardMode.Numeric:
                main = _resources.GetLayout(LayoutType.Numeric, map.Numeric);
                break;
            case KeyboardMode.NumericAdvanced:
                main = _resources.GetLayout(LayoutType.NumericAdvanced, map.NumericAdvanced);
                break;
            case KeyboardMode.Phone:
                main = _resources.GetLayout(LayoutType.Phone, map.Phone);
                break;
            case KeyboardMode.Phone2:
                main = _resources.GetLayout(LayoutType.Phone2, map.Phone2);
                break;
            default:
                throw new ArgumentOutOfRangeException(nameof(mode));
        }

        var arrangement = Merge(main, modifier, extension);
        var rows = new List<List<ComputedKey>>(arrangement.Count);
        for (var r = 0; r < arrangement.Count; r++)
        {
            var row = new List<ComputedKey>(arrangement[r].Count);
            for (var c = 0; c < arrangement[r].Count; c++)
            {
                row.Add(new ComputedKey(arrangement[r][c], r, c));
            }
            rows.Add(row);
        }

        if (mode == KeyboardMode.Characters && rows.Count > 0)
        {
            AddHints(rows, subtype);
        }

        var mapping = _resources.GetPopupMapping(subtype.PopupMapping);
        var defaultMapping = _resources.GetPopupMapping(new ComponentName(Subtype.CoreLocalization, "default"));
        return new ComputedKeyboard(mode, subtype, main?.Direction ?? LayoutDirection.Ltr, rows, mapping, defaultMapping);
    }

    private KeyboardLayout? ResolveModifier(KeyboardLayout? main, LayoutType modType)
    {
        if (main?.Modifier is { } mod)
        {
            var custom = _resources.GetLayout(modType, mod);
            if (custom is not null)
            {
                return custom;
            }
        }
        return _resources.GetLayout(modType, DefaultModifier);
    }

    /// <summary>Merges extension, main and modifier arrangements into rows.</summary>
    internal static List<IReadOnlyList<AbstractKeyData>> Merge(KeyboardLayout? main, KeyboardLayout? modifier, KeyboardLayout? extension)
    {
        var result = new List<IReadOnlyList<AbstractKeyData>>();
        if (extension is not null)
        {
            result.AddRange(extension.Arrangement.Rows);
        }
        var mainRows = main?.Arrangement.Rows ?? [];
        var modRows = modifier?.Arrangement.Rows ?? [];
        if (mainRows.Count > 0 && modRows.Count > 0)
        {
            for (var i = 0; i < mainRows.Count; i++)
            {
                if (i + 1 < mainRows.Count)
                {
                    result.Add(mainRows[i]);
                    continue;
                }
                // Merge the last main row into the first modifier row, replacing the placeholder (code 0).
                var merged = new List<AbstractKeyData>();
                var replaced = false;
                foreach (var modKey in modRows[0])
                {
                    if (modKey is TextKeyData { Code: 0 } && !replaced)
                    {
                        merged.AddRange(mainRows[i]);
                        replaced = true;
                    }
                    else
                    {
                        merged.Add(modKey);
                    }
                }
                result.Add(merged);
            }
            for (var i = 1; i < modRows.Count; i++)
            {
                result.Add(modRows[i]);
            }
        }
        else if (mainRows.Count > 0)
        {
            result.AddRange(mainRows);
        }
        else
        {
            result.AddRange(modRows);
        }
        return result;
    }

    private void AddHints(List<List<ComputedKey>> rows, Subtype subtype)
    {
        var symbols = Merge(
            _resources.GetLayout(LayoutType.Symbols, subtype.Layouts.Symbols),
            ResolveModifier(_resources.GetLayout(LayoutType.Symbols, subtype.Layouts.Symbols), LayoutType.SymbolsMod),
            _resources.GetLayout(LayoutType.NumericRow, subtype.Layouts.NumericRow));
        if (symbols.Count == 0)
        {
            return;
        }

        // Number hints always go on the first row.
        var first = rows[0];
        for (var k = 0; k < first.Count && k < symbols[0].Count; k++)
        {
            first[k].NumberHintSource = symbols[0][k];
        }

        // Symbol hints are bottom aligned.
        var offset = rows.Count - symbols.Count;
        for (var r = 0; r < rows.Count; r++)
        {
            var sr = r - offset;
            if (sr < 0 || sr >= symbols.Count)
            {
                continue;
            }
            var row = rows[r];
            var symbolRow = symbols[sr];
            for (var k = 0; k < row.Count && k < symbolRow.Count; k++)
            {
                row[k].SymbolHintSource = symbolRow[k];
            }
        }
    }
}
