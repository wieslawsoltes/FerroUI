use std::fmt;
use std::ops::Deref;

/// A transparency level a window can be asked to render with.
///
/// The set of levels is closed: values are obtained from the associated
/// functions ([`none`](Self::none), [`transparent`](Self::transparent), ...).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WindowTransparencyLevel {
    value: &'static str,
}

impl WindowTransparencyLevel {
    const fn new(value: &'static str) -> Self {
        Self { value }
    }

    /// The window background is black where the client area is transparent.
    pub const fn none() -> Self {
        Self::new("None")
    }

    /// The window background is transparent.
    pub const fn transparent() -> Self {
        Self::new("Transparent")
    }

    /// The window background is a blur-behind where the client area is
    /// transparent.
    pub const fn blur() -> Self {
        Self::new("Blur")
    }

    /// The window background is a blur-behind with a high blur radius. This
    /// level may fall back to blur, if the platform doesn't support acrylic.
    pub const fn acrylic_blur() -> Self {
        Self::new("AcrylicBlur")
    }

    /// The window background is based on desktop wallpaper tint with a blur.
    /// This will only work on Windows 11.
    pub const fn mica() -> Self {
        Self::new("Mica")
    }
}

impl fmt::Display for WindowTransparencyLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.value)
    }
}

/// A read-only list of transparency levels, in order of preference.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WindowTransparencyLevelCollection {
    list: Vec<WindowTransparencyLevel>,
}

impl WindowTransparencyLevelCollection {
    /// Creates a collection from a list of levels.
    pub fn new(list: impl Into<Vec<WindowTransparencyLevel>>) -> Self {
        Self { list: list.into() }
    }
}

impl Deref for WindowTransparencyLevelCollection {
    type Target = [WindowTransparencyLevel];

    fn deref(&self) -> &[WindowTransparencyLevel] {
        &self.list
    }
}

impl<'a> IntoIterator for &'a WindowTransparencyLevelCollection {
    type Item = &'a WindowTransparencyLevel;
    type IntoIter = std::slice::Iter<'a, WindowTransparencyLevel>;

    fn into_iter(self) -> Self::IntoIter {
        self.list.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_writes_the_level_name() {
        assert_eq!(WindowTransparencyLevel::none().to_string(), "None");
        assert_eq!(WindowTransparencyLevel::transparent().to_string(), "Transparent");
        assert_eq!(WindowTransparencyLevel::blur().to_string(), "Blur");
        assert_eq!(WindowTransparencyLevel::acrylic_blur().to_string(), "AcrylicBlur");
        assert_eq!(WindowTransparencyLevel::mica().to_string(), "Mica");
    }

    #[test]
    fn levels_compare_by_value() {
        assert_eq!(WindowTransparencyLevel::blur(), WindowTransparencyLevel::blur());
        assert_ne!(WindowTransparencyLevel::blur(), WindowTransparencyLevel::acrylic_blur());
    }

    #[test]
    fn collection_exposes_its_items_in_order() {
        let collection = WindowTransparencyLevelCollection::new([
            WindowTransparencyLevel::mica(),
            WindowTransparencyLevel::acrylic_blur(),
        ]);
        assert_eq!(collection.len(), 2);
        assert_eq!(collection[0], WindowTransparencyLevel::mica());
        assert_eq!(
            collection.iter().map(ToString::to_string).collect::<Vec<_>>(),
            ["Mica", "AcrylicBlur"]
        );
        assert!(WindowTransparencyLevelCollection::default().is_empty());
    }
}
