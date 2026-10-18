//! `Selection/TreeDataGridCellSelectionInteraction.cs`: the view input and visible-index mapping of
//! a cell selection of the core.

use super::{ITreeDataGridSelectionInteraction, IUntypedCellSelection};
use crate::TreeDataGrid;
use ferroui_base::input::{
    Key, KeyEventArgs, KeyModifiers, PointerEventArgs, PointerPressedEventArgs, PointerReleasedEventArgs, PointerType,
    PointerUpdateKind,
};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{Point, Visual};
use ferroui_controls::Control;
use ferroui_treedatagrid_core::{CellIndex, ITreeDataGridSource};
use std::cell::Cell;
use std::rc::Rc;

/// The interaction of a cell selection with the control (`TreeDataGridCellSelectionInteraction<TModel>`).
pub struct TreeDataGridCellSelectionInteraction {
    source: Rc<dyn ITreeDataGridSource>,
    selection: Rc<dyn IUntypedCellSelection>,
    token: Cell<Option<u64>>,
    selection_changed: HandlerList<dyn Fn()>,
    pressed_point: Cell<Option<Point>>,
}

impl TreeDataGridCellSelectionInteraction {
    /// The interaction of the cell selection `selection` of `source`.
    pub fn new(source: Rc<dyn ITreeDataGridSource>, selection: Rc<dyn IUntypedCellSelection>) -> Rc<Self> {
        let this = Rc::new(Self {
            source,
            selection: selection.clone(),
            token: Cell::new(None),
            selection_changed: HandlerList::new(),
            pressed_point: Cell::new(None),
        });
        let weak = Rc::downgrade(&this);
        let token = selection.add_selection_changed(Rc::new(move || {
            if let Some(this) = weak.upgrade() {
                this.on_selection_changed();
            }
        }));
        this.token.set(Some(token));
        this
    }

    /// The cell selection.
    pub fn selection(&self) -> &Rc<dyn IUntypedCellSelection> {
        &self.selection
    }

    fn on_selection_changed(&self) {
        for (_, handler) in self.selection_changed.snapshot().iter() {
            handler();
        }
    }

    /// Stops following the selection (`Dispose`).
    pub fn dispose(&self) {
        if let Some(token) = self.token.take() {
            self.selection.remove_selection_changed(token);
        }
    }

    fn row_count(&self) -> i32 {
        self.source.rows().count() as i32
    }

    fn pointer_select(&self, sender: &TreeDataGrid, column: i32, row: i32, e: &PointerEventArgs) {
        let Some(target) = self.try_get_index(column, row) else {
            return;
        };
        let visual: &Visual = sender;
        let right_button = matches!(
            e.get_current_point(Some(visual)).properties.pointer_update_kind,
            PointerUpdateKind::RightButtonPressed | PointerUpdateKind::RightButtonReleased
        );
        if (!right_button || !self.selection.is_selected(&target)) && !sender.query_cancel_selection() {
            self.select(
                target,
                !right_button && e.key_modifiers().contains(KeyModifiers::SHIFT) && !self.selection.single_select(),
            );
        }
        e.set_handled(true);
    }

    fn select(&self, target: CellIndex, range: bool) {
        let rows = self.source.rows();
        let anchor = self.selection.anchor_index();
        let anchor_row = rows.model_index_to_row_index(&anchor.row_index);
        if !range || anchor.column_index < 0 || anchor_row < 0 {
            self.selection.set_selected_index(target);
        } else {
            let row = rows.model_index_to_row_index(&target.row_index);
            self.selection.set_selected_range(
                &anchor,
                Self::inclusive_count(target.column_index - anchor.column_index),
                Self::inclusive_count(row - anchor_row),
            );
        }
    }

    fn inclusive_count(delta: i32) -> i32 {
        if delta >= 0 {
            delta + 1
        } else {
            delta - 1
        }
    }

    fn try_get_index(&self, column: i32, row: i32) -> Option<CellIndex> {
        let model_column = self.model_column_index(column);
        if model_column < 0 || row < 0 || row >= self.row_count() {
            return None;
        }
        Some(CellIndex::new(model_column, self.source.rows().row_index_to_model_index(row)))
    }

    /// The index in the source's columns of the visible column at `visible`.
    fn model_column_index(&self, visible: i32) -> i32 {
        if visible < 0 {
            return -1;
        }
        let mut visible = visible;
        for i in 0..self.source.column_count() {
            if self.source.column(i).is_visible() {
                if visible == 0 {
                    return i as i32;
                }
                visible -= 1;
            }
        }
        -1
    }

    /// The visible index of the column at `model` in the source's columns.
    fn visible_column_index(&self, model: i32) -> i32 {
        if model < 0 || model as usize >= self.source.column_count() || !self.source.column(model as usize).is_visible() {
            return -1;
        }
        (0..model as usize).filter(|&i| self.source.column(i).is_visible()).count() as i32
    }
}

impl ITreeDataGridSelectionInteraction for TreeDataGridCellSelectionInteraction {
    fn add_selection_changed(&self, handler: Rc<dyn Fn()>) -> u64 {
        self.selection_changed.add(handler)
    }

    fn remove_selection_changed(&self, token: u64) {
        self.selection_changed.remove(token);
    }

    fn is_cell_selected(&self, column_index: i32, row_index: i32) -> bool {
        self.try_get_index(column_index, row_index).is_some_and(|index| self.selection.is_selected(&index))
    }

    fn on_key_down(&self, sender: &TreeDataGrid, e: &KeyEventArgs) {
        if e.handled() || self.row_count() == 0 {
            return;
        }
        let Some(rows_presenter) = sender.rows_presenter() else {
            return;
        };
        let Some(columns) = sender.columns().filter(|columns| columns.count() > 0) else {
            return;
        };
        let (dx, dy) = match e.key {
            Key::Up => (0, -1),
            Key::Down => (0, 1),
            Key::Left => (-1, 0),
            Key::Right => (1, 0),
            _ => (0, 0),
        };
        if dx == 0 && dy == 0 {
            return;
        }
        let shift = e.key_modifiers.contains(KeyModifiers::SHIFT) && !self.selection.single_select();
        let current = if shift { self.selection.range_anchor_index() } else { self.selection.selected_index() };
        let column = self.visible_column_index(current.column_index);
        let row = self.source.rows().model_index_to_row_index(&current.row_index);
        let next_column = (if column < 0 { 0 } else { column + dx }).clamp(0, columns.count() as i32 - 1);
        let next_row = (if row < 0 { 0 } else { row + dy }).clamp(0, self.row_count() - 1);
        let Some(target) = self.try_get_index(next_column, next_row) else {
            return;
        };
        if sender.query_cancel_selection() {
            return;
        }
        self.select(target, shift);
        if let Some(headers) = sender.column_headers_presenter() {
            headers.bring_into_view(next_column, None);
        }
        let realized = rows_presenter.bring_into_view(next_row, None);
        if let Some(cell) = realized
            .and_then(|realized| realized.cast::<crate::primitives::TreeDataGridRow>())
            .and_then(|row| row.try_get_cell(next_column))
        {
            cell.focus();
        }
        e.set_handled(true);
    }

    fn on_pointer_pressed(&self, sender: &TreeDataGrid, e: &PointerPressedEventArgs) {
        self.pressed_point.set(None);
        let visual: &Visual = sender;
        let select_on_press = e.pointer().type_() == PointerType::Mouse
            || (e.pointer().type_() == PointerType::Pen && e.get_current_point(Some(visual)).properties.is_right_button_pressed);
        let cell = if !e.handled() && select_on_press {
            e.source()
                .and_then(|source| source.cast::<Control>())
                .and_then(|control| sender.try_get_cell_of(Some(&*control)))
                .filter(|cell| !self.is_cell_selected(cell.column_index(), cell.row_index()))
        } else {
            None
        };
        if let Some(cell) = cell {
            self.pointer_select(sender, cell.column_index(), cell.row_index(), e);
        } else if !e.handled() {
            self.pressed_point.set(Some(e.get_position(Some(visual))));
        }
    }

    fn on_pointer_released(&self, sender: &TreeDataGrid, e: &PointerReleasedEventArgs) {
        let pressed = self.pressed_point.take();
        if e.handled() {
            return;
        }
        let Some(start) = pressed else {
            return;
        };
        let cell = e.source().and_then(|source| source.cast::<Control>()).and_then(|control| sender.try_get_cell_of(Some(&*control)));
        if let Some(cell) = cell {
            let visual: &Visual = sender;
            let end = e.get_position(Some(visual));
            if (end.x - start.x).abs() <= 3.0 && (end.y - start.y).abs() <= 3.0 {
                self.pointer_select(sender, cell.column_index(), cell.row_index(), e);
            }
        }
    }
}

impl Drop for TreeDataGridCellSelectionInteraction {
    fn drop(&mut self) {
        self.dispose();
    }
}
