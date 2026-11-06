use std::borrow::Cow;

use crate::utilities::Locale;

/// Resolves the ISO 15924 script subtag implied by a locale.
///
/// Used to disambiguate locale-sensitive scripts during font fallback (e.g.
/// a Han ideograph should resolve to a Japanese font under `ja-JP`, a
/// Simplified-Chinese font under `zh-CN`, a Traditional-Chinese font under
/// `zh-TW`).
pub(crate) struct Bcp47ScriptResolver;

impl Bcp47ScriptResolver {
    /// Returns the four-letter ISO 15924 script subtag (e.g. `"Jpan"`,
    /// `"Hans"`, `"Hant"`, `"Kore"`, `"Latn"`, `"Cyrl"`) implied by the
    /// locale, or `None` when no script can be inferred (the unknown locale
    /// `und`, the invariant culture, among them).
    pub fn get_script_subtag(culture: Option<&Locale>) -> Option<Cow<'static, str>> {
        let culture = culture?;

        if *culture == Locale::UNKNOWN {
            return None;
        }

        // 1. Explicit script subtag wins. The locale holds it in title case (Latn, Hans,
        //    Cyrl, ...).
        if let Some(explicit_script) = culture.id.script {
            return Some(Cow::Owned(explicit_script.as_str().to_owned()));
        }

        let language = culture.id.language.as_str();
        let is = |candidates: &[&str]| candidates.contains(&language);

        // 2. Japanese (Han + Hiragana + Katakana). The Jpan supercode picks Japanese fonts.
        if is(&["ja"]) {
            return Some(Cow::Borrowed("Jpan"));
        }

        // 3. Korean (Hangul + Han). Kore picks Korean fonts.
        if is(&["ko"]) {
            return Some(Cow::Borrowed("Kore"));
        }

        // 4. Chinese - Hans vs Hant disambiguation. Default to Hans when only "zh" is supplied.
        if is(&["zh"]) {
            if culture.id.region.is_some_and(|region| Self::is_traditional_chinese_region(region.as_str())) {
                return Some(Cow::Borrowed("Hant"));
            }

            return Some(Cow::Borrowed("Hans"));
        }

        // 5. Other well-known language to script mappings that disambiguate Unicode scripts.
        if is(&["ru", "uk", "bg", "be", "sr", "mk"]) {
            return Some(Cow::Borrowed("Cyrl"));
        }

        if is(&["en", "de", "fr", "es", "it", "pt", "nl", "sv", "no", "da", "fi", "pl", "cs", "tr"]) {
            return Some(Cow::Borrowed("Latn"));
        }

        if is(&["ar", "fa", "ur"]) {
            return Some(Cow::Borrowed("Arab"));
        }

        if is(&["he", "yi"]) {
            return Some(Cow::Borrowed("Hebr"));
        }

        if is(&["th"]) {
            return Some(Cow::Borrowed("Thai"));
        }

        if is(&["el"]) {
            return Some(Cow::Borrowed("Grek"));
        }

        None
    }

    /// Whether a region of Chinese implies the traditional script (the locale holds the
    /// region in upper case).
    fn is_traditional_chinese_region(region: &str) -> bool {
        ["TW", "HK", "MO"].contains(&region)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_expected_script_subtag() {
        for (culture_name, expected) in [
            ("ja", "Jpan"),
            ("ja-JP", "Jpan"),
            ("ko", "Kore"),
            ("ko-KR", "Kore"),
            ("zh", "Hans"),
            ("zh-CN", "Hans"),
            ("zh-SG", "Hans"),
            ("zh-TW", "Hant"),
            ("zh-HK", "Hant"),
            ("zh-MO", "Hant"),
            ("zh-Hans-CN", "Hans"),
            ("zh-Hant-TW", "Hant"),
            ("ru", "Cyrl"),
            ("ru-RU", "Cyrl"),
            ("en", "Latn"),
            ("en-US", "Latn"),
            ("de-DE", "Latn"),
            ("ar", "Arab"),
            ("he", "Hebr"),
            ("th", "Thai"),
            ("el", "Grek"),
            ("sr-Cyrl", "Cyrl"),
            ("sr-Latn-RS", "Latn"),
        ] {
            let culture = Locale::try_from_str(culture_name).unwrap();

            assert_eq!(
                Bcp47ScriptResolver::get_script_subtag(Some(&culture)).as_deref(),
                Some(expected),
                "{culture_name}"
            );
        }
    }

    #[test]
    fn invariant_culture_returns_null() {
        assert_eq!(Bcp47ScriptResolver::get_script_subtag(Some(&Locale::UNKNOWN)), None);
    }

    #[test]
    fn null_culture_returns_null() {
        assert_eq!(Bcp47ScriptResolver::get_script_subtag(None), None);
    }

    #[test]
    fn unknown_language_returns_null() {
        // 'xx' is reserved as a private-use language tag; no canonical script.
        let culture = Locale::try_from_str("xx").unwrap();

        assert_eq!(Bcp47ScriptResolver::get_script_subtag(Some(&culture)), None);
    }

    #[test]
    fn explicit_script_subtag_is_normalised_to_title_case() {
        // The locale normalizes the case of the subtags of the tag it parses.
        let culture = Locale::try_from_str("sr-cYRL-rs").unwrap();

        assert_eq!(Bcp47ScriptResolver::get_script_subtag(Some(&culture)).as_deref(), Some("Cyrl"));
    }
}
