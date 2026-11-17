use crate::interop::navigation_helper;
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use wasm_bindgen::JsValue;

type TraversalHandler = dyn Fn(&str, i32);

ferroui_base::ferro_thread_local! {
    static TRAVERSAL_HANDLER: RefCell<Option<Weak<TraversalHandler>>> = const { RefCell::new(None) };
}

/// Opts an app into browser Back and Forward navigation on the page's main thread.
///
/// The browser owns the history cursor. Call [`push`](Self::push) when opening a new
/// route in the app, and render the destination route in the traversal callback.
/// The address stays the same. Routes are stored in browser session history, so
/// use identifiers that are safe to store there.
///
/// Keep this handle alive for the lifetime of the router. Dropping it disconnects
/// the callback. Entries already added to browser history remain in place.
pub struct BrowserNavigationHistory {
    _handler: Rc<TraversalHandler>,
}

impl BrowserNavigationHistory {
    /// Stores `initial_route` in the current browser entry, taking ownership of its
    /// `history.state`. Render the initial route separately. On a browser traversal,
    /// `on_traversed` receives the destination route and the change in app history
    /// position. A negative change means Back and a positive change means Forward.
    /// The browser has already moved to the destination when the callback runs.
    ///
    /// Returns a JavaScript error if a router or the legacy browser Back handler
    /// is already installed, or if the browser refuses to update history.
    pub fn new(
        initial_route: &str,
        on_traversed: impl Fn(&str, i32) + 'static,
    ) -> Result<Self, JsValue> {
        navigation_helper::start_history(initial_route)?;
        let handler: Rc<TraversalHandler> = Rc::new(on_traversed);
        TRAVERSAL_HANDLER.with(|slot| *slot.borrow_mut() = Some(Rc::downgrade(&handler)));
        Ok(Self { _handler: handler })
    }

    /// Adds a new route after the current entry, clearing any Forward entries.
    /// This only records the route. The caller renders it after a successful push.
    ///
    /// Returns a JavaScript error if the current entry belongs to another router
    /// or if the browser refuses to update history.
    pub fn push(&self, route: &str) -> Result<(), JsValue> {
        navigation_helper::push_history(route)
    }

    pub(crate) fn on_traversed(route: &str, delta: i32) {
        let handler = TRAVERSAL_HANDLER.with(|slot| slot.borrow().as_ref().and_then(Weak::upgrade));
        if let Some(handler) = handler {
            handler(route, delta);
        }
    }
}

impl Drop for BrowserNavigationHistory {
    fn drop(&mut self) {
        navigation_helper::stop_history();
        TRAVERSAL_HANDLER.with(|slot| *slot.borrow_mut() = None);
    }
}
