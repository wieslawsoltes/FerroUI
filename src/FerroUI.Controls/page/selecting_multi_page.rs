use super::{
    MultiPage, MultiPageImpl, NavigatedFromEventArgs, NavigatedToEventArgs, NavigationType, Page, PageImpl,
    PageSelectionChangedEventArgs,
};
use crate::primitives::TemplatedControlImpl;
use crate::ControlImpl;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{
    Interactive, InteractiveImpl, RoutedEvent, RoutedEventHandlerToken, RoutingStrategies,
};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, ferro_routed_event, DirectProperty,
    FerroObjectImpl, FerroProperty, Ref, StyledElementImpl, VisualImpl,
};
use std::cell::{Cell, RefCell};

/// Base class for multi-page controls with index-based selection.
///
/// This class is abstract: its abstract member panics unless overridden.
#[repr(C)]
pub struct SelectingMultiPage {
    base: MultiPage,
    selected_index: Cell<i32>,
    selected_page: RefCell<Option<Ref<Page>>>,
}

ferro_class! {
    SelectingMultiPage: MultiPage, virtuals SelectingMultiPageImpl: MultiPageImpl {
        /// Applies the requested index change.
        fn apply_selected_index(this, index: i32);
    }
}

ferro_class_info!(SelectingMultiPage {});

ferro_impl_classes!(
    SelectingMultiPage: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    PageImpl,
    MultiPageImpl
);

impl SelectingMultiPageImpl for SelectingMultiPage {
    fn apply_selected_index(_this: &Self, _index: i32) {
        panic!("SelectingMultiPage is abstract.")
    }
}

ferro_properties! {
    impl SelectingMultiPage {
        /// Defines the `SelectedIndex` property.
        pub fn selected_index_property() -> DirectProperty<SelectingMultiPage, i32> {
            FerroProperty::register_direct::<SelectingMultiPage, _>(
                "SelectedIndex",
                |o| o.selected_index(),
                Some(|o, v| o.set_selected_index(v)),
                0,
            )
        }

        /// Defines the `SelectedPage` property.
        pub fn selected_page_property() -> DirectProperty<SelectingMultiPage, Option<Ref<Page>>> {
            FerroProperty::register_direct::<SelectingMultiPage, _>("SelectedPage", |o| o.selected_page(), None, None)
        }
    }
}

impl SelectingMultiPage {
    ferro_routed_event!(
        /// Defines the `SelectionChanged` routed event.
        pub fn selection_changed_event() -> RoutedEvent<PageSelectionChangedEventArgs> {
            RoutedEvent::register::<SelectingMultiPage, _>("SelectionChanged", RoutingStrategies::BUBBLE)
        }
    );

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: MultiPage::construct(), selected_index: Cell::new(-1), selected_page: RefCell::new(None) }
    }

    /// Gets or sets the zero-based index of the selected page.
    pub fn selected_index(&self) -> i32 {
        self.selected_index.get()
    }

    pub fn set_selected_index(&self, value: i32) {
        self.apply_selected_index(value)
    }

    /// Gets the currently selected page, or `None` if no page is selected.
    pub fn selected_page(&self) -> Option<Ref<Page>> {
        self.selected_page.borrow().clone()
    }

    /// Raised when the selected page changes.
    pub fn selection_changed(
        &self,
        handler: impl Fn(&Interactive, &PageSelectionChangedEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::selection_changed_event(), handler)
    }

    /// Commits a selection change and fires lifecycle events on the
    /// outgoing and incoming pages.
    ///
    /// Raises `SelectionChanged`, then `NavigatedFrom` on the outgoing page
    /// and `NavigatedTo` on the incoming page. If the incoming and outgoing
    /// pages are the same object no events are fired. Subclasses should
    /// call this method rather than modifying selection state directly.
    pub fn commit_selection(&self, new_index: i32, new_page: Option<Ref<Page>>, navigation_type: NavigationType) {
        let previous_page = self.selected_page();
        self.set_and_raise_cell(Self::selected_index_property(), &self.selected_index, new_index);
        self.set_and_raise(Self::selected_page_property(), &self.selected_page, new_page.clone());
        self.set_current_value(Page::current_page_property(), new_page.clone());
        if !same_page(&previous_page, &new_page) {
            self.raise_event(&PageSelectionChangedEventArgs::new(
                Some(Self::selection_changed_event()),
                previous_page.clone(),
                new_page.clone(),
            ));

            if let Some(previous_page) = &previous_page {
                previous_page.send_navigated_from(&NavigatedFromEventArgs::new(new_page.clone(), navigation_type));
            }

            if let Some(new_page) = &new_page {
                new_page.send_navigated_to(&NavigatedToEventArgs::new(previous_page, navigation_type));
            }
        }
    }

    /// Stores the selected index without routing through a child control.
    pub fn store_selected_index(&self, index: i32) {
        self.set_and_raise_cell(Self::selected_index_property(), &self.selected_index, index);
    }

    /// Returns the page at `index` from the `Pages` collection, or `None`
    /// if the index is out of range.
    pub fn resolve_page_at_index(&self, index: i32) -> Option<Ref<Page>> {
        let pages = self.pages()?;
        if index < 0 {
            return None;
        }
        pages.try_get(index as usize)
    }
}

/// Whether two page references are the same object (or both null).
pub(crate) fn same_page(a: &Option<Ref<Page>>, b: &Option<Ref<Page>>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => a.ptr_eq(b),
        _ => false,
    }
}
