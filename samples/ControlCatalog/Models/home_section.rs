//! Port of `Models/HomeSection.cs`.

use crate::models::PageItem;
use crate::pages::SectionPage;
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::ferro_markup_type;
use ferroui_base::media::StreamGeometry;
use ferroui_base::Ref;
use ferroui_controls::ItemsSource;
use mini_mvvm::ViewModelBase;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A section of the catalog: a titled group of pages, with a page of its
/// own that lists them.
pub struct HomeSection {
    base: ViewModelBase,
    title: String,
    icon_data: Ref<StreamGeometry>,
    items: RefCell<Option<Rc<Vec<Rc<PageItem>>>>>,
    current_page: RefCell<Option<Rc<PageItem>>>,
    is_expanded: Cell<bool>,
    page_item: Rc<PageItem>,
}

impl PartialEq for HomeSection {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for HomeSection {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl HomeSection {
    /// `new HomeSection(title, iconData)`.
    pub fn new(title: &str, icon_data: Ref<StreamGeometry>) -> Rc<HomeSection> {
        Rc::new_cyclic(|this| {
            let section = this.clone();
            let page_item = PageItem::with_section_title(
                title,
                move || {
                    let section = section.upgrade().expect("the section of the page outlives its page item");
                    SectionPage::with_home_section(section).upcast()
                },
                icon_data.clone(),
                "",
                None,
                std::rc::Weak::new(),
                None,
            );
            HomeSection {
                base: ViewModelBase::new(),
                title: title.to_string(),
                icon_data,
                items: RefCell::new(None),
                current_page: RefCell::new(None),
                is_expanded: Cell::new(false),
                page_item,
            }
        })
    }

    pub fn title(&self) -> String {
        self.title.clone()
    }

    pub fn icon_data(&self) -> Ref<StreamGeometry> {
        self.icon_data.clone()
    }

    pub fn items(&self) -> Option<Rc<Vec<Rc<PageItem>>>> {
        self.items.borrow().clone()
    }

    pub fn set_items(&self, value: Option<Rc<Vec<Rc<PageItem>>>>) {
        *self.items.borrow_mut() = value;
    }

    pub fn is_section_visible(&self) -> bool {
        self.items().is_some_and(|items| items.iter().any(|x| x.is_visible()))
    }

    /// The page being shown, when it is this section's own page or one of
    /// its pages.
    pub fn current_page(&self) -> Option<Rc<PageItem>> {
        self.current_page.borrow().clone()
    }

    pub fn set_current_page(&self, value: Option<Rc<PageItem>>) {
        if self.base.raise_and_set_if_changed(&self.current_page, value, "CurrentPage") {
            self.base.raise_property_changed("IsCurrent");
            self.base.raise_property_changed("ShowsSelection");
        }
    }

    pub fn is_current(&self) -> bool {
        self.current_page.borrow().is_some()
    }

    /// Whether the section's pages are listed. Set when the user opens a
    /// section, when a search matches, and automatically for the section
    /// holding the current page.
    pub fn is_expanded(&self) -> bool {
        self.is_expanded.get()
    }

    pub fn set_is_expanded(&self, value: bool) {
        if self.base.raise_and_set_if_changed_cell(&self.is_expanded, value, "IsExpanded") {
            self.base.raise_property_changed("ShowsSelection");
        }
    }

    /// The section carries the selection marker for its own page, and for
    /// one of its pages while they are hidden. Once it is open the current
    /// page carries it, so the drawer never shows two markers at once.
    pub fn shows_selection(&self) -> bool {
        let is_own_page = self.current_page().is_some_and(|page| Rc::ptr_eq(&page, &self.page_item));
        is_own_page || (self.is_current() && !self.is_expanded())
    }

    pub fn page_item(&self) -> Rc<PageItem> {
        self.page_item.clone()
    }

    pub fn raise_section_visibility_changed(&self) {
        self.base.raise_property_changed("IsSectionVisible");
    }
}

ferro_markup_type!(class HomeSection {
    this: Rc<HomeSection>,
    handles: [HomeSection, Rc<HomeSection>, Option<Rc<HomeSection>>],
    properties: [
        Title: String { get: |this: &Rc<HomeSection>| this.title() },
        IconData: Ref<StreamGeometry> { get: |this: &Rc<HomeSection>| this.icon_data() },
        // A list a binding delivers to an items source property.
        Items: Option<ItemsSource> {
            get: |this: &Rc<HomeSection>| {
                this.items().map(|items| ItemsSource::from_values(items.iter().cloned()))
            }
        },
        IsSectionVisible: bool { get: |this: &Rc<HomeSection>| this.is_section_visible() },
        CurrentPage: Option<Rc<PageItem>> {
            get: |this: &Rc<HomeSection>| this.current_page(),
            set: |this: &Rc<HomeSection>, value: Option<Rc<PageItem>>| this.set_current_page(value)
        },
        IsCurrent: bool { get: |this: &Rc<HomeSection>| this.is_current() },
        IsExpanded: bool {
            get: |this: &Rc<HomeSection>| this.is_expanded(),
            set: |this: &Rc<HomeSection>, value: bool| this.set_is_expanded(value)
        },
        ShowsSelection: bool { get: |this: &Rc<HomeSection>| this.shows_selection() },
        PageItem: Rc<PageItem> { get: |this: &Rc<HomeSection>| this.page_item() },
    ],
    notify_property_changed: HomeSection,
});
