//! Native model descriptors. Core models remain independent of the control layer.
use ferroribbon_core::{commands::RibbonCommandCatalog, model::*, *};
use ferroui_base::{
    data::{core::ValueTypes, model::ModelTypes},
    metadata::{MarkupType, MarkupTyped},
};
use ferroui_system::{
    collections::object_model::ObservableCollection, component_model::INotifyPropertyChanged,
};
use std::{any::TypeId, rc::Rc};
mod node;
pub use node::*;
mod item;
pub use item::*;
mod root;
pub use root::*;
mod gallery;
pub use gallery::*;
mod structure;
pub use structure::*;
mod menu;
pub use menu::*;
mod collections;
pub use collections::*;
fn string(v: Option<RibbonString>) -> Result<Option<String>, String> {
    v.map(|s| s.to_utf8().map_err(|e| e.to_string()))
        .transpose()
}
fn index(i: i32) -> usize {
    usize::try_from(i).expect("ArgumentOutOfRange: index")
}
fn plain<T: INotifyPropertyChanged + PartialEq + 'static>() {
    if !ModelTypes::is_registered(TypeId::of::<T>()) {
        ModelTypes::register::<T>(|b| b.notify_property_changed());
    }
}
/// Adds runtime notifications for a model. Declare concrete native casts at the
/// composition root; generic cast functions are not closed build-model facts.
pub fn register_model_notifications<T: INotifyPropertyChanged + PartialEq + 'static>() {
    plain::<T>();
}
pub(crate) fn register() {
    ferroribbon_core::model::register_ribbon_backstage_item_model::<
        ferroribbon_core::model::RibbonBackstageItemModel,
    >()
    .expect("built-in model hierarchy");
    ValueTypes::register_reference::<ferroribbon_core::model::RibbonBackstageItemModel>();
    plain::<ferroribbon_core::model::RibbonBackstageItemModel>();
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonBackstageItemModel>,
        Rc<dyn ferroribbon_core::model::RibbonBackstageItemModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonBackstageItemModel>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ferroribbon_core::model::register_ribbon_backstage_model::<
        ferroribbon_core::model::RibbonBackstageModel,
    >()
    .expect("built-in model hierarchy");
    ValueTypes::register_reference::<ferroribbon_core::model::RibbonBackstageModel>();
    plain::<ferroribbon_core::model::RibbonBackstageModel>();
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonBackstageModel>,
        Rc<dyn ferroribbon_core::model::RibbonBackstageModelImpl>,
    >(|v| v.clone());
    ferroribbon_core::model::register_ribbon_button_group_model::<
        ferroribbon_core::model::RibbonButtonGroupModel,
    >()
    .expect("built-in model hierarchy");
    ValueTypes::register_reference::<ferroribbon_core::model::RibbonButtonGroupModel>();
    plain::<ferroribbon_core::model::RibbonButtonGroupModel>();
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonButtonGroupModel>,
        Rc<dyn ferroribbon_core::model::RibbonButtonGroupModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonButtonGroupModel>,
        Rc<dyn ferroribbon_core::model::RibbonItemModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonButtonGroupModel>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ferroribbon_core::model::register_ribbon_button_model::<
        ferroribbon_core::model::RibbonButtonModel,
    >()
    .expect("built-in model hierarchy");
    ValueTypes::register_reference::<ferroribbon_core::model::RibbonButtonModel>();
    plain::<ferroribbon_core::model::RibbonButtonModel>();
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonButtonModel>,
        Rc<dyn ferroribbon_core::model::RibbonButtonModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonButtonModel>,
        Rc<dyn ferroribbon_core::model::RibbonItemModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonButtonModel>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ferroribbon_core::model::register_ribbon_check_box_model::<
        ferroribbon_core::model::RibbonCheckBoxModel,
    >()
    .expect("built-in model hierarchy");
    ValueTypes::register_reference::<ferroribbon_core::model::RibbonCheckBoxModel>();
    plain::<ferroribbon_core::model::RibbonCheckBoxModel>();
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonCheckBoxModel>,
        Rc<dyn ferroribbon_core::model::RibbonCheckBoxModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonCheckBoxModel>,
        Rc<dyn ferroribbon_core::model::RibbonItemModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonCheckBoxModel>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ferroribbon_core::model::register_ribbon_color_picker_model::<
        ferroribbon_core::model::RibbonColorPickerModel,
    >()
    .expect("built-in model hierarchy");
    ValueTypes::register_reference::<ferroribbon_core::model::RibbonColorPickerModel>();
    plain::<ferroribbon_core::model::RibbonColorPickerModel>();
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonColorPickerModel>,
        Rc<dyn ferroribbon_core::model::RibbonColorPickerModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonColorPickerModel>,
        Rc<dyn ferroribbon_core::model::RibbonItemModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonColorPickerModel>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ferroribbon_core::model::register_ribbon_combo_box_model::<
        ferroribbon_core::model::RibbonComboBoxModel,
    >()
    .expect("built-in model hierarchy");
    ValueTypes::register_reference::<ferroribbon_core::model::RibbonComboBoxModel>();
    plain::<ferroribbon_core::model::RibbonComboBoxModel>();
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonComboBoxModel>,
        Rc<dyn ferroribbon_core::model::RibbonComboBoxModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonComboBoxModel>,
        Rc<dyn ferroribbon_core::model::RibbonItemModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonComboBoxModel>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ferroribbon_core::model::register_ribbon_contextual_group_model::<
        ferroribbon_core::model::RibbonContextualGroupModel,
    >()
    .expect("built-in model hierarchy");
    ValueTypes::register_reference::<ferroribbon_core::model::RibbonContextualGroupModel>();
    plain::<ferroribbon_core::model::RibbonContextualGroupModel>();
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonContextualGroupModel>,
        Rc<dyn ferroribbon_core::model::RibbonContextualGroupModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonContextualGroupModel>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ferroribbon_core::model::register_ribbon_custom_item_model::<
        ferroribbon_core::model::RibbonCustomItemModel,
    >()
    .expect("built-in model hierarchy");
    ValueTypes::register_reference::<ferroribbon_core::model::RibbonCustomItemModel>();
    plain::<ferroribbon_core::model::RibbonCustomItemModel>();
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonCustomItemModel>,
        Rc<dyn ferroribbon_core::model::RibbonCustomItemModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonCustomItemModel>,
        Rc<dyn ferroribbon_core::model::RibbonItemModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonCustomItemModel>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ferroribbon_core::model::register_ribbon_drop_down_button_model::<
        ferroribbon_core::model::RibbonDropDownButtonModel,
    >()
    .expect("built-in model hierarchy");
    ValueTypes::register_reference::<ferroribbon_core::model::RibbonDropDownButtonModel>();
    plain::<ferroribbon_core::model::RibbonDropDownButtonModel>();
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonDropDownButtonModel>,
        Rc<dyn ferroribbon_core::model::RibbonDropDownButtonModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonDropDownButtonModel>,
        Rc<dyn ferroribbon_core::model::RibbonItemModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonDropDownButtonModel>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ferroribbon_core::model::register_ribbon_gallery_item_model::<
        ferroribbon_core::model::RibbonGalleryItemModel,
    >()
    .expect("built-in model hierarchy");
    ValueTypes::register_reference::<ferroribbon_core::model::RibbonGalleryItemModel>();
    plain::<ferroribbon_core::model::RibbonGalleryItemModel>();
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonGalleryItemModel>,
        Rc<dyn ferroribbon_core::model::RibbonGalleryItemModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonGalleryItemModel>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ferroribbon_core::model::register_ribbon_gallery_model::<
        ferroribbon_core::model::RibbonGalleryModel,
    >()
    .expect("built-in model hierarchy");
    ValueTypes::register_reference::<ferroribbon_core::model::RibbonGalleryModel>();
    plain::<ferroribbon_core::model::RibbonGalleryModel>();
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonGalleryModel>,
        Rc<dyn ferroribbon_core::model::RibbonGalleryModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonGalleryModel>,
        Rc<dyn ferroribbon_core::model::RibbonItemModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonGalleryModel>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ferroribbon_core::model::register_ribbon_grid_picker_model::<
        ferroribbon_core::model::RibbonGridPickerModel,
    >()
    .expect("built-in model hierarchy");
    ValueTypes::register_reference::<ferroribbon_core::model::RibbonGridPickerModel>();
    plain::<ferroribbon_core::model::RibbonGridPickerModel>();
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonGridPickerModel>,
        Rc<dyn ferroribbon_core::model::RibbonGridPickerModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonGridPickerModel>,
        Rc<dyn ferroribbon_core::model::RibbonItemModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonGridPickerModel>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ferroribbon_core::model::register_ribbon_group_model::<ferroribbon_core::model::RibbonGroupModel>().expect("built-in model hierarchy");
    ValueTypes::register_reference::<ferroribbon_core::model::RibbonGroupModel>();
    plain::<ferroribbon_core::model::RibbonGroupModel>();
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonGroupModel>,
        Rc<dyn ferroribbon_core::model::RibbonGroupModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonGroupModel>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ferroribbon_core::model::register_ribbon_label_model::<ferroribbon_core::model::RibbonLabelModel>().expect("built-in model hierarchy");
    ValueTypes::register_reference::<ferroribbon_core::model::RibbonLabelModel>();
    plain::<ferroribbon_core::model::RibbonLabelModel>();
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonLabelModel>,
        Rc<dyn ferroribbon_core::model::RibbonLabelModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonLabelModel>,
        Rc<dyn ferroribbon_core::model::RibbonItemModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonLabelModel>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ferroribbon_core::model::register_ribbon_menu_item_model::<
        ferroribbon_core::model::RibbonMenuItemModel,
    >()
    .expect("built-in model hierarchy");
    ValueTypes::register_reference::<ferroribbon_core::model::RibbonMenuItemModel>();
    plain::<ferroribbon_core::model::RibbonMenuItemModel>();
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonMenuItemModel>,
        Rc<dyn ferroribbon_core::model::RibbonMenuItemModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonMenuItemModel>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ferroribbon_core::model::register_ribbon_model::<ferroribbon_core::model::RibbonModel>()
        .expect("built-in model hierarchy");
    ValueTypes::register_reference::<ferroribbon_core::model::RibbonModel>();
    plain::<ferroribbon_core::model::RibbonModel>();
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonModel>,
        Rc<dyn ferroribbon_core::model::RibbonModelImpl>,
    >(|v| v.clone());
    ferroribbon_core::model::register_ribbon_row_model::<ferroribbon_core::model::RibbonRowModel>()
        .expect("built-in model hierarchy");
    ValueTypes::register_reference::<ferroribbon_core::model::RibbonRowModel>();
    plain::<ferroribbon_core::model::RibbonRowModel>();
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonRowModel>,
        Rc<dyn ferroribbon_core::model::RibbonRowModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonRowModel>,
        Rc<dyn ferroribbon_core::model::RibbonButtonGroupModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonRowModel>,
        Rc<dyn ferroribbon_core::model::RibbonItemModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonRowModel>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ferroribbon_core::model::register_ribbon_segment_model::<
        ferroribbon_core::model::RibbonSegmentModel,
    >()
    .expect("built-in model hierarchy");
    ValueTypes::register_reference::<ferroribbon_core::model::RibbonSegmentModel>();
    plain::<ferroribbon_core::model::RibbonSegmentModel>();
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonSegmentModel>,
        Rc<dyn ferroribbon_core::model::RibbonSegmentModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonSegmentModel>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ferroribbon_core::model::register_ribbon_segmented_model::<
        ferroribbon_core::model::RibbonSegmentedModel,
    >()
    .expect("built-in model hierarchy");
    ValueTypes::register_reference::<ferroribbon_core::model::RibbonSegmentedModel>();
    plain::<ferroribbon_core::model::RibbonSegmentedModel>();
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonSegmentedModel>,
        Rc<dyn ferroribbon_core::model::RibbonSegmentedModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonSegmentedModel>,
        Rc<dyn ferroribbon_core::model::RibbonItemModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonSegmentedModel>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ferroribbon_core::model::register_ribbon_separator_model::<
        ferroribbon_core::model::RibbonSeparatorModel,
    >()
    .expect("built-in model hierarchy");
    ValueTypes::register_reference::<ferroribbon_core::model::RibbonSeparatorModel>();
    plain::<ferroribbon_core::model::RibbonSeparatorModel>();
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonSeparatorModel>,
        Rc<dyn ferroribbon_core::model::RibbonSeparatorModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonSeparatorModel>,
        Rc<dyn ferroribbon_core::model::RibbonItemModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonSeparatorModel>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ferroribbon_core::model::register_ribbon_slider_model::<
        ferroribbon_core::model::RibbonSliderModel,
    >()
    .expect("built-in model hierarchy");
    ValueTypes::register_reference::<ferroribbon_core::model::RibbonSliderModel>();
    plain::<ferroribbon_core::model::RibbonSliderModel>();
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonSliderModel>,
        Rc<dyn ferroribbon_core::model::RibbonSliderModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonSliderModel>,
        Rc<dyn ferroribbon_core::model::RibbonItemModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonSliderModel>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ferroribbon_core::model::register_ribbon_spinner_model::<
        ferroribbon_core::model::RibbonSpinnerModel,
    >()
    .expect("built-in model hierarchy");
    ValueTypes::register_reference::<ferroribbon_core::model::RibbonSpinnerModel>();
    plain::<ferroribbon_core::model::RibbonSpinnerModel>();
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonSpinnerModel>,
        Rc<dyn ferroribbon_core::model::RibbonSpinnerModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonSpinnerModel>,
        Rc<dyn ferroribbon_core::model::RibbonItemModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonSpinnerModel>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ferroribbon_core::model::register_ribbon_split_button_model::<
        ferroribbon_core::model::RibbonSplitButtonModel,
    >()
    .expect("built-in model hierarchy");
    ValueTypes::register_reference::<ferroribbon_core::model::RibbonSplitButtonModel>();
    plain::<ferroribbon_core::model::RibbonSplitButtonModel>();
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonSplitButtonModel>,
        Rc<dyn ferroribbon_core::model::RibbonSplitButtonModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonSplitButtonModel>,
        Rc<dyn ferroribbon_core::model::RibbonDropDownButtonModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonSplitButtonModel>,
        Rc<dyn ferroribbon_core::model::RibbonItemModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonSplitButtonModel>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ferroribbon_core::model::register_ribbon_tab_model::<ferroribbon_core::model::RibbonTabModel>()
        .expect("built-in model hierarchy");
    ValueTypes::register_reference::<ferroribbon_core::model::RibbonTabModel>();
    plain::<ferroribbon_core::model::RibbonTabModel>();
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonTabModel>,
        Rc<dyn ferroribbon_core::model::RibbonTabModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonTabModel>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ferroribbon_core::model::register_ribbon_text_box_model::<
        ferroribbon_core::model::RibbonTextBoxModel,
    >()
    .expect("built-in model hierarchy");
    ValueTypes::register_reference::<ferroribbon_core::model::RibbonTextBoxModel>();
    plain::<ferroribbon_core::model::RibbonTextBoxModel>();
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonTextBoxModel>,
        Rc<dyn ferroribbon_core::model::RibbonTextBoxModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonTextBoxModel>,
        Rc<dyn ferroribbon_core::model::RibbonItemModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonTextBoxModel>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ferroribbon_core::model::register_ribbon_toggle_button_model::<
        ferroribbon_core::model::RibbonToggleButtonModel,
    >()
    .expect("built-in model hierarchy");
    ValueTypes::register_reference::<ferroribbon_core::model::RibbonToggleButtonModel>();
    plain::<ferroribbon_core::model::RibbonToggleButtonModel>();
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonToggleButtonModel>,
        Rc<dyn ferroribbon_core::model::RibbonToggleButtonModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonToggleButtonModel>,
        Rc<dyn ferroribbon_core::model::RibbonButtonModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonToggleButtonModel>,
        Rc<dyn ferroribbon_core::model::RibbonItemModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonToggleButtonModel>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ferroribbon_core::model::register_ribbon_node_model::<
        ferroribbon_core::model::RibbonMenuHeaderModel,
    >()
    .expect("built-in model hierarchy");
    ValueTypes::register_reference::<ferroribbon_core::model::RibbonMenuHeaderModel>();
    plain::<ferroribbon_core::model::RibbonMenuHeaderModel>();
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonMenuHeaderModel>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ferroribbon_core::model::register_ribbon_node_model::<
        ferroribbon_core::model::RibbonMenuSeparatorModel,
    >()
    .expect("built-in model hierarchy");
    ValueTypes::register_reference::<ferroribbon_core::model::RibbonMenuSeparatorModel>();
    plain::<ferroribbon_core::model::RibbonMenuSeparatorModel>();
    ValueTypes::register_cast::<
        Rc<ferroribbon_core::model::RibbonMenuSeparatorModel>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_reference::<ferroribbon_core::model::RibbonScreenTip>();
    plain::<ferroribbon_core::model::RibbonScreenTip>();
    ValueTypes::register_reference_view::<
        Rc<dyn ferroribbon_core::model::RibbonBackstageItemModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_nullable::<Rc<dyn ferroribbon_core::model::RibbonBackstageItemModelImpl>>(
    );
    ValueTypes::register_cast::<
        Rc<dyn ferroribbon_core::model::RibbonBackstageItemModelImpl>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_reference_view::<Rc<dyn ferroribbon_core::model::RibbonBackstageModelImpl>>(
        |v| v.clone(),
    );
    ValueTypes::register_nullable::<Rc<dyn ferroribbon_core::model::RibbonBackstageModelImpl>>();
    ValueTypes::register_reference_view::<
        Rc<dyn ferroribbon_core::model::RibbonButtonGroupModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_nullable::<Rc<dyn ferroribbon_core::model::RibbonButtonGroupModelImpl>>();
    ValueTypes::register_cast::<
        Rc<dyn ferroribbon_core::model::RibbonButtonGroupModelImpl>,
        Rc<dyn ferroribbon_core::model::RibbonItemModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<dyn ferroribbon_core::model::RibbonButtonGroupModelImpl>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_reference_view::<Rc<dyn ferroribbon_core::model::RibbonButtonModelImpl>>(
        |v| v.clone(),
    );
    ValueTypes::register_nullable::<Rc<dyn ferroribbon_core::model::RibbonButtonModelImpl>>();
    ValueTypes::register_cast::<
        Rc<dyn ferroribbon_core::model::RibbonButtonModelImpl>,
        Rc<dyn ferroribbon_core::model::RibbonItemModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<dyn ferroribbon_core::model::RibbonButtonModelImpl>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_reference_view::<Rc<dyn ferroribbon_core::model::RibbonCheckBoxModelImpl>>(
        |v| v.clone(),
    );
    ValueTypes::register_nullable::<Rc<dyn ferroribbon_core::model::RibbonCheckBoxModelImpl>>();
    ValueTypes::register_cast::<
        Rc<dyn ferroribbon_core::model::RibbonCheckBoxModelImpl>,
        Rc<dyn ferroribbon_core::model::RibbonItemModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<dyn ferroribbon_core::model::RibbonCheckBoxModelImpl>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_reference_view::<
        Rc<dyn ferroribbon_core::model::RibbonColorPickerModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_nullable::<Rc<dyn ferroribbon_core::model::RibbonColorPickerModelImpl>>();
    ValueTypes::register_cast::<
        Rc<dyn ferroribbon_core::model::RibbonColorPickerModelImpl>,
        Rc<dyn ferroribbon_core::model::RibbonItemModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<dyn ferroribbon_core::model::RibbonColorPickerModelImpl>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_reference_view::<Rc<dyn ferroribbon_core::model::RibbonComboBoxModelImpl>>(
        |v| v.clone(),
    );
    ValueTypes::register_nullable::<Rc<dyn ferroribbon_core::model::RibbonComboBoxModelImpl>>();
    ValueTypes::register_cast::<
        Rc<dyn ferroribbon_core::model::RibbonComboBoxModelImpl>,
        Rc<dyn ferroribbon_core::model::RibbonItemModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<dyn ferroribbon_core::model::RibbonComboBoxModelImpl>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_reference_view::<
        Rc<dyn ferroribbon_core::model::RibbonContextualGroupModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_nullable::<Rc<dyn ferroribbon_core::model::RibbonContextualGroupModelImpl>>(
    );
    ValueTypes::register_cast::<
        Rc<dyn ferroribbon_core::model::RibbonContextualGroupModelImpl>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_reference_view::<Rc<dyn ferroribbon_core::model::RibbonCustomItemModelImpl>>(
        |v| v.clone(),
    );
    ValueTypes::register_nullable::<Rc<dyn ferroribbon_core::model::RibbonCustomItemModelImpl>>();
    ValueTypes::register_cast::<
        Rc<dyn ferroribbon_core::model::RibbonCustomItemModelImpl>,
        Rc<dyn ferroribbon_core::model::RibbonItemModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<dyn ferroribbon_core::model::RibbonCustomItemModelImpl>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_reference_view::<
        Rc<dyn ferroribbon_core::model::RibbonDropDownButtonModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_nullable::<Rc<dyn ferroribbon_core::model::RibbonDropDownButtonModelImpl>>(
    );
    ValueTypes::register_cast::<
        Rc<dyn ferroribbon_core::model::RibbonDropDownButtonModelImpl>,
        Rc<dyn ferroribbon_core::model::RibbonItemModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<dyn ferroribbon_core::model::RibbonDropDownButtonModelImpl>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_reference_view::<
        Rc<dyn ferroribbon_core::model::RibbonGalleryItemModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_nullable::<Rc<dyn ferroribbon_core::model::RibbonGalleryItemModelImpl>>();
    ValueTypes::register_cast::<
        Rc<dyn ferroribbon_core::model::RibbonGalleryItemModelImpl>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_reference_view::<Rc<dyn ferroribbon_core::model::RibbonGalleryModelImpl>>(
        |v| v.clone(),
    );
    ValueTypes::register_nullable::<Rc<dyn ferroribbon_core::model::RibbonGalleryModelImpl>>();
    ValueTypes::register_cast::<
        Rc<dyn ferroribbon_core::model::RibbonGalleryModelImpl>,
        Rc<dyn ferroribbon_core::model::RibbonItemModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<dyn ferroribbon_core::model::RibbonGalleryModelImpl>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_reference_view::<Rc<dyn ferroribbon_core::model::RibbonGridPickerModelImpl>>(
        |v| v.clone(),
    );
    ValueTypes::register_nullable::<Rc<dyn ferroribbon_core::model::RibbonGridPickerModelImpl>>();
    ValueTypes::register_cast::<
        Rc<dyn ferroribbon_core::model::RibbonGridPickerModelImpl>,
        Rc<dyn ferroribbon_core::model::RibbonItemModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<dyn ferroribbon_core::model::RibbonGridPickerModelImpl>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_reference_view::<Rc<dyn ferroribbon_core::model::RibbonGroupModelImpl>>(
        |v| v.clone(),
    );
    ValueTypes::register_nullable::<Rc<dyn ferroribbon_core::model::RibbonGroupModelImpl>>();
    ValueTypes::register_cast::<
        Rc<dyn ferroribbon_core::model::RibbonGroupModelImpl>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_reference_view::<Rc<dyn ferroribbon_core::model::RibbonItemModelImpl>>(
        |v| v.clone(),
    );
    ValueTypes::register_nullable::<Rc<dyn ferroribbon_core::model::RibbonItemModelImpl>>();
    ValueTypes::register_cast::<
        Rc<dyn ferroribbon_core::model::RibbonItemModelImpl>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_reference_view::<Rc<dyn ferroribbon_core::model::RibbonLabelModelImpl>>(
        |v| v.clone(),
    );
    ValueTypes::register_nullable::<Rc<dyn ferroribbon_core::model::RibbonLabelModelImpl>>();
    ValueTypes::register_cast::<
        Rc<dyn ferroribbon_core::model::RibbonLabelModelImpl>,
        Rc<dyn ferroribbon_core::model::RibbonItemModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<dyn ferroribbon_core::model::RibbonLabelModelImpl>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_reference_view::<Rc<dyn ferroribbon_core::model::RibbonMenuItemModelImpl>>(
        |v| v.clone(),
    );
    ValueTypes::register_nullable::<Rc<dyn ferroribbon_core::model::RibbonMenuItemModelImpl>>();
    ValueTypes::register_cast::<
        Rc<dyn ferroribbon_core::model::RibbonMenuItemModelImpl>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_reference_view::<Rc<dyn ferroribbon_core::model::RibbonModelImpl>>(|v| {
        v.clone()
    });
    ValueTypes::register_nullable::<Rc<dyn ferroribbon_core::model::RibbonModelImpl>>();
    ValueTypes::register_reference_view::<Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>>(
        |v| v.clone(),
    );
    ValueTypes::register_nullable::<Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>>();
    ValueTypes::register_reference_view::<Rc<dyn ferroribbon_core::model::RibbonRowModelImpl>>(
        |v| v.clone(),
    );
    ValueTypes::register_nullable::<Rc<dyn ferroribbon_core::model::RibbonRowModelImpl>>();
    ValueTypes::register_cast::<
        Rc<dyn ferroribbon_core::model::RibbonRowModelImpl>,
        Rc<dyn ferroribbon_core::model::RibbonButtonGroupModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<dyn ferroribbon_core::model::RibbonRowModelImpl>,
        Rc<dyn ferroribbon_core::model::RibbonItemModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<dyn ferroribbon_core::model::RibbonRowModelImpl>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_reference_view::<Rc<dyn ferroribbon_core::model::RibbonSegmentModelImpl>>(
        |v| v.clone(),
    );
    ValueTypes::register_nullable::<Rc<dyn ferroribbon_core::model::RibbonSegmentModelImpl>>();
    ValueTypes::register_cast::<
        Rc<dyn ferroribbon_core::model::RibbonSegmentModelImpl>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_reference_view::<Rc<dyn ferroribbon_core::model::RibbonSegmentedModelImpl>>(
        |v| v.clone(),
    );
    ValueTypes::register_nullable::<Rc<dyn ferroribbon_core::model::RibbonSegmentedModelImpl>>();
    ValueTypes::register_cast::<
        Rc<dyn ferroribbon_core::model::RibbonSegmentedModelImpl>,
        Rc<dyn ferroribbon_core::model::RibbonItemModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<dyn ferroribbon_core::model::RibbonSegmentedModelImpl>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_reference_view::<Rc<dyn ferroribbon_core::model::RibbonSeparatorModelImpl>>(
        |v| v.clone(),
    );
    ValueTypes::register_nullable::<Rc<dyn ferroribbon_core::model::RibbonSeparatorModelImpl>>();
    ValueTypes::register_cast::<
        Rc<dyn ferroribbon_core::model::RibbonSeparatorModelImpl>,
        Rc<dyn ferroribbon_core::model::RibbonItemModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<dyn ferroribbon_core::model::RibbonSeparatorModelImpl>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_reference_view::<Rc<dyn ferroribbon_core::model::RibbonSliderModelImpl>>(
        |v| v.clone(),
    );
    ValueTypes::register_nullable::<Rc<dyn ferroribbon_core::model::RibbonSliderModelImpl>>();
    ValueTypes::register_cast::<
        Rc<dyn ferroribbon_core::model::RibbonSliderModelImpl>,
        Rc<dyn ferroribbon_core::model::RibbonItemModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<dyn ferroribbon_core::model::RibbonSliderModelImpl>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_reference_view::<Rc<dyn ferroribbon_core::model::RibbonSpinnerModelImpl>>(
        |v| v.clone(),
    );
    ValueTypes::register_nullable::<Rc<dyn ferroribbon_core::model::RibbonSpinnerModelImpl>>();
    ValueTypes::register_cast::<
        Rc<dyn ferroribbon_core::model::RibbonSpinnerModelImpl>,
        Rc<dyn ferroribbon_core::model::RibbonItemModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<dyn ferroribbon_core::model::RibbonSpinnerModelImpl>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_reference_view::<
        Rc<dyn ferroribbon_core::model::RibbonSplitButtonModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_nullable::<Rc<dyn ferroribbon_core::model::RibbonSplitButtonModelImpl>>();
    ValueTypes::register_cast::<
        Rc<dyn ferroribbon_core::model::RibbonSplitButtonModelImpl>,
        Rc<dyn ferroribbon_core::model::RibbonDropDownButtonModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<dyn ferroribbon_core::model::RibbonSplitButtonModelImpl>,
        Rc<dyn ferroribbon_core::model::RibbonItemModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<dyn ferroribbon_core::model::RibbonSplitButtonModelImpl>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_reference_view::<Rc<dyn ferroribbon_core::model::RibbonTabModelImpl>>(
        |v| v.clone(),
    );
    ValueTypes::register_nullable::<Rc<dyn ferroribbon_core::model::RibbonTabModelImpl>>();
    ValueTypes::register_cast::<
        Rc<dyn ferroribbon_core::model::RibbonTabModelImpl>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_reference_view::<Rc<dyn ferroribbon_core::model::RibbonTextBoxModelImpl>>(
        |v| v.clone(),
    );
    ValueTypes::register_nullable::<Rc<dyn ferroribbon_core::model::RibbonTextBoxModelImpl>>();
    ValueTypes::register_cast::<
        Rc<dyn ferroribbon_core::model::RibbonTextBoxModelImpl>,
        Rc<dyn ferroribbon_core::model::RibbonItemModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<dyn ferroribbon_core::model::RibbonTextBoxModelImpl>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_reference_view::<
        Rc<dyn ferroribbon_core::model::RibbonToggleButtonModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_nullable::<Rc<dyn ferroribbon_core::model::RibbonToggleButtonModelImpl>>();
    ValueTypes::register_cast::<
        Rc<dyn ferroribbon_core::model::RibbonToggleButtonModelImpl>,
        Rc<dyn ferroribbon_core::model::RibbonButtonModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<dyn ferroribbon_core::model::RibbonToggleButtonModelImpl>,
        Rc<dyn ferroribbon_core::model::RibbonItemModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_cast::<
        Rc<dyn ferroribbon_core::model::RibbonToggleButtonModelImpl>,
        Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
    >(|v| v.clone());
    ValueTypes::register_reference::<RibbonIcon>();
    plain::<RibbonScreenTip>();
    ValueTypes::register_reference::<RibbonCommandCatalog>();
    ModelTypes::register_notifying_collection::<ModelColors>();
    ModelTypes::register_notifying_collection::<ModelSwatches>();
    ModelTypes::register_notifying_collection::<ModelSegments>();
    ModelTypes::register_notifying_collection::<ModelItems>();
    ModelTypes::register_notifying_collection::<ModelNodes>();
    ModelTypes::register_notifying_collection::<ModelTabs>();
    ModelTypes::register_notifying_collection::<ModelGroups>();
    ModelTypes::register_notifying_collection::<ModelContextualGroups>();
    ModelTypes::register_notifying_collection::<ModelBackstageItems>();
    ModelTypes::register_notifying_collection::<ModelGalleryItems>();
    ModelTypes::register_notifying_collection::<ModelStrings>();
    gallery::register_views();
    MarkupType::register_all(&[
        ObservableObjectMetadata::MARKUP,
        RibbonNodeModelMetadata::MARKUP,
        RibbonItemModelMetadata::MARKUP,
        RibbonButtonModelMetadata::MARKUP,
        RibbonToggleButtonModelMetadata::MARKUP,
        RibbonDropDownButtonModelMetadata::MARKUP,
        RibbonSplitButtonModelMetadata::MARKUP,
        RibbonCheckBoxModelMetadata::MARKUP,
        RibbonComboBoxModelMetadata::MARKUP,
        RibbonSpinnerModelMetadata::MARKUP,
        RibbonTextBoxModelMetadata::MARKUP,
        RibbonSliderModelMetadata::MARKUP,
        RibbonGridPickerModelMetadata::MARKUP,
        RibbonColorPickerModelMetadata::MARKUP,
        RibbonSegmentModelMetadata::MARKUP,
        RibbonSegmentedModelMetadata::MARKUP,
        RibbonCustomItemModelMetadata::MARKUP,
        RibbonMenuItemModelMetadata::MARKUP,
        RibbonMenuSeparatorModelMetadata::MARKUP,
        RibbonMenuHeaderModelMetadata::MARKUP,
        RibbonGroupModelMetadata::MARKUP,
        RibbonTabModelMetadata::MARKUP,
        RibbonContextualGroupModelMetadata::MARKUP,
        RibbonBackstageItemModelMetadata::MARKUP,
        RibbonBackstageModelMetadata::MARKUP,
        RibbonModelMetadata::MARKUP,
        ModelColorsMetadata::MARKUP,
        ModelSwatchesMetadata::MARKUP,
        ModelSegmentsMetadata::MARKUP,
        ModelItemsMetadata::MARKUP,
        ModelNodesMetadata::MARKUP,
        ModelTabsMetadata::MARKUP,
        ModelGroupsMetadata::MARKUP,
        ModelContextualGroupsMetadata::MARKUP,
        ModelBackstageItemsMetadata::MARKUP,
        ModelGalleryItemsMetadata::MARKUP,
        ModelStringsMetadata::MARKUP,
        RibbonGalleryModelMetadata::MARKUP,
        RibbonGalleryItemModelMetadata::MARKUP,
    ]);
}
