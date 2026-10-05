use crate::utilities::Uri;

/// One comma separated segment of a font family name: the family name and,
/// when the segment has the form `source#name`, the location of the font.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct FontSourceIdentifier {
    pub name: String,
    pub source: Option<Uri>,
}

impl FontSourceIdentifier {
    pub fn new(name: String, source: Option<Uri>) -> Self {
        Self { name, source }
    }
}
