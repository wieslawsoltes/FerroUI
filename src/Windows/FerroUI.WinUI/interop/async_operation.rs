//! An asynchronous operation of the Windows Runtime as a future: what the
//! reference awaits (`await view.GetTextAsync()`, `await StartDragAsync`).
//!
//! The future gives the operation a completion handler when it is first
//! polled. The handler is agile: the operation calls it on the thread it
//! completes on, and all it does there is send the status through a
//! one-shot channel (`ferroui_system::task::oneshot`), which wakes the
//! task. The task of the integration runs on the dispatcher of the UI thread
//! (`Dispatcher::to_task_scheduler().start_local`), whose waker posts to
//! that thread, so the results are read on the UI thread, where the await
//! of the reference resumes too (its synchronization context is the one of
//! the dispatcher queue of WinUI).

use super::*;
use ferroui_microcom::{make_com_with, ComPtr, HResult, InterfaceEntry, E_ABORT, E_FAIL};
use ferroui_system::task::oneshot;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Mutex, PoisonError};
use std::task::{Context, Poll};

/// The sender of the status of an operation, which its completion handler holds.
pub(crate) type CompletionSender = oneshot::Sender<AsyncStatus>;

/// Sends the status the completion handler reports; the operation calls its handler once, and a
/// second call finds the sender taken.
fn complete(completion: &Mutex<Option<CompletionSender>>, status: AsyncStatus) {
    let sender = completion.lock().unwrap_or_else(PoisonError::into_inner).take();
    if let Some(sender) = sender {
        // A future that was dropped does not want the status.
        let _ = sender.send(status);
    }
}

/// An interface `IAsyncOperation<T>` of the bindings.
pub(crate) trait AsyncOperationInterface: ferroui_microcom::Interface {
    type Output;

    /// Gives the operation a completion handler that sends the status.
    fn set_completion(&self, completion: CompletionSender) -> Result<(), HResult>;

    /// `GetResults`.
    fn results(&self) -> Result<Self::Output, HResult>;
}

macro_rules! async_operation {
    ($operation:ident, $handler:ident, $handler_impl:ident, $output:ty) => {
        impl AsyncOperationInterface for $operation {
            type Output = $output;

            fn set_completion(&self, completion: CompletionSender) -> Result<(), HResult> {
                struct Handler(Mutex<Option<CompletionSender>>);

                impl $handler_impl for Handler {
                    fn invoke(&self, _async_info: Option<&$operation>, async_status: AsyncStatus) -> Result<(), HResult> {
                        complete(&self.0, async_status);
                        Ok(())
                    }
                }

                impl IAgileObjectImpl for Handler {}

                let handler: ComPtr<$handler> =
                    make_com_with(Handler(Mutex::new(Some(completion))), [InterfaceEntry::of::<IAgileObject>()]);
                self.set_completed(Some(&handler))
            }

            fn results(&self) -> Result<$output, HResult> {
                self.get_results()
            }
        }
    };
}

async_operation!(
    IAsyncOperationDataPackageOperation,
    AsyncOperationCompletedHandlerDataPackageOperation,
    AsyncOperationCompletedHandlerDataPackageOperationImpl,
    DataPackageOperation
);
async_operation!(
    IAsyncOperationStorageItems,
    AsyncOperationCompletedHandlerStorageItems,
    AsyncOperationCompletedHandlerStorageItemsImpl,
    Option<ComPtr<IVectorViewStorageItem>>
);
async_operation!(IAsyncOperationString, AsyncOperationCompletedHandlerString, AsyncOperationCompletedHandlerStringImpl, isize);
async_operation!(
    IAsyncOperationStorageFile,
    AsyncOperationCompletedHandlerStorageFile,
    AsyncOperationCompletedHandlerStorageFileImpl,
    Option<ComPtr<IStorageFile>>
);
async_operation!(
    IAsyncOperationStorageFolder,
    AsyncOperationCompletedHandlerStorageFolder,
    AsyncOperationCompletedHandlerStorageFolderImpl,
    Option<ComPtr<IStorageFolder>>
);

/// Awaits an operation: its results, or the error it ended with (the
/// exception the await of the reference throws).
pub(crate) fn wait<O: AsyncOperationInterface>(operation: ComPtr<O>) -> AsyncOperationFuture<O> {
    AsyncOperationFuture { operation, status: None }
}

pub(crate) struct AsyncOperationFuture<O: AsyncOperationInterface> {
    operation: ComPtr<O>,
    /// The receiver of the status, from the first poll on.
    status: Option<oneshot::Receiver<AsyncStatus>>,
}

impl<O: AsyncOperationInterface> Unpin for AsyncOperationFuture<O> {}

impl<O: AsyncOperationInterface> Future for AsyncOperationFuture<O> {
    type Output = Result<O::Output, HResult>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = &mut *self;
        if this.status.is_none() {
            let (sender, receiver) = oneshot::channel();
            this.status = Some(receiver);
            // An operation that has completed already calls the handler before this returns.
            if let Err(error) = this.operation.set_completion(sender) {
                return Poll::Ready(Err(error));
            }
        }

        let status = match this.status.as_mut().map(|receiver| Pin::new(receiver).poll(cx)) {
            Some(Poll::Ready(Ok(status))) => status,
            Some(Poll::Pending) => return Poll::Pending,
            // Deviation (DEVIATIONS.md, WinUI integration): a completion handler that the
            // operation released without calling it ends the wait as aborted; the await of the
            // reference would never resume.
            Some(Poll::Ready(Err(_))) | None => return Poll::Ready(Err(HResult(E_ABORT))),
        };
        match status {
            AsyncStatus::Completed => Poll::Ready(this.operation.results()),
            AsyncStatus::Canceled => Poll::Ready(Err(HResult(E_ABORT))),
            _ => {
                let error = this
                    .operation
                    .cast::<IAsyncInfo>()
                    .and_then(|info| info.error_code())
                    .map(|code| if code < 0 { HResult(code as u32) } else { HResult(E_FAIL) })
                    .unwrap_or(HResult(E_FAIL));
                Poll::Ready(Err(error))
            }
        }
    }
}
