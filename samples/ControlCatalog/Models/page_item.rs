//! Port of `Models/PageItem.cs`.

use crate::controls::SampleInfo;
use crate::models::HomeSection;
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::ferro_markup_type;
use ferroui_base::media::text_formatting::unicode::{Codepoint, GeneralCategory};
use ferroui_base::media::StreamGeometry;
use ferroui_base::Ref;
use ferroui_controls::Page;
use mini_mvvm::ViewModelBase;
use std::cell::Cell;
use std::rc::{Rc, Weak};
use unicode_normalization::UnicodeNormalization;

/// A page of the catalog as the navigation lists and the search see it:
/// its header, icon, description and section, and the factory of the page.
pub struct PageItem {
    base: ViewModelBase,
    header: String,
    factory: Box<dyn Fn() -> Ref<Page>>,
    icon_data: Option<Ref<StreamGeometry>>,
    description: Option<String>,
    section_title: String,
    /// The section the page belongs to. Held weakly: the section owns its
    /// items.
    section: Weak<HomeSection>,
    samples: Option<Rc<Vec<Rc<SampleInfo>>>>,
    search_key: String,
    is_visible: Cell<bool>,
}

impl PartialEq for PageItem {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for PageItem {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl PageItem {
    /// `new PageItem(header, factory, iconData, description, section, samples)`.
    pub fn new(
        header: &str,
        factory: impl Fn() -> Ref<Page> + 'static,
        icon_data: Ref<StreamGeometry>,
        description: &str,
        section: Option<&Rc<HomeSection>>,
        samples: Option<Rc<Vec<Rc<SampleInfo>>>>,
    ) -> Rc<PageItem> {
        Self::with_section_title(
            header,
            factory,
            icon_data,
            description,
            section.map(|section| section.title()).as_deref(),
            section.map(Rc::downgrade).unwrap_or_default(),
            samples,
        )
    }

    /// The constructor, for a section that is still being created: its title
    /// and a weak handle of it.
    pub(crate) fn with_section_title(
        header: &str,
        factory: impl Fn() -> Ref<Page> + 'static,
        icon_data: Ref<StreamGeometry>,
        description: &str,
        section_title: Option<&str>,
        section: Weak<HomeSection>,
        samples: Option<Rc<Vec<Rc<SampleInfo>>>>,
    ) -> Rc<PageItem> {
        Rc::new(PageItem {
            base: ViewModelBase::new(),
            header: header.to_string(),
            factory: Box::new(factory),
            icon_data: Some(icon_data),
            description: Some(description.to_string()),
            section_title: section_title.unwrap_or_default().to_string(),
            section,
            search_key: Self::build_search_key(header, Some(description), section_title, samples.as_deref()),
            samples,
            is_visible: Cell::new(true),
        })
    }

    pub fn header(&self) -> String {
        self.header.clone()
    }

    pub fn icon_data(&self) -> Option<Ref<StreamGeometry>> {
        self.icon_data.clone()
    }

    pub fn description(&self) -> Option<String> {
        self.description.clone()
    }

    /// The title of the section of the page; empty for a page without a
    /// section.
    pub fn section(&self) -> String {
        self.section_title.clone()
    }

    /// The samples a gallery page offers. The registry lives statically on
    /// the page class and is passed here at registration, so search can
    /// match the page by its samples without constructing it.
    pub fn samples(&self) -> Option<Rc<Vec<Rc<SampleInfo>>>> {
        self.samples.clone()
    }

    pub fn is_visible(&self) -> bool {
        self.is_visible.get()
    }

    pub fn set_is_visible(&self, value: bool) {
        self.base.raise_and_set_if_changed_cell(&self.is_visible, value, "IsVisible");
        if let Some(section) = self.section.upgrade() {
            section.raise_section_visibility_changed();
        }
    }

    pub fn create_page(&self) -> Ref<Page> {
        (self.factory)()
    }

    pub fn matches_search(&self, search_key: &str) -> bool {
        self.search_key.contains(search_key)
    }

    fn build_search_key(
        header: &str,
        description: Option<&str>,
        section: Option<&str>,
        samples: Option<&Vec<Rc<SampleInfo>>>,
    ) -> String {
        let mut parts = vec![header.to_string(), description.unwrap_or_default().to_string(), section.unwrap_or_default().to_string()];
        if let Some(samples) = samples {
            for sample in samples.iter() {
                parts.push(sample.title());
                parts.push(sample.description());
            }
        }
        let parts: Vec<&str> = parts.iter().map(String::as_str).collect();
        Self::create_search_key(&parts)
    }

    /// The key a text is searched by: its letters and digits, without
    /// diacritics, in upper case.
    ///
    /// The managed original works on UTF-16 code units and maps each to
    /// upper case on its own; here the scalar values of the text are
    /// processed, and a character whose upper case is more than one
    /// character is kept as it is (the invariant upper-casing of a single
    /// code unit never expands).
    pub fn create_search_key(values: &[&str]) -> String {
        let mut builder = String::with_capacity(256);

        for value in values {
            for c in value.nfkd() {
                let category = Codepoint::new(c as u32).general_category();

                if matches!(
                    category,
                    GeneralCategory::NonspacingMark | GeneralCategory::SpacingMark | GeneralCategory::EnclosingMark
                ) {
                    continue;
                }

                if is_letter_or_digit(category) {
                    let mut upper = c.to_uppercase();
                    match (upper.next(), upper.next()) {
                        (Some(single), None) => builder.push(single),
                        _ => builder.push(c),
                    }
                }
            }
        }

        builder
    }
}

/// `char.IsLetterOrDigit`: the letter categories and the decimal digits.
fn is_letter_or_digit(category: GeneralCategory) -> bool {
    matches!(
        category,
        GeneralCategory::UppercaseLetter
            | GeneralCategory::LowercaseLetter
            | GeneralCategory::TitlecaseLetter
            | GeneralCategory::ModifierLetter
            | GeneralCategory::OtherLetter
            | GeneralCategory::DecimalNumber
    )
}

ferro_markup_type!(class PageItem {
    this: Rc<PageItem>,
    handles: [PageItem, Rc<PageItem>, Option<Rc<PageItem>>],
    properties: [
        Header: String { get: |this: &Rc<PageItem>| this.header() },
        IconData: Option<Ref<StreamGeometry>> { get: |this: &Rc<PageItem>| this.icon_data() },
        Description: Option<String> { get: |this: &Rc<PageItem>| this.description() },
        Section: String { get: |this: &Rc<PageItem>| this.section() },
        IsVisible: bool {
            get: |this: &Rc<PageItem>| this.is_visible(),
            set: |this: &Rc<PageItem>, value: bool| this.set_is_visible(value)
        },
    ],
    notify_property_changed: PageItem,
});

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;

    #[test]
    fn create_search_key_keeps_letters_and_digits_in_upper_case() {
        assert_eq!("BUTTON2STATECHECKBOXES", PageItem::create_search_key(&["Button", "2-state check boxes!"]));
    }

    #[test]
    fn create_search_key_removes_diacritics() {
        assert_eq!("LAVENIRCAFE", PageItem::create_search_key(&["L'Avenir", "caf\u{e9}"]));
        // Compatibility decomposition: the ligature becomes its letters.
        assert_eq!("FI", PageItem::create_search_key(&["\u{fb01}"]));
    }

    #[test]
    fn create_search_key_of_nothing_is_empty() {
        assert_eq!("", PageItem::create_search_key(&[]));
        assert_eq!("", PageItem::create_search_key(&["  -- "]));
    }
}
