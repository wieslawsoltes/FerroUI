use crate::media::media_collection::media_collection_type;
use crate::media::{MediaCollection, TextDecoration, TextDecorationLocation};
use crate::utilities::{FormatError, SpanStringTokenizer};
use crate::Ref;

media_collection_type!(
    /// A collection that holds `TextDecoration` objects.
    ///
    /// A shared handle to an observable list; equality is reference equality.
    TextDecorationCollection,
    Ref<TextDecoration>
);

impl TextDecorationCollection {
    /// Creates an empty collection.
    pub fn new() -> Self {
        Self(MediaCollection::new())
    }

    /// Creates a collection from text decorations.
    pub fn from_items(text_decorations: impl IntoIterator<Item = Ref<TextDecoration>>) -> Self {
        Self(MediaCollection::from_items(text_decorations))
    }

    /// Parses a `TextDecorationCollection` string: a comma separated list of
    /// decoration locations (`Underline`, `Overline`, `Strikethrough`, `Baseline`).
    pub fn parse(s: &str) -> Result<TextDecorationCollection, FormatError> {
        let mut locations = Vec::new();

        SpanStringTokenizer::with_message(s, "Invalid text decoration.").scope(|tokenizer| {
            while let Some(name) = tokenizer.try_read_span()? {
                let location = Self::get_text_decoration_location(name)?;

                if locations.contains(&location) {
                    return Err(FormatError::new("Text decoration already specified."));
                }

                locations.push(location);
            }
            Ok(())
        })?;

        let text_decorations = TextDecorationCollection::new();

        for text_decoration_location in locations {
            let text_decoration = TextDecoration::new();
            text_decoration.set_location(text_decoration_location);
            text_decorations.add(text_decoration);
        }

        Ok(text_decorations)
    }

    /// Parses a text decoration enum value.
    fn get_text_decoration_location(s: &str) -> Result<TextDecorationLocation, FormatError> {
        const NAMES: [(&str, TextDecorationLocation); 4] = [
            ("Underline", TextDecorationLocation::Underline),
            ("Overline", TextDecorationLocation::Overline),
            ("Strikethrough", TextDecorationLocation::Strikethrough),
            ("Baseline", TextDecorationLocation::Baseline),
        ];

        NAMES
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(s))
            .map(|(_, location)| *location)
            .ok_or(FormatError::new("Could not parse text decoration."))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_parse_text_decorations() {
        let baseline = TextDecorationCollection::parse("baseline").unwrap();
        assert_eq!(TextDecorationLocation::Baseline, baseline.get(0).location());

        let underline = TextDecorationCollection::parse("underline").unwrap();
        assert_eq!(TextDecorationLocation::Underline, underline.get(0).location());

        let overline = TextDecorationCollection::parse("overline").unwrap();
        assert_eq!(TextDecorationLocation::Overline, overline.get(0).location());

        let strikethrough = TextDecorationCollection::parse("strikethrough").unwrap();
        assert_eq!(TextDecorationLocation::Strikethrough, strikethrough.get(0).location());
    }

    #[test]
    fn should_reject_duplicates_and_unknown_names() {
        assert!(TextDecorationCollection::parse("underline, underline").is_err());
        assert!(TextDecorationCollection::parse("wavy").is_err());
        assert_eq!(TextDecorationCollection::parse("Underline, Strikethrough").unwrap().len(), 2);
    }
}
