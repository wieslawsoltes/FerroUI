//! The back callback of an activity (API 33).
//!
//! The reference derives the class from the back pressed callback of
//! AndroidX and adds it to the dispatcher of the activity. The port uses
//! the back callback of the window of the platform
//! (docs/porting/android-platform.md, section 3.3), which the Java activity
//! registers and forwards to this object; where the reference disables
//! itself and asks the dispatcher again to reach the default action, the
//! answer of [`handle_on_back_pressed`](BackPressedCallback::handle_on_back_pressed)
//! tells the Java activity to do the default action of the system.

use crate::ferro_activity::FerroActivity;
use std::rc::{Rc, Weak};

pub(crate) struct BackPressedCallback {
    activity: Weak<FerroActivity>,
}

impl BackPressedCallback {
    pub fn new(activity: &Rc<FerroActivity>) -> Self {
        Self { activity: Rc::downgrade(activity) }
    }

    /// The back button was pressed. The answer is whether the default
    /// action of the system follows.
    pub fn handle_on_back_pressed(&self) -> bool {
        let Some(activity) = self.activity.upgrade() else {
            return true;
        };
        activity.on_back_invoked();

        activity.should_navigate_back()
    }
}
