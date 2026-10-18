//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/LetTest.cs` of Rx.NET.
//! Adapted: `Let` is the method `let_`.

use ferroui_reactive::{Observable, ObservableExt};
use std::cell::Cell;

#[test]
fn let_calls_function_immediately() {
    let called = Cell::new(false);
    let _ = Observable::empty::<i32>().let_(|x| {
        called.set(true);
        x
    });
    assert!(called.get());
}

// Not ported: Let_ArgumentChecking - argument validation
