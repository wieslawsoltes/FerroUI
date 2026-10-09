//! Port of `SetupTests.cs` of the upstream unit test project of the
//! headless platform.
//!
//! The constructor and `Dispose` of the test class (the set-up and the
//! tear-down under NUnit) are `SetupTests::new` and its `Drop`. The static
//! count of the original is a count of the session thread: the two test
//! assemblies, which are two processes upstream, are two threads of this
//! one.

use super::test_application::ferro_theory;
use std::cell::Cell;

thread_local! {
    static INSTANCE_COUNT: Cell<i32> = const { Cell::new(0) };
}

struct SetupTests;

impl SetupTests {
    fn new() -> Self {
        INSTANCE_COUNT.set(INSTANCE_COUNT.get() + 1);
        Self
    }
}

impl Drop for SetupTests {
    fn drop(&mut self) {
        INSTANCE_COUNT.set(INSTANCE_COUNT.get() - 1);
    }
}

// The parameter is used to run the test several times.
fn setup_tear_down_should_work(_index: i32) {
    let _this = SetupTests::new();
    assert_eq!(1, INSTANCE_COUNT.get());
}
ferro_theory!(setup_tear_down_should_work {
    index_1: (1),
    index_2: (2),
});
