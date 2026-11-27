//! The occupancy map of WinUI's `VariableSizedWrapGrid` (`dxaml/xcp/dxaml/lib/
//! VariableSizedWrapGrid_OccupancyMap.cpp` of microsoft-ui-xaml, commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md).
//!
//! The map always fills its cells row by row inside a column, whatever the orientation of the
//! panel: its rows are the cells of a line (the direction the panel grows in first) and its
//! columns are the lines. A cell holds the index of its element plus one, so zero is a free cell.
//! WinUI allocates the columns in blocks of 50 as they are reached; here each column is
//! allocated when an element is first written to it, and a column that was never written reads
//! as free.

use ferroui_base::input::NavigationDirection;
use ferroui_base::{Rect, Size};

/// Where an element's top left cell is in the map.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct MapLocation {
    row: usize,
    column: usize,
}

/// The axis a navigation moves the current cell along.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Axis {
    Row,
    Column,
}

/// The cells of a `VariableSizedWrapGrid` and where each element is (WinUI's
/// `VariableSizedWrapGrid::OccupancyMap` with its `OccupancyBlock`).
#[derive(Debug, Default)]
pub(crate) struct OccupancyMap {
    max_row: usize,
    max_column: usize,
    max_used_row: usize,
    max_used_column: usize,
    is_horizontal_orientation: bool,
    current_row: usize,
    current_column: usize,
    item_size: Size,
    columns: Vec<Option<Box<[u32]>>>,
    locations: Vec<MapLocation>,
    element_count: usize,
}

impl OccupancyMap {
    /// `CreateOccupancyMap`: an empty map of `max_row` by `max_column` cells for `item_count`
    /// elements.
    pub(crate) fn new(
        max_row: usize,
        max_column: usize,
        is_horizontal_orientation: bool,
        item_size: Size,
        item_count: usize,
    ) -> Self {
        Self {
            max_row,
            max_column,
            is_horizontal_orientation,
            item_size,
            columns: Vec::new(),
            locations: vec![MapLocation::default(); item_count],
            element_count: item_count,
            ..Self::default()
        }
    }

    /// The number of elements the map placed (`UpdateElementCount` when it is full).
    pub(crate) fn element_count(&self) -> usize {
        self.element_count
    }

    pub(crate) fn update_element_count(&mut self, element_count: usize) {
        self.element_count = element_count;
    }

    fn get_item(&self, row: usize, column: usize) -> u32 {
        self.columns
            .get(column)
            .and_then(|cells| cells.as_ref())
            .map_or(0, |cells| cells[row])
    }

    fn set_item(&mut self, row: usize, column: usize, element: u32) {
        if self.columns.len() <= column {
            self.columns.resize_with(column + 1, || None);
        }
        let max_row = self.max_row;
        self.columns[column].get_or_insert_with(|| vec![0; max_row].into_boxed_slice())[row] = element;
    }

    fn is_occupied(&self, row: usize, column: usize) -> bool {
        self.get_item(row, column) != 0
    }

    /// `SetCurrentItem`: makes the element's top left cell the current cell; false (and the
    /// first cell current) when the map did not place the element.
    pub(crate) fn set_current_item(&mut self, element_index: usize) -> bool {
        self.current_column = 0;
        self.current_row = 0;
        if element_index < self.element_count {
            let location = self.locations[element_index];
            self.current_column = location.column;
            self.current_row = location.row;
            return true;
        }
        false
    }

    /// `GetCurrentItemRect`: the bounds of the element at the current cell, from the cells it
    /// fills around it.
    pub(crate) fn current_item_rect(&self) -> Rect {
        let value = self.get_item(self.current_row, self.current_column);
        let row = self.current_row;
        let column = self.current_column;

        let mut column_left = column;
        while column_left > 0 && self.get_item(row, column_left - 1) == value {
            column_left -= 1;
        }
        let mut column_right = column;
        while column_right + 1 < self.max_column && self.get_item(row, column_right + 1) == value {
            column_right += 1;
        }
        let mut row_top = row;
        while row_top > 0 && self.get_item(row_top - 1, column) == value {
            row_top -= 1;
        }
        let mut row_bottom = row;
        while row_bottom + 1 < self.max_row && self.get_item(row_bottom + 1, column) == value {
            row_bottom += 1;
        }

        let column_span = (column_right - column_left + 1) as f64;
        let row_span = (row_bottom - row_top + 1) as f64;
        let (width, height) = (self.item_size.width, self.item_size.height);
        if self.is_horizontal_orientation {
            Rect::new(
                width * row as f64,
                height * column as f64,
                width * row_span,
                height * column_span,
            )
        } else {
            Rect::new(
                width * column as f64,
                height * row as f64,
                width * column_span,
                height * row_span,
            )
        }
    }

    /// `FillCurrentItemRect`: writes the element into the cells of its spans from the current
    /// cell (the cells outside the map are left out) and moves the current cell after it.
    fn fill_current_item_rect(&mut self, row_span: usize, column_span: usize, element: u32) {
        for column in 0..column_span {
            for row in 0..row_span {
                if self.current_column + column < self.max_column && self.current_row + row < self.max_row {
                    self.set_item(self.current_row + row, self.current_column + column, element);
                }
            }
        }
        self.locations[element as usize - 1] = MapLocation {
            row: self.current_row,
            column: self.current_column,
        };
        self.current_row += row_span;
        if self.current_row >= self.max_row {
            self.current_row = 0;
            self.current_column += 1;
        }
    }

    /// `FindAndFillNextAvailableRect`: places the element (`element` is its index plus one) at
    /// the next free cells that fit its spans; false when the map is full.
    pub(crate) fn find_and_fill_next_available_rect(
        &mut self,
        row_span: usize,
        column_span: usize,
        element: u32,
    ) -> bool {
        let (row_span, column_span) = if self.is_horizontal_orientation {
            (column_span, row_span)
        } else {
            (row_span, column_span)
        };
        if self.find_next_available_rect(row_span, column_span) {
            return false;
        }
        self.fill_current_item_rect(row_span, column_span, element);
        true
    }

    /// `FindNextAvailableRectInternal`: moves the current cell to the next one from which the
    /// spans are free; true when there is none (the map is full, the first cell is current).
    fn find_next_available_rect(&mut self, row_span: usize, column_span: usize) -> bool {
        let row_span = row_span.min(self.max_row);
        let mut column_span = column_span.min(self.max_column);
        while self.current_column < self.max_column {
            let mut is_occupied = true;
            if self.current_row == self.max_row {
                self.current_row = 0;
                self.current_column += 1;
            }
            for column in 0..column_span {
                // A span past the last column is cut to the columns left, for this cell and the
                // ones after it.
                if column + self.current_column >= self.max_column {
                    column_span = self.max_column - self.current_column;
                    break;
                }
                for row in 0..row_span {
                    is_occupied = true;
                    if self.current_row + row < self.max_row && self.current_column + column < self.max_column {
                        is_occupied = self.is_occupied(self.current_row + row, self.current_column + column);
                    }
                    if is_occupied {
                        break;
                    }
                }
                if is_occupied {
                    break;
                }
            }
            if !is_occupied {
                break;
            }
            self.current_row += 1;
            if self.current_row == self.max_row || self.current_row + row_span > self.max_row {
                self.current_row = 0;
                self.current_column += 1;
            }
        }
        if self.current_column >= self.max_column || self.current_row >= self.max_row {
            self.current_column = 0;
            self.current_row = 0;
            return true;
        }
        false
    }

    /// `FindUsedGridSize`: the number of columns and of rows with an occupied cell, at least one
    /// each (the lines and the cells per line of the panel).
    pub(crate) fn find_used_grid_size(&mut self) -> (usize, usize) {
        let mut last_column = 0;
        for column in 0..self.max_column {
            if (0..self.max_row).any(|row| self.is_occupied(row, column)) {
                last_column = column;
            }
        }
        let mut last_row = 0;
        for row in 0..self.max_row {
            if (0..self.max_column).any(|column| self.is_occupied(row, column)) {
                last_row = row;
            }
        }
        self.max_used_row = last_row + 1;
        self.max_used_column = last_column + 1;
        (self.max_used_column, self.max_used_row)
    }

    /// `GetTargetIndexFromNavigationAction`: the index of the element the navigation from the
    /// element `source_index` reaches, or `None` when there is none in that direction. The current
    /// cell is remembered between calls, so moving across a large tile and back returns to the
    /// line it was entered from. `Next`, `Previous`, `PageUp` and `PageDown` are not handled
    /// here (WinUI leaves the first two to the selector).
    pub(crate) fn target_index(&mut self, source_index: usize, direction: NavigationDirection) -> Option<usize> {
        if source_index >= self.element_count {
            return None;
        }
        if self.get_item(self.current_row, self.current_column) as usize != source_index + 1 {
            let location = self.locations[source_index];
            self.current_row = location.row;
            self.current_column = location.column;
        }
        let horizontal = self.is_horizontal_orientation;
        let valid = match direction {
            NavigationDirection::First => {
                self.current_row = 0;
                self.current_column = 0;
                true
            }
            NavigationDirection::Last => {
                let location = self.locations[self.element_count - 1];
                self.current_row = location.row;
                self.current_column = location.column;
                true
            }
            NavigationDirection::Left => self.move_up_or_left(if horizontal { Axis::Row } else { Axis::Column }),
            NavigationDirection::Up => self.move_up_or_left(if horizontal { Axis::Column } else { Axis::Row }),
            NavigationDirection::Right => {
                if horizontal {
                    self.move_down_or_right(Axis::Row, self.max_used_row)
                } else {
                    self.move_down_or_right(Axis::Column, self.max_used_column)
                }
            }
            NavigationDirection::Down => {
                if horizontal {
                    self.move_down_or_right(Axis::Column, self.max_used_column)
                } else {
                    self.move_down_or_right(Axis::Row, self.max_used_row)
                }
            }
            _ => false,
        };
        if !valid {
            return None;
        }
        let element = self.get_item(self.current_row, self.current_column);
        (element != 0).then(|| element as usize - 1)
    }

    fn index_mut(&mut self, axis: Axis) -> &mut usize {
        match axis {
            Axis::Row => &mut self.current_row,
            Axis::Column => &mut self.current_column,
        }
    }

    /// `MoveUpOrLeft`: moves the current cell back along the axis to the first cell of another
    /// element.
    fn move_up_or_left(&mut self, axis: Axis) -> bool {
        let element = self.get_item(self.current_row, self.current_column);
        let mut current = element;
        while current == element || current == 0 {
            let index = self.index_mut(axis);
            if *index == 0 {
                return false;
            }
            *index -= 1;
            current = self.get_item(self.current_row, self.current_column);
        }
        true
    }

    /// `MoveDownOrRight`: moves the current cell forward along the axis, within `max_count`
    /// cells, to the first cell of another element.
    fn move_down_or_right(&mut self, axis: Axis, max_count: usize) -> bool {
        let element = self.get_item(self.current_row, self.current_column);
        let mut current = element;
        while current == element || current == 0 {
            let index = self.index_mut(axis);
            if *index + 1 >= max_count {
                return false;
            }
            *index += 1;
            current = self.get_item(self.current_row, self.current_column);
        }
        true
    }
}
