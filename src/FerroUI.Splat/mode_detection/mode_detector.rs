//! `ModeDetector`.

use super::{DefaultModeDetector, IModeDetector};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

// Deviation (DEVIATIONS.md, Splat): the detector and its cached result are static state of the
// process upstream; here they are per thread, as the locator is.
ferroui_system::ferro_thread_local! {
    static CURRENT: RefCell<Option<Rc<dyn IModeDetector>>> = const { RefCell::new(None) };
    static CACHED_IN_UNIT_TEST_RUNNER_RESULT: Cell<Option<bool>> = const { Cell::new(None) };
}

/// Whether the code runs under a unit test runner, as the current detector tells, cached
/// (`ModeDetector`). The detector is a [`DefaultModeDetector`] until it is overridden.
pub struct ModeDetector;

impl ModeDetector {
    fn current() -> Rc<dyn IModeDetector> {
        if let Some(current) = CURRENT.with(|current| current.borrow().clone()) {
            return current;
        }
        let current: Rc<dyn IModeDetector> = Rc::new(DefaultModeDetector::new());
        CURRENT.with(|slot| *slot.borrow_mut() = Some(current.clone()));
        current
    }

    /// Replaces the detector and forgets the cached result (`OverrideModeDetector`).
    pub fn override_mode_detector(mode_detector: Rc<dyn IModeDetector>) {
        CURRENT.with(|slot| *slot.borrow_mut() = Some(mode_detector));
        CACHED_IN_UNIT_TEST_RUNNER_RESULT.with(|cached| cached.set(None));
    }

    /// Whether the code runs under a unit test runner (`InUnitTestRunner`): the cached result, or
    /// what the detector tells, `false` when it cannot tell.
    pub fn in_unit_test_runner() -> bool {
        if let Some(cached) = CACHED_IN_UNIT_TEST_RUNNER_RESULT.with(|cached| cached.get()) {
            return cached;
        }
        let result = Self::current().in_unit_test_runner();
        CACHED_IN_UNIT_TEST_RUNNER_RESULT.with(|cached| cached.set(result));
        result.unwrap_or(false)
    }

    /// `GetState`: the detector and the cached result.
    #[doc(hidden)]
    pub fn get_state() -> (Rc<dyn IModeDetector>, Option<bool>) {
        (Self::current(), CACHED_IN_UNIT_TEST_RUNNER_RESULT.with(|cached| cached.get()))
    }

    /// `RestoreState`.
    #[doc(hidden)]
    pub fn restore_state(state: (Rc<dyn IModeDetector>, Option<bool>)) {
        CURRENT.with(|slot| *slot.borrow_mut() = Some(state.0));
        CACHED_IN_UNIT_TEST_RUNNER_RESULT.with(|cached| cached.set(state.1));
    }

    /// `ResetState`: a new default detector, no cached result.
    #[doc(hidden)]
    pub fn reset_state() {
        CURRENT.with(|slot| *slot.borrow_mut() = Some(Rc::new(DefaultModeDetector::new())));
        CACHED_IN_UNIT_TEST_RUNNER_RESULT.with(|cached| cached.set(None));
    }
}
