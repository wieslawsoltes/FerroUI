// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md in the containing module.

use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LinedFlowLayoutError {
    ItemIndexOutOfBounds,
    RangeStartMustBeNonnegative,
    RangeStartCannotIncrease,
    ArrayDoesNotCoverRequestedRange,
    ArrayLengthsMustMatch,
    CannotShareLayout,
}
impl fmt::Display for LinedFlowLayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::ItemIndexOutOfBounds => "Item index is outside the items source",
            Self::RangeStartMustBeNonnegative => "ItemsRangeStartIndex must be nonnegative",
            Self::RangeStartCannotIncrease => "ItemsRangeStartIndex cannot be increased",
            Self::ArrayDoesNotCoverRequestedRange => {
                "The supplied array must cover the complete requested interval"
            }
            Self::ArrayLengthsMustMatch => "All provided arrays must have the same length",
            Self::CannotShareLayout => "LinedFlowLayout cannot be shared",
        })
    }
}
impl std::error::Error for LinedFlowLayoutError {}

struct ItemsInfoRequest {
    start: Cell<i32>,
    requested_start: i32,
    requested_length: i32,
    established_length: Cell<usize>,
    min_width: Cell<f64>,
    max_width: Cell<f64>,
    ratios: RefCell<Vec<f64>>,
    min_widths: RefCell<Vec<f64>>,
    max_widths: RefCell<Vec<f64>>,
    active: Cell<bool>,
}

/// Shared event arguments. Callers may extend a request backwards or return
/// information for the complete source, but every supplied array must cover
/// the original request and have the same length.
#[derive(Clone)]
pub struct LinedFlowLayoutItemsInfoRequestedEventArgs(Rc<ItemsInfoRequest>);
impl PartialEq for LinedFlowLayoutItemsInfoRequestedEventArgs {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl LinedFlowLayoutItemsInfoRequestedEventArgs {
    pub(crate) fn new(start: i32, length: i32) -> Self {
        Self(Rc::new(ItemsInfoRequest {
            start: Cell::new(start),
            requested_start: start,
            requested_length: length,
            established_length: Cell::new(0),
            min_width: Cell::new(-1.0),
            max_width: Cell::new(-1.0),
            ratios: RefCell::new(Vec::new()),
            min_widths: RefCell::new(Vec::new()),
            max_widths: RefCell::new(Vec::new()),
            active: Cell::new(true),
        }))
    }
    pub fn items_range_start_index(&self) -> i32 {
        self.0.start.get()
    }
    pub fn set_items_range_start_index(&self, value: i32) -> Result<(), LinedFlowLayoutError> {
        if value < 0 {
            return Err(LinedFlowLayoutError::RangeStartMustBeNonnegative);
        }
        if value > self.0.start.get() {
            return Err(LinedFlowLayoutError::RangeStartCannotIncrease);
        }
        if self.0.established_length.get() > 0
            && value as i64 + (self.0.established_length.get() as i64)
                < self.0.requested_start as i64 + self.0.requested_length as i64
        {
            return Err(LinedFlowLayoutError::ArrayDoesNotCoverRequestedRange);
        }
        self.0.start.set(value);
        Ok(())
    }
    pub fn items_range_requested_length(&self) -> i32 {
        self.0.requested_length
    }
    pub fn min_width(&self) -> f64 {
        self.0.min_width.get()
    }
    pub fn set_min_width(&self, value: f64) {
        self.0.min_width.set(if value < 0.0 { -1.0 } else { value });
    }
    pub fn max_width(&self) -> f64 {
        self.0.max_width.get()
    }
    pub fn set_max_width(&self, value: f64) {
        self.0.max_width.set(if value < 0.0 { -1.0 } else { value });
    }
    fn establish_length(&self, length: usize) -> Result<(), LinedFlowLayoutError> {
        if self.0.established_length.get() == length {
            return Ok(());
        }
        if self.0.start.get() as i64 + (length as i64)
            < self.0.requested_start as i64 + self.0.requested_length as i64
        {
            return Err(LinedFlowLayoutError::ArrayDoesNotCoverRequestedRange);
        }
        if self.0.established_length.get() > 0 {
            return Err(LinedFlowLayoutError::ArrayLengthsMustMatch);
        }
        self.0.established_length.set(length);
        Ok(())
    }
    pub fn set_desired_aspect_ratios(&self, values: &[f64]) -> Result<(), LinedFlowLayoutError> {
        if self.0.active.get() {
            self.establish_length(values.len())?;
            *self.0.ratios.borrow_mut() = values.to_vec();
        }
        Ok(())
    }
    pub fn set_min_widths(&self, values: &[f64]) -> Result<(), LinedFlowLayoutError> {
        if self.0.active.get() {
            self.establish_length(values.len())?;
            *self.0.min_widths.borrow_mut() = values.to_vec();
        }
        Ok(())
    }
    pub fn set_max_widths(&self, values: &[f64]) -> Result<(), LinedFlowLayoutError> {
        if self.0.active.get() {
            self.establish_length(values.len())?;
            *self.0.max_widths.borrow_mut() = values.to_vec();
        }
        Ok(())
    }
    pub(crate) fn complete(&self) -> LinedFlowItemsInfo {
        self.0.active.set(false);
        LinedFlowItemsInfo {
            start: self.0.start.get(),
            ratios: std::mem::take(&mut *self.0.ratios.borrow_mut()),
            min_widths: std::mem::take(&mut *self.0.min_widths.borrow_mut()),
            max_widths: std::mem::take(&mut *self.0.max_widths.borrow_mut()),
            min_width: self.0.min_width.get(),
            max_width: self.0.max_width.get(),
        }
    }
}

#[derive(Clone)]
pub(crate) struct LinedFlowItemsInfo {
    pub start: i32,
    pub ratios: Vec<f64>,
    pub min_widths: Vec<f64>,
    pub max_widths: Vec<f64>,
    pub min_width: f64,
    pub max_width: f64,
}
impl Default for LinedFlowItemsInfo {
    fn default() -> Self {
        Self {
            start: -1,
            ratios: Vec::new(),
            min_widths: Vec::new(),
            max_widths: Vec::new(),
            min_width: -1.0,
            max_width: -1.0,
        }
    }
}
impl LinedFlowItemsInfo {
    pub fn end(&self) -> i32 {
        self.start + self.ratios.len() as i32
    }
    pub fn contains(&self, index: i32) -> bool {
        self.start >= 0 && index >= self.start && index < self.end()
    }
    pub fn minimum(&self, index: i32) -> f64 {
        self.min_widths
            .get((index - self.start) as usize)
            .copied()
            .map(|value| value.max(self.min_width))
            .unwrap_or(self.min_width)
    }
    pub fn maximum(&self, index: i32) -> f64 {
        self.max_widths
            .get((index - self.start) as usize)
            .copied()
            .map(|value| value.min(self.max_width))
            .unwrap_or(self.max_width)
    }
}
