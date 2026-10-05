use std::rc::Rc;

use super::dispatcher_operation::DispatcherException;
use super::{Dispatcher, DispatcherUnhandledExceptionEventArgs, DispatcherUnhandledExceptionFilterEventArgs};
use crate::reactive::{Disposable, IDisposable};

fn exception_key(exception: &DispatcherException) -> usize {
    let payload: &(dyn std::any::Any + Send) = &**exception;
    payload as *const (dyn std::any::Any + Send) as *const () as usize
}

impl Dispatcher {
    /// Occurs when a callback posted with [`post`](Self::post) or
    /// [`post_local`](Self::post_local) panics and the panic is not caught.
    ///
    /// A handler can mark the panic as handled, which stops it from being
    /// re-raised on the dispatcher thread. Handlers must be written with
    /// care to avoid secondary panics.
    ///
    /// Callbacks run through `invoke`/`invoke_async` do not raise this
    /// event: their panic is delivered to whoever waits for the operation.
    ///
    /// # Panics
    /// Panics when called from a thread other than the dispatcher thread.
    pub fn unhandled_exception(
        &self,
        handler: impl for<'a> Fn(&DispatcherUnhandledExceptionEventArgs<'a>) + 'static,
    ) -> Rc<dyn IDisposable> {
        self.verify_access();
        let Some(local) = self.try_local() else {
            return Disposable::empty();
        };
        let token = local.unhandled_exception.add(Rc::new(handler));
        let local = Rc::downgrade(&local);
        Disposable::create(move || {
            if let Some(local) = local.upgrade() {
                local.unhandled_exception.remove(token);
            }
        })
    }

    /// Occurs first when a posted callback panics, to decide whether
    /// [`unhandled_exception`](Self::unhandled_exception) is raised at all.
    ///
    /// The filter runs regardless of whether there are
    /// `unhandled_exception` handlers. Setting
    /// [`request_catch`](DispatcherUnhandledExceptionFilterEventArgs::set_request_catch)
    /// to `false` skips the `unhandled_exception` event.
    ///
    /// # Panics
    /// Panics when called from a thread other than the dispatcher thread.
    pub fn unhandled_exception_filter(
        &self,
        handler: impl for<'a> Fn(&DispatcherUnhandledExceptionFilterEventArgs<'a>) + 'static,
    ) -> Rc<dyn IDisposable> {
        self.verify_access();
        let Some(local) = self.try_local() else {
            return Disposable::empty();
        };
        let token = local.unhandled_exception_filter.add(Rc::new(handler));
        let local = Rc::downgrade(&local);
        Disposable::create(move || {
            if let Some(local) = local.upgrade() {
                local.unhandled_exception_filter.remove(token);
            }
        })
    }

    /// Exception filter returns true if exception should be caught.
    pub(crate) fn exception_filter(&self, e: &DispatcherException) -> bool {
        let Some(local) = self.try_local() else {
            return false;
        };

        // see whether this dispatcher has already seen the exception.
        // This can happen when the dispatcher is re-entered via
        // push_frame (or similar).
        let key = exception_key(e);
        {
            let mut seen = local.seen_exceptions.borrow_mut();
            if seen.contains(&key) {
                // we've seen this exception before - don't catch it
                return false;
            }
            // first time we've seen this exception - remember it
            seen.push(key);
        }

        // By default, Request catch if there's anyone signed up to catch it;
        let mut request_catch = !local.unhandled_exception.is_empty();

        // The app can hook up an exception filter to avoid catching it.
        // The filter will run REGARDLESS of whether there are exception handlers.
        if !local.unhandled_exception_filter.is_empty() {
            // The default request_catch value that is passed in the args
            // should be returned unchanged if filters don't set them explicitly.
            let dispatcher = self.to_arc();
            let args = DispatcherUnhandledExceptionFilterEventArgs::new(&dispatcher, &**e, request_catch);
            for (_, handler) in local.unhandled_exception_filter.snapshot().iter() {
                handler(&args);
            }
            request_catch = args.request_catch();
        }

        request_catch
    }

    pub(crate) fn catch_exception(&self, e: &DispatcherException) -> bool {
        let Some(local) = self.try_local() else {
            return false;
        };
        let mut handled = false;

        if !local.unhandled_exception.is_empty() {
            let dispatcher = self.to_arc();
            let args = DispatcherUnhandledExceptionEventArgs::new(&dispatcher, &**e, false);
            for (_, handler) in local.unhandled_exception.snapshot().iter() {
                handler(&args);
            }
            handled = args.handled();
        }

        handled
    }

    /// Returns true, if exception was handled.
    pub(crate) fn try_catch_when(&self, e: &DispatcherException) -> bool {
        if self.exception_filter(e) {
            if !self.catch_exception(e) {
                return false;
            }
        } else {
            return false;
        }

        // The exception stops here and its payload is about to be released:
        // its address must not be mistaken for a later exception.
        self.forget_exception(e);
        true
    }

    /// Removes the "already seen" mark of an exception.
    pub(crate) fn forget_exception(&self, e: &DispatcherException) {
        if let Some(local) = self.try_local() {
            let key = exception_key(e);
            local.seen_exceptions.borrow_mut().retain(|seen| *seen != key);
        }
    }

    /// Removes the mark of an exception that is about to leave the
    /// dispatcher: no operation of this dispatcher is left on the stack that
    /// could see it again.
    pub(crate) fn forget_exception_if_outermost(&self, e: &DispatcherException) {
        if let Some(local) = self.try_local() {
            if local.executing_depth.get() == 0 {
                let key = exception_key(e);
                local.seen_exceptions.borrow_mut().retain(|seen| *seen != key);
            }
        }
    }
}
