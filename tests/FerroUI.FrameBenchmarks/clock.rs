//! The frame index every scene follows.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

ferroui_base::ferro_thread_local! {
    static FRAME: Cell<i32> = const { Cell::new(0) };
    static FOLLOWERS: RefCell<Vec<Rc<dyn Fn(i32)>>> = const { RefCell::new(Vec::new()) };
}

/// The frame index: advanced once per animation frame by the [`FrameHost`](crate::FrameHost).
pub struct Clock;

impl Clock {
    pub fn frame() -> i32 {
        FRAME.get()
    }

    /// Advances the index and calls every follower in the order they started following.
    pub fn advance() -> i32 {
        let frame = FRAME.get() + 1;
        FRAME.set(frame);
        let followers = FOLLOWERS.with_borrow(|followers| followers.clone());
        for follower in followers.iter() {
            follower(frame);
        }
        frame
    }

    /// Calls `on_frame` every frame from now on.
    pub fn follow(on_frame: impl Fn(i32) + 'static) {
        FOLLOWERS.with_borrow_mut(|followers| followers.push(Rc::new(on_frame)));
    }

    /// Forgets every follower and sets the index back to 0: a new scene starts.
    pub fn reset() {
        FOLLOWERS.with_borrow_mut(Vec::clear);
        FRAME.set(0);
    }
}
