//! `Selection/TreeDataGridRowSelectionInteraction.cs`: how a row selection handles the input of
//! the control: keyboard navigation and expansion, page up and down, pointer selection with ranges
//! and toggles, and type-ahead text search.

use super::{ITreeDataGridSelectionInteraction, IUntypedRowSelection};
use crate::primitives::{TreeDataGridRow, TreeDataGridRowsPresenter};
use crate::TreeDataGrid;
use ferroui_base::input::{
    KeyEventArgs, KeyModifiers, NavigationDirection, PointerEventArgs, PointerPressedEventArgs, PointerReleasedEventArgs,
    PointerType, PointerUpdateKind, TextInputEventArgs,
};
use ferroui_base::threading::Dispatcher;
use ferroui_base::utilities::HandlerList;
use ferroui_base::{BoxedValue, Point, Rect, Ref, Visual};
use ferroui_controls::presenters::ScrollContentPresenter;
use ferroui_controls::{Control, SelectionMode, TopLevel};
use ferroui_treedatagrid_core::models::RowRef;
use ferroui_treedatagrid_core::selection::{
    ITreeSelectionModel, TreeSelectionModelIndexesChangedEventArgs, TreeSelectionModelSelectionChangedEventArgs,
    TreeSelectionModelSourceResetEventArgs,
};
use ferroui_treedatagrid_core::{ITreeDataGridSource, IndexPath};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

// How the interaction follows the changes of its selection.
enum StateSubscription {
    // `StateChanged` of a row selection of the core.
    State(Rc<dyn IUntypedRowSelection>, u64),
    // The events of a tree selection whose row selection type is not known: the changes of the
    // selection, of its indexes, and resets of its source.
    Model { selection: u64, indexes: u64, reset: u64 },
}

/// The interaction of a row selection with the control (`TreeDataGridRowSelectionInteraction<TModel>`).
pub struct TreeDataGridRowSelectionInteraction {
    source: Rc<dyn ITreeDataGridSource>,
    selection: Rc<dyn ITreeSelectionModel>,
    subscription: RefCell<Option<StateSubscription>>,
    selection_changed: HandlerList<dyn Fn()>,
    // `s_InvalidPoint` is `None`.
    pressed_point: Cell<Option<Point>>,
    last_char_pressed_time: Cell<i64>,
    typed_word: RefCell<String>,
}

impl TreeDataGridRowSelectionInteraction {
    /// The interaction of the row selection `selection` of `source`, following its `StateChanged`.
    pub fn new(source: Rc<dyn ITreeDataGridSource>, selection: Rc<dyn IUntypedRowSelection>) -> Rc<Self> {
        let this = Self::create(source, selection.model());
        let weak = Rc::downgrade(&this);
        let token = selection.add_state_changed(Rc::new(move || {
            if let Some(this) = weak.upgrade() {
                this.on_state_changed();
            }
        }));
        *this.subscription.borrow_mut() = Some(StateSubscription::State(selection, token));
        this
    }

    /// The interaction of a tree selection of `source` whose row selection type is not known: it
    /// follows the selection's own events in place of `StateChanged`.
    pub fn with_model(source: Rc<dyn ITreeDataGridSource>, selection: Rc<dyn ITreeSelectionModel>) -> Rc<Self> {
        let this = Self::create(source, selection.clone());
        let changed = {
            let weak = Rc::downgrade(&this);
            move || {
                if let Some(this) = weak.upgrade() {
                    this.on_state_changed();
                }
            }
        };
        let c = changed.clone();
        let selection_token =
            selection.add_selection_changed(Rc::new(move |_: &dyn TreeSelectionModelSelectionChangedEventArgs| c()));
        let c = changed.clone();
        let indexes_token = selection.add_indexes_changed(Rc::new(move |_: &TreeSelectionModelIndexesChangedEventArgs| c()));
        let reset_token = selection.add_source_reset(Rc::new(move |_: &TreeSelectionModelSourceResetEventArgs| changed()));
        *this.subscription.borrow_mut() =
            Some(StateSubscription::Model { selection: selection_token, indexes: indexes_token, reset: reset_token });
        this
    }

    fn create(source: Rc<dyn ITreeDataGridSource>, selection: Rc<dyn ITreeSelectionModel>) -> Rc<Self> {
        Rc::new(Self {
            source,
            selection,
            subscription: RefCell::new(None),
            selection_changed: HandlerList::new(),
            pressed_point: Cell::new(None),
            last_char_pressed_time: Cell::new(0),
            typed_word: RefCell::new(String::new()),
        })
    }

    /// The tree selection model of the rows.
    pub fn selection(&self) -> &Rc<dyn ITreeSelectionModel> {
        &self.selection
    }

    /// The source of the rows.
    pub fn source(&self) -> &Rc<dyn ITreeDataGridSource> {
        &self.source
    }

    fn on_state_changed(&self) {
        for (_, handler) in self.selection_changed.snapshot().iter() {
            handler();
        }
    }

    /// Stops following the selection (`Dispose`).
    pub fn dispose(&self) {
        let subscription = self.subscription.borrow_mut().take();
        match subscription {
            Some(StateSubscription::State(selection, token)) => selection.remove_state_changed(token),
            Some(StateSubscription::Model { selection, indexes, reset }) => {
                self.selection.remove_selection_changed(selection);
                self.selection.remove_indexes_changed(indexes);
                self.selection.remove_source_reset(reset);
            }
            None => {}
        }
    }

    fn row_count(&self) -> i32 {
        self.source.rows().count() as i32
    }

    fn is_hierarchical(&self) -> bool {
        self.source.is_hierarchical()
    }

    fn get_row(&self, index: i32) -> RowRef {
        self.source.rows().get(index as usize)
    }

    fn model_index_to_row_index(&self, index: &IndexPath) -> i32 {
        self.source.rows().model_index_to_row_index(index)
    }

    fn row_index_to_model_index(&self, index: i32) -> IndexPath {
        self.source.rows().row_index_to_model_index(index)
    }

    fn single_select(&self) -> bool {
        self.selection.single_select()
    }

    fn count(&self) -> i32 {
        self.selection.count()
    }

    fn selected_index(&self) -> IndexPath {
        self.selection.selected_index()
    }

    fn set_selected_index(&self, value: IndexPath) {
        self.selection.set_selected_index(value);
    }

    fn anchor_index(&self) -> IndexPath {
        self.selection.anchor_index()
    }

    fn range_anchor_index(&self) -> IndexPath {
        self.selection.range_anchor_index()
    }

    fn is_selected(&self, index: &IndexPath) -> bool {
        self.selection.is_selected(index)
    }

    /// `HandleTextInput`: type-ahead search of the columns that search their text, from the row
    /// after `selected_row_index`.
    pub fn handle_text_input(&self, text: Option<&str>, tree_data_grid: &TreeDataGrid, selected_row_index: i32) {
        let Some(text) = text else {
            return;
        };
        let Some(columns) = tree_data_grid.columns() else {
            return;
        };
        let Some(typed_char) = text.to_uppercase().chars().next() else {
            return;
        };

        let now = Dispatcher::ui_thread().now();
        let mut time = 0;
        if self.last_char_pressed_time.get() > 0 {
            time = now - self.last_char_pressed_time.get();
        }

        let candidate_pattern = if time < 500 {
            let typed_word = self.typed_word.borrow().clone();
            let mut chars = typed_word.chars();
            if typed_word.chars().count() == 1 && chars.next() == Some(typed_char) {
                typed_word
            } else {
                format!("{typed_word}{typed_char}")
            }
        } else {
            typed_char.to_string()
        };

        for i in 0..columns.count() {
            let column = columns.get(i);
            let Some(cell_column) = column.as_cell_column() else {
                continue;
            };
            // An expander column searches the text of its inner column (`HierarchicalExpanderColumn<TModel>`
            // whose `Inner` is an `ITextSearchableColumn<TModel>`): its presentation forwards both
            // members to the inner column.
            if cell_column.is_text_search_enabled() {
                self.search(tree_data_grid, &candidate_pattern, selected_row_index, &column);
            }
        }
        self.last_char_pressed_time.set(now);
    }

    fn search(&self, tree_data_grid: &TreeDataGrid, candidate_pattern: &str, selected_row_index: i32, column: &crate::models::ColumnRef) {
        let mut found = false;
        let start = if candidate_pattern.chars().count() == 1 { selected_row_index + 1 } else { selected_row_index };
        let mut i = start.max(0);
        while i <= self.row_count() - 1 {
            found = self.search_and_select_row(tree_data_grid, candidate_pattern, i, self.get_row(i).model(), column);
            if found {
                break;
            }
            i += 1;
        }
        if !found {
            let mut i = 0;
            while i <= selected_row_index && i < self.row_count() {
                found = self.search_and_select_row(tree_data_grid, candidate_pattern, i, self.get_row(i).model(), column);
                if found {
                    break;
                }
                i += 1;
            }
        }
    }

    fn search_and_select_row(
        &self,
        tree_data_grid: &TreeDataGrid,
        candidate_pattern: &str,
        new_index: i32,
        model: Option<BoxedValue>,
        column: &crate::models::ColumnRef,
    ) -> bool {
        let (Some(model), Some(cell_column)) = (model, column.as_cell_column()) else {
            return false;
        };
        if let Some(value) = cell_column.select_value(&model) {
            if value.to_uppercase().starts_with(candidate_pattern) {
                self.update_selection(tree_data_grid, new_index, true, false, false, false);
                if let Some(presenter) = tree_data_grid.rows_presenter() {
                    presenter.bring_into_view(new_index, None);
                }
                *self.typed_word.borrow_mut() = candidate_pattern.to_string();
                return true;
            }
        }
        false
    }

    fn is_row_fully_visible_to_user(row: &TreeDataGridRow) -> bool {
        if let Some(scroll_content_presenter) = row.find_ancestor_of_type::<ScrollContentPresenter>(false) {
            if let Some(transform) = row.transform_to_visual(&scroll_content_presenter) {
                let transformed_bounds = Rect::from_size(row.bounds().size()).transform_to_aabb(transform);
                let bounds = scroll_content_presenter.bounds();
                if bounds.contains(transformed_bounds.top_left()) && bounds.contains(transformed_bounds.bottom_right()) {
                    return true;
                }
            }
        }
        false
    }

    fn get_row_index_if_fully_visible(control: Option<&Ref<Control>>) -> Option<i32> {
        let row = control?.cast::<TreeDataGridRow>()?;
        if Self::is_row_fully_visible_to_user(&row) {
            Some(row.row_index())
        } else {
            None
        }
    }

    fn pointer_select(&self, sender: &TreeDataGrid, row: &TreeDataGridRow, e: &PointerEventArgs) {
        let visual: &Visual = sender;
        let point = e.get_current_point(Some(visual));

        let command_modifiers =
            sender.get_platform_settings().map(|settings| settings.hotkey_configuration().command_modifiers);
        let toggle_modifier = command_modifiers.is_some_and(|modifiers| e.key_modifiers().contains(modifiers));
        let is_right_button = matches!(
            point.properties.pointer_update_kind,
            PointerUpdateKind::RightButtonPressed | PointerUpdateKind::RightButtonReleased
        );

        self.update_selection(
            sender,
            row.row_index(),
            true,
            e.key_modifiers().contains(KeyModifiers::SHIFT),
            toggle_modifier,
            is_right_button,
        );
        e.set_handled(true);
    }

    fn update_selection(
        &self,
        tree_data_grid: &TreeDataGrid,
        row_index: i32,
        select: bool,
        range_modifier: bool,
        toggle_modifier: bool,
        right_button: bool,
    ) {
        let model_index = self.row_index_to_model_index(row_index);

        if model_index == IndexPath::UNSELECTED {
            return;
        }

        let mode = if self.single_select() { SelectionMode::SINGLE } else { SelectionMode::MULTIPLE };
        let multi = mode.contains(SelectionMode::MULTIPLE);
        let toggle = toggle_modifier || mode.contains(SelectionMode::TOGGLE);
        let range = multi && range_modifier;

        if !select {
            if self.is_selected(&model_index) && !tree_data_grid.query_cancel_selection() {
                self.selection.deselect(&model_index);
            }
        } else if right_button {
            if !self.is_selected(&model_index) && !tree_data_grid.query_cancel_selection() {
                self.set_selected_index(model_index);
            }
        } else if range {
            if !tree_data_grid.query_cancel_selection() {
                let anchor = self.range_anchor_index();
                let mut i = self.model_index_to_row_index(&anchor).max(0);
                let step = if i < row_index { 1 } else { -1 };

                self.selection.begin_batch_update();
                self.selection.clear();
                loop {
                    let m = self.row_index_to_model_index(i);
                    self.selection.select(&m);
                    if i == row_index {
                        break;
                    }
                    i += step;
                }
                self.selection.end_batch_update();
            }
        } else if multi && toggle {
            if !tree_data_grid.query_cancel_selection() {
                if self.is_selected(&model_index) {
                    self.selection.deselect(&model_index);
                } else {
                    self.selection.select(&model_index);
                }
            }
        } else if toggle {
            if !tree_data_grid.query_cancel_selection() {
                let value = if self.selected_index() == model_index { IndexPath::new(-1) } else { model_index };
                self.set_selected_index(value);
            }
        } else if (self.selected_index() != model_index || self.count() > 1) && !tree_data_grid.query_cancel_selection() {
            self.set_selected_index(model_index);
        }
    }

    fn try_key_expand_collapse(&self, tree_data_grid: &TreeDataGrid, direction: NavigationDirection, focused: &TreeDataGridRow) -> bool {
        if tree_data_grid.rows_presenter().is_none() || focused.row_index() < 0 {
            return false;
        }

        let row = self.get_row(focused.row_index());

        if let Some(expander) = row.as_expander() {
            if direction == NavigationDirection::Right && !expander.is_expanded() {
                expander.set_is_expanded(true);
                return true;
            } else if direction == NavigationDirection::Left && expander.is_expanded() {
                expander.set_is_expanded(false);
                return true;
            }
        }

        false
    }

    fn move_selection(
        &self,
        tree_data_grid: &TreeDataGrid,
        direction: NavigationDirection,
        range_modifier: bool,
        focused: Option<&Ref<TreeDataGridRow>>,
    ) -> bool {
        if tree_data_grid.rows_presenter().is_none()
            || tree_data_grid.columns().is_some_and(|columns| columns.count() == 0)
            || self.row_count() == 0
        {
            return false;
        }

        let current_row_index = match focused {
            Some(focused) => focused.row_index(),
            None => self.model_index_to_row_index(&self.selected_index()),
        };

        let new_row_index = if direction == NavigationDirection::First || direction == NavigationDirection::Last {
            if direction == NavigationDirection::First {
                0
            } else {
                self.row_count() - 1
            }
        } else {
            let y = match direction {
                NavigationDirection::Up => -1,
                NavigationDirection::Down => 1,
                _ => 0,
            };
            (current_row_index + y).min(self.row_count() - 1).max(0)
        };

        if new_row_index != current_row_index {
            self.update_selection(tree_data_grid, new_row_index, true, range_modifier, false, false);
            if let Some(presenter) = tree_data_grid.rows_presenter() {
                presenter.bring_into_view(new_row_index, None);
            }
            Self::focus_row(tree_data_grid, tree_data_grid.try_get_row(new_row_index).map(|row| row.upcast::<Control>()));
            true
        } else {
            false
        }
    }

    fn focus_row(owner: &TreeDataGrid, control: Option<Ref<Control>>) {
        let Some(row) = owner.try_get_row_of(control.as_deref()) else {
            return;
        };
        let Some(cells_presenter) = row.cells_presenter() else {
            return;
        };

        // Get the column index of the currently focused cell if possible: we'll try to focus the
        // same column in the new row.
        let visual: &Visual = owner;
        let current_focus = TopLevel::get_top_level(Some(visual))
            .and_then(|top_level| top_level.focus_manager().get_focused_element())
            .and_then(|focused| focused.cast::<Control>());
        let new_cell = current_focus
            .and_then(|current_focus| owner.try_get_cell_of(Some(&*current_focus)))
            .and_then(|current_cell| row.try_get_cell(current_cell.column_index()))
            .filter(|new_cell| new_cell.focusable());

        if let Some(new_cell) = new_cell {
            new_cell.focus();
        } else {
            // Otherwise, just focus the first focusable cell in the row.
            for cell in cells_presenter.get_realized_elements() {
                if cell.focusable() {
                    cell.focus();
                    break;
                }
            }
        }
    }

    fn presenter(sender: &TreeDataGrid) -> Option<Ref<TreeDataGridRowsPresenter>> {
        sender.rows_presenter()
    }
}

impl ITreeDataGridSelectionInteraction for TreeDataGridRowSelectionInteraction {
    fn add_selection_changed(&self, handler: Rc<dyn Fn()>) -> u64 {
        self.selection_changed.add(handler)
    }

    fn remove_selection_changed(&self, token: u64) {
        self.selection_changed.remove(token);
    }

    fn is_row_selected(&self, row_model: &RowRef) -> bool {
        match row_model.as_model_indexable() {
            Some(indexable) => self.is_selected(&indexable.model_index_path()),
            None => false,
        }
    }

    fn is_row_selected_at(&self, row_index: i32) -> bool {
        if row_index >= 0 && row_index < self.row_count() {
            if let Some(indexable) = self.get_row(row_index).as_model_indexable() {
                return self.is_selected(&indexable.model_index_path());
            }
        }

        false
    }

    fn on_key_down(&self, sender: &TreeDataGrid, e: &KeyEventArgs) {
        let Some(rows_presenter) = Self::presenter(sender) else {
            return;
        };

        if e.handled() {
            return;
        }

        let direction = e.key.to_navigation_direction(KeyModifiers::NONE);
        let ctrl = e.key_modifiers.contains(KeyModifiers::CONTROL);
        let shift = e.key_modifiers.contains(KeyModifiers::SHIFT);

        let Some(direction) = direction else {
            return;
        };

        let anchor_row_index = self.model_index_to_row_index(&self.anchor_index());

        rows_presenter.bring_into_view(anchor_row_index, None);

        let anchor = sender.try_get_row(anchor_row_index);

        if let Some(anchor) = &anchor {
            if !ctrl {
                e.set_handled(self.try_key_expand_collapse(sender, direction, anchor));
            }
        }

        if !e.handled() && (!ctrl || shift) {
            e.set_handled(self.move_selection(sender, direction, shift, anchor.as_ref()));
        }

        if !e.handled() && direction == NavigationDirection::Left && self.is_hierarchical() && anchor_row_index > 0 {
            let new_index = self.model_index_to_row_index(&self.anchor_index().parent());
            self.update_selection(sender, new_index, true, false, false, false);
            let realized = rows_presenter.bring_into_view(new_index, None);
            Self::focus_row(sender, realized);
        }

        if !e.handled()
            && direction == NavigationDirection::Right
            && self.is_hierarchical()
            && anchor_row_index >= 0
            && self.get_row(anchor_row_index).as_expander().is_some_and(|expander| expander.is_expanded())
        {
            let new_index = anchor_row_index + 1;
            self.update_selection(sender, new_index, true, false, false, false);
            rows_presenter.bring_into_view(new_index, None);
        }
    }

    fn on_preview_key_down(&self, sender: &TreeDataGrid, e: &KeyEventArgs) {
        use ferroui_base::input::Key;

        let update_selection_and_bring_into_view = |new_index: i32| {
            self.update_selection(sender, new_index, true, false, false, false);
            let realized = sender.rows_presenter().and_then(|presenter| presenter.bring_into_view(new_index, None));
            Self::focus_row(sender, realized);
            e.set_handled(true);
        };

        if e.key != Key::PageDown && e.key != Key::PageUp {
            return;
        }
        let (Some(presenter), Some(rows)) = (sender.rows_presenter(), sender.rows()) else {
            return;
        };
        let items_count = rows.count() as i32;
        let children = presenter.realized_elements();
        let children_count = children.len() as i32;
        if children_count == 0 {
            return;
        }

        let mut new_index = 0;
        let mut is_index_set = false;
        let selected_index = rows.model_index_to_row_index(&self.selected_index());
        let selected_row_visible =
            || sender.try_get_row(selected_index).is_some_and(|row| Self::is_row_fully_visible_to_user(&row));

        if e.key == Key::PageDown {
            for i in (0..children_count).rev() {
                if let Some(index) = Self::get_row_index_if_fully_visible(children[i as usize].as_ref()) {
                    new_index = index;
                    is_index_set = true;
                    break;
                }
            }
            if is_index_set && selected_index != new_index && selected_row_visible() {
                update_selection_and_bring_into_view(new_index);
                return;
            } else if children_count + selected_index - 1 <= items_count {
                new_index = children_count + selected_index - 2;
            } else {
                new_index = items_count - 1;
            }
        } else if e.key == Key::PageUp {
            for i in 0..children_count {
                if let Some(index) = Self::get_row_index_if_fully_visible(children[i as usize].as_ref()) {
                    new_index = index;
                    is_index_set = true;
                    break;
                }
            }
            if is_index_set && selected_index != new_index && selected_row_visible() {
                update_selection_and_bring_into_view(new_index);
                return;
            } else if is_index_set && selected_index - children_count + 2 > 0 {
                new_index = selected_index - children_count + 2;
            } else {
                new_index = 0;
            }
        }
        update_selection_and_bring_into_view(new_index);
    }

    fn on_text_input(&self, sender: &TreeDataGrid, e: &TextInputEventArgs) {
        let anchor = self.model_index_to_row_index(&self.anchor_index());
        self.handle_text_input(e.text.as_deref(), sender, anchor);
    }

    fn on_pointer_pressed(&self, sender: &TreeDataGrid, e: &PointerPressedEventArgs) {
        // Select a row on pointer pressed if:
        //
        // - It's a mouse click, not touch: we don't want to select on touch scroll gesture start
        // - It's a pen secondary button press, we don't want to select on primary button scroll gesture start
        // - The row isn't already selected: we don't want to deselect an existing multiple selection
        //   if the user is trying to drag multiple rows
        //
        // Otherwise select on pointer release.
        let pointer_support_selection_on_press = match e.pointer().type_() {
            PointerType::Mouse => true,
            PointerType::Pen => e.get_current_point(None).properties.is_right_button_pressed,
            _ => false,
        };

        let row = if !e.handled() && pointer_support_selection_on_press {
            let source = e.source().and_then(|source| source.cast::<Control>());
            source.and_then(|source| sender.try_get_row_of(Some(&*source)))
        } else {
            None
        };
        let row = row.filter(|row| !self.is_selected(&self.row_index_to_model_index(row.row_index())));

        if let Some(row) = row {
            self.pointer_select(sender, &row, e);
            self.pressed_point.set(None);
        } else {
            let visual: &Visual = sender;
            self.pressed_point.set(Some(e.get_position(Some(visual))));
        }
    }

    fn on_pointer_released(&self, sender: &TreeDataGrid, e: &PointerReleasedEventArgs) {
        if e.handled() {
            return;
        }
        let Some(pressed_point) = self.pressed_point.get() else {
            return;
        };
        let source = e.source().and_then(|source| source.cast::<Control>());
        if let Some(row) = source.and_then(|source| sender.try_get_row_of(Some(&*source))) {
            let visual: &Visual = sender;
            let p = e.get_position(Some(visual));
            if (p.x - pressed_point.x).abs() <= 3.0 || (p.y - pressed_point.y).abs() <= 3.0 {
                self.pointer_select(sender, &row, e);
            }
        }
    }
}

impl Drop for TreeDataGridRowSelectionInteraction {
    fn drop(&mut self) {
        self.dispose();
    }
}
