//! One spawned thread in a web page: the check of the threaded build of
//! the browser target (`scripts/build-browser.sh thread_spawn --threads`).
//!
//! The page starts a thread with `std::thread::spawn`, which adds the
//! numbers from 1 to 1000 and sends the sum back over a channel. The main
//! thread never waits for it: the host page asks for the state from a timer
//! (`threadSpawnState`) and writes it into the element `result`, where the
//! behaviour test (`scripts/browser/tests/thread_spawn.test.mjs`) reads it.
//! A browser does not let the main thread of a page block, and the framework
//! will not block it either.
//!
//! The example uses nothing of the framework, on purpose: it checks the
//! toolchain (the rebuilt standard library, the thread pool, the shared
//! memory) and the cross-origin isolation of the site, and stays valid while
//! the render thread of the browser backend is being written. In a build
//! without threads it compiles and reports that the thread could not start.

#![cfg_attr(target_os = "emscripten", no_main)]

use std::cell::RefCell;
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use wasm_bindgen::prelude::*;

/// What the spawned thread sends back: the sum, and whether it ran on a
/// thread other than the one that started it.
type Report = (u64, bool);

thread_local! {
    /// The receiving end of the channel while the thread runs.
    static PENDING: RefCell<Option<Receiver<Report>>> = const { RefCell::new(None) };
    /// The last state, kept once the thread has answered or failed.
    static STATE: RefCell<String> = RefCell::new("state=idle".to_string());
}

/// Starts the thread. The outcome is read with [`thread_spawn_state`].
#[wasm_bindgen(js_name = threadSpawnStart)]
pub fn thread_spawn_start() {
    let (sender, receiver) = channel::<Report>();
    let starter = std::thread::current().id();
    let spawned = std::thread::Builder::new().name("thread_spawn".to_string()).spawn(move || {
        let sum: u64 = (1..=1000u64).sum();
        // The receiver is gone when the page was told to start again.
        let _ = sender.send((sum, std::thread::current().id() != starter));
    });
    match spawned {
        // The handle is dropped: the thread is detached and nothing joins it.
        Ok(_) => {
            PENDING.with(|pending| *pending.borrow_mut() = Some(receiver));
            STATE.with(|state| *state.borrow_mut() = "state=running".to_string());
        }
        Err(error) => {
            PENDING.with(|pending| *pending.borrow_mut() = None);
            STATE.with(|state| *state.borrow_mut() = format!("state=failed error={error}"));
        }
    }
}

/// The state as a line of `name=value` pairs: `atomics` (whether the module
/// was built with threads), `state` (`idle`, `running`, `done` or `failed`),
/// and with `done` the `value` the thread sent and `other_thread`.
#[wasm_bindgen(js_name = threadSpawnState)]
pub fn thread_spawn_state() -> String {
    let received = PENDING.with(|pending| pending.borrow().as_ref().map(|receiver| receiver.try_recv()));
    match received {
        Some(Ok((value, other_thread))) => {
            PENDING.with(|pending| *pending.borrow_mut() = None);
            STATE.with(|state| *state.borrow_mut() = format!("state=done value={value} other_thread={other_thread}"));
        }
        Some(Err(TryRecvError::Disconnected)) => {
            PENDING.with(|pending| *pending.borrow_mut() = None);
            STATE.with(|state| *state.borrow_mut() = "state=failed error=the thread ended without sending a value".to_string());
        }
        Some(Err(TryRecvError::Empty)) | None => {}
    }
    let state = STATE.with(|state| state.borrow().clone());
    format!("atomics={} {state}", cfg!(target_feature = "atomics"))
}

/// Outside a web page the example runs the same two calls and prints the
/// outcome, so that the workspace checks cover it.
#[cfg(not(target_os = "emscripten"))]
fn main() {
    thread_spawn_start();
    let mut state = thread_spawn_state();
    while state.contains("state=running") {
        std::thread::sleep(std::time::Duration::from_millis(1));
        state = thread_spawn_state();
    }
    println!("{state}");
}
