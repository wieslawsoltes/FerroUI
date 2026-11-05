use crate::data::core::ValueType;
use crate::utilities::{CultureInfo, FormatError, Locale};

/// Converts the IETF language tag of a culture (`de-DE`, `ar-SA`) to the
/// culture: the converter of the `ConverterCulture` property of the
/// bindings in markup.
///
/// The original is a type converter that the property names with an
/// attribute. Here a type states its conversion from text in its markup
/// metadata (`parse:`), so the metadata of `CultureInfo` names
/// [`convert_from`](Self::convert_from), and every property of that type,
/// the converter culture of the bindings among them, takes a language tag.
// Deviation (DEVIATIONS.md, Bindings): not a class derived from the type converter contract
// (which is in the markup crate), and not named by an attribute of the property.
pub struct CultureInfoIetfLanguageTagConverter;

impl CultureInfoIetfLanguageTagConverter {
    /// Whether the converter converts from the type: only from text.
    pub fn can_convert_from(source_type: ValueType) -> bool {
        source_type.is_string()
    }

    /// The culture of an IETF language tag
    /// (`CultureInfo.GetCultureInfoByIetfLanguageTag`): the tag is parsed into
    /// a [`Locale`] and the culture is the culture of the locale
    /// ([`CultureInfo::from_locale`]), so the case of its subtags is
    /// normalized (`ar-sa` is `ar-SA`); the empty tag is the invariant
    /// culture. A text that is not a BCP-47 language tag is refused with the
    /// error of the reference: the old names `zh-CHT` and `zh-CHS`, the
    /// traditional sort of Spanish `es-ES_tradnl`, a private name such as
    /// `x-private`, a text with `_` as separator (`en_US`).
    pub fn convert_from(culture_name: &str) -> Result<CultureInfo, FormatError> {
        Self::convert_to_locale(culture_name).map(CultureInfo::from_locale)
    }

    /// The locale of an IETF language tag: the locale the tag is parsed into,
    /// the unknown locale `und` for the empty tag (the invariant culture), or
    /// the error of [`convert_from`](Self::convert_from) for a text that is
    /// not a BCP-47 language tag.
    pub fn convert_to_locale(culture_name: &str) -> Result<Locale, FormatError> {
        if culture_name.is_empty() {
            return Ok(Locale::UNKNOWN);
        }
        Locale::try_from_str(culture_name).map_err(|_| {
            FormatError::from_string(format!("Culture IETF Name {culture_name} is not a recognized IETF name."))
        })
    }
}

#[cfg(test)]
mod tests {
    // Not upstream tests: the converter has none of its own there (it is covered by the
    // markup tests that set a converter culture).
    use super::*;

    #[test]
    fn a_language_tag_gives_its_culture() {
        let culture = CultureInfoIetfLanguageTagConverter::convert_from("ar-SA").expect("a culture");

        assert_eq!(culture, CultureInfo::get_culture_info("ar-SA"));
        assert_eq!(culture.name(), "ar-SA");
        assert!(CultureInfoIetfLanguageTagConverter::convert_from("").expect("the invariant culture").is_invariant());
    }

    #[test]
    fn the_names_that_are_not_ietf_tags_are_refused() {
        // Not from upstream (C5 of #423): the tag is parsed into a locale, so every text that is
        // no BCP-47 tag is refused, the private names and the ones with `_` among them.
        for name in ["zh-CHT", "zh-CHS", "es-ES_tradnl", "x-private", "en_US", "en-", "not a tag"] {
            let error = CultureInfoIetfLanguageTagConverter::convert_from(name).expect_err("an error");
            assert_eq!(error.to_string(), format!("Culture IETF Name {name} is not a recognized IETF name."));
            let error = CultureInfoIetfLanguageTagConverter::convert_to_locale(name).expect_err("an error");
            assert_eq!(error.to_string(), format!("Culture IETF Name {name} is not a recognized IETF name."));
        }
    }

    /// Not from upstream (C5 of #423): the culture of a tag is the culture of its locale, with
    /// the case of the subtags normalized, the name and the Windows language identifier of the
    /// locale; the empty tag and `und` are the invariant culture.
    #[test]
    fn a_language_tag_is_parsed_into_a_locale() {
        use crate::utilities::locale;

        for (tag, locale, name, lcid) in [
            ("ar-SA", locale!("ar-SA"), "ar-SA", 0x0401),
            ("ar-sa", locale!("ar-SA"), "ar-SA", 0x0401),
            ("ZH-hant-tw", locale!("zh-Hant-TW"), "zh-Hant-TW", 0x1000),
            ("en-US-u-hc-h12", locale!("en-US-u-hc-h12"), "en-US-u-hc-h12", 0x1000),
        ] {
            assert_eq!(CultureInfoIetfLanguageTagConverter::convert_to_locale(tag).expect("a locale"), locale, "{tag}");
            let culture = CultureInfoIetfLanguageTagConverter::convert_from(tag).expect("a culture");
            assert_eq!(culture.locale(), &locale, "{tag}");
            assert_eq!(culture.name(), name, "{tag}");
            assert_eq!(culture.lcid(), lcid, "{tag}");
            assert_eq!(culture, CultureInfo::from_locale(locale));
        }
        for tag in ["", "und"] {
            assert_eq!(CultureInfoIetfLanguageTagConverter::convert_to_locale(tag).expect("a locale"), Locale::UNKNOWN);
            assert!(CultureInfoIetfLanguageTagConverter::convert_from(tag).expect("a culture").is_invariant(), "{tag:?}");
        }
    }

    #[test]
    fn the_converter_converts_from_text_only() {
        assert!(CultureInfoIetfLanguageTagConverter::can_convert_from(ValueType::of::<String>()));
        assert!(!CultureInfoIetfLanguageTagConverter::can_convert_from(ValueType::of::<i32>()));
    }
}
