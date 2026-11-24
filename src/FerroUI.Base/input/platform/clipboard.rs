use super::{
    ClipboardContentOptions, ClipboardError, ClipboardHistoryItemsResult, ClipboardHistoryItemsResultStatus, IClipboard,
    IClipboardImpl,
};
use crate::input::{IAsyncDataTransfer, LocalBoxFuture};
use crate::reactive::{Disposable, Subscription};
use crate::utilities::HandlerList;
use std::cell::RefCell;
use std::future::Future;
use std::rc::{Rc, Weak};

/// Implementation of [`IClipboard`] on top of a platform clipboard.
pub struct Clipboard {
    this: Weak<Clipboard>,
    clipboard_impl: Rc<dyn IClipboardImpl>,
    last_data_transfer: RefCell<Option<Rc<dyn IAsyncDataTransfer>>>,
    /// The handlers of `content_changed`.
    content_changed: HandlerList<dyn Fn()>,
    /// The subscription to the platform's changes while `content_changed` has a handler.
    platform_changes: RefCell<Option<Subscription>>,
}

impl Clipboard {
    /// Creates the clipboard for a platform implementation.
    pub fn new(clipboard_impl: Rc<dyn IClipboardImpl>) -> Rc<Clipboard> {
        Rc::new_cyclic(|this| Clipboard {
            this: this.clone(),
            clipboard_impl,
            content_changed: HandlerList::new(),
            platform_changes: RefCell::new(None),
            last_data_transfer: RefCell::new(None) })
    }

    fn take_last_data_transfer(&self) -> Option<Rc<dyn IAsyncDataTransfer>> {
        self.last_data_transfer.borrow_mut().take()
    }

    fn raise_content_changed(&self) {
        for (_, handler) in self.content_changed.snapshot().iter() {
            handler();
        }
    }

    /// Reports a change this clipboard made, where the platform does not report it: what
    /// `operation` resolves to, once it succeeded.
    fn report_own_change(
        &self,
        operation: LocalBoxFuture<Result<(), ClipboardError>>,
    ) -> LocalBoxFuture<Result<(), ClipboardError>> {
        if self.clipboard_impl.as_clipboard_change_impl().is_some() || self.content_changed.is_empty() {
            return operation;
        }
        let this = self.this.clone();
        Box::pin(ReportOwnChange { operation, this })
    }
}

/// An operation of the clipboard that reports the change it made when it succeeds.
struct ReportOwnChange {
    operation: LocalBoxFuture<Result<(), ClipboardError>>,
    this: Weak<Clipboard>,
}

impl Future for ReportOwnChange {
    type Output = Result<(), ClipboardError>;

    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        let result = std::task::ready!(self.operation.as_mut().poll(cx));
        if let (true, Some(this)) = (result.is_ok(), self.this.upgrade()) {
            this.raise_content_changed();
        }
        std::task::Poll::Ready(result)
    }
}

impl IClipboard for Clipboard {
    fn clear_async(&self) -> LocalBoxFuture<Result<(), ClipboardError>> {
        if let Some(last_data_transfer) = self.take_last_data_transfer() {
            last_data_transfer.dispose();
        }

        self.report_own_change(self.clipboard_impl.clear_async())
    }

    fn set_data_async(
        &self,
        data_transfer: Option<Rc<dyn IAsyncDataTransfer>>,
    ) -> LocalBoxFuture<Result<(), ClipboardError>> {
        let Some(data_transfer) = data_transfer else {
            return self.clear_async();
        };

        if self.clipboard_impl.as_owned_clipboard_impl().is_some() {
            let old = self.last_data_transfer.replace(Some(data_transfer.clone()));
            drop(old);
        }

        self.report_own_change(self.clipboard_impl.set_data_async(data_transfer))
    }

    fn flush_async(&self) -> LocalBoxFuture<Result<(), ClipboardError>> {
        match self.clipboard_impl.as_flushable_clipboard_impl() {
            Some(flushable) => flushable.flush_async(),
            None => Box::pin(std::future::ready(Ok(()))),
        }
    }

    fn try_get_data_async(&self) -> LocalBoxFuture<Result<Option<Rc<dyn IAsyncDataTransfer>>, ClipboardError>> {
        self.clipboard_impl.try_get_data_async()
    }

    fn try_get_in_process_data_async(
        &self,
    ) -> LocalBoxFuture<Result<Option<Rc<dyn IAsyncDataTransfer>>, ClipboardError>> {
        if self.last_data_transfer.borrow().is_none() {
            return Box::pin(std::future::ready(Ok(None)));
        }

        let Some(owned_clipboard_impl) = self.clipboard_impl.as_owned_clipboard_impl() else {
            return Box::pin(std::future::ready(Ok(None)));
        };

        let is_current_owner = owned_clipboard_impl.is_current_owner_async();
        let this = self.this.clone();

        Box::pin(async move {
            // A failed ownership query leaves the kept data transfer in place.
            let is_current_owner = is_current_owner.await?;
            let Some(this) = this.upgrade() else {
                return Ok(None);
            };

            if !is_current_owner {
                drop(this.take_last_data_transfer());
            }

            let result = this.last_data_transfer.borrow().clone();
            Ok(result)
        })
    }

    fn content_changed(&self, handler: Rc<dyn Fn()>) -> Subscription {
        let first = self.content_changed.is_empty();
        let token = self.content_changed.add(handler);
        let changes = if first { self.clipboard_impl.as_clipboard_change_impl() } else { None };
        if let Some(changes) = changes {
            let this = self.this.clone();
            let subscription = changes.subscribe_content_changed(Rc::new(move || {
                if let Some(this) = this.upgrade() {
                    this.raise_content_changed();
                }
            }));
            self.platform_changes.replace(Some(subscription));
        }
        let this = self.this.clone();
        Subscription::new(Disposable::create(move || {
            let Some(this) = this.upgrade() else { return };
            this.content_changed.remove(token);
            if this.content_changed.is_empty() {
                // The platform stops observing the clipboard.
                drop(this.platform_changes.take());
            }
        }))
    }

    fn is_content_changed_supported(&self) -> bool {
        self.clipboard_impl.as_clipboard_change_impl().is_some()
    }

    fn is_history_enabled(&self) -> bool {
        self.clipboard_impl.as_clipboard_history_impl().is_some_and(|history| history.is_history_enabled())
    }

    fn is_roaming_enabled(&self) -> bool {
        self.clipboard_impl.as_clipboard_history_impl().is_some_and(|history| history.is_roaming_enabled())
    }

    fn set_data_with_options_async(
        &self,
        data_transfer: Option<Rc<dyn IAsyncDataTransfer>>,
        options: ClipboardContentOptions,
    ) -> LocalBoxFuture<Result<(), ClipboardError>> {
        let (Some(data_transfer), Some(history)) = (data_transfer.clone(), self.clipboard_impl.as_clipboard_history_impl())
        else {
            return self.set_data_async(data_transfer);
        };
        if self.clipboard_impl.as_owned_clipboard_impl().is_some() {
            drop(self.last_data_transfer.replace(Some(data_transfer.clone())));
        }
        self.report_own_change(history.set_data_with_options_async(data_transfer, options))
    }

    fn get_history_items_async(&self) -> LocalBoxFuture<Result<ClipboardHistoryItemsResult, ClipboardError>> {
        match self.clipboard_impl.as_clipboard_history_impl() {
            Some(history) => history.get_history_items_async(),
            None => Box::pin(std::future::ready(Ok(ClipboardHistoryItemsResult::empty(
                ClipboardHistoryItemsResultStatus::NotSupported,
            )))),
        }
    }
}
