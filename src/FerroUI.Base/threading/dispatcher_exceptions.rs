use std::cell::{Cell, RefCell};
use std::error::Error;
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

    /// Raises an error of application code that has no caller to return it to: a
    /// callback the framework runs, such as an event handler, a posted job, a
    /// layout or property changed callback, or the handler of a markup event.
    ///
    /// The error goes through [`unhandled_exception_filter`](Self::unhandled_exception_filter)
    /// and [`unhandled_exception`](Self::unhandled_exception), as the panic of a posted
    /// callback does, so a handler can mark it handled; the payload of the event is the
    /// error, a `Box<dyn Error + Send + Sync>`
    /// ([`DispatcherUnhandledExceptionEventArgs::error`]). This is where the callbacks
    /// that return a [`CallbackResult`](super::CallbackResult) send their errors (#567).
    ///
    /// Called off the dispatcher thread, the error is posted to it.
    ///
    /// # Panics
    /// An error no handler handles is fatal, as an unhandled exception is in the
    /// reference: with `panic = "unwind"` its message is logged and unwinds as the
    /// payload (a `String`, as of a panic with it), to the top of the dispatcher
    /// loop without being filtered again; with `panic = "abort"` the process ends
    /// with a panic that carries the message.
    pub fn raise_unhandled_exception(&self, error: impl Into<Box<dyn Error + Send + Sync>>) {
        self.raise_unhandled_error(error.into())
    }

    /// Runs `work` and returns the message of the first exception of application code
    /// that reaches the unhandled-exception events of this dispatcher while it runs and
    /// that no other handler handles: an error raised with
    /// [`raise_unhandled_exception`](Self::raise_unhandled_exception) (by a
    /// [`CallbackResult`](super::CallbackResult) callback, a markup delegate, an
    /// animation an animator rejects) or, in a build that unwinds, the panic of a job.
    /// The exception is marked handled.
    ///
    /// This is how a development tool (the Designer, the previewer, hot reload) fails a
    /// step with the error of the code it previews, without catching a panic (#567).
    /// Only the innermost capture of the thread records an exception.
    ///
    /// # Panics
    /// Panics when called from a thread other than the dispatcher thread.
    pub fn capture_unhandled_exceptions<T>(&self, work: impl FnOnce() -> T) -> Result<T, String> {
        crate::ferro_thread_local! {
            static CAPTURE_DEPTH: Cell<usize> = const { Cell::new(0) };
        }
        struct Capture(Rc<dyn IDisposable>);
        impl Drop for Capture {
            fn drop(&mut self) {
                CAPTURE_DEPTH.set(CAPTURE_DEPTH.get() - 1);
                self.0.dispose();
            }
        }

        self.verify_access();
        let depth = CAPTURE_DEPTH.get() + 1;
        CAPTURE_DEPTH.set(depth);
        let first = Rc::new(RefCell::new(None::<String>));
        let seen = first.clone();
        let capture = Capture(self.unhandled_exception(move |args| {
            if args.handled() || CAPTURE_DEPTH.get() != depth {
                return;
            }
            let mut first = seen.borrow_mut();
            if first.is_none() {
                *first = Some(args.exception_message().unwrap_or("an error of application code").to_owned());
            }
            args.set_handled(true);
        }));
        let result = work();
        drop(capture);
        match first.take() {
            Some(message) => Err(message),
            None => Ok(result),
        }
    }

    #[inline(never)]
    fn raise_unhandled_error(&self, error: Box<dyn Error + Send + Sync>) {
        if !self.check_access() {
            let dispatcher = self.to_arc();
            self.post(move || dispatcher.raise_unhandled_error(error), super::DispatcherPriority::SEND);
            return;
        }
        let exception: DispatcherException = Box::new(error);
        if self.try_catch_when(&exception) {
            return;
        }
        self.forget_exception_if_outermost(&exception);
        fail_unhandled(self, exception)
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

/// Ends an error that no handler of the dispatcher handled (#567).
#[cold]
fn fail_unhandled(dispatcher: &Dispatcher, exception: DispatcherException) -> ! {
    let message = super::dispatcher_unhandled_exception_event_args::exception_text(&*exception)
        .unwrap_or_else(|| "an error of application code".into());
    drop(exception);
    #[cfg(panic = "unwind")]
    {
        // No panic ran the hook for an error value: say what ends the thread.
        match crate::logging::Logger::try_get(crate::logging::LogEventLevel::Error, crate::logging::LogArea::CONTROL) {
            Some(logger) => logger.log_with_values(None, "Unhandled exception: {Error}", &[&message]),
            None => eprintln!("Unhandled exception: {message}"),
        }
        // The payload is the message, as the payload of a panic with it is; the operations of
        // the dispatcher it unwinds through have seen it already.
        let payload: DispatcherException = Box::new(message);
        if let Some(local) = dispatcher.try_local() {
            if local.executing_depth.get() > 0 {
                local.seen_exceptions.borrow_mut().push(exception_key(&payload));
            }
        }
        std::panic::resume_unwind(payload)
    }
    #[cfg(not(panic = "unwind"))]
    {
        let _ = dispatcher;
        panic!("Unhandled exception: {message}")
    }
}
