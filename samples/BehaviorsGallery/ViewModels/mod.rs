//! The view models of the gallery (namespace `BehaviorsGallery.ViewModels`): one module per
//! reference file of `ViewModels/`, and the helpers of the port ([`command`], [`items`]).
//!
//! The view models several pages share are listed in the tables of this module. A view model only
//! one page uses is registered by that page (its entry of the page table, `pages/mod.rs`), and
//! its module line below stays a comment until the page is ported: the page agent removes the
//! comment marker of that line and touches nothing else here (`PAGES.md`, "How to port a page").

use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_base::TypeInfo;

/// The getters and the notifying setters of the properties of a view model (the `[Reactive]`
/// properties of the reference, which its source generator writes): each field is a `RefCell` of
/// the type of the property, and the setter raises the change with the name of the property.
///
/// Not `reactive_properties!` of the ReactiveUI port: the struct of a view model is declared
/// outside a macro, so that the scanner of the build reads it (a type the documents name needs a
/// public path the scanner found).
///
/// ```ignore
/// reactive_accessors!(ItemViewModel {
///     value, set_value, "Value": Option<String>;
/// });
/// ```
macro_rules! reactive_accessors {
    ($type_:ty { $($field:ident, $setter:ident, $name:literal: $field_ty:ty;)* }) => {
        #[allow(dead_code)]
        impl $type_ {
            $(
                #[doc = concat!("`", $name, "`.")]
                pub fn $field(&self) -> $field_ty {
                    ::core::clone::Clone::clone(&*self.$field.borrow())
                }

                #[doc = concat!("Sets `", $name, "` and raises its change.")]
                pub fn $setter(&self, value: $field_ty) {
                    let _ = ::ferroui_reactiveui::IReactiveObjectExt::raise_and_set_if_changed(self, &self.$field, value, $name);
                }
            )*
        }
    };
}
#[allow(unused_imports)]
pub(crate) use reactive_accessors;

pub mod command;
pub mod items;

// The view models of the reference, in the order of its files; the owner of a commented line is
// the page that ports it.
pub mod cursor_view_model; // owner: CursorView
pub mod custom_animator_view_model; // owner: AnimationBuilderView (also RunAnimationTriggerView, StartBuiltAnimationActionView)
pub mod drag_and_drop_sample_view_model;
pub mod drag_drop_commands_view_model; // owner: DragDropCommandsView
pub mod drag_item_view_model;
pub mod drag_node_view_model;
pub mod draggable_view_model; // owner: DraggableView
pub mod drawn_cursor_view_model; // owner: DrawnCursorView
pub mod execute_command_behaviors_view_model; // owner: ExecuteCommandBehaviorsView (also ButtonExecuteCommandOnKeyDownBehaviorView)
pub mod file_system_view_model; // owner: FileSystemView
pub mod fluid_move_behavior_view_model; // owner: FluidMoveBehaviorView
pub mod icon_view_model; // owner: IconView
pub mod item_view_model;
pub mod key_gesture_trigger_view_model; // owner: KeyGestureTriggerView
pub mod main_window_view_model;
pub mod network_view_model; // owner: NetworkView
pub mod pointer_triggers_view_model; // owner: PointerTriggersView
#[path = "ReactiveUI/mod.rs"]
pub mod reactive_ui;
pub mod sample_view_model;
pub mod view_model_base;

pub use command::Command;
pub use drag_and_drop_sample_view_model::DragAndDropSampleViewModel;
pub use drag_item_view_model::DragItemViewModel;
pub use drag_node_view_model::DragNodeViewModel;
pub use item_view_model::ItemViewModel;
pub use main_window_view_model::MainWindowViewModel;
pub use sample_view_model::SampleViewModel;
pub use view_model_base::ViewModelBase;

/// The classes of this namespace (`X::TYPE`): none, the view models are not classes of the object
/// model.
pub(crate) const TYPES: &[&TypeInfo] = &[];

/// The shared view models declared with `ferro_markup_type!` (`<X as MarkupTyped>::MARKUP`).
pub(crate) const MARKUP_TYPES: &[&MarkupType] = &[
    <DragAndDropSampleViewModel as MarkupTyped>::MARKUP,
    <DragItemViewModel as MarkupTyped>::MARKUP,
    <DragNodeViewModel as MarkupTyped>::MARKUP,
    <ItemViewModel as MarkupTyped>::MARKUP,
    <MainWindowViewModel as MarkupTyped>::MARKUP,
    <SampleViewModel as MarkupTyped>::MARKUP,
];

/// What the untyped value conversions must know about the shared view models.
pub(crate) fn register_value_types() {
    ValueTypes::register_reference::<DragAndDropSampleViewModel>();
    ValueTypes::register_reference::<DragItemViewModel>();
    ValueTypes::register_reference::<DragNodeViewModel>();
    ValueTypes::register_reference::<ItemViewModel>();
    ValueTypes::register_reference::<MainWindowViewModel>();
    ValueTypes::register_reference::<SampleViewModel>();
    ValueTypes::register_display::<DragItemViewModel>();
    ValueTypes::register_display::<DragNodeViewModel>();
    ValueTypes::register_display::<ItemViewModel>();
}
