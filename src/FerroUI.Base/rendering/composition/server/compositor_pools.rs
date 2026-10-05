use super::{IDirtyRectCollector, TreeWalkerFrame};
use crate::platform::LtrbRect;
use crate::Matrix;
use std::cell::RefCell;
use std::rc::Rc;

/// A pool of stacks, so that tree walks do not allocate.
pub struct StackPool<T> {
    stacks: RefCell<Vec<Vec<T>>>,
}

impl<T> Default for StackPool<T> {
    fn default() -> Self {
        Self { stacks: RefCell::new(Vec::new()) }
    }
}

impl<T> StackPool<T> {
    pub fn rent(&self) -> Vec<T> {
        self.stacks.borrow_mut().pop().unwrap_or_default()
    }

    pub fn return_stack(&self, mut stack: Vec<T>) {
        stack.clear();
        self.stacks.borrow_mut().push(stack);
    }
}

/// The object pools of a server compositor.
#[derive(Default)]
pub struct CompositorPools {
    pub tree_walker_frame_stack_pool: StackPool<TreeWalkerFrame>,
    pub matrix_stack_pool: StackPool<Matrix>,
    pub ltrb_rect_stack_pool: StackPool<LtrbRect>,
    pub double_stack_pool: StackPool<f64>,
    pub int_stack_pool: StackPool<i32>,
    pub dirty_rect_collector_stack_pool: StackPool<Rc<dyn IDirtyRectCollector>>,
}
