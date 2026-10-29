//! Port of `Views/Pages/StorageProviderView` of the reference sample: the class of
//! `pages/StorageProviderView.xaml`, whose folder picker behavior adds the picked folders to the
//! file items of the view model.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use crate::view_models::MainWindowViewModel;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref, TypeInfo, WeakRef};
use ferroui_controls::{Button, Control, UserControl};
use ferroui_xaml_interactions::{ButtonOpenFolderPickerBehavior, FolderPickerEventArgs};
use ferroui_xaml_interactivity::Interaction;
use std::rc::Rc;

#[repr(C)]
pub struct StorageProviderView {
    base: UserControl,
}

user_control_class!(StorageProviderView);
ferro_class_info!(StorageProviderView {
    new: StorageProviderView::new
});
xaml_class!(StorageProviderView, "/pages/StorageProviderView.xaml");

impl StorageProviderView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `StorageProviderView()`.
    ///
    /// Deviation (DEVIATIONS.md, Behaviors gallery): the document cannot name the handler of the
    /// `Pick` event (an event of the Rust API); the page subscribes it here.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        if let Some(button) = this.find_control::<Button>("OpenFoldersBehaviorButton") {
            let behaviors = Interaction::get_behaviors(&button);
            for behavior in behaviors.list().to_vec() {
                if let Some(behavior) = behavior.cast::<ButtonOpenFolderPickerBehavior>() {
                    let view = this.downgrade();
                    let _ = behavior
                        .pick(move |e| Self::button_open_folder_picker_behavior_on_pick(&view, e));
                }
            }
        }

        this
    }

    /// `ButtonOpenFolderPickerBehavior_OnPick(object?, FolderPickerEventArgs)`.
    fn button_open_folder_picker_behavior_on_pick(this: &WeakRef<Self>, e: &FolderPickerEventArgs) {
        let Some(this) = this.upgrade() else { return };
        let Some(view_model) = this
            .data_context()
            .and_then(|data_context| as_view_model(&data_context))
        else {
            return;
        };

        let items = view_model.file_items();

        for folder in e.folders() {
            if let Some(items) = &items {
                items.add(Rc::new(folder.path()) as BoxedValue);
            }

            if view_model.documents_folder().is_none() {
                view_model.set_documents_folder(Some(folder.clone()));
            }
        }

        e.set_handled(true);
    }
}

/// The data context as the view model of the window (`DataContext is MainWindowViewModel`).
fn as_view_model(data_context: &BoxedValue) -> Option<Rc<MainWindowViewModel>> {
    crate::view_models::items::as_item::<MainWindowViewModel>(data_context)
}

/// Creates the page.
fn create() -> Ref<Control> {
    StorageProviderView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[StorageProviderView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Storage Provider",
    reference: "Views/Pages/StorageProviderView",
    create: Some(create),
    types: TYPES,
    classes: &[&StorageProviderView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
