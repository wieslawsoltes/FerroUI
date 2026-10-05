//! Helpers shared by the controls.

mod ancestor_finder;
mod border_render_helper;

pub use ancestor_finder::AncestorFinder;
pub use border_render_helper::BorderRenderHelper;

mod string_utils;
mod undo_redo_helper;
pub use string_utils::StringUtils;
pub use undo_redo_helper::{IUndoRedoHost, UndoRedoHelper};

mod masked_text_provider;
pub use masked_text_provider::*;

mod binding_evaluator;
mod collection_changed_event_manager;
mod collection_utils;
mod realized_stack_elements;
mod virtualizing_snap_points_list;

pub use binding_evaluator::{BindingEvaluator};
pub use collection_changed_event_manager::{ICollectionChangedListener, CollectionChangedEventManager};
pub use collection_utils::{CollectionUtils};
pub(crate) use realized_stack_elements::{RealizedStackElements};
pub(crate) use virtualizing_snap_points_list::{VirtualizingSnapPointsList};

#[cfg(test)]
mod collection_changed_event_manager_tests;

mod clipboard_helper;
mod primary_selection_helper;
pub(crate) use clipboard_helper::ClipboardHelper;
pub(crate) use primary_selection_helper::PrimarySelectionHelper;

// --- autocomplete ---
mod i_selection_adapter;
mod selecting_items_control_selection_adapter;
pub use i_selection_adapter::ISelectionAdapter;
pub use selecting_items_control_selection_adapter::{
    SelectingItemsControlSelectionAdapter, SelectingItemsControlSelectionAdapterOverrides,
};

// --- datetimepickers ---
mod time_utils;
pub(crate) use time_utils::TimeUtils;

// --- debug display (native menu, table view) ---
pub(crate) mod debug_display;
