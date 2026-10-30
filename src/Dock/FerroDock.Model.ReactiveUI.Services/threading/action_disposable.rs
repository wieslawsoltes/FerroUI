// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use ferroui_system::IDisposable;
use std::cell::RefCell;
use std::rc::Rc;

pub(crate) struct ActionDisposable {
    dispose: RefCell<Option<Box<dyn FnOnce()>>>,
}

impl ActionDisposable {
    pub(crate) fn new(dispose: impl FnOnce() + 'static) -> Rc<Self> {
        Rc::new(Self {
            dispose: RefCell::new(Some(Box::new(dispose))),
        })
    }
}

impl IDisposable for ActionDisposable {
    fn dispose(&self) {
        let action = self.dispose.borrow_mut().take();
        if let Some(action) = action {
            action();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn dispose_runs_the_action_once_across_all_handles() {
        let count = Rc::new(Cell::new(0));
        let captured = count.clone();
        let disposable = ActionDisposable::new(move || captured.set(captured.get() + 1));
        let other = disposable.clone();
        disposable.dispose();
        other.dispose();
        assert_eq!(count.get(), 1);
    }

    #[test]
    fn dropping_a_handle_does_not_dispose_it() {
        let count = Rc::new(Cell::new(0));
        let captured = count.clone();
        drop(ActionDisposable::new(move || captured.set(1)));
        assert_eq!(count.get(), 0);
    }

    #[test]
    fn an_action_can_reenter_dispose_without_borrowing_or_running_twice() {
        let handle = Rc::new(RefCell::new(None::<Rc<ActionDisposable>>));
        let captured = handle.clone();
        let disposable =
            ActionDisposable::new(move || captured.borrow().as_ref().unwrap().dispose());
        *handle.borrow_mut() = Some(disposable.clone());
        disposable.dispose();
        assert!(disposable.dispose.borrow().is_none());
    }

    #[test]
    fn a_panicking_action_is_still_removed_and_its_captures_are_released() {
        let captured = Rc::new(());
        let weak = Rc::downgrade(&captured);
        let disposable = ActionDisposable::new(move || {
            let _keep_alive = captured;
            panic!("disposal failed");
        });
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| disposable.dispose()))
                .is_err()
        );
        disposable.dispose();
        assert!(weak.upgrade().is_none());
    }
}
