//! A window whose events a transfer waits for (the port of
//! `EventStreamWindow.cs`).

use crate::selections::i_x_event_waiter::{IXEventWaiter, XEventPredicate};
use crate::selections::selection_helper::{create_event_window, TaskCompletionSource};
use crate::x11_platform::FerroX11Platform;
use crate::x11_window_info::X11WindowInfo;
use crate::xlib::{self, XEvent, XID};
use ferroui_base::input::LocalBoxFuture;
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::time::{Duration, Instant};

/// Someone waiting for an event: the condition, the completion and the
/// moment the wait gives up.
struct Listener<C> {
    filter: XEventPredicate,
    tcs: C,
    timeout: Instant,
}

/// The listeners of a window, apart from the window: who waits, in which
/// order they are asked, and when they give up.
struct Listeners<C> {
    listeners: Vec<Listener<C>>,
    // We are adding listeners to an intermediate collection to avoid freshly added listeners to be called
    // in the same event loop iteration and potentially processing an event that was not meant for them.
    added_listeners: Vec<Listener<C>>,
}

impl<C> Listeners<C> {
    fn new() -> Self {
        Self { listeners: Vec::new(), added_listeners: Vec::new() }
    }

    fn add(&mut self, filter: XEventPredicate, tcs: C, timeout: Instant) {
        self.added_listeners.push(Listener { filter, tcs, timeout });
    }

    fn merge_listeners(&mut self) {
        self.listeners.append(&mut self.added_listeners);
    }

    /// Removes the listeners `matches` selects, in the order they were
    /// added, and returns their completions.
    fn take_where(&mut self, mut matches: impl FnMut(&Listener<C>) -> bool) -> Vec<C> {
        self.merge_listeners();
        let mut taken = Vec::new();
        let mut kept = Vec::with_capacity(self.listeners.len());
        for listener in self.listeners.drain(..) {
            if matches(&listener) {
                taken.push(listener.tcs);
            } else {
                kept.push(listener);
            }
        }
        self.listeners = kept;
        taken
    }

    /// The completions of the listeners that accept an event.
    fn take_accepting(&mut self, xev: &XEvent) -> Vec<C> {
        self.take_where(|listener| (listener.filter)(xev))
    }

    /// The completions of the listeners that gave up before `now`.
    fn take_timed_out(&mut self, now: Instant) -> Vec<C> {
        self.take_where(|listener| listener.timeout < now)
    }

    /// The completions of every listener.
    fn take_all(&mut self) -> Vec<C> {
        self.take_where(|_| true)
    }

    /// Whether nobody was waiting before the last merge.
    fn is_empty(&self) -> bool {
        self.listeners.is_empty()
    }
}

type EventCompletion = Rc<TaskCompletionSource<Option<XEvent>>>;

pub struct EventStreamWindow {
    platform: Weak<FerroX11Platform>,
    handle: Cell<XID>,
    listeners: RefCell<Listeners<EventCompletion>>,
    timeout_timer: RefCell<Option<Rc<DispatcherTimer>>>,
    is_foreign: bool,
}

impl EventStreamWindow {
    /// Listens to the events of a window of its own, or of
    /// `foreign_window`: a window of another client, whose events the
    /// caller selects.
    ///
    /// As in the reference, a foreign window takes the place of whatever
    /// the platform knew under that identifier, and is forgotten when the
    /// stream is disposed.
    pub fn new(platform: &Rc<FerroX11Platform>, foreign_window: Option<XID>) -> Rc<Self> {
        let this = Rc::new(Self {
            platform: Rc::downgrade(platform),
            handle: Cell::new(0),
            listeners: RefCell::new(Listeners::new()),
            timeout_timer: RefCell::new(None),
            is_foreign: foreign_window.is_some(),
        });

        let weak = Rc::downgrade(&this);
        let on_event: Rc<dyn Fn(&mut XEvent)> = Rc::new(move |xev: &mut XEvent| {
            if let Some(this) = weak.upgrade() {
                this.on_event(xev);
            }
        });
        match foreign_window {
            Some(foreign_window) => {
                this.handle.set(foreign_window);
                platform.set_window(foreign_window, X11WindowInfo::new(on_event, None));
            }
            None => this.handle.set(create_event_window(platform, on_event)),
        }

        // The timer is started, as the one of the reference is by its
        // constructor; its first tick stops it when nobody waits.
        let weak = Rc::downgrade(&this);
        let timeout_timer =
            DispatcherTimer::with_callback(Duration::from_secs(1), DispatcherPriority::BACKGROUND, move |_| {
                if let Some(this) = weak.upgrade() {
                    this.on_timer();
                }
            });
        *this.timeout_timer.borrow_mut() = Some(timeout_timer);

        this
    }

    pub fn handle(&self) -> XID {
        self.handle.get()
    }

    fn timeout_timer(&self) -> Option<Rc<DispatcherTimer>> {
        self.timeout_timer.borrow().clone()
    }

    fn on_timer(&self) {
        let (timed_out, is_empty) = {
            let mut listeners = self.listeners.borrow_mut();
            let timed_out = listeners.take_timed_out(Instant::now());
            (timed_out, listeners.is_empty())
        };
        // The completions run with the listeners released: whoever waited
        // may wait again at once.
        for tcs in timed_out {
            tcs.try_set_result(None);
        }
        if is_empty {
            if let Some(timeout_timer) = self.timeout_timer() {
                timeout_timer.stop();
            }
        }
    }

    fn on_event(&self, xev: &mut XEvent) {
        // The conditions of the listeners only look at the event; what
        // waits for it continues after the listeners are released.
        let accepting = self.listeners.borrow_mut().take_accepting(xev);
        for tcs in accepting {
            tcs.try_set_result(Some(*xev));
        }
    }
}

impl IXEventWaiter for EventStreamWindow {
    fn wait_for_event_async(&self, predicate: XEventPredicate, timeout: Duration) -> LocalBoxFuture<Option<XEvent>> {
        let tcs = TaskCompletionSource::new();
        self.listeners.borrow_mut().add(predicate, tcs.clone(), Instant::now() + timeout);

        if let Some(timeout_timer) = self.timeout_timer() {
            timeout_timer.start();
        }
        Box::pin(tcs.task())
    }
}

impl IDisposable for EventStreamWindow {
    fn dispose(&self) {
        if let Some(timeout_timer) = self.timeout_timer() {
            timeout_timer.stop();
        }

        // The reference can be disposed once; a second call here does
        // nothing to the server.
        let handle = self.handle.replace(0);
        if handle != 0 {
            if let Some(platform) = self.platform.upgrade() {
                platform.remove_window(handle);
                if self.is_foreign {
                    xlib::x_select_input(platform.display(), handle, 0);
                } else {
                    xlib::x_destroy_window(platform.display(), handle);
                }
            }
        }

        let to_dispose = self.listeners.borrow_mut().take_all();
        for tcs in to_dispose {
            tcs.try_set_result(None);
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file. The window
    // itself needs a server; the tests cover the order in which the
    // listeners of a window are served.
    use super::*;
    use crate::x11_structs::XEventName;

    fn property_notify(atom: xlib::Atom) -> XEvent {
        let mut event = xlib::new_event();
        let property_event = xlib::property_event_mut(&mut event);
        property_event.type_ = XEventName::PropertyNotify as i32;
        property_event.atom = atom;
        event
    }

    fn accepts(atom: xlib::Atom) -> XEventPredicate {
        Box::new(move |event: &XEvent| xlib::property_event(event).atom == atom)
    }

    #[test]
    fn an_event_completes_every_listener_that_accepts_it_in_the_order_they_were_added() {
        let mut listeners = Listeners::<u32>::new();
        let far = Instant::now() + Duration::from_secs(60);
        listeners.add(accepts(1), 10, far);
        listeners.add(accepts(2), 20, far);
        listeners.add(accepts(1), 30, far);

        assert_eq!(listeners.take_accepting(&property_notify(1)), [10, 30]);
        // The listeners that were served no longer wait.
        assert!(listeners.take_accepting(&property_notify(1)).is_empty());
        assert_eq!(listeners.take_accepting(&property_notify(2)), [20]);
        assert!(listeners.is_empty());
    }

    #[test]
    fn listeners_give_up_after_their_timeout() {
        let mut listeners = Listeners::<u32>::new();
        let now = Instant::now();
        listeners.add(accepts(1), 10, now + Duration::from_secs(5));
        listeners.add(accepts(1), 20, now + Duration::from_secs(10));

        assert!(listeners.take_timed_out(now + Duration::from_secs(5)).is_empty());
        assert!(!listeners.is_empty());
        assert_eq!(listeners.take_timed_out(now + Duration::from_secs(6)), [10]);
        assert!(!listeners.is_empty());
        assert_eq!(listeners.take_timed_out(now + Duration::from_secs(11)), [20]);
        assert!(listeners.is_empty());
    }

    #[test]
    fn disposing_releases_every_listener() {
        let mut listeners = Listeners::<u32>::new();
        let far = Instant::now() + Duration::from_secs(60);
        listeners.add(accepts(1), 10, far);
        listeners.merge_listeners();
        listeners.add(accepts(2), 20, far);
        assert_eq!(listeners.take_all(), [10, 20]);
        assert!(listeners.is_empty());
    }
}
