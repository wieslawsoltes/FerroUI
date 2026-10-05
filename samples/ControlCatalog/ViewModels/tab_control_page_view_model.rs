//! Port of `ViewModels/TabControlPageViewModel.cs`.

use ferroui_base::data::model::{BindableList, Event, INotifyPropertyChanged};
use ferroui_base::ferro_markup_type;
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::media::IImage;
use ferroui_controls::{Dock, ItemsSource};
use mini_mvvm::ViewModelBase;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The view model of the tab control page.
pub struct TabControlPageViewModel {
    base: ViewModelBase,
    tab_placement: Cell<Dock>,
    tabs: RefCell<Option<Rc<BindableList<Rc<TabControlPageViewModelItem>>>>>,
}

impl PartialEq for TabControlPageViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for TabControlPageViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl TabControlPageViewModel {
    pub fn new() -> Rc<TabControlPageViewModel> {
        Rc::new(Self { base: ViewModelBase::new(), tab_placement: Cell::new(Dock::default()), tabs: RefCell::new(None) })
    }

    pub fn tabs(&self) -> Option<Rc<BindableList<Rc<TabControlPageViewModelItem>>>> {
        self.tabs.borrow().clone()
    }

    pub fn set_tabs(&self, value: Option<Rc<BindableList<Rc<TabControlPageViewModelItem>>>>) {
        *self.tabs.borrow_mut() = value;
    }

    pub fn tab_placement(&self) -> Dock {
        self.tab_placement.get()
    }

    pub fn set_tab_placement(&self, value: Dock) {
        self.base.raise_and_set_if_changed_cell(&self.tab_placement, value, "TabPlacement");
    }
}

ferro_markup_type!(class TabControlPageViewModel {
    this: Rc<TabControlPageViewModel>,
    handles: [TabControlPageViewModel, Rc<TabControlPageViewModel>, Option<Rc<TabControlPageViewModel>>],
    constructors: [() => TabControlPageViewModel::new],
    properties: [
        // A list a binding delivers to an items source property.
        Tabs: Option<ItemsSource> { get: |this: &Rc<TabControlPageViewModel>| this.tabs().map(ItemsSource::from) },
        TabPlacement: Dock {
            get: |this: &Rc<TabControlPageViewModel>| this.tab_placement(),
            set: |this: &Rc<TabControlPageViewModel>, value: Dock| this.set_tab_placement(value)
        },
    ],
    notify_property_changed: TabControlPageViewModel,
});

/// A tab of the tab control that is generated from data.
pub struct TabControlPageViewModelItem {
    header: RefCell<Option<String>>,
    text: RefCell<Option<String>>,
    image: RefCell<Option<Rc<Bitmap>>>,
    is_enabled: Cell<bool>,
}

impl PartialEq for TabControlPageViewModelItem {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl TabControlPageViewModelItem {
    pub fn new() -> Rc<TabControlPageViewModelItem> {
        Rc::new(Self {
            header: RefCell::new(None),
            text: RefCell::new(None),
            image: RefCell::new(None),
            is_enabled: Cell::new(true),
        })
    }

    pub fn header(&self) -> Option<String> {
        self.header.borrow().clone()
    }

    pub fn set_header(&self, value: Option<String>) {
        *self.header.borrow_mut() = value;
    }

    pub fn with_header(self: Rc<Self>, value: &str) -> Rc<Self> {
        self.set_header(Some(value.to_string()));
        self
    }

    pub fn text(&self) -> Option<String> {
        self.text.borrow().clone()
    }

    pub fn set_text(&self, value: Option<String>) {
        *self.text.borrow_mut() = value;
    }

    pub fn with_text(self: Rc<Self>, value: &str) -> Rc<Self> {
        self.set_text(Some(value.to_string()));
        self
    }

    pub fn image(&self) -> Option<Rc<Bitmap>> {
        self.image.borrow().clone()
    }

    pub fn set_image(&self, value: Option<Rc<Bitmap>>) {
        *self.image.borrow_mut() = value;
    }

    pub fn with_image(self: Rc<Self>, value: Rc<Bitmap>) -> Rc<Self> {
        self.set_image(Some(value));
        self
    }

    pub fn is_enabled(&self) -> bool {
        self.is_enabled.get()
    }

    pub fn set_is_enabled(&self, value: bool) {
        self.is_enabled.set(value);
    }

    pub fn with_is_enabled(self: Rc<Self>, value: bool) -> Rc<Self> {
        self.set_is_enabled(value);
        self
    }
}

ferro_markup_type!(class TabControlPageViewModelItem {
    this: Rc<TabControlPageViewModelItem>,
    handles: [TabControlPageViewModelItem, Rc<TabControlPageViewModelItem>, Option<Rc<TabControlPageViewModelItem>>],
    constructors: [() => TabControlPageViewModelItem::new],
    properties: [
        Header: Option<String> {
            get: |this: &Rc<TabControlPageViewModelItem>| this.header(),
            set: |this: &Rc<TabControlPageViewModelItem>, value: Option<String>| this.set_header(value)
        },
        Text: Option<String> {
            get: |this: &Rc<TabControlPageViewModelItem>| this.text(),
            set: |this: &Rc<TabControlPageViewModelItem>, value: Option<String>| this.set_text(value)
        },
        // Declared with the contract of the bitmap: a bitmap handle has no equality and
        // cannot be the type of a declared property.
        Image: Option<Rc<dyn IImage>> {
            get: |this: &Rc<TabControlPageViewModelItem>| this.image().map(|image| image as Rc<dyn IImage>)
        },
        IsEnabled: bool {
            get: |this: &Rc<TabControlPageViewModelItem>| this.is_enabled(),
            set: |this: &Rc<TabControlPageViewModelItem>, value: bool| this.set_is_enabled(value)
        },
    ],
});

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;

    #[test]
    fn a_tab_is_enabled_until_it_is_disabled() {
        let item = TabControlPageViewModelItem::new().with_header("Arch").with_text("first");
        assert!(item.is_enabled());
        assert_eq!(Some("Arch".to_string()), item.header());
        assert!(item.image().is_none());
        assert!(!item.with_is_enabled(false).is_enabled());
    }

    #[test]
    fn the_placement_notifies_when_it_changes() {
        let view_model = TabControlPageViewModel::new();
        let count = Rc::new(Cell::new(0));
        let sink = count.clone();
        view_model.property_changed().add(Rc::new(move |_: &str| sink.set(sink.get() + 1)));

        assert_eq!(Dock::Left, view_model.tab_placement());
        assert!(view_model.tabs().is_none());
        view_model.set_tab_placement(Dock::Top);
        view_model.set_tab_placement(Dock::Top);
        assert_eq!(1, count.get());
    }
}
