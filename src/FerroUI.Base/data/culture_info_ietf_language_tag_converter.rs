use crate::data::core::ValueType;
use crate::utilities::{CultureInfo, FormatError};

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
    /// (`CultureInfo.GetCultureInfoByIetfLanguageTag`).
    pub fn convert_from(culture_name: &str) -> Result<CultureInfo, FormatError> {
        CultureInfo::get_culture_info_by_ietf_language_tag(culture_name).ok_or_else(|| {
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
        for name in ["zh-CHT", "zh-CHS", "es-ES_tradnl"] {
            let error = CultureInfoIetfLanguageTagConverter::convert_from(name).expect_err("an error");
            assert_eq!(error.to_string(), format!("Culture IETF Name {name} is not a recognized IETF name."));
        }
    }

    #[test]
    fn the_converter_converts_from_text_only() {
        assert!(CultureInfoIetfLanguageTagConverter::can_convert_from(ValueType::of::<String>()));
        assert!(!CultureInfoIetfLanguageTagConverter::can_convert_from(ValueType::of::<i32>()));
    }
}
