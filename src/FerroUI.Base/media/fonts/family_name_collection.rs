use std::fmt;

use crate::media::font_source_identifier::FontSourceIdentifier;

/// A list of font family names: the primary name followed by its fallbacks.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FamilyNameCollection {
    names: Vec<String>,
}

impl FamilyNameCollection {
    /// Creates a collection from a comma separated list of family names.
    pub fn new(family_names: &str) -> Self {
        Self { names: Self::split_names(family_names) }
    }

    pub(crate) fn from_font_sources(font_sources: &[FontSourceIdentifier]) -> Self {
        Self { names: font_sources.iter().map(|source| source.name.clone()).collect() }
    }

    fn split_names(names: &str) -> Vec<String> {
        names.split(',').map(|name| name.trim().to_owned()).collect()
    }

    /// Gets the primary family name.
    pub fn primary_family_name(&self) -> &str {
        &self.names[0]
    }

    /// Gets a value indicating whether fallbacks are defined.
    pub fn has_fallbacks(&self) -> bool {
        self.names.len() > 1
    }

    /// The number of names.
    pub fn count(&self) -> usize {
        self.names.len()
    }

    /// The name at `index`.
    pub fn get(&self, index: usize) -> &str {
        &self.names[index]
    }

    /// Iterates over the names.
    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.names.iter().map(String::as_str)
    }
}

impl std::ops::Index<usize> for FamilyNameCollection {
    type Output = str;

    fn index(&self, index: usize) -> &str {
        &self.names[index]
    }
}

/// The names joined with `", "`.
impl fmt::Display for FamilyNameCollection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, name) in self.names.iter().enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }
            f.write_str(name)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_be_equal() {
        let family_names = FamilyNameCollection::new("Arial, Times New Roman");

        assert_eq!(family_names, FamilyNameCollection::new("Arial, Times New Roman"));
    }

    #[test]
    fn names_are_split_and_trimmed() {
        let family_names = FamilyNameCollection::new("Arial ,  Times New Roman");

        assert_eq!(family_names.primary_family_name(), "Arial");
        assert!(family_names.has_fallbacks());
        assert_eq!(family_names.count(), 2);
        assert_eq!(&family_names[1], "Times New Roman");
        assert_eq!(family_names.to_string(), "Arial, Times New Roman");
        assert!(!FamilyNameCollection::new("Arial").has_fallbacks());
    }
}
