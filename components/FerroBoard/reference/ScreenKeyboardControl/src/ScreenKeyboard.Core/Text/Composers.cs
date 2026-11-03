namespace ScreenKeyboard.Text;

/// <summary>
/// A composer transforms characters as they are typed, based on the preceding text.
/// Used for scripts such as Hangul (jamo → syllables), Japanese Kana (dakuten) or Vietnamese Telex.
/// </summary>
public interface IComposer
{
    /// <summary>Composer id.</summary>
    string Id { get; }

    /// <summary>Human readable name.</summary>
    string Label { get; }

    /// <summary>How many characters before the cursor the composer needs to read.</summary>
    int ToRead { get; }

    /// <summary>
    /// Computes the edit for inserting <paramref name="toInsert"/> after <paramref name="precedingText"/>.
    /// </summary>
    /// <returns>Number of characters to delete before the cursor and the text to insert.</returns>
    (int Delete, string Insert) GetActions(string precedingText, string toInsert);
}

/// <summary>The default composer: simply appends the text.</summary>
public sealed class AppenderComposer : IComposer
{
    public static AppenderComposer Instance { get; } = new();
    public string Id => "appender";
    public string Label => "Appender";
    public int ToRead => 0;
    public (int Delete, string Insert) GetActions(string precedingText, string toInsert) => (0, toInsert);
}

/// <summary>A rule based composer (e.g. Vietnamese Telex/VNI): suffix rules replace the typed sequence.</summary>
public sealed class RulesComposer : IComposer
{
    private readonly IReadOnlyList<KeyValuePair<string, string>> _orderedRules;

    public RulesComposer(string id, string label, IReadOnlyDictionary<string, string> rules)
    {
        Id = id;
        Label = label;
        Rules = rules;
        _orderedRules = rules.OrderByDescending(r => r.Key.Length).ToList();
        ToRead = Math.Max(0, rules.Keys.DefaultIfEmpty(string.Empty).Max(k => k.Length) - 1);
    }

    public string Id { get; }
    public string Label { get; }
    public int ToRead { get; }
    public IReadOnlyDictionary<string, string> Rules { get; }

    public (int Delete, string Insert) GetActions(string precedingText, string toInsert)
    {
        var str = precedingText + toInsert;
        var lower = str.ToLowerInvariant();
        foreach (var rule in _orderedRules)
        {
            if (rule.Key.Length > 0 && lower.EndsWith(rule.Key, StringComparison.Ordinal))
            {
                var firstOfKey = str.Substring(str.Length - rule.Key.Length, 1);
                var value = firstOfKey.ToUpperInvariant() == firstOfKey && firstOfKey.ToLowerInvariant() != firstOfKey
                    ? rule.Value.ToUpperInvariant()
                    : rule.Value;
                return (rule.Key.Length - 1, value);
            }
        }
        return (0, toInsert);
    }
}

/// <summary>Composes Hangul jamo into syllable blocks (port of FlorisBoard's HangulUnicode composer).</summary>
public sealed class HangulComposer : IComposer
{
    private const string Initials = "ㄱㄲㄴㄷㄸㄹㅁㅂㅃㅅㅆㅇㅈㅉㅊㅋㅌㅍㅎ";
    private const string Medials = "ㅏㅐㅑㅒㅓㅔㅕㅖㅗㅘㅙㅚㅛㅜㅝㅞㅟㅠㅡㅢㅣ";
    private const string Finals = "_ㄱㄲㄳㄴㄵㄶㄷㄹㄺㄻㄼㄽㄾㄿㅀㅁㅂㅄㅅㅆㅇㅈㅊㅋㅌㅍㅎ";

    private static readonly Dictionary<char, (string Seconds, string Composed)> s_medialComp = new()
    {
        ['ㅗ'] = ("ㅏㅐㅣ", "ㅘㅙㅚ"),
        ['ㅜ'] = ("ㅓㅔㅣ", "ㅝㅞㅟ"),
        ['ㅡ'] = ("ㅣ", "ㅢ"),
    };

    private static readonly Dictionary<char, (string Seconds, string Composed)> s_finalComp = new()
    {
        ['ㄱ'] = ("ㅅ", "ㄳ"),
        ['ㄴ'] = ("ㅈㅎ", "ㄵㄶ"),
        ['ㄹ'] = ("ㄱㅁㅂㅅㅌㅍㅎ", "ㄺㄻㄼㄽㄾㄿㅀ"),
        ['ㅂ'] = ("ㅅ", "ㅄ"),
    };

    private static readonly Dictionary<char, (char First, char Second)> s_finalCompRev = Reverse(s_finalComp);

    private static Dictionary<char, (char, char)> Reverse(Dictionary<char, (string Seconds, string Composed)> map)
    {
        var result = new Dictionary<char, (char, char)>();
        foreach (var pair in map)
        {
            for (var i = 0; i < pair.Value.Seconds.Length; i++)
            {
                result[pair.Value.Composed[i]] = (pair.Key, pair.Value.Seconds[i]);
            }
        }
        return result;
    }

    public string Id => "hangul-unicode";
    public string Label => "Hangul Unicode";
    public int ToRead => 1;

    private static char Syllable(int ini, int med, int fin) => (char)(ini * 588 + med * 28 + fin + 44032);

    public (int Delete, string Insert) GetActions(string precedingText, string toInsert)
    {
        if (precedingText.Length == 0 || toInsert.Length == 0)
        {
            return (0, toInsert);
        }
        var c = toInsert[0];
        var last = precedingText[precedingText.Length - 1];
        int lastOrd = last;

        if (Initials.IndexOf(last) >= 0 && Medials.IndexOf(c) >= 0)
        {
            return (1, Syllable(Initials.IndexOf(last), Medials.IndexOf(c), 0).ToString());
        }
        if (lastOrd >= 44032 && lastOrd <= 55203)
        {
            var ini = (lastOrd - 44032) / 588;
            var med = (lastOrd - 44032 - ini * 588) / 28;
            var fin = (lastOrd - 44032) % 28;
            if (c == '_')
            {
                return (0, toInsert);
            }
            if (fin == 0 && Finals.IndexOf(c) > 0)
            {
                return (1, Syllable(ini, med, Finals.IndexOf(c)).ToString());
            }
            if (s_finalComp.TryGetValue(Finals[fin], out var fc) && fc.Seconds.IndexOf(c) >= 0)
            {
                return (1, Syllable(ini, med, Finals.IndexOf(fc.Composed[fc.Seconds.IndexOf(c)])).ToString());
            }
            if (fin != 0 && !s_finalCompRev.ContainsKey(Finals[fin]) && Medials.IndexOf(c) >= 0 && Initials.IndexOf(Finals[fin]) >= 0)
            {
                return (1, $"{Syllable(ini, med, 0)}{Syllable(Initials.IndexOf(Finals[fin]), Medials.IndexOf(c), 0)}");
            }
            if (s_finalCompRev.TryGetValue(Finals[fin], out var rev) && Medials.IndexOf(c) >= 0)
            {
                return (1, $"{Syllable(ini, med, Finals.IndexOf(rev.First))}{Syllable(Initials.IndexOf(rev.Second), Medials.IndexOf(c), 0)}");
            }
            if (s_medialComp.TryGetValue(Medials[med], out var mc) && mc.Seconds.IndexOf(c) >= 0 && fin == 0)
            {
                return (1, Syllable(ini, Medials.IndexOf(mc.Composed[mc.Seconds.IndexOf(c)]), 0).ToString());
            }
        }
        else if (s_medialComp.TryGetValue(last, out var mc2) && mc2.Seconds.IndexOf(c) >= 0)
        {
            return (1, mc2.Composed[mc2.Seconds.IndexOf(c)].ToString());
        }
        else if (s_finalComp.TryGetValue(last, out var fc2) && fc2.Seconds.IndexOf(c) >= 0)
        {
            return (1, fc2.Composed[fc2.Seconds.IndexOf(c)].ToString());
        }
        return (0, toInsert);
    }
}

/// <summary>
/// Japanese Kana composer: applies dakuten (゛), handakuten (゜) and small-kana modifiers to the
/// preceding kana (port of FlorisBoard's KanaUnicode composer).
/// </summary>
public sealed class KanaComposer : IComposer
{
    private const char Dakuten = '゙';
    private const char DakutenSpacing = '゛';
    private const char Handakuten = '゚';
    private const char HandakutenSpacing = '゜';
    private const char SmallSentinel = '〓'; // 〓 (KeyCode.KanaSmall)

    private static readonly Dictionary<char, char> s_daku = Pairs(
        "うゔかがきぎくぐけげこごさざしじすずせぜそぞただちぢつづてでとどはばひびふぶへべほぼ" +
        "ウヴカガキギクグケゲコゴサザシジスズセゼソゾタダチヂツヅテデトドハバヒビフブヘベホボワヷヰヸヱヹヲヺゝゞヽヾ");

    private static readonly Dictionary<char, char> s_handaku = Pairs("はぱひぴふぷへぺほぽハパヒピフプヘペホポ");

    private static readonly Dictionary<char, string> s_small = new()
    {
        ['あ'] = "ぁ", ['い'] = "ぃ", ['え'] = "ぇ", ['う'] = "ぅ", ['お'] = "ぉ", ['か'] = "ゕ", ['け'] = "ゖ",
        ['つ'] = "っ", ['や'] = "ゃ", ['ゆ'] = "ゅ", ['よ'] = "ょ", ['わ'] = "ゎ",
        ['ア'] = "ァ", ['イ'] = "ィ", ['エ'] = "ェ", ['ウ'] = "ゥ", ['オ'] = "ォ", ['カ'] = "ヵ", ['ク'] = "ㇰ",
        ['ケ'] = "ヶ", ['シ'] = "ㇱ", ['ス'] = "ㇲ", ['ツ'] = "ッ", ['ト'] = "ㇳ", ['ヌ'] = "ㇴ", ['ハ'] = "ㇵ",
        ['ヒ'] = "ㇶ", ['フ'] = "ㇷ", ['ヘ'] = "ㇸ", ['ホ'] = "ㇹ", ['ム'] = "ㇺ", ['ヤ'] = "ャ", ['ユ'] = "ュ",
        ['ヨ'] = "ョ", ['ラ'] = "ㇻ", ['リ'] = "ㇼ", ['ル'] = "ㇽ", ['レ'] = "ㇾ", ['ロ'] = "ㇿ", ['ワ'] = "ヮ",
    };

    private static readonly Dictionary<char, char> s_reverseDaku = s_daku.ToDictionary(p => p.Value, p => p.Key);
    private static readonly Dictionary<char, char> s_reverseHandaku = s_handaku.ToDictionary(p => p.Value, p => p.Key);
    private static readonly Dictionary<string, char> s_reverseSmall = s_small.ToDictionary(p => p.Value, p => p.Key);

    private static Dictionary<char, char> Pairs(string s)
    {
        var map = new Dictionary<char, char>();
        for (var i = 0; i + 1 < s.Length; i += 2)
        {
            map[s[i]] = s[i + 1];
        }
        return map;
    }

    public string Id => "kana-unicode";
    public string Label => "Kana Unicode";
    public int ToRead => 1;

    public (int Delete, string Insert) GetActions(string precedingText, string toInsert)
    {
        if (toInsert.Length == 0)
        {
            return (0, toInsert);
        }
        var c = toInsert[0];
        var isDaku = c is Dakuten or DakutenSpacing;
        var isHandaku = c is Handakuten or HandakutenSpacing;
        var isSmall = c == SmallSentinel;
        if (precedingText.Length == 0)
        {
            return isDaku || isHandaku || isSmall ? (0, string.Empty) : (0, toInsert);
        }
        var last = precedingText[precedingText.Length - 1];
        if (isDaku)
        {
            if (s_daku.TryGetValue(last, out var d)) return (1, d.ToString());
            if (s_reverseDaku.TryGetValue(last, out var r)) return (1, r.ToString());
            return (0, toInsert);
        }
        if (isHandaku)
        {
            if (s_handaku.TryGetValue(last, out var h)) return (1, h.ToString());
            if (s_reverseHandaku.TryGetValue(last, out var r)) return (1, r.ToString());
            return (0, toInsert);
        }
        if (isSmall)
        {
            if (s_small.TryGetValue(last, out var s)) return (1, s);
            if (s_reverseSmall.TryGetValue(last.ToString(), out var r)) return (1, r.ToString());
            return (0, string.Empty);
        }
        return (0, toInsert);
    }
}
