//! Clipboard completion must observe edits before a held document batch publishes
//! its Changed event. The platform operation is genuinely pending during each edit.
use super::*;
use ferroui_base::input::platform::{
    Clipboard, ClipboardError, ClipboardErrorKind, IClipboard, IClipboardImpl,
};
use ferroui_base::input::{IAsyncDataTransfer, LocalBoxFuture};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[derive(Default)]
struct ClipboardCompletion {
    released: Cell<bool>,
    waker: RefCell<Option<Waker>>,
}

struct PendingClipboard {
    inner: Rc<crate::testing::TestClipboardImpl>,
    completion: Rc<ClipboardCompletion>,
}

impl PendingClipboard {
    fn new() -> Rc<Self> {
        Rc::new(Self {
            inner: crate::testing::TestClipboardImpl::new(),
            completion: Rc::new(ClipboardCompletion::default()),
        })
    }

    fn clipboard(self: &Rc<Self>) -> Rc<dyn IClipboard> {
        Clipboard::new(self.clone())
    }

    fn wait(&self) -> LocalBoxFuture<()> {
        let completion = self.completion.clone();
        Box::pin(std::future::poll_fn(move |context| {
            if completion.released.get() {
                Poll::Ready(())
            } else {
                *completion.waker.borrow_mut() = Some(context.waker().clone());
                Poll::Pending
            }
        }))
    }

    fn complete(&self) {
        self.completion.released.set(true);
        if let Some(waker) = self.completion.waker.borrow_mut().take() {
            waker.wake();
        }
    }
}

impl IClipboardImpl for PendingClipboard {
    fn try_get_data_async(
        &self,
    ) -> LocalBoxFuture<Result<Option<Rc<dyn IAsyncDataTransfer>>, ClipboardError>> {
        let operation = self.inner.try_get_data_async();
        let completion = self.wait();
        Box::pin(async move {
            completion.await;
            operation.await
        })
    }

    fn set_data_async(
        &self,
        data_transfer: Rc<dyn IAsyncDataTransfer>,
    ) -> LocalBoxFuture<Result<(), ClipboardError>> {
        let operation = self.inner.set_data_async(data_transfer);
        let completion = self.wait();
        Box::pin(async move {
            completion.await;
            operation.await
        })
    }

    fn clear_async(&self) -> LocalBoxFuture<Result<(), ClipboardError>> {
        self.inner.clear_async()
    }
}

#[test]
fn rich_edit_pending_cut_preserves_replacement_before_document_batch_notification() {
    let _scope = Dispatcher::unit_test_scope();
    let clipboard = PendingClipboard::new();
    let control = RichEditBox::new();
    control.set_text("original").unwrap();
    control.select_all();
    let selection = control.editor().selection();
    let document = control.document();
    let revision = document.revision();
    let mutation = document.mutation_version();
    let mut cut = control.copy_to_clipboard_async(clipboard.clipboard(), true);
    assert!(
        cut.as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    assert_eq!(clipboard.inner.set_data_count(), 1);

    let batch = document.begin_edit();
    document.replace_text(0..8, "replaced").unwrap();
    let expected = document.snapshot();
    let history = (control.can_undo(), control.can_redo());
    assert_eq!(document.revision(), revision);
    assert_ne!(document.mutation_version(), mutation);
    assert_eq!(control.editor().selection(), selection);

    clipboard.complete();
    ready(cut).unwrap();
    assert_eq!(clipboard.inner.text().as_deref(), Some("original"));
    assert_eq!(document.text(), "replaced");
    assert_eq!(document.snapshot(), expected);
    assert_eq!(control.editor().selection(), selection);
    assert_eq!((control.can_undo(), control.can_redo()), history);
    assert_eq!(document.revision(), revision);
    drop(batch);
    assert_ne!(document.revision(), revision);
}

#[test]
fn rich_edit_pending_paste_rejects_replacement_before_document_batch_notification() {
    let _scope = Dispatcher::unit_test_scope();
    let clipboard = PendingClipboard::new();
    clipboard.inner.set_text(Some("clipboard"));
    let control = RichEditBox::new();
    control.set_text("original").unwrap();
    control.select_all();
    let selection = control.editor().selection();
    let document = control.document();
    let revision = document.revision();
    let mutation = document.mutation_version();
    let mut paste = control.paste_from_clipboard_async(clipboard.clipboard(), false);
    assert!(
        paste
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    assert_eq!(clipboard.inner.try_get_data_count(), 1);

    let batch = document.begin_edit();
    document.replace_text(0..8, "replaced").unwrap();
    let expected = document.snapshot();
    let history = (control.can_undo(), control.can_redo());
    assert_eq!(document.revision(), revision);
    assert_ne!(document.mutation_version(), mutation);
    assert_eq!(control.editor().selection(), selection);

    clipboard.complete();
    let error = ready(paste).unwrap_err();
    assert_eq!(error.kind(), ClipboardErrorKind::Canceled);
    assert_eq!(document.text(), "replaced");
    assert_eq!(document.snapshot(), expected);
    assert_eq!(control.editor().selection(), selection);
    assert_eq!((control.can_undo(), control.can_redo()), history);
    assert_eq!(document.revision(), revision);
    drop(batch);
    assert_ne!(document.revision(), revision);
}
