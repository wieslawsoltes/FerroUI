//! Work on another thread whose result comes back to the UI thread.

use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;

type Waiting = Box<dyn FnOnce(Box<dyn Any + Send>)>;

ferroui_base::ferro_thread_local! {
    /// What waits on the UI thread for a result, by token: what it holds is not `Send`.
    static WAITING: RefCell<HashMap<u64, Waiting>> = RefCell::new(HashMap::new());
    static NEXT_TOKEN: Cell<u64> = const { Cell::new(1) };
}

fn deliver(token: u64, result: Box<dyn Any + Send>) {
    if let Some(done) = WAITING.with(|waiting| waiting.borrow_mut().remove(&token)) {
        done(result);
    }
}

/// Runs `work` on a new thread, then `done` with its result on the UI thread. Called on the UI
/// thread. When no thread can be started, `work` does not run and `Err` gives `done` back.
pub(crate) fn run_off_thread<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
    done: impl FnOnce(T) + 'static,
) -> std::io::Result<()> {
    let token = NEXT_TOKEN.with(|next| next.replace(next.get() + 1));
    let waiting: Waiting = Box::new(move |result| {
        if let Ok(result) = result.downcast::<T>() {
            done(*result);
        }
    });
    WAITING.with(|table| table.borrow_mut().insert(token, waiting));
    let dispatcher = Dispatcher::ui_thread();
    let spawned = std::thread::Builder::new().name("ferroui-web-fonts".into()).spawn(move || {
        let result: Box<dyn Any + Send> = Box::new(work());
        dispatcher.post(move || deliver(token, result), DispatcherPriority::BACKGROUND);
    });
    if let Err(error) = spawned {
        WAITING.with(|table| table.borrow_mut().remove(&token));
        return Err(error);
    }
    Ok(())
}
