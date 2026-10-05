use super::{ClipboardError, IClipboard, IClipboardImpl};
use crate::input::{IAsyncDataTransfer, LocalBoxFuture};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// Implementation of [`IClipboard`] on top of a platform clipboard.
pub struct Clipboard {
    this: Weak<Clipboard>,
    clipboard_impl: Rc<dyn IClipboardImpl>,
    last_data_transfer: RefCell<Option<Rc<dyn IAsyncDataTransfer>>>,
}

impl Clipboard {
    /// Creates the clipboard for a platform implementation.
    pub fn new(clipboard_impl: Rc<dyn IClipboardImpl>) -> Rc<Clipboard> {
        Rc::new_cyclic(|this| Clipboard { this: this.clone(), clipboard_impl, last_data_transfer: RefCell::new(None) })
    }

    fn take_last_data_transfer(&self) -> Option<Rc<dyn IAsyncDataTransfer>> {
        self.last_data_transfer.borrow_mut().take()
    }
}

impl IClipboard for Clipboard {
    fn clear_async(&self) -> LocalBoxFuture<Result<(), ClipboardError>> {
        if let Some(last_data_transfer) = self.take_last_data_transfer() {
            last_data_transfer.dispose();
        }

        self.clipboard_impl.clear_async()
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

        self.clipboard_impl.set_data_async(data_transfer)
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
}
