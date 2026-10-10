//! Port of `HamburgerMenu/HamburgerMenu.cs`.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventArgs};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::IBrush;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, Rect, Ref, StyledElementImpl, StyledProperty, Visual, VisualImpl,
};
use ferroui_controls::primitives::{
    SelectingItemsControl, SelectingItemsControlImpl, TemplateAppliedEventArgs, TemplatedControlImpl, TemplatedControlImplExt,
};
use ferroui_controls::{
    ControlImpl, ControlImplExt, ItemsControlImpl, SplitView, SplitViewDisplayMode, TabControl, TabControlImpl, TabItem, TextBox,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[repr(C)]
pub struct HamburgerMenu {
    base: TabControl,
    split_view: RefCell<Option<Ref<SplitView>>>,
    search_box: RefCell<Option<Ref<TextBox>>>,
    initialized: Cell<bool>,
}

ferro_class!(HamburgerMenu: TabControl);
ferro_class_info!(HamburgerMenu { new: HamburgerMenu::new });
ferro_impl_classes!(
    HamburgerMenu: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ItemsControlImpl,
    SelectingItemsControlImpl,
    TabControlImpl
);

ferro_properties! {
    impl HamburgerMenu {
        pub fn pane_background_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            SplitView::pane_background_property().add_owner::<HamburgerMenu>()
        }

        pub fn content_background_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            FerroProperty::register::<HamburgerMenu, _>("ContentBackground", None)
        }

        pub fn expanded_mode_threshold_width_property() -> StyledProperty<i32> {
            FerroProperty::register::<HamburgerMenu, _>("ExpandedModeThresholdWidth", 1008)
        }
    }
}

impl TemplatedControlImpl for HamburgerMenu {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);

        *this.split_view.borrow_mut() = e.name_scope().find_as::<SplitView>("PART_NavigationPane");
        *this.search_box.borrow_mut() = e.name_scope().find_as::<TextBox>("PART_SearchBox");

        let search_box = this.search_box.borrow().clone();
        if let Some(search_box) = search_box {
            let weak = this.to_ref().downgrade();
            search_box.text_changed(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.on_search_text_changed();
                }
            });
        }
    }
}

impl ControlImpl for HamburgerMenu {
    fn on_loaded(this: &Self, e: &RoutedEventArgs) {
        Self::parent_on_loaded(this, e);

        if !this.initialized.get() {
            this.initialized.set(true);
            this.sort_items();
        }
    }
}

impl FerroObjectImpl for HamburgerMenu {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Visual::bounds_property().as_property() && this.split_view.borrow().is_some() {
            let (old_bounds, new_bounds) = change.get_old_and_new_value::<Rect>();
            this.ensure_split_view_mode(old_bounds, new_bounds);
        }

        if change.property() == SelectingItemsControl::selected_item_property().as_property() {
            let split_view = this.split_view.borrow().clone();
            if let Some(split_view) = split_view {
                if split_view.display_mode() == SplitViewDisplayMode::Overlay {
                    split_view.set_current_value(SplitView::is_pane_open_property(), false);
                }
            }
        }
    }
}

impl HamburgerMenu {
    pub fn construct() -> Self {
        Self {
            base: TabControl::construct(),
            split_view: RefCell::new(None),
            search_box: RefCell::new(None),
            initialized: Cell::new(false),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn pane_background(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::pane_background_property())
    }

    pub fn set_pane_background(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::pane_background_property(), value)
    }

    pub fn content_background(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::content_background_property())
    }

    pub fn set_content_background(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::content_background_property(), value)
    }

    pub fn expanded_mode_threshold_width(&self) -> i32 {
        self.get_value(Self::expanded_mode_threshold_width_property())
    }

    pub fn set_expanded_mode_threshold_width(&self, value: i32) {
        self.set_value(Self::expanded_mode_threshold_width_property(), value)
    }

    /// `Items.OfType<TabItem>()`, each with the item as the collection holds it.
    fn tab_items(&self) -> Vec<(BoxedValue, Ref<TabItem>)> {
        let items = self.items().view().to_vec();
        items.into_iter().flatten().filter_map(|item| from_markup_value::<Ref<TabItem>>(&Some(item.clone())).map(|tab| (item, tab))).collect()
    }

    /// `item.Header?.ToString() ?? ""`.
    fn header_text(item: &TabItem) -> String {
        item.header().map(|header| ValueTypes::to_display_string(Some(&header))).unwrap_or_default()
    }

    /// The key of an ordinal comparison that ignores case.
    fn ordinal_ignore_case(text: &str) -> Vec<u16> {
        text.to_uppercase().encode_utf16().collect()
    }

    fn sort_items(&self) {
        let items = self.tab_items();
        let mut sorted = items.clone();
        sorted.sort_by_key(|(_, item)| Self::ordinal_ignore_case(&Self::header_text(item)));

        // Only reorder if needed
        let mut needs_sort = false;
        for i in 0..items.len() {
            if !Ref::ptr_eq(&items[i].1, &sorted[i].1) {
                needs_sort = true;
                break;
            }
        }

        if needs_sort {
            self.items().clear();
            for (item, _) in sorted {
                self.items().add(Some(item));
            }
        }
    }

    fn on_search_text_changed(&self) {
        let search_text = self.search_box.borrow().as_ref().and_then(|search_box| search_box.text());
        let filter = search_text.filter(|text| !text.trim().is_empty());

        for (_, item) in self.tab_items() {
            match &filter {
                Some(search_text) => {
                    let header = Self::header_text(&item);
                    item.set_is_visible(header.to_uppercase().contains(&search_text.to_uppercase()));
                }
                None => item.set_is_visible(true),
            }
        }
    }

    fn ensure_split_view_mode(&self, _old_bounds: Rect, new_bounds: Rect) {
        let split_view = self.split_view.borrow().clone();
        if let Some(split_view) = split_view {
            let threshold = f64::from(self.expanded_mode_threshold_width());

            if new_bounds.width >= threshold {
                split_view.set_display_mode(SplitViewDisplayMode::Inline);
                split_view.set_is_pane_open(true);
            } else if new_bounds.width < threshold {
                split_view.set_display_mode(SplitViewDisplayMode::Overlay);
                split_view.set_is_pane_open(false);
            }
        }
    }
}
