use crate::media::fonts::bcp47_script_resolver::Bcp47ScriptResolver;
use crate::media::fonts::{FontCodePageCoverage, OpenTypeTag};
use crate::media::text_formatting::unicode::{Codepoint, Script};
use crate::media::GlyphTypeface;
use crate::utilities::CultureInfo;

/// Script-specific hints used by the font fallback: which scripts need a
/// locale aware choice, which OS/2 Unicode range bit and probe codepoint tell
/// that a font covers a script, and which OpenType script tags a font must
/// declare to shape a complex script.
pub(crate) struct FontFallbackScriptHints;

impl FontFallbackScriptHints {
    /// Whether the choice of a fallback font for this script depends on the
    /// culture (scripts with a probe codepoint).
    pub fn is_locale_sensitive(script: Script) -> bool {
        Self::get_probe_codepoint(script) != 0
    }

    /// Returns the script to drive font selection for the codepoint, refining
    /// the base Unicode script using the culture when the codepoint is
    /// locale-sensitive (e.g. a Han ideograph under `ja-JP` is treated as
    /// Hiragana so Japanese fonts are preferred).
    ///
    /// When `culture` is `None` or invariant, or the codepoint is not
    /// locale-sensitive, the result is the codepoint's script unchanged.
    pub fn refine_with_culture(codepoint: Codepoint, culture: Option<&CultureInfo>) -> Script {
        let script = codepoint.script();

        if script == Script::KatakanaOrHiragana {
            return if codepoint.has_script_extension(Script::Hiragana) { Script::Hiragana } else { Script::Katakana };
        }

        if script != Script::Han {
            return script;
        }

        match Bcp47ScriptResolver::get_script_subtag(culture).as_deref() {
            Some("Jpan") => Script::Hiragana,
            Some("Kore") => Script::Hangul,
            _ => script,
        }
    }

    /// Returns `true` when the candidate self-declares coverage of the
    /// script or code page implied by the culture.
    ///
    /// Two independent declarations are consulted, either of which is
    /// sufficient: the `meta` table's design/supported languages (a BCP-47
    /// prefix match against the culture name) and the OS/2 code page range
    /// bits (matched against the script the culture implies). Fonts that
    /// declare neither return `false`, and so do a `None` or invariant
    /// culture.
    pub fn is_font_compatible_with_culture(candidate: &GlyphTypeface, culture: Option<&CultureInfo>) -> bool {
        let Some(culture) = culture else {
            return false;
        };

        if culture.is_invariant() {
            return false;
        }

        if candidate.declares_language_coverage(Some(culture)) {
            return true;
        }

        let coverage = candidate.code_page_coverage();

        if coverage == FontCodePageCoverage::None {
            return false;
        }

        let required = match Bcp47ScriptResolver::get_script_subtag(Some(culture)).as_deref() {
            Some("Jpan") => FontCodePageCoverage::JapaneseJis,
            Some("Kore") => FontCodePageCoverage::KoreanWansung | FontCodePageCoverage::KoreanJohab,
            Some("Hans") => FontCodePageCoverage::ChineseSimplified,
            Some("Hant") => FontCodePageCoverage::ChineseTraditional,
            Some("Cyrl") => FontCodePageCoverage::Cyrillic,
            Some("Grek") => FontCodePageCoverage::Greek,
            Some("Arab") => FontCodePageCoverage::Arabic,
            Some("Hebr") => FontCodePageCoverage::Hebrew,
            Some("Thai") => FontCodePageCoverage::Thai,
            Some("Latn") => FontCodePageCoverage::Latin1,
            _ => return false,
        };

        coverage.intersects(required)
    }

    /// A codepoint every font designed for the script covers, or 0 for
    /// scripts that are not tracked.
    pub fn get_probe_codepoint(script: Script) -> i32 {
        match script {
            Script::Han => 0x4E2D,
            Script::Hiragana => 0x3042,
            Script::Katakana => 0x30A2,
            Script::KatakanaOrHiragana => 0x3042,
            Script::Hangul => 0xAC00,
            Script::Bopomofo => 0x3105,
            Script::Arabic => 0x0627,
            Script::Hebrew => 0x05D0,
            Script::Devanagari => 0x0915,
            Script::Bengali => 0x0995,
            Script::Thai => 0x0E01,
            Script::Tibetan => 0x0F40,
            Script::Cyrillic => 0x0410,
            Script::Greek => 0x0391,
            _ => 0,
        }
    }

    /// The bit of the OS/2 `ulUnicodeRange1..4` fields that declares support
    /// for the script.
    pub fn try_get_os2_bit(script: Script) -> Option<i32> {
        Some(match script {
            Script::Greek => 7,
            Script::Cyrillic => 9,
            Script::Hebrew => 11,
            Script::Arabic => 13,
            Script::Devanagari => 15,
            Script::Bengali => 16,
            Script::Thai => 24,
            Script::Hiragana => 49,
            Script::Katakana => 50,
            Script::KatakanaOrHiragana => 49,
            Script::Bopomofo => 51,
            Script::Hangul => 56,
            Script::Han => 59,
            Script::Tibetan => 70,
            _ => return None,
        })
    }

    /// For scripts that cannot be rendered without OpenType shaping rules:
    /// the script tags (new and old shaping model) of which a font must
    /// declare at least one in GSUB/GPOS. `None` for simple scripts.
    pub fn try_get_complex_shaping_tags(script: Script) -> Option<(OpenTypeTag, OpenTypeTag)> {
        const fn tag(tag: &[u8; 4]) -> OpenTypeTag {
            OpenTypeTag::from_bytes(tag[0], tag[1], tag[2], tag[3])
        }

        const fn single(value: &[u8; 4]) -> (OpenTypeTag, OpenTypeTag) {
            (tag(value), tag(value))
        }

        Some(match script {
            Script::Arabic => single(b"arab"),
            Script::Syriac => single(b"syrc"),
            Script::Mongolian => single(b"mong"),
            Script::Thaana => single(b"thaa"),
            Script::Khmer => single(b"khmr"),
            Script::Tibetan => single(b"tibt"),
            Script::Sinhala => single(b"sinh"),
            Script::Devanagari => (tag(b"dev2"), tag(b"deva")),
            Script::Bengali => (tag(b"bng2"), tag(b"beng")),
            Script::Gurmukhi => (tag(b"gur2"), tag(b"guru")),
            Script::Gujarati => (tag(b"gjr2"), tag(b"gujr")),
            Script::Oriya => (tag(b"ory2"), tag(b"orya")),
            Script::Tamil => (tag(b"tml2"), tag(b"taml")),
            Script::Telugu => (tag(b"tel2"), tag(b"telu")),
            Script::Kannada => (tag(b"knd2"), tag(b"knda")),
            Script::Malayalam => (tag(b"mlm2"), tag(b"mlym")),
            Script::Myanmar => (tag(b"mym2"), tag(b"mymr")),
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refine_with_culture_returns_han_for_han_without_culture() {
        // U+4E2D - Han script.
        let cp = Codepoint::new(0x4E2D);

        assert_eq!(FontFallbackScriptHints::refine_with_culture(cp, None), Script::Han);
    }

    #[test]
    fn refine_with_culture_han_with_japanese_culture_maps_to_hiragana() {
        let cp = Codepoint::new(0x4E2D);
        let refined = FontFallbackScriptHints::refine_with_culture(cp, Some(&CultureInfo::get_culture_info("ja-JP")));

        assert_eq!(refined, Script::Hiragana);
    }

    #[test]
    fn refine_with_culture_han_with_korean_culture_maps_to_hangul() {
        let cp = Codepoint::new(0x4E2D);
        let refined = FontFallbackScriptHints::refine_with_culture(cp, Some(&CultureInfo::get_culture_info("ko-KR")));

        assert_eq!(refined, Script::Hangul);
    }

    #[test]
    fn refine_with_culture_han_with_chinese_culture_stays_han() {
        let cp = Codepoint::new(0x4E2D);
        let simplified = FontFallbackScriptHints::refine_with_culture(cp, Some(&CultureInfo::get_culture_info("zh-CN")));
        let traditional =
            FontFallbackScriptHints::refine_with_culture(cp, Some(&CultureInfo::get_culture_info("zh-TW")));

        assert_eq!(simplified, Script::Han);
        assert_eq!(traditional, Script::Han);
    }

    #[test]
    fn refine_with_culture_common_codepoint_passes_through() {
        // U+30FC has the primary script Common (its Hiragana/Katakana membership lives in the
        // script extensions and is consulted directly through `Codepoint::has_script_extension`).
        // The refinement only applies to codepoints whose primary script is ambiguous.
        let cp = Codepoint::new(0x30FC);

        assert_eq!(FontFallbackScriptHints::refine_with_culture(cp, None), cp.script());
    }

    #[test]
    fn refine_with_culture_latin_codepoint_returns_latin_unchanged() {
        let cp = Codepoint::new('A' as u32);
        let refined = FontFallbackScriptHints::refine_with_culture(cp, Some(&CultureInfo::get_culture_info("en-US")));

        assert_eq!(refined, Script::Latin);
    }

    #[test]
    fn is_locale_sensitive_true_for_han() {
        assert!(FontFallbackScriptHints::is_locale_sensitive(Script::Han));
    }

    #[test]
    fn is_locale_sensitive_false_for_latin() {
        assert!(!FontFallbackScriptHints::is_locale_sensitive(Script::Latin));
    }

    #[test]
    fn try_get_os2_bit_returns_spec_bit() {
        for (script, expected) in [
            (Script::Hiragana, 49),
            (Script::Katakana, 50),
            (Script::Hangul, 56),
            (Script::Han, 59),
            (Script::Cyrillic, 9),
            (Script::Arabic, 13),
        ] {
            assert_eq!(FontFallbackScriptHints::try_get_os2_bit(script), Some(expected));
        }
    }

    #[test]
    fn try_get_os2_bit_latin_returns_false() {
        assert_eq!(FontFallbackScriptHints::try_get_os2_bit(Script::Latin), None);
    }
}
