// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md for license information.

//! `NullSelector`, `SingleSelector`, `MultipleSelector` and `ExtendedSelector` of WinUI's
//! `ItemsView`: what an interaction with an item and a focus move do to the selection.

use crate::ItemsViewSelectionMode;
use ferroui_controls_repeater::selection::{IndexPath, SelectionModel};

fn is_selected(model: &SelectionModel, index: &IndexPath) -> bool {
    model.is_selected_at(index).ok().flatten().unwrap_or(false)
}

/// Clears the selection and selects from the anchor to `index`, keeping the anchor.
fn select_only_range_from_anchor(model: &SelectionModel, index: &IndexPath) {
    if let Some(anchor) = model.anchor_index() {
        model.clear_selection();
        let _ = model.set_anchor_index(Some(anchor));
        let _ = model.select_range_from_anchor_to(index);
    }
}

/// `OnInteractedAction`: the item was clicked, or Enter or Space was pressed on it.
pub(super) fn on_interacted(mode: ItemsViewSelectionMode, model: &SelectionModel, index: &IndexPath, ctrl: bool, shift: bool) {
    match mode {
        ItemsViewSelectionMode::None => {}
        ItemsViewSelectionMode::Single => {
            model.set_single_select(true);
            if ctrl && is_selected(model, index) {
                let _ = model.deselect_at(index);
            } else {
                let _ = model.select_at(index);
            }
        }
        ItemsViewSelectionMode::Multiple => {
            if shift {
                if let Some(anchor) = model.anchor_index() {
                    let anchor_selected = is_selected(model, &anchor);
                    if anchor_selected != is_selected(model, index) {
                        let _ = if anchor_selected {
                            model.select_range_from_anchor_to(index)
                        } else {
                            model.deselect_range_from_anchor_to(index)
                        };
                    }
                }
            } else if is_selected(model, index) {
                let _ = model.deselect_at(index);
            } else {
                let _ = model.select_at(index);
            }
        }
        ItemsViewSelectionMode::Extended => {
            if shift {
                select_only_range_from_anchor(model, index);
            } else if ctrl {
                if is_selected(model, index) {
                    let _ = model.deselect_at(index);
                } else {
                    let _ = model.select_at(index);
                }
            } else if !is_selected(model, index) {
                model.clear_selection();
                let _ = model.select_at(index);
            }
        }
    }
}

/// `OnFocusedAction`: the keyboard moved the current item to `index`.
pub(super) fn on_focused(mode: ItemsViewSelectionMode, model: &SelectionModel, index: &IndexPath, ctrl: bool, shift: bool) {
    match mode {
        ItemsViewSelectionMode::None => {}
        ItemsViewSelectionMode::Single => {
            model.set_single_select(true);
            if !ctrl {
                let _ = model.select_at(index);
            }
        }
        ItemsViewSelectionMode::Multiple => {
            if shift {
                if let Some(anchor) = model.anchor_index() {
                    let _ = if is_selected(model, &anchor) {
                        model.select_range_from_anchor_to(index)
                    } else {
                        model.deselect_range_from_anchor_to(index)
                    };
                }
            }
        }
        ItemsViewSelectionMode::Extended => {
            if shift && ctrl {
                if model.anchor_index().is_some() {
                    let _ = model.select_range_from_anchor_to(index);
                }
            } else if shift {
                select_only_range_from_anchor(model, index);
            } else if !ctrl {
                model.clear_selection();
                let _ = model.select_at(index);
            }
        }
    }
}
