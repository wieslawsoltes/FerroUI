// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md in the containing module.

use super::{FlowLayoutAlgorithm, VirtualizingLayoutContext};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObject, FerroObjectImpl,
    Ref,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[repr(C)]
pub struct StackLayoutState {
    base: FerroObject,
    pub flow_algorithm: FlowLayoutAlgorithm,
    estimation_buffer: RefCell<[f64; 100]>,
    last_element_size: Cell<f64>,
    total_element_size: Cell<f64>,
    max_arrange_bounds: Cell<f64>,
    are_elements_measured_regular: Cell<bool>,
    total_elements_measured: Cell<i32>,
}
ferro_class!(StackLayoutState: FerroObject);
ferro_class_info!(StackLayoutState {
    new: StackLayoutState::new
});
ferro_impl_classes!(StackLayoutState: FerroObjectImpl);
impl StackLayoutState {
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
            flow_algorithm: FlowLayoutAlgorithm::new(),
            estimation_buffer: RefCell::new([0.0; 100]),
            last_element_size: Cell::new(0.0),
            total_element_size: Cell::new(0.0),
            max_arrange_bounds: Cell::new(0.0),
            are_elements_measured_regular: Cell::new(true),
            total_elements_measured: Cell::new(0),
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
    pub fn on_element_measured(&self, index: i32, major: f64, minor: f64) {
        let slot = index as usize % 100;
        let previous = self.estimation_buffer.borrow()[slot];
        if previous == 0.0 {
            self.total_elements_measured
                .set(self.total_elements_measured.get() + 1);
        }
        self.total_element_size
            .set(self.total_element_size.get() - previous + major);
        let last = self.last_element_size.replace(major);
        self.estimation_buffer.borrow_mut()[slot] = major;
        if self.are_elements_measured_regular.get() && last != 0.0 && last != major {
            self.are_elements_measured_regular.set(false);
        }
        self.max_arrange_bounds
            .set(self.max_arrange_bounds.get().max(minor));
    }
    pub fn on_measure_start(&self) {
        self.max_arrange_bounds.set(0.0);
    }
    pub fn on_element_sizes_reset(&self) {
        self.are_elements_measured_regular.set(true);
        self.total_elements_measured.set(0);
        self.total_element_size.set(0.0);
        self.last_element_size.set(0.0);
        *self.estimation_buffer.borrow_mut() = [0.0; 100];
    }
    pub fn total_element_size(&self) -> f64 {
        self.total_element_size.get()
    }
    pub fn max_arrange_bounds(&self) -> f64 {
        self.max_arrange_bounds.get()
    }
    pub fn are_elements_measured_regular(&self) -> bool {
        self.are_elements_measured_regular.get()
    }
    pub fn total_elements_measured(&self) -> i32 {
        self.total_elements_measured.get()
    }
}
