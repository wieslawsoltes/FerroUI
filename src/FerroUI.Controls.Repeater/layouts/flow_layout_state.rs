// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md in the containing module.

use super::{FlowLayoutAlgorithm, VirtualizingLayoutContext};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObject, FerroObjectImpl,
    Ref, Size,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[repr(C)]
pub struct FlowLayoutState {
    base: FerroObject,
    pub flow_algorithm: FlowLayoutAlgorithm,
    line_size_buffer: RefCell<[f64; 100]>,
    items_per_line_buffer: RefCell<[f64; 100]>,
    total_line_size: Cell<f64>,
    total_lines_measured: Cell<i32>,
    total_items_per_line: Cell<f64>,
    special_element_desired_size: Cell<Size>,
}
ferro_class!(FlowLayoutState: FerroObject);
ferro_class_info!(FlowLayoutState {
    new: FlowLayoutState::new
});
ferro_impl_classes!(FlowLayoutState: FerroObjectImpl);
impl FlowLayoutState {
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
            flow_algorithm: FlowLayoutAlgorithm::new(),
            line_size_buffer: RefCell::new([0.0; 100]),
            items_per_line_buffer: RefCell::new([0.0; 100]),
            total_line_size: Cell::new(0.0),
            total_lines_measured: Cell::new(0),
            total_items_per_line: Cell::new(0.0),
            special_element_desired_size: Cell::new(Size::default()),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn initialize_for_context(&self, context: &Ref<VirtualizingLayoutContext>) {
        self.flow_algorithm.initialize_for_context(context);
        context.set_layout_state(Some(Rc::new(self.to_ref())));
    }
    pub fn uninitialize_for_context(&self, context: &Ref<VirtualizingLayoutContext>) {
        self.flow_algorithm.uninitialize_for_context(context);
    }
    pub fn on_line_arranged(
        &self,
        start: i32,
        count: i32,
        size: f64,
        context: &Ref<VirtualizingLayoutContext>,
    ) {
        if self.total_lines_measured.get() == 0 || start + count != context.item_count() {
            let index = start as usize % 100;
            let previous_size = self.line_size_buffer.borrow()[index];
            let previous_count = self.items_per_line_buffer.borrow()[index];
            if previous_size == 0.0 {
                self.total_lines_measured
                    .set(self.total_lines_measured.get() + 1);
            }
            self.total_line_size
                .set(self.total_line_size.get() - previous_size + size);
            self.total_items_per_line
                .set(self.total_items_per_line.get() - previous_count + count as f64);
            self.line_size_buffer.borrow_mut()[index] = size;
            self.items_per_line_buffer.borrow_mut()[index] = count as f64;
        }
    }
    pub fn total_line_size(&self) -> f64 {
        self.total_line_size.get()
    }
    pub fn total_lines_measured(&self) -> i32 {
        self.total_lines_measured.get()
    }
    pub fn total_items_per_line(&self) -> f64 {
        self.total_items_per_line.get()
    }
    pub fn special_element_desired_size(&self) -> Size {
        self.special_element_desired_size.get()
    }
    pub fn set_special_element_desired_size(&self, value: Size) {
        self.special_element_desired_size.set(value);
    }
}
