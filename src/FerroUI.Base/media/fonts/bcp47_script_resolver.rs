use std::borrow::Cow;

use crate::utilities::CultureInfo;

/// Resolves the ISO 15924 script subtag implied by a culture.
///
/// Used to disambiguate locale-sensitive scripts during font fallback (e.g.
/// a Han ideograph should resolve to a Japanese font under `ja-JP`, a
/// Simplified-Chinese font under `zh-CN`, a Traditional-Chinese font under
/// `zh-TW`).
pub(crate) struct Bcp47ScriptResolver;

impl Bcp47ScriptResolver {
    /// Returns the four-letter ISO 15924 script subtag (e.g. `"Jpan"`,
    /// `"Hans"`, `"Hant"`, `"Kore"`, `"Latn"`, `"Cyrl"`) implied by the
    /// culture, or `None` when no script can be inferred.
    pub fn get_script_subtag(culture: Option<&CultureInfo>) -> Option<Cow<'static, str>> {
        let culture = culture?;

        if culture.is_invariant() {
            return None;
        }

        let name = culture.name();

        if name.is_empty() {
            return None;
        }

        // 1. Explicit script subtag wins (BCP-47 second subtag, four letters).
        if let Some(explicit_script) = Self::try_extract_script_subtag(name) {
            return Some(Cow::Owned(explicit_script));
        }

        let (language, rest) = match name.find(['-', '_']) {
            Some(language_end) => (&name[..language_end], &name[language_end + 1..]),
            None => (name, ""),
        };

        let is = |candidates: &[&str]| candidates.iter().any(|candidate| language.eq_ignore_ascii_case(candidate));

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
            if Self::is_traditional_chinese_region(rest) {
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

    fn try_extract_script_subtag(name: &str) -> Option<String> {
        // Look for a four-letter ISO 15924 script subtag, e.g. "zh-Hans-CN" or "sr-Cyrl".
        // The language subtag (the first one) is never a script.
        name.split(['-', '_'])
            .skip(1)
            .find(|subtag| subtag.len() == 4 && subtag.bytes().all(|b| b.is_ascii_alphabetic()))
            .map(|subtag| {
                // Normalise to title case (Latn, Hans, Cyrl, ...).
                let mut script = subtag.to_ascii_lowercase();
                script[..1].make_ascii_uppercase();
                script
            })
    }

    fn is_traditional_chinese_region(rest: &str) -> bool {
        // Iterate region/script subtags following "zh-".
        for subtag in rest.split(['-', '_']) {
            let is = |candidates: &[&str]| candidates.iter().any(|candidate| subtag.eq_ignore_ascii_case(candidate));

            if is(&["Hant"]) {
                return true;
            }

            if is(&["Hans"]) {
                return false;
            }

            if is(&["TW", "HK", "MO"]) {
                return true;
            }

            if is(&["CN", "SG", "MY"]) {
                return false;
            }
        }

        false
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
            let culture = CultureInfo::get_culture_info(culture_name);

            assert_eq!(
                Bcp47ScriptResolver::get_script_subtag(Some(&culture)).as_deref(),
                Some(expected),
                "{culture_name}"
            );
        }
    }

    #[test]
    fn invariant_culture_returns_none() {
        assert_eq!(Bcp47ScriptResolver::get_script_subtag(Some(&CultureInfo::invariant_culture())), None);
    }

    #[test]
    fn null_culture_returns_none() {
        assert_eq!(Bcp47ScriptResolver::get_script_subtag(None), None);
    }

    #[test]
    fn unknown_language_returns_none() {
        // 'xx' is reserved as a private-use language tag; no canonical script.
        let culture = CultureInfo::get_culture_info("xx");

        assert_eq!(Bcp47ScriptResolver::get_script_subtag(Some(&culture)), None);
    }

    #[test]
    fn explicit_script_subtag_is_normalised_to_title_case() {
        let culture = CultureInfo::get_culture_info("sr_cYRL_rs");

        assert_eq!(Bcp47ScriptResolver::get_script_subtag(Some(&culture)).as_deref(), Some("Cyrl"));
    }
}
