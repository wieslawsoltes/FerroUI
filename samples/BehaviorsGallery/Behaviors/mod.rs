//! The behaviors of the sample (namespace `BehaviorsGallery.Behaviors`, which the default XML
//! namespace of the framework includes): one module per reference file of `Behaviors/`.
//!
//! The drop handlers several pages share are listed in the tables of this module. A behavior only
//! one page uses is registered by that page (its entry of the page table, `pages/mod.rs`), and its
//! module line below stays a comment until the page is ported: the page agent removes the comment
//! marker of that line and touches nothing else here (`PAGES.md`, "How to port a page").

use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_base::TypeInfo;
use ferroui_xaml_interactions_draganddrop::IDropHandler;
use std::rc::Rc;

// The behaviors of the reference, in the order of its files; the owner of a commented line is the
// page that ports it.
pub mod actual_theme_variant_changed_message_behavior; // owner: ActualThemeVariantChangedBehaviorView
pub mod attached_to_logical_tree_message_behavior; // owner: AttachedToLogicalTreeBehaviorView
pub mod data_context_changed_message_behavior; // owner: DataContextChangedBehaviorView
pub mod disposable_action_behavior; // owner: DisposableActionView
pub mod disposing_message_behavior; // owner: DisposingBehaviorView
pub mod files_drop_handler; // owner: FileDropHandlerView
pub mod initialized_message_behavior; // owner: InitializedBehaviorView
// pub mod items_data_grid_drop_handler; // owner: DragAndDropView (no DataGrid in the framework: not ported)
pub mod items_list_box_drop_handler;
pub mod loaded_message_behavior; // owner: LoadedBehaviorView
pub mod managed_items_list_box_drop_handler; // owner: DragAndDropView
pub mod nodes_list_box_drop_handler; // owner: DragAndDropView
pub mod nodes_tree_view_drop_handler;
pub mod resources_changed_message_behavior; // owner: ResourcesChangedBehaviorView

pub use items_list_box_drop_handler::ItemsListBoxDropHandler;
pub use nodes_tree_view_drop_handler::NodesTreeViewDropHandler;

/// The classes of this namespace (`X::TYPE`) the shared behaviors declare: none (the drop handlers
/// are not classes of the object model).
pub(crate) const TYPES: &[&TypeInfo] = &[];

/// The shared types of this namespace declared with `ferro_markup_type!`.
pub(crate) const MARKUP_TYPES: &[&MarkupType] = &[
    <ItemsListBoxDropHandler as MarkupTyped>::MARKUP,
    <NodesTreeViewDropHandler as MarkupTyped>::MARKUP,
];

/// What the untyped value conversions must know about the shared types of this namespace: the
/// drop handlers as drop handlers.
pub(crate) fn register_value_types() {
    ValueTypes::register_reference::<ItemsListBoxDropHandler>();
    ValueTypes::register_cast::<Rc<ItemsListBoxDropHandler>, Rc<dyn IDropHandler>>(|handler| {
        handler.clone()
    });
    ValueTypes::register_reference::<NodesTreeViewDropHandler>();
    ValueTypes::register_cast::<Rc<NodesTreeViewDropHandler>, Rc<dyn IDropHandler>>(|handler| {
        handler.clone()
    });
}
