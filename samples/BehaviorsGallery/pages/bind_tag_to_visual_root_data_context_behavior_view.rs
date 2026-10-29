//! Port of `Views/Pages/BindTagToVisualRootDataContextBehaviorView` of the reference sample: the class of `pages/BindTagToVisualRootDataContextBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use crate::view_models::items::as_item;
use crate::view_models::MainWindowViewModel;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, TextBlock, UserControl};
use std::cell::RefCell;
use std::rc::Rc;

#[repr(C)]
pub struct BindTagToVisualRootDataContextBehaviorView {
    base: UserControl,
    data_context_changed: RefCell<Option<Rc<dyn IDisposable>>>,
}

user_control_class!(BindTagToVisualRootDataContextBehaviorView);
ferro_class_info!(BindTagToVisualRootDataContextBehaviorView { new: BindTagToVisualRootDataContextBehaviorView::new });
xaml_class!(BindTagToVisualRootDataContextBehaviorView, "/pages/BindTagToVisualRootDataContextBehaviorView.xaml");

impl BindTagToVisualRootDataContextBehaviorView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            data_context_changed: RefCell::new(None),
        }
    }

    /// `BindTagToVisualRootDataContextBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        // Deviation (DEVIATIONS.md, Behaviors gallery): `DataContext="{Binding Items[0]}"` of the
        // text block; the items are an enumerable without an indexer for markup, so the page sets
        // the first item when its data context changes.
        let page = this.downgrade();
        let subscription = this.data_context_changed(move || {
            if let Some(page) = page.upgrade() {
                page.update_item_text();
            }
        });
        *this.data_context_changed.borrow_mut() = Some(subscription);
        this.update_item_text();
        this
    }

    /// Sets the data context of the text block to the first item of the view model.
    fn update_item_text(&self) {
        let Some(item_text) = self.find_control::<TextBlock>("ItemText") else {
            return;
        };
        let first = self
            .data_context()
            .and_then(|data_context| as_item::<MainWindowViewModel>(&data_context))
            .and_then(|view_model| view_model.items())
            .and_then(|items| items.to_vec().first().cloned());
        if let Some(first) = first {
            item_text.set_data_context(Some(first));
        }
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    BindTagToVisualRootDataContextBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[BindTagToVisualRootDataContextBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "BindTagToVisualRootDataContextBehavior",
    reference: "Views/Pages/BindTagToVisualRootDataContextBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&BindTagToVisualRootDataContextBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
