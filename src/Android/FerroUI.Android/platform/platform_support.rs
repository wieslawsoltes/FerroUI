//! Support for requests an activity answers later: the request codes, and
//! the check of a permission that asks the user when it is not granted.
//!
//! The reference awaits a task completion source that the handler of the
//! result completes, and adds the handler to an event of the activity
//! (`+=`) and removes it when the result arrived (`-=`). Here the task
//! completion source is [`Completion`], and the event, which the activity
//! keeps as one handler ([`IActivityResultHandler`]), is chained by
//! [`add_activity_result`] and [`add_request_permissions_result`].

use crate::i_activity_result_handler::{
    ActivityResultHandler, IActivityResultHandler, Intent, RequestPermissionsResultHandler, PERMISSION_GRANTED,
};
use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::atomic::{AtomicI32, Ordering};
use std::task::{Context, Poll, Waker};

static S_LAST_REQUEST_CODE: AtomicI32 = AtomicI32::new(20000);

pub(crate) fn get_next_request_code() -> i32 {
    S_LAST_REQUEST_CODE.fetch_add(1, Ordering::Relaxed)
}

/// Whether every permission of a result was granted
/// (`arg3.All(p => p == Permission.Granted)`).
pub(crate) fn all_granted(grant_results: &[i32]) -> bool {
    grant_results.iter().all(|p| *p == PERMISSION_GRANTED)
}

struct State<T> {
    result: Option<T>,
    waker: Option<Waker>,
}

/// A value that arrives later, on the UI thread: what stands for the task
/// completion sources of the reference, whose tasks the callers await.
/// This is the side that sets the value.
///
/// Not from the reference.
pub(crate) struct Completion<T> {
    state: Rc<RefCell<State<T>>>,
}

/// The side of a [`Completion`] that waits for the value.
pub(crate) struct CompletionFuture<T> {
    state: Rc<RefCell<State<T>>>,
}

impl<T> Clone for Completion<T> {
    fn clone(&self) -> Self {
        Self { state: self.state.clone() }
    }
}

impl<T> Completion<T> {
    /// Creates a completion and the future of its value.
    pub(crate) fn new() -> (Completion<T>, CompletionFuture<T>) {
        let state = Rc::new(RefCell::new(State { result: None, waker: None }));
        (Completion { state: state.clone() }, CompletionFuture { state })
    }

    /// Sets the value, unless one was set; tells whether it was set now.
    pub(crate) fn try_set_result(&self, result: T) -> bool {
        let waker = {
            let mut state = self.state.borrow_mut();
            if state.result.is_some() {
                return false;
            }
            state.result = Some(result);
            state.waker.take()
        };
        if let Some(waker) = waker {
            waker.wake();
        }
        true
    }
}

impl<T> Future for CompletionFuture<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        let mut state = self.state.borrow_mut();
        match state.result.take() {
            Some(result) => Poll::Ready(result),
            None => {
                state.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

/// One handler of an event of an activity and the handler that was there
/// before it.
struct Link<H: ?Sized> {
    previous: Option<Rc<H>>,
    own: RefCell<Option<Rc<H>>>,
}

/// A handler that was added to an event of an activity, to remove it with.
///
/// Not from the reference, where the event is a multicast delegate. The
/// activity of the port keeps one handler, so adding puts a handler there
/// that calls the one that was there and then the new one, and removing
/// puts the earlier one back. A handler that is removed while a later one
/// is still there stays in the chain and only passes the call on.
pub(crate) struct Subscription<H: ?Sized> {
    link: Rc<Link<H>>,
    combined: Rc<H>,
}

pub(crate) type ActivityResultSubscription = Subscription<dyn Fn(i32, i32, Option<&Intent>)>;
pub(crate) type RequestPermissionsResultSubscription = Subscription<dyn Fn(i32, &[String], &[i32])>;

/// `mainActivity.ActivityResult += handler`.
pub(crate) fn add_activity_result(
    main_activity: &dyn IActivityResultHandler,
    handler: ActivityResultHandler,
) -> ActivityResultSubscription {
    let link = Rc::new(Link { previous: main_activity.activity_result(), own: RefCell::new(Some(handler)) });
    let combined: ActivityResultHandler = {
        let link = link.clone();
        Rc::new(move |request_code: i32, result_code: i32, data: Option<&Intent>| {
            if let Some(previous) = &link.previous {
                previous(request_code, result_code, data);
            }
            let own = link.own.borrow().clone();
            if let Some(own) = own {
                own(request_code, result_code, data);
            }
        })
    };
    main_activity.set_activity_result(Some(combined.clone()));
    Subscription { link, combined }
}

/// `mainActivity.RequestPermissionsResult += handler`.
pub(crate) fn add_request_permissions_result(
    main_activity: &dyn IActivityResultHandler,
    handler: RequestPermissionsResultHandler,
) -> RequestPermissionsResultSubscription {
    let link = Rc::new(Link { previous: main_activity.request_permissions_result(), own: RefCell::new(Some(handler)) });
    let combined: RequestPermissionsResultHandler = {
        let link = link.clone();
        Rc::new(move |request_code: i32, permissions: &[String], grant_results: &[i32]| {
            if let Some(previous) = &link.previous {
                previous(request_code, permissions, grant_results);
            }
            let own = link.own.borrow().clone();
            if let Some(own) = own {
                own(request_code, permissions, grant_results);
            }
        })
    };
    main_activity.set_request_permissions_result(Some(combined.clone()));
    Subscription { link, combined }
}

impl Subscription<dyn Fn(i32, i32, Option<&Intent>)> {
    /// `mainActivity.ActivityResult -= handler`.
    pub(crate) fn remove(&self, main_activity: &dyn IActivityResultHandler) {
        let own = self.link.own.borrow_mut().take();
        // The handler may be the one that is running: it is released after the borrow.
        drop(own);
        if main_activity.activity_result().is_some_and(|current| Rc::ptr_eq(&current, &self.combined)) {
            main_activity.set_activity_result(self.link.previous.clone());
        }
    }
}

impl Subscription<dyn Fn(i32, &[String], &[i32])> {
    /// `mainActivity.RequestPermissionsResult -= handler`.
    pub(crate) fn remove(&self, main_activity: &dyn IActivityResultHandler) {
        let own = self.link.own.borrow_mut().take();
        // The handler may be the one that is running: it is released after the borrow.
        drop(own);
        if main_activity.request_permissions_result().is_some_and(|current| Rc::ptr_eq(&current, &self.combined)) {
            main_activity.set_request_permissions_result(self.link.previous.clone());
        }
    }
}

#[cfg(target_os = "android")]
pub(crate) use imp::check_permission;

#[cfg(target_os = "android")]
mod imp {
    use super::{
        add_request_permissions_result, all_granted, get_next_request_code, Completion,
        RequestPermissionsResultSubscription,
    };
    use crate::ferro_activity::FerroActivity;
    use crate::i_activity_result_handler::{RequestPermissionsResultHandler, PERMISSION_GRANTED};
    use crate::interop::java::{call_int, call_void, new_string_array, JavaObject, JavaValue};
    use crate::interop::natives::sdk_int;
    use ferroui_base::input::LocalBoxFuture;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// Whether the application has the permission; the user is asked when
    /// it does not. `activity` is an `android.app.Activity`.
    ///
    /// As the reference, the part before the result is awaited runs when
    /// the function is called.
    ///
    /// # Panics
    /// Panics when the activity is not one of the framework (the reference
    /// throws `InvalidOperationException`).
    pub(crate) fn check_permission(activity: &JavaObject, permission: &str) -> LocalBoxFuture<bool> {
        let Some(main_activity) = FerroActivity::from_java(activity) else {
            panic!("Main activity must implement IActivityResultHandler interface.");
        };

        if sdk_int() < 23 {
            return Box::pin(std::future::ready(true));
        }

        if call_int(activity, "checkSelfPermission", "(Ljava/lang/String;)I", &[JavaValue::String(permission)])
            == PERMISSION_GRANTED
        {
            return Box::pin(std::future::ready(true));
        }

        let current_request_code = get_next_request_code();
        let (tcs, task) = Completion::<bool>::new();
        let subscription: Rc<RefCell<Option<RequestPermissionsResultSubscription>>> = Rc::new(RefCell::new(None));
        let request_permissions_result: RequestPermissionsResultHandler = {
            let subscription = subscription.clone();
            // The activity keeps the handler, so the handler does not keep the activity.
            let main_activity = Rc::downgrade(&main_activity);
            Rc::new(move |request_code: i32, _arg2: &[String], arg3: &[i32]| {
                if current_request_code != request_code {
                    return;
                }

                let subscription = subscription.borrow_mut().take();
                if let (Some(subscription), Some(main_activity)) = (subscription, main_activity.upgrade()) {
                    subscription.remove(&*main_activity);
                }

                let _ = tcs.try_set_result(all_granted(arg3));
            })
        };
        *subscription.borrow_mut() = Some(add_request_permissions_result(&*main_activity, request_permissions_result));
        let permissions = new_string_array(&[permission.to_string()]);
        call_void(
            activity,
            "requestPermissions",
            "([Ljava/lang/String;I)V",
            &[JavaValue::Object(Some(&permissions)), JavaValue::Int(current_request_code)],
        );

        Box::pin(task)
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of the Android backend that run without a
    // device.
    use super::*;
    use crate::i_activity_result_handler::{PERMISSION_DENIED, RESULT_CANCELED, RESULT_OK};
    use std::cell::Cell;
    use std::sync::atomic::AtomicUsize;
    use std::sync::Arc;
    use std::task::Wake;

    struct CountingWaker(AtomicUsize);

    impl Wake for CountingWaker {
        fn wake(self: Arc<Self>) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[derive(Default)]
    struct Activity {
        activity_result: RefCell<Option<ActivityResultHandler>>,
        request_permissions_result: RefCell<Option<RequestPermissionsResultHandler>>,
    }

    impl IActivityResultHandler for Activity {
        fn activity_result(&self) -> Option<ActivityResultHandler> {
            self.activity_result.borrow().clone()
        }

        fn set_activity_result(&self, value: Option<ActivityResultHandler>) {
            *self.activity_result.borrow_mut() = value;
        }

        fn request_permissions_result(&self) -> Option<RequestPermissionsResultHandler> {
            self.request_permissions_result.borrow().clone()
        }

        fn set_request_permissions_result(&self, value: Option<RequestPermissionsResultHandler>) {
            *self.request_permissions_result.borrow_mut() = value;
        }
    }

    impl Activity {
        fn raise_activity_result(&self, request_code: i32, result_code: i32) {
            let handler = self.activity_result();
            if let Some(handler) = handler {
                handler(request_code, result_code, None);
            }
        }

        fn raise_request_permissions_result(&self, request_code: i32, grant_results: &[i32]) {
            let handler = self.request_permissions_result();
            if let Some(handler) = handler {
                handler(request_code, &[], grant_results);
            }
        }
    }

    #[test]
    fn request_codes_are_consecutive_and_begin_at_20000() {
        let first = get_next_request_code();
        let second = get_next_request_code();
        assert!(first >= 20000);
        assert!(second > first);
    }

    #[test]
    fn a_result_is_granted_when_every_permission_is() {
        assert!(all_granted(&[]));
        assert!(all_granted(&[PERMISSION_GRANTED, PERMISSION_GRANTED]));
        assert!(!all_granted(&[PERMISSION_GRANTED, PERMISSION_DENIED]));
    }

    #[test]
    fn the_future_is_ready_once_the_value_is_set() {
        let (completion, mut future) = Completion::<u32>::new();
        let counter = Arc::new(CountingWaker(AtomicUsize::new(0)));
        let waker = Waker::from(counter.clone());
        let mut cx = Context::from_waker(&waker);

        assert_eq!(Poll::Pending, Pin::new(&mut future).poll(&mut cx));
        assert_eq!(0, counter.0.load(Ordering::SeqCst));
        assert!(completion.try_set_result(7));
        assert_eq!(1, counter.0.load(Ordering::SeqCst));
        // The first value stays.
        assert!(!completion.clone().try_set_result(8));
        assert_eq!(Poll::Ready(7), Pin::new(&mut future).poll(&mut cx));
    }

    #[test]
    fn a_value_set_before_the_first_poll_is_returned_by_it() {
        let (completion, mut future) = Completion::<&str>::new();
        assert!(completion.try_set_result("done"));
        let waker = Waker::from(Arc::new(CountingWaker(AtomicUsize::new(0))));
        let mut cx = Context::from_waker(&waker);
        assert_eq!(Poll::Ready("done"), Pin::new(&mut future).poll(&mut cx));
    }

    #[test]
    fn handlers_are_called_in_the_order_they_were_added() {
        let activity = Activity::default();
        let calls = Rc::new(RefCell::new(Vec::new()));
        let add = |name: &'static str| {
            let calls = calls.clone();
            add_activity_result(
                &activity,
                Rc::new(move |request_code: i32, result_code: i32, _data: Option<&Intent>| {
                    calls.borrow_mut().push((name, request_code, result_code))
                }),
            )
        };
        let _first = add("first");
        let _second = add("second");

        activity.raise_activity_result(20000, RESULT_OK);

        assert_eq!(vec![("first", 20000, RESULT_OK), ("second", 20000, RESULT_OK)], *calls.borrow());
    }

    #[test]
    fn the_last_handler_that_is_removed_leaves_the_earlier_one() {
        let activity = Activity::default();
        let calls = Rc::new(Cell::new(0));
        let first = {
            let calls = calls.clone();
            add_activity_result(
                &activity,
                Rc::new(move |_: i32, _: i32, _: Option<&Intent>| calls.set(calls.get() + 1)),
            )
        };
        let earlier = activity.activity_result().expect("the handler was added");
        let second = {
            let calls = calls.clone();
            add_activity_result(
                &activity,
                Rc::new(move |_: i32, _: i32, _: Option<&Intent>| calls.set(calls.get() + 10)),
            )
        };

        second.remove(&activity);
        assert!(Rc::ptr_eq(&earlier, &activity.activity_result().expect("the earlier handler is back")));
        activity.raise_activity_result(1, RESULT_CANCELED);
        assert_eq!(1, calls.get());

        first.remove(&activity);
        assert!(activity.activity_result().is_none());
    }

    #[test]
    fn a_handler_removed_under_a_later_one_is_not_called_again() {
        let activity = Activity::default();
        let calls = Rc::new(RefCell::new(Vec::new()));
        let first = {
            let calls = calls.clone();
            add_request_permissions_result(
                &activity,
                Rc::new(move |_: i32, _: &[String], _: &[i32]| calls.borrow_mut().push("first")),
            )
        };
        let _second = {
            let calls = calls.clone();
            add_request_permissions_result(
                &activity,
                Rc::new(move |_: i32, _: &[String], _: &[i32]| calls.borrow_mut().push("second")),
            )
        };

        first.remove(&activity);
        activity.raise_request_permissions_result(1, &[PERMISSION_GRANTED]);

        assert_eq!(vec!["second"], *calls.borrow());
    }

    #[test]
    fn a_handler_removes_itself_while_it_runs() {
        let activity = Rc::new(Activity::default());
        let subscription: Rc<RefCell<Option<RequestPermissionsResultSubscription>>> = Rc::new(RefCell::new(None));
        let (completion, mut future) = Completion::<bool>::new();
        let handler: RequestPermissionsResultHandler = {
            let subscription = subscription.clone();
            let activity = Rc::downgrade(&activity);
            Rc::new(move |request_code: i32, _permissions: &[String], grant_results: &[i32]| {
                if request_code != 20001 {
                    return;
                }
                let subscription = subscription.borrow_mut().take();
                if let (Some(subscription), Some(activity)) = (subscription, activity.upgrade()) {
                    subscription.remove(&*activity);
                }
                let _ = completion.try_set_result(all_granted(grant_results));
            })
        };
        *subscription.borrow_mut() = Some(add_request_permissions_result(&*activity, handler));

        // The result of another request leaves the handler where it is.
        activity.raise_request_permissions_result(20000, &[PERMISSION_GRANTED]);
        assert!(activity.request_permissions_result().is_some());

        activity.raise_request_permissions_result(20001, &[PERMISSION_DENIED]);
        assert!(activity.request_permissions_result().is_none());
        let waker = Waker::from(Arc::new(CountingWaker(AtomicUsize::new(0))));
        let mut cx = Context::from_waker(&waker);
        assert_eq!(Poll::Ready(false), Pin::new(&mut future).poll(&mut cx));
    }
}
