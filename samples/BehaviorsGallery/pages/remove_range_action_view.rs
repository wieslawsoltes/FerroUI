//! Port of `Views/Pages/RemoveRangeActionView` of the reference sample: the class of `pages/RemoveRangeActionView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::PageInfo;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::primitives::SelectedItemsList;
use ferroui_controls::{Control, ItemsSource, UserControl};

#[repr(C)]
pub struct RemoveRangeActionView {
    base: UserControl,
}

user_control_class!(RemoveRangeActionView);
ferro_class_info!(RemoveRangeActionView { new: RemoveRangeActionView::new });
xaml_class!(RemoveRangeActionView, "/pages/RemoveRangeActionView.xaml");

impl RemoveRangeActionView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `RemoveRangeActionView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    RemoveRangeActionView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[RemoveRangeActionView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "RemoveRangeAction",
    reference: "Views/Pages/RemoveRangeActionView",
    create: Some(create),
    types: TYPES,
    classes: &[&RemoveRangeActionView::XAML_CLASS],
    markup_types: &[],
    register_value_types,
};

/// What the untyped value conversions must know for the document of the page: the selected items
/// of a list box (`SelectedItems`) as the items of the action (`Items`).
///
/// Deviation (DEVIATIONS.md, Behaviors gallery): the reference binds the selected items (an
/// `IList`) to the items of the action (an `IEnumerable`) directly; the framework registers no
/// conversion from the list of the selected items to the untyped list of items the action
/// declares (`ItemsSource`), so the page registers it: the items are the shared list of the
/// selected items, not a copy.
fn register_value_types() {
    ValueTypes::register_conversion::<SelectedItemsList, ItemsSource>(|list| {
        Some(ItemsSource::from(list.0.clone()))
    });
    ValueTypes::register_conversion::<SelectedItemsList, Option<ItemsSource>>(|list| {
        Some(Some(ItemsSource::from(list.0.clone())))
    });
}
