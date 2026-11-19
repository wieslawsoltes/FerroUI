//! Events: a handler is a C function with an integer token ([`FerroEventFn`]) or with a `void *`
//! and a function that releases it ([`FerroEventDataFn`], [`FerroReleaseFn`]). Nothing is
//! allocated per event: the handle of the sender is the one the object has (remembered by the
//! subscription, so a dispatch is a check of the handle and one indirect call).

use crate::error::{status, status_or_value, Error, Result};
use crate::handles;
use crate::ids::{self, Event};
use crate::thread;
use crate::{
    FerroEventDataFn, FerroEventFn, FerroHandle, FerroReleaseFn, FerroStatus, FERRO_NOT_FOUND, FERRO_NOT_SUPPORTED,
    FERRO_NULL_POINTER, FERRO_TYPE_MISMATCH,
};
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEvent, RoutedEventArgs, RoutedEventHandlerToken};
use ferroui_base::metadata::{MarkupDelegate, MarkupValue};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{FerroObject, Upcast, WeakRef};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ffi::c_void;
use std::rc::Rc;

/// The C function a handler calls.
enum Callback {
    Token { function: unsafe extern "C" fn(i64, FerroHandle, i64) -> i64, token: i64 },
    Data(UserData),
}

/// The user data of a handler, released when the handler is dropped.
struct UserData {
    function: unsafe extern "C" fn(*mut c_void, FerroHandle, i64) -> i64,
    user_data: *mut c_void,
    release: FerroReleaseFn,
}

impl Drop for UserData {
    fn drop(&mut self) {
        if let Some(release) = self.release {
            // SAFETY: the caller gave the release function for this user data.
            unsafe { release(self.user_data) };
        }
    }
}

impl Callback {
    #[inline]
    fn call(&self, sender: FerroHandle, arg: i64) -> i64 {
        // SAFETY: the caller gave functions of the documented signatures.
        unsafe {
            match self {
                Callback::Token { function, token } => function(*token, sender, arg),
                Callback::Data(data) => (data.function)(data.user_data, sender, arg),
            }
        }
    }
}

/// A handler: the callback and the handle of the sender it last saw.
struct Handler {
    callback: Callback,
    sender: Cell<FerroHandle>,
}

impl Handler {
    /// Calls the callback with the handle of `sender`: the remembered one while it is still the
    /// handle of the object, else the handle the object has, else a handle made for the call
    /// (released after it, unless the callback retained it).
    fn invoke(&self, sender: &FerroObject, arg: i64) -> i64 {
        let address = handles::address(sender);
        let remembered = self.sender.get();
        if remembered != 0 && handles::is_handle_of(remembered, address) {
            return self.callback.call(remembered, arg);
        }
        if let Some(handle) = handles::find(address) {
            self.sender.set(handle);
            return self.callback.call(handle, arg);
        }
        let handle = handles::insert(sender.to_ref());
        let result = self.callback.call(handle, arg);
        let _ = handles::release(handle);
        result
    }
}

/// What [`ferro_unsubscribe`] removes.
enum Subscription {
    Routed { object: WeakRef<FerroObject>, event: RoutedEvent, token: RoutedEventHandlerToken },
    Plain(Option<Rc<dyn IDisposable>>),
}

#[derive(Default)]
struct Subscriptions {
    next: i64,
    map: HashMap<i64, Subscription>,
}

ferroui_base::ferro_thread_local! {
    static SUBSCRIPTIONS: RefCell<Subscriptions> = RefCell::new(Subscriptions::default());
}

fn subscribe(object: FerroHandle, event: i64, callback: Callback) -> Result<i64> {
    thread::check()?;
    let object = handles::get(object)?;
    let event = ids::event(object.get_type(), event)?;
    let handler = Rc::new(Handler { callback, sender: Cell::new(0) });
    let subscription = match event {
        Event::Routed(event) => {
            let interactive = object.cast::<Interactive>().ok_or_else(|| {
                Error::new(FERRO_TYPE_MISMATCH, format!("a {} has no routed events", object.get_type().name()))
            })?;
            let route_handler: ferroui_base::interactivity::RouteHandler =
                Rc::new(move |sender: &Interactive, args: &dyn IRoutedEventArgs| {
                    let sender: &FerroObject = sender.upcast();
                    if handler.invoke(sender, 0) != 0 {
                        args.set_handled(true);
                    }
                });
            let token =
                interactive.add_route_handler(&event, route_handler, Interactive::DEFAULT_ROUTES, false);
            Subscription::Routed { object: object.downgrade(), event, token }
        }
        Event::Plain(event) => {
            let weak = object.downgrade();
            let delegate = MarkupDelegate::new(move |_arguments: &[MarkupValue]| {
                if let Some(object) = weak.upgrade() {
                    handler.invoke(&object, 0);
                }
                None
            });
            let disposable = event
                .subscribe(Some(Rc::new(object.clone())), delegate)
                .map_err(|error| Error::new(FERRO_NOT_SUPPORTED, error.to_string()))?;
            Subscription::Plain(disposable)
        }
    };
    let id = SUBSCRIPTIONS.with_borrow_mut(|subscriptions| {
        subscriptions.next += 1;
        let id = subscriptions.next;
        subscriptions.map.insert(id, subscription);
        id
    });
    Ok(id)
}

/// Attaches `function` to the event `event` ([`ferro_event_id`](crate::ferro_event_id)) of
/// `object`: it is called with `token` on every raise of the event, on the UI thread (see
/// [`FerroEventFn`]). For a routed event the handler listens to the direct and the bubbling route
/// of events that are not handled yet, and a result other than `0` marks the event as handled.
/// Returns a positive subscription ID for [`ferro_unsubscribe`], or a negative status. UI thread
/// only.
#[no_mangle]
pub extern "C" fn ferro_subscribe(object: FerroHandle, event: i64, function: FerroEventFn, token: i64) -> i64 {
    status_or_value(|| {
        let function = function.ok_or_else(|| Error::new(FERRO_NULL_POINTER, "the function is NULL"))?;
        subscribe(object, event, Callback::Token { function, token })
    })
}

/// [`ferro_subscribe`] with a `void *` in place of the token: `function` is called with
/// `user_data`, and `release` (which may be `NULL`) is called with it once the handler is gone,
/// removed by [`ferro_unsubscribe`] or dropped with its object. If the subscription fails,
/// `release` is called before this returns. UI thread only.
#[no_mangle]
pub extern "C" fn ferro_subscribe_data(
    object: FerroHandle,
    event: i64,
    function: FerroEventDataFn,
    user_data: *mut c_void,
    release: FerroReleaseFn,
) -> i64 {
    status_or_value(|| {
        let Some(function) = function else {
            // The user data is released as on any failure.
            drop(UserData { function: noop, user_data, release });
            return Err(Error::new(FERRO_NULL_POINTER, "the function is NULL"));
        };
        subscribe(object, event, Callback::Data(UserData { function, user_data, release }))
    })
}

unsafe extern "C" fn noop(_: *mut c_void, _: FerroHandle, _: i64) -> i64 {
    0
}

/// Removes the handler of the subscription `subscription` ([`ferro_subscribe`]); its user data is
/// released ([`ferro_subscribe_data`]). [`FERRO_NOT_FOUND`] for an ID that is not a live
/// subscription. UI thread only.
#[no_mangle]
pub extern "C" fn ferro_unsubscribe(subscription: i64) -> FerroStatus {
    status(|| {
        thread::check()?;
        let removed = SUBSCRIPTIONS.with_borrow_mut(|subscriptions| subscriptions.map.remove(&subscription));
        // Removed with no borrow held: dropping a handler releases its user data, which runs code
        // of the caller.
        match removed {
            None => Err(Error::new(FERRO_NOT_FOUND, format!("{subscription} is not a subscription"))),
            Some(Subscription::Routed { object, event, token }) => {
                if let Some(interactive) = object.upgrade().and_then(|object| object.cast::<Interactive>()) {
                    interactive.remove_handler(&event, token);
                }
                Ok(())
            }
            Some(Subscription::Plain(disposable)) => {
                if let Some(disposable) = disposable {
                    disposable.dispose();
                }
                Ok(())
            }
        }
    })
}

/// Raises the routed event `event` on `object` with plain event arguments, as the control does
/// when it raises it (`Click` of a button, as a click does): for tests and for driving an
/// interface from code. [`FERRO_NOT_SUPPORTED`] for an event whose arguments carry more than the
/// routing state (a pointer event). UI thread only.
#[no_mangle]
pub extern "C" fn ferro_raise(object: FerroHandle, event: i64) -> FerroStatus {
    status(|| {
        thread::check()?;
        let object = handles::get(object)?;
        let Event::Routed(event) = ids::event(object.get_type(), event)? else {
            return Err(Error::new(FERRO_NOT_SUPPORTED, "only a routed event can be raised"));
        };
        if !event.event_args_is::<RoutedEventArgs>() || event.event_args_type() != std::any::TypeId::of::<RoutedEventArgs>() {
            return Err(Error::new(
                FERRO_NOT_SUPPORTED,
                format!("the arguments of {event} are {}, not plain routed event arguments", event.event_args_type_name()),
            ));
        }
        let interactive = object
            .cast::<Interactive>()
            .ok_or_else(|| Error::new(FERRO_NOT_FOUND, "the object has no routed events"))?;
        interactive.raise_event(&RoutedEventArgs::with_event(&event));
        Ok(())
    })
}
