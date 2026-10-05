//! A minimal counterpart of .NET's `System.Globalization.TextInfo`: the
//! text conventions of a culture the framework reads (the list separator).

use super::i_culture_data_provider::find_culture_data;
use super::CultureInfo;
use std::rc::Rc;

thread_local! {
    static INVARIANT: Rc<TextInfo> = Rc::new(TextInfo::new());
}

/// The text conventions of a culture (C# `TextInfo`).
///
/// Only the conventions of the invariant culture are built in: those of
/// other cultures come from the provider registered with the locator (see
/// [`ICultureDataProvider`](super::ICultureDataProvider)); a culture without
/// data gets the conventions of its nearest parent that has data and finally
/// the invariant ones.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextInfo {
    list_separator: String,
}

impl TextInfo {
    /// The conventions of the invariant culture, to be changed with the
    /// setters.
    pub fn new() -> TextInfo {
        TextInfo { list_separator: ",".to_owned() }
    }

    /// The conventions of `culture`: the data the registered provider has
    /// for the culture or the nearest of its parents, else the invariant
    /// data.
    pub(crate) fn for_culture(culture: &CultureInfo) -> Rc<TextInfo> {
        find_culture_data(culture, |provider, name| provider.get_text_info(name)).unwrap_or_else(Self::invariant_info)
    }

    /// The conventions of the invariant culture.
    pub fn invariant_info() -> Rc<TextInfo> {
        INVARIANT.with(Rc::clone)
    }

    /// The conventions of the current culture.
    pub fn current_info() -> Rc<TextInfo> {
        CultureInfo::current_culture().text_info()
    }

    /// Gets the string that separates items in a list (C# `ListSeparator`).
    pub fn list_separator(&self) -> &str {
        &self.list_separator
    }

    /// Sets the string that separates items in a list.
    pub fn set_list_separator(&mut self, value: impl Into<String>) {
        self.list_separator = value.into();
    }

    /// The conventions with the given list separator.
    pub fn with_list_separator(mut self, value: impl Into<String>) -> Self {
        self.set_list_separator(value);
        self
    }
}

impl Default for TextInfo {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invariant_list_separator_is_a_comma() {
        assert_eq!(",", CultureInfo::invariant_culture().text_info().list_separator());
        assert_eq!(",", TextInfo::invariant_info().list_separator());
    }

    #[test]
    fn culture_without_data_uses_the_invariant_list_separator() {
        assert_eq!(",", CultureInfo::get_culture_info("pl-PL").text_info().list_separator());
    }

    #[test]
    fn list_separator_can_be_changed() {
        let info = TextInfo::new().with_list_separator(";");
        assert_eq!(";", info.list_separator());
    }
}
