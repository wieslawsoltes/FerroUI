use std::fmt;
use std::rc::Rc;

use crate::media::text_formatting::TextCollapsingProperties;
use crate::media::{
    TextCollapsingCreateInfo, TextLeadingPrefixTrimming, TextNoneTrimming, TextPathSegmentTrimming,
    TextTrailingTrimming,
};
use crate::utilities::FormatError;

/// Describes how text is trimmed when it overflows.
///
/// Handled as `Rc<dyn TextTrimming>`; the predefined values
/// (`<dyn TextTrimming>::none()`, ...) are one instance per thread and can be
/// compared with `Rc::ptr_eq`.
pub trait TextTrimming: fmt::Display + 'static {
    /// Creates the `TextCollapsingProperties` used to collapse the text.
    fn create_collapsing_properties(&self, create_info: &TextCollapsingCreateInfo) -> Rc<dyn TextCollapsingProperties>;
}

/// Text trimmings are compared by identity, as the reference types they are
/// upstream.
impl PartialEq for dyn TextTrimming {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}

/// C# `TextTrimming.DefaultEllipsisChar`.
///
/// Internal upstream; public so that the Skia unit tests reach it.
pub const DEFAULT_ELLIPSIS_CHAR: &str = "\u{2026}";

struct Instances {
    none: Rc<dyn TextTrimming>,
    character_ellipsis: Rc<dyn TextTrimming>,
    word_ellipsis: Rc<dyn TextTrimming>,
    prefix_character_ellipsis: Rc<dyn TextTrimming>,
    leading_character_ellipsis: Rc<dyn TextTrimming>,
    path_segment_ellipsis: Rc<dyn TextTrimming>,
}

thread_local! {
    static INSTANCES: Instances = Instances {
        none: Rc::new(TextNoneTrimming),
        character_ellipsis: Rc::new(TextTrailingTrimming::new(DEFAULT_ELLIPSIS_CHAR, false)),
        word_ellipsis: Rc::new(TextTrailingTrimming::new(DEFAULT_ELLIPSIS_CHAR, true)),
        prefix_character_ellipsis: Rc::new(TextLeadingPrefixTrimming::new(DEFAULT_ELLIPSIS_CHAR, 8)),
        leading_character_ellipsis: Rc::new(TextLeadingPrefixTrimming::new(DEFAULT_ELLIPSIS_CHAR, 0)),
        path_segment_ellipsis: Rc::new(TextPathSegmentTrimming::new(DEFAULT_ELLIPSIS_CHAR)),
    };
}

impl dyn TextTrimming {
    /// Text is not trimmed.
    pub fn none() -> Rc<dyn TextTrimming> {
        INSTANCES.with(|instances| instances.none.clone())
    }

    /// Text is trimmed at a character boundary. An ellipsis (...) is drawn in place of remaining text.
    pub fn character_ellipsis() -> Rc<dyn TextTrimming> {
        INSTANCES.with(|instances| instances.character_ellipsis.clone())
    }

    /// Text is trimmed at a word boundary. An ellipsis (...) is drawn in place of remaining text.
    pub fn word_ellipsis() -> Rc<dyn TextTrimming> {
        INSTANCES.with(|instances| instances.word_ellipsis.clone())
    }

    /// Text is trimmed after a given prefix length. An ellipsis (...) is drawn in between prefix and suffix and represents remaining text.
    pub fn prefix_character_ellipsis() -> Rc<dyn TextTrimming> {
        INSTANCES.with(|instances| instances.prefix_character_ellipsis.clone())
    }

    /// Text is trimmed at a character boundary starting from the beginning. An ellipsis (...) is drawn in place of remaining text.
    pub fn leading_character_ellipsis() -> Rc<dyn TextTrimming> {
        INSTANCES.with(|instances| instances.leading_character_ellipsis.clone())
    }

    /// Text is trimmed at a path segment boundary. An ellipsis (...) is drawn in place of remaining text.
    pub fn path_segment_ellipsis() -> Rc<dyn TextTrimming> {
        INSTANCES.with(|instances| instances.path_segment_ellipsis.clone())
    }

    /// Parses a text trimming string. Names are case insensitive.
    pub fn parse(s: &str) -> Result<Rc<dyn TextTrimming>, FormatError> {
        let matches = |name: &str| name.eq_ignore_ascii_case(s);

        if matches("None") {
            Ok(Self::none())
        } else if matches("CharacterEllipsis") {
            Ok(Self::character_ellipsis())
        } else if matches("WordEllipsis") {
            Ok(Self::word_ellipsis())
        } else if matches("PrefixCharacterEllipsis") {
            Ok(Self::prefix_character_ellipsis())
        } else if matches("PathSegmentEllipsis") {
            Ok(Self::path_segment_ellipsis())
        } else {
            Err(FormatError::from_string(format!("Invalid text trimming string: '{s}'.")))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_is_case_insensitive_and_returns_the_shared_instances() {
        assert!(Rc::ptr_eq(&<dyn TextTrimming>::parse("none").unwrap(), &<dyn TextTrimming>::none()));
        assert!(Rc::ptr_eq(
            &<dyn TextTrimming>::parse("CHARACTERELLIPSIS").unwrap(),
            &<dyn TextTrimming>::character_ellipsis()
        ));
        assert!(Rc::ptr_eq(&<dyn TextTrimming>::parse("WordEllipsis").unwrap(), &<dyn TextTrimming>::word_ellipsis()));
        assert!(Rc::ptr_eq(
            &<dyn TextTrimming>::parse("prefixCharacterEllipsis").unwrap(),
            &<dyn TextTrimming>::prefix_character_ellipsis()
        ));
        assert!(Rc::ptr_eq(
            &<dyn TextTrimming>::parse("PathSegmentEllipsis").unwrap(),
            &<dyn TextTrimming>::path_segment_ellipsis()
        ));
    }

    #[test]
    fn parse_rejects_unknown_names() {
        // Upstream does not parse "LeadingCharacterEllipsis" either.
        assert!(<dyn TextTrimming>::parse("LeadingCharacterEllipsis").is_err());

        let error = <dyn TextTrimming>::parse("Foo").err().unwrap();

        assert_eq!(error.to_string(), "Invalid text trimming string: 'Foo'.");
    }

    #[test]
    fn to_string_gives_the_parse_names() {
        assert_eq!(<dyn TextTrimming>::none().to_string(), "None");
        assert_eq!(<dyn TextTrimming>::character_ellipsis().to_string(), "CharacterEllipsis");
        assert_eq!(<dyn TextTrimming>::word_ellipsis().to_string(), "WordEllipsis");
        assert_eq!(<dyn TextTrimming>::prefix_character_ellipsis().to_string(), "PrefixCharacterEllipsis");
        assert_eq!(<dyn TextTrimming>::leading_character_ellipsis().to_string(), "PrefixCharacterEllipsis");
        assert_eq!(<dyn TextTrimming>::path_segment_ellipsis().to_string(), "PathSegmentEllipsis");
    }
}
