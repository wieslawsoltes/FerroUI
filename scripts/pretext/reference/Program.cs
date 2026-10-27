using System.Globalization;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;
using Pretext;
using Pretext.Tests;

if (args.Length != 2)
{
    Console.Error.WriteLine("usage: Reference <cases.json> <output.json>");
    return 2;
}

var options = new JsonSerializerOptions(JsonSerializerDefaults.Web) { WriteIndented = true };
options.Converters.Add(new JsonStringEnumConverter());
var cases = JsonSerializer.Deserialize<List<Case>>(File.ReadAllText(args[0]), options)!;
var results = new List<object>();
foreach (var input in cases)
{
    PretextLayout.SetTextMeasurerFactory(new DelegateTextMeasurerFactory(MeasureWidth));
    PretextLayout.SetLocale(input.Locale);
    var prepared = PretextLayout.PrepareWithSegments(input.Text, input.Font, new PrepareOptions(input.WhiteSpace, input.WordBreak));
    var lines = PretextLayout.LayoutWithLines(prepared, input.Width, input.LineHeight);
    var streamed = new List<LayoutLine>();
    var cursor = new LayoutCursor(0, 0);
    while (PretextLayout.LayoutNextLine(prepared, cursor, input.Width) is { } line)
    {
        streamed.Add(line);
        if (line.End == cursor) throw new InvalidOperationException("reference cursor did not progress");
        cursor = line.End;
    }
    results.Add(new {
        input,
        prepared.Segments,
        prepared.Widths,
        prepared.Kinds,
        prepared.LineEndFitAdvances,
        prepared.LineEndPaintAdvances,
        prepared.BreakableWidths,
        prepared.BreakablePrefixWidths,
        prepared.Chunks,
        prepared.SegmentLevels,
        prepared.DiscretionaryHyphenWidth,
        prepared.TabStopAdvance,
        prepared.SimpleLineWalkFastPath,
        naturalWidth = PretextLayout.MeasureNaturalWidth(prepared),
        stats = PretextLayout.MeasureLineStats(prepared, input.Width),
        metrics = PretextLayout.Layout(prepared, input.Width, input.LineHeight),
        lines,
        streamed,
    });
}
File.WriteAllText(args[1], JsonSerializer.Serialize(new {
    sourceCommit = "e70a544e43f7ecffee76d2d22a0224213d7fbba3",
    cases = results,
}, options) + "\n");
Console.WriteLine($"Generated {results.Count} unchanged-source parity cases.");
return 0;

// Same deterministic scalar measurement as PretextLayoutParityTests.cs. The
// context-dependent decimal advances distinguish prefix widths from sums.
static double MeasureWidth(string text, string font)
{
    var size = PretextFontParser.Parse(font).Size;
    var width = 0d;
    var previousWasDecimalDigit = false;
    foreach (var rune in text.EnumerateRunes())
    {
        var ch = rune.ToString();
        var code = rune.Value;
        if (ch == " ") { width += size * 0.33; previousWasDecimalDigit = false; }
        else if (ch == "\t") { width += size * 1.32; previousWasDecimalDigit = false; }
        else if ((code >= 0x1F300 && code <= 0x1FAFF) || (code >= 0x2600 && code <= 0x26FF) || (code >= 0x2700 && code <= 0x27BF) || code == 0xFE0F)
        { width += size; previousWasDecimalDigit = false; }
        else if (Rune.GetUnicodeCategory(rune) == UnicodeCategory.DecimalDigitNumber)
        { width += size * (previousWasDecimalDigit ? 0.48 : 0.52); previousWasDecimalDigit = true; }
        else if (IsWide(code)) { width += size; previousWasDecimalDigit = false; }
        else if (".,!?;:%)]}'\"”’»›…—-".Contains(ch, StringComparison.Ordinal))
        { width += size * 0.4; previousWasDecimalDigit = false; }
        else { width += size * 0.6; previousWasDecimalDigit = false; }
    }
    return width;
}

static bool IsWide(int code) => code is
    >= 0x4E00 and <= 0x9FFF or >= 0x3400 and <= 0x4DBF or >= 0xF900 and <= 0xFAFF or
    >= 0x2F800 and <= 0x2FA1F or >= 0x20000 and <= 0x2A6DF or >= 0x2A700 and <= 0x2B73F or
    >= 0x2B740 and <= 0x2B81F or >= 0x2B820 and <= 0x2CEAF or >= 0x2CEB0 and <= 0x2EBEF or
    >= 0x2EBF0 and <= 0x2EE5D or >= 0x30000 and <= 0x3134F or >= 0x31350 and <= 0x323AF or
    >= 0x323B0 and <= 0x33479 or >= 0x3000 and <= 0x303F or >= 0x3040 and <= 0x309F or
    >= 0x30A0 and <= 0x30FF or >= 0xAC00 and <= 0xD7AF or >= 0xFF00 and <= 0xFFEF;

sealed record Case(string Text, string Font, WhiteSpaceMode WhiteSpace, WordBreakMode WordBreak, string? Locale, double Width, double LineHeight);
