use super::{IServerClockItem, ServerObjectAnimations};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

/// The clock items and the dirty animated objects of a server compositor.
#[derive(Default)]
pub struct ServerCompositorAnimations {
    clock_items: RefCell<Vec<Rc<dyn IServerClockItem>>>,
    clock_items_to_update: RefCell<Vec<Rc<dyn IServerClockItem>>>,
    dirty_animated_object_queue: RefCell<VecDeque<Rc<ServerObjectAnimations>>>,
}

fn same_item(a: &Rc<dyn IServerClockItem>, b: &Rc<dyn IServerClockItem>) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b))
}

impl ServerCompositorAnimations {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_to_clock(&self, item: Rc<dyn IServerClockItem>) {
        let mut items = self.clock_items.borrow_mut();
        if !items.iter().any(|i| same_item(i, &item)) {
            items.push(item);
        }
    }

    pub fn remove_from_clock(&self, item: &Rc<dyn IServerClockItem>) {
        self.clock_items.borrow_mut().retain(|i| !same_item(i, item));
    }

    pub fn process(&self) {
        let mut to_update = std::mem::take(&mut *self.clock_items_to_update.borrow_mut());
        to_update.extend(self.clock_items.borrow().iter().cloned());
        for animation in &to_update {
            animation.on_tick();
        }
        to_update.clear();
        *self.clock_items_to_update.borrow_mut() = to_update;

        loop {
            let Some(animation) = self.dirty_animated_object_queue.borrow_mut().pop_front() else { break };
            animation.evaluate_animations();
        }
    }

    pub fn need_next_tick(&self) -> bool {
        !self.clock_items.borrow().is_empty()
    }

    pub fn add_dirty_animated_object(&self, obj: Rc<ServerObjectAnimations>) {
        let mut queue = self.dirty_animated_object_queue.borrow_mut();
        if !queue.iter().any(|o| Rc::ptr_eq(o, &obj)) {
            queue.push_back(obj);
        }
    }
}
