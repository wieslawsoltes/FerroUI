using System.Globalization;
using System.Text;

namespace ScreenKeyboard;

/// <summary>Unicode aware text helpers used by the editor and the NLP components.</summary>
public static class TextUtils
{
    /// <summary>Returns the code points of a string.</summary>
    public static int[] GetCodePoints(string text)
    {
        var list = new List<int>(text.Length);
        for (var i = 0; i < text.Length; i++)
        {
            if (char.IsHighSurrogate(text[i]) && i + 1 < text.Length && char.IsLowSurrogate(text[i + 1]))
            {
                list.Add(char.ConvertToUtf32(text[i], text[i + 1]));
                i++;
            }
            else
            {
                list.Add(text[i]);
            }
        }
        return list.ToArray();
    }

    /// <summary>
    /// Returns the length (in UTF-16 code units) of the last text element (grapheme cluster) of
    /// <paramref name="text"/>, so that deleting it never splits surrogate pairs, combining marks or emoji sequences.
    /// </summary>
    public static int LastGraphemeLength(string text)
    {
        if (text.Length == 0)
        {
            return 0;
        }
        var enumerator = StringInfo.GetTextElementEnumerator(text);
        var lastIndex = 0;
        while (enumerator.MoveNext())
        {
            lastIndex = enumerator.ElementIndex;
        }
        return text.Length - lastIndex;
    }

    /// <summary>Returns the length of the first grapheme cluster of <paramref name="text"/>.</summary>
    public static int FirstGraphemeLength(string text)
    {
        if (text.Length == 0)
        {
            return 0;
        }
        var enumerator = StringInfo.GetTextElementEnumerator(text);
        enumerator.MoveNext();
        var element = (string)enumerator.Current;
        return element.Length;
    }

    /// <summary>Returns <c>true</c> if the character is considered part of a word.</summary>
    public static bool IsWordChar(char c) =>
        char.IsLetterOrDigit(c) || c == '\'' || c == '’' || c == '_' || c == '-' ||
        CharUnicodeInfo.GetUnicodeCategory(c) is UnicodeCategory.NonSpacingMark or UnicodeCategory.SpacingCombiningMark;

    /// <summary>Returns <c>true</c> if the character is a letter-like word character (no joiners).</summary>
    public static bool IsWordCoreChar(char c) =>
        char.IsLetterOrDigit(c) ||
        CharUnicodeInfo.GetUnicodeCategory(c) is UnicodeCategory.NonSpacingMark or UnicodeCategory.SpacingCombiningMark;

    /// <summary>
    /// Gets the word currently being typed directly before the caret: the trailing run of word characters
    /// of <paramref name="textBeforeCursor"/>. Leading/trailing apostrophes and hyphens are excluded.
    /// </summary>
    public static string GetCurrentWord(string textBeforeCursor)
    {
        var end = textBeforeCursor.Length;
        var start = end;
        while (start > 0 && IsWordChar(textBeforeCursor[start - 1]))
        {
            start--;
        }
        // Do not treat leading joiners as part of the word.
        while (start < end && !IsWordCoreChar(textBeforeCursor[start]))
        {
            start++;
        }
        return textBeforeCursor.Substring(start, end - start);
    }

    /// <summary>Finds the index where the word before <paramref name="index"/> starts (for delete-word).</summary>
    public static int FindPreviousWordBoundary(string text, int index)
    {
        var i = Math.Min(index, text.Length);
        // Skip whitespace.
        while (i > 0 && char.IsWhiteSpace(text[i - 1]))
        {
            i--;
        }
        if (i > 0 && !IsWordChar(text[i - 1]))
        {
            // Delete a run of punctuation.
            var c = text[i - 1];
            while (i > 0 && !IsWordChar(text[i - 1]) && !char.IsWhiteSpace(text[i - 1]))
            {
                i--;
                if (text[i] != c)
                {
                    break;
                }
            }
            return i;
        }
        while (i > 0 && IsWordChar(text[i - 1]))
        {
            i--;
        }
        return i;
    }

    /// <summary>Finds the index where the word after <paramref name="index"/> ends (for forward delete-word).</summary>
    public static int FindNextWordBoundary(string text, int index)
    {
        var i = Math.Max(0, index);
        while (i < text.Length && char.IsWhiteSpace(text[i]))
        {
            i++;
        }
        if (i < text.Length && !IsWordChar(text[i]))
        {
            return i + 1;
        }
        while (i < text.Length && IsWordChar(text[i]))
        {
            i++;
        }
        return i;
    }

    /// <summary>Removes diacritics from a string ("é" → "e").</summary>
    public static string RemoveDiacritics(string text)
    {
        var normalized = text.Normalize(NormalizationForm.FormD);
        var sb = new StringBuilder(normalized.Length);
        foreach (var c in normalized)
        {
            if (CharUnicodeInfo.GetUnicodeCategory(c) != UnicodeCategory.NonSpacingMark)
            {
                sb.Append(c);
            }
        }
        return sb.ToString().Normalize(NormalizationForm.FormC);
    }

    /// <summary>Returns the base (diacritic-free, lower-case) character for a character.</summary>
    public static char BaseChar(char c)
    {
        var s = c.ToString().Normalize(NormalizationForm.FormD);
        return char.ToLowerInvariant(s[0]);
    }

    /// <summary>Applies the capitalization pattern of <paramref name="pattern"/> to <paramref name="word"/>.</summary>
    public static string MatchCase(string word, string pattern, CultureInfo culture)
    {
        if (pattern.Length == 0 || word.Length == 0)
        {
            return word;
        }
        var letters = pattern.Where(char.IsLetter).ToArray();
        if (letters.Length > 1 && letters.All(char.IsUpper))
        {
            return word.ToUpper(culture);
        }
        if (char.IsUpper(pattern[0]))
        {
            return Capitalize(word, culture);
        }
        return word;
    }

    /// <summary>Upper-cases the first letter of <paramref name="word"/>.</summary>
    public static string Capitalize(string word, CultureInfo culture)
    {
        if (word.Length == 0)
        {
            return word;
        }
        var first = FirstGraphemeLength(word);
        return word.Substring(0, first).ToUpper(culture) + word.Substring(first);
    }
}
