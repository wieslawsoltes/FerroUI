//! Port of `Embedding/MacHelper.cs`.

use std::cell::Cell;

thread_local! {
    static IS_INITIALIZED: Cell<bool> = const { Cell::new(false) };
}

pub struct MacHelper;

impl MacHelper {
    /// Makes AppKit usable from the sample, once.
    ///
    /// The managed original initializes its AppKit binding here. The Objective-C runtime calls
    /// of the port (`objc.rs`) need no initialization (AppKit is linked, and the application
    /// object exists once the platform is set up), so what is left is the check that it does.
    pub fn ensure_initialized() {
        if IS_INITIALIZED.with(Cell::get) {
            return;
        }
        IS_INITIALIZED.with(|is_initialized| is_initialized.set(true));
        assert!(!super::objc::class("NSApplication").is_null(), "AppKit is loaded");
    }
}
