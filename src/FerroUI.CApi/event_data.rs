//! The data of the routed event being handled: while a handler of a routed event runs
//! ([`ferro_subscribe`](crate::ferro_subscribe)), the arguments of the event are the current
//! event of the UI thread, and these functions read what they carry beyond the routing state (the
//! position and the button of a pointer event, the key of a key event). A handler receives
//! scalars only, so a language whose callbacks are a named function and integers (Osprey, #950)
//! reads the arguments it needs from inside the handler. [`ferro_focus`] moves the keyboard focus.

use crate::error::{guard, Error, Result};
use crate::handles;
use crate::thread;
use crate::{FerroHandle, FERRO_INVALID_STATE, FERRO_NOT_SUPPORTED, FERRO_TYPE_MISMATCH};
use ferroui_base::input::{InputElement, KeyEventArgs, PointerEventArgs, PointerUpdateKind, TappedEventArgs};
use ferroui_base::interactivity::IRoutedEventArgs;
use ferroui_base::{Point, Visual};
use std::any::TypeId;
use std::cell::{Cell, RefCell};
use std::ffi::{c_char, CString};

ferroui_base::ferro_thread_local! {
    /// The arguments of the routed event whose handler runs, while it runs.
    static CURRENT: Cell<Option<*const dyn IRoutedEventArgs>> = const { Cell::new(None) };
    /// The text the last `ferro_event_key` of the thread returned.
    static LAST_KEY: RefCell<CString> = RefCell::new(CString::default());
}

/// Puts back the current event of before a handler, also when the handler panics.
struct Restore(Option<*const dyn IRoutedEventArgs>);

impl Drop for Restore {
    fn drop(&mut self) {
        CURRENT.set(self.0);
    }
}

/// Runs `body` (a handler) with `args` as the current event; an event raised inside the handler
/// is the current one while its own handlers run.
pub(crate) fn handling<R>(args: &dyn IRoutedEventArgs, body: impl FnOnce() -> R) -> R {
    let _restore = Restore(CURRENT.replace(Some(args as *const dyn IRoutedEventArgs)));
    body()
}

/// Calls `read` with the arguments of the current event.
fn with_current<R>(read: impl FnOnce(&dyn IRoutedEventArgs) -> Result<R>) -> Result<R> {
    thread::check()?;
    let args = CURRENT.get().ok_or_else(|| {
        Error::new(FERRO_INVALID_STATE, "no routed event is being handled: call it from the handler of the event")
    })?;
    // SAFETY: the pointer is set only while the handler of the event runs (`handling`), and the
    // arguments outlive the handler.
    read(unsafe { &*args })
}

fn args_of<T: 'static>(args: &dyn IRoutedEventArgs) -> Option<&T> {
    args.query_args(TypeId::of::<T>()).and_then(|args| args.downcast_ref::<T>())
}

/// The position of the pointer of the current event relative to `relative_to`.
fn position(relative_to: FerroHandle) -> Result<Point> {
    thread::check()?;
    let visual = match relative_to {
        0 => None,
        handle => {
            let object = handles::get(handle)?;
            let visual = object.cast::<Visual>().ok_or_else(|| {
                Error::new(FERRO_TYPE_MISMATCH, format!("a {} is not a visual", object.get_type().name()))
            })?;
            Some(visual)
        }
    };
    with_current(|args| {
        if let Some(pointer) = args_of::<PointerEventArgs>(args) {
            return Ok(pointer.get_position(visual.as_deref()));
        }
        if let Some(tapped) = args_of::<TappedEventArgs>(args) {
            return Ok(tapped.get_position(visual.as_deref()));
        }
        Err(Error::new(FERRO_NOT_SUPPORTED, "the event being handled has no pointer position"))
    })
}

fn coordinate(relative_to: FerroHandle, axis: fn(Point) -> f64) -> f64 {
    guard(f64::NAN, || match position(relative_to) {
        Ok(point) => axis(point),
        Err(error) => {
            error.record();
            f64::NAN
        }
    })
}

/// The horizontal position of the pointer of the routed event being handled (a pointer event, or
/// a tap: `Tapped`, `DoubleTapped`), relative to the visual `relative_to`, or to the top level
/// the event happened in for `0`. NaN on a failure (recorded: [`FERRO_INVALID_STATE`] outside a
/// handler, [`FERRO_NOT_SUPPORTED`] for an event without a position). UI thread only.
#[no_mangle]
pub extern "C" fn ferro_event_x(relative_to: FerroHandle) -> f64 {
    coordinate(relative_to, |point| point.x)
}

/// The vertical position of the pointer of the routed event being handled; see
/// [`ferro_event_x`]. UI thread only.
#[no_mangle]
pub extern "C" fn ferro_event_y(relative_to: FerroHandle) -> f64 {
    coordinate(relative_to, |point| point.y)
}

/// The button a pointer event being handled is about: `1` left, `2` right, `3` middle, `4` and
/// `5` the first and second extra buttons (the values of `MouseButton`), for a press or a release;
/// `0` for a pointer event that is not about a button (a move). A negative status for an event
/// that is not a pointer event, or outside a handler. UI thread only.
#[no_mangle]
pub extern "C" fn ferro_event_button() -> i64 {
    crate::error::status_or_value(|| {
        with_current(|args| {
            let pointer = args_of::<PointerEventArgs>(args)
                .ok_or_else(|| Error::new(FERRO_NOT_SUPPORTED, "the event being handled is not a pointer event"))?;
            Ok(match pointer.properties().pointer_update_kind {
                PointerUpdateKind::LeftButtonPressed | PointerUpdateKind::LeftButtonReleased => 1,
                PointerUpdateKind::RightButtonPressed | PointerUpdateKind::RightButtonReleased => 2,
                PointerUpdateKind::MiddleButtonPressed | PointerUpdateKind::MiddleButtonReleased => 3,
                PointerUpdateKind::XButton1Pressed | PointerUpdateKind::XButton1Released => 4,
                PointerUpdateKind::XButton2Pressed | PointerUpdateKind::XButton2Released => 5,
                PointerUpdateKind::Other => 0,
            })
        })
    })
}

/// The key of the key event being handled (`KeyDown`, `KeyUp`), as the name of the member of
/// `Key` (`Return`, which is also named `Enter`; `Escape`; `A`), NUL-terminated and owned by the
/// library until the next call of `ferro_event_key` on the thread. `NULL` on a failure (recorded:
/// an event that is not a key event, or outside a handler). UI thread only.
#[no_mangle]
pub extern "C" fn ferro_event_key() -> *const c_char {
    guard(std::ptr::null(), || {
        let name = with_current(|args| {
            let key = args_of::<KeyEventArgs>(args)
                .ok_or_else(|| Error::new(FERRO_NOT_SUPPORTED, "the event being handled is not a key event"))?;
            Ok(format!("{:?}", key.key))
        });
        match name {
            Ok(name) => LAST_KEY.with_borrow_mut(|last| {
                // A name of a member has no NUL.
                *last = CString::new(name).unwrap_or_default();
                last.as_ptr()
            }),
            Err(error) => {
                error.record();
                std::ptr::null()
            }
        }
    })
}

/// Moves the keyboard focus to `object` (an input element: a text box, a button), as a click on it
/// does. [`FERRO_INVALID_STATE`] when it cannot take the focus now (it is not focusable, not
/// enabled, or not in a shown window yet: posting the call with [`ferro_post`](crate::ferro_post)
/// focuses it once it is). UI thread only.
#[no_mangle]
pub extern "C" fn ferro_focus(object: FerroHandle) -> crate::FerroStatus {
    crate::error::status(|| {
        thread::check()?;
        let object = handles::get(object)?;
        let input = object.cast::<InputElement>().ok_or_else(|| {
            Error::new(FERRO_TYPE_MISMATCH, format!("a {} is not an input element", object.get_type().name()))
        })?;
        if input.focus() {
            Ok(())
        } else {
            Err(Error::new(FERRO_INVALID_STATE, "the element cannot take the focus now"))
        }
    })
}
