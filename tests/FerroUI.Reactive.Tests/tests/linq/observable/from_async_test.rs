//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/FromAsyncTest.cs` of Rx.NET.
//! Adapted: a function returning a `Task` returns a future (a `Task` is one); `Task.Factory.StartNew`
//! is `tasks::run`, and the blocking `Single()` is `block_on` of `single_async().to_task()`.

use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::threading::tasks::{block_on, run};
use std::cell::Cell;
use std::rc::Rc;

// Not ported: FromAsync_Func_ArgumentChecking - argument validation

#[test]
fn from_async_func_success() {
    let n = 42;

    let i = Rc::new(Cell::new(0));
    let counter = i.clone();
    let xs = Observable::from_async(move || {
        counter.set(counter.get() + 1);
        run(move || Ok(n))
    });

    assert_eq!(n, block_on(xs.single_async().to_task()).unwrap());
    assert_eq!(1, i.get());

    assert_eq!(n, block_on(xs.single_async().to_task()).unwrap());
    assert_eq!(2, i.get());
}
