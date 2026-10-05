use ferroui_base::input::platform::{Clipboard, ClipboardError, ClipboardErrorKind, IClipboard, IClipboardImpl};
use ferroui_base::input::{
    AsyncDataTransferExtensions, DataFormat, DataTransfer, DataTransferItem, IAsyncDataTransfer, LocalBoxFuture,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

/// How the operations of a failing [`TestClipboardImpl`] end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TestClipboardFailure {
    /// The operation never completes, like one the platform abandoned.
    NeverCompletes,
    /// The operation panics when it is awaited, like one on a clipboard
    /// that was disposed.
    Panics,
    /// The operation resolves to the error of a kind
    /// ([`ClipboardError::from_kind`]): the failure a platform clipboard
    /// reports (where the reference throws an exception of the matching
    /// type from the awaited task).
    Fails(ClipboardErrorKind),
}

impl TestClipboardFailure {
    /// The error the operations resolve to, for [`Fails`](Self::Fails).
    pub fn error(self) -> Option<ClipboardError> {
        match self {
            TestClipboardFailure::Fails(kind) => Some(ClipboardError::from_kind(kind)),
            TestClipboardFailure::NeverCompletes | TestClipboardFailure::Panics => None,
        }
    }
}

/// An in-memory platform clipboard: it keeps the data placed on it and
/// counts the calls. Every operation completes immediately, unless the
/// clipboard was created with [`failing`](Self::failing).
#[derive(Default)]
pub struct TestClipboardImpl {
    data: RefCell<Option<Rc<dyn IAsyncDataTransfer>>>,
    set_data_count: Cell<usize>,
    try_get_data_count: Cell<usize>,
    clear_count: Cell<usize>,
    failure: Option<TestClipboardFailure>,
}

impl TestClipboardImpl {
    /// Creates an empty clipboard.
    pub fn new() -> Rc<TestClipboardImpl> {
        Rc::new(TestClipboardImpl::default())
    }

    /// Creates a clipboard whose operations count the call and then fail
    /// the way `failure` says, without touching the data.
    pub fn failing(failure: TestClipboardFailure) -> Rc<TestClipboardImpl> {
        Rc::new(TestClipboardImpl { failure: Some(failure), ..TestClipboardImpl::default() })
    }

    /// Creates a clipboard whose operations count the call and then
    /// resolve to the error of `kind`, without touching the data.
    pub fn throwing(kind: ClipboardErrorKind) -> Rc<TestClipboardImpl> {
        Self::failing(TestClipboardFailure::Fails(kind))
    }

    fn fail<T: 'static>(&self) -> Option<LocalBoxFuture<Result<T, ClipboardError>>> {
        match self.failure? {
            TestClipboardFailure::NeverCompletes => Some(Box::pin(std::future::pending())),
            TestClipboardFailure::Panics => Some(Box::pin(async { panic!("The clipboard failed.") })),
            TestClipboardFailure::Fails(kind) => Some(Box::pin(std::future::ready(Err(ClipboardError::from_kind(kind))))),
        }
    }

    /// The clipboard of the toolkit over this platform clipboard, as a
    /// top-level gets it from the optional features of its platform
    /// implementation.
    pub fn clipboard(self: &Rc<Self>) -> Rc<dyn IClipboard> {
        Clipboard::new(self.clone())
    }

    /// How many times data was placed on the clipboard.
    pub fn set_data_count(&self) -> usize {
        self.set_data_count.get()
    }

    /// How many times the data of the clipboard was requested.
    pub fn try_get_data_count(&self) -> usize {
        self.try_get_data_count.get()
    }

    /// How many times the clipboard was cleared.
    pub fn clear_count(&self) -> usize {
        self.clear_count.get()
    }

    /// The data on the clipboard.
    pub fn data(&self) -> Option<Rc<dyn IAsyncDataTransfer>> {
        self.data.borrow().clone()
    }

    /// Puts a text on the clipboard, or empties it, the way another
    /// application would: no call is counted.
    pub fn set_text(&self, text: Option<&str>) {
        *self.data.borrow_mut() = text.map(|text| {
            let data_transfer = DataTransfer::new();
            data_transfer.add(DataTransferItem::create(&DataFormat::text(), Some(text.to_owned())));
            data_transfer as Rc<dyn IAsyncDataTransfer>
        });
    }

    /// The text on the clipboard: `None` when the clipboard is empty, holds
    /// no text or its data fails or does not answer immediately.
    pub fn text(&self) -> Option<String> {
        let data = self.data()?;
        let mut text = data.try_get_value_async(&DataFormat::text());
        match text.as_mut().poll(&mut Context::from_waker(Waker::noop())) {
            Poll::Ready(text) => text.ok().flatten(),
            Poll::Pending => None,
        }
    }
}

impl IClipboardImpl for TestClipboardImpl {
    fn try_get_data_async(&self) -> LocalBoxFuture<Result<Option<Rc<dyn IAsyncDataTransfer>>, ClipboardError>> {
        self.try_get_data_count.set(self.try_get_data_count.get() + 1);
        if let Some(failed) = self.fail() {
            return failed;
        }
        Box::pin(std::future::ready(Ok(self.data())))
    }

    fn set_data_async(&self, data_transfer: Rc<dyn IAsyncDataTransfer>) -> LocalBoxFuture<Result<(), ClipboardError>> {
        self.set_data_count.set(self.set_data_count.get() + 1);
        if let Some(failed) = self.fail() {
            return failed;
        }
        *self.data.borrow_mut() = Some(data_transfer);
        Box::pin(std::future::ready(Ok(())))
    }

    fn clear_async(&self) -> LocalBoxFuture<Result<(), ClipboardError>> {
        self.clear_count.set(self.clear_count.get() + 1);
        if let Some(failed) = self.fail() {
            return failed;
        }
        *self.data.borrow_mut() = None;
        Box::pin(std::future::ready(Ok(())))
    }
}
