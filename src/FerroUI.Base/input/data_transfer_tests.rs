//! Tests for data formats, data transfers, their sync/async adapters and
//! the clipboard.

use super::platform::*;
use super::*;
use crate::reactive::IDisposable;
use crate::threading::{Dispatcher, DispatcherPriority};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::pin::Pin;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

fn poll_once<T>(future: &mut LocalBoxFuture<T>) -> Poll<T> {
    future.as_mut().poll(&mut Context::from_waker(Waker::noop()))
}

/// Resolves a future that is expected to complete synchronously.
fn ready_result<T>(mut future: LocalBoxFuture<T>) -> T {
    match poll_once(&mut future) {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("the future is not ready"),
    }
}

/// Resolves an operation that is expected to complete synchronously and to
/// succeed.
fn ready<T>(future: LocalBoxFuture<Result<T, ClipboardError>>) -> T {
    match ready_result(future) {
        Ok(value) => value,
        Err(error) => panic!("the operation failed: {error}"),
    }
}

fn panics(f: impl FnOnce()) -> bool {
    catch_unwind(AssertUnwindSafe(f)).is_err()
}

// --- DataFormat -----------------------------------------------------------------

#[test]
fn create_in_process_format_returns_format_with_in_process_kind() {
    let format = DataFormat::create_in_process_format::<String>("my-format");

    assert_eq!(DataFormatKind::InProcess, format.kind());
}

#[test]
fn create_in_process_format_returns_format_with_correct_identifier() {
    let format = DataFormat::create_in_process_format::<String>("my-format");

    assert_eq!("my-format", format.identifier());
}

#[test]
fn create_in_process_format_panics_on_empty_identifier() {
    assert!(panics(|| {
        DataFormat::create_in_process_format::<String>("");
    }));
}

#[test]
fn create_in_process_format_allows_non_ascii_identifiers() {
    let format = DataFormat::create_in_process_format::<String>("日本語フォーマット");

    assert_eq!("日本語フォーマット", format.identifier());
}

#[test]
fn to_system_name_panics_for_in_process() {
    let format = DataFormat::create_in_process_format::<String>("test");

    assert!(panics(|| {
        format.to_system_name("prefix.");
    }));
}

#[test]
fn in_process_format_equality_same_identifier() {
    let format1 = DataFormat::create_in_process_format::<String>("my-format");
    let format2 = DataFormat::create_in_process_format::<String>("my-format");

    assert_eq!(format1, format2);
    assert!(format1 == format2);
}

#[test]
fn in_process_format_inequality_different_identifier() {
    let format1 = DataFormat::create_in_process_format::<String>("format-a");
    let format2 = DataFormat::create_in_process_format::<String>("format-b");

    assert_ne!(format1, format2);
    assert!(format1 != format2);
}

#[test]
fn in_process_format_inequality_different_kind_same_identifier() {
    let in_process = DataFormat::create_in_process_format::<String>("test-format");
    let application = DataFormat::create_string_application_format("test-format");

    assert_ne!(in_process.as_data_format(), application.as_data_format());
}

#[test]
fn try_get_raw_with_mismatched_format_returns_none_for_single_format_item() {
    let item = DataTransferItem::create_text(Some("hello"));

    let result = item.try_get_raw(&DataFormat::bitmap());

    assert!(result.is_none());
}

#[test]
fn in_process_format_works_with_data_transfer_item_set_and_get() {
    let format = DataFormat::create_in_process_format::<String>("my-inprocess");
    let item = DataTransferItem::new();
    item.set(&format, Some("hello".to_string()));

    let value = item.try_get_value(&format);

    assert_eq!(Some("hello".to_string()), value);
}

#[test]
fn in_process_format_coexists_with_other_formats_in_data_transfer() {
    let in_process_format = DataFormat::create_in_process_format::<String>("my-inprocess");
    let item = DataTransferItem::new();
    item.set_text(Some("plain text"));
    item.set(&in_process_format, Some("in-process data".to_string()));

    let data_transfer = DataTransfer::new();
    data_transfer.add(item.clone());

    assert!(data_transfer.formats().contains(DataFormat::text().as_data_format()));
    assert!(data_transfer.formats().contains(in_process_format.as_data_format()));
    assert_eq!(Some("plain text".to_string()), item.try_get_value(&DataFormat::text()));
    assert_eq!(Some("in-process data".to_string()), item.try_get_value(&in_process_format));
}

#[test]
fn universal_formats_have_stable_identity_and_display() {
    assert_eq!(DataFormat::text(), DataFormat::text());
    assert_eq!(DataFormatKind::Universal, DataFormat::text().kind());
    assert_eq!("Universal: Text", DataFormat::text().to_string());
    assert_eq!("Universal: Bitmap", DataFormat::bitmap().to_string());
    assert!(DataFormat::text().as_data_format() != DataFormat::bitmap().as_data_format());
    assert!(panics(|| {
        DataFormat::text().to_system_name("prefix.");
    }));
}

#[test]
fn application_format_identifier_is_validated() {
    let format = DataFormat::create_bytes_application_format("My.format-1");
    assert_eq!(DataFormatKind::Application, format.kind());
    assert_eq!("prefix.My.format-1", format.to_system_name("prefix."));

    assert!(panics(|| {
        DataFormat::create_bytes_application_format("");
    }));
    assert!(panics(|| {
        DataFormat::create_string_application_format("with space");
    }));
    assert!(panics(|| {
        DataFormat::create_string_application_format("with/slash");
    }));
    assert!(panics(|| {
        DataFormat::create_string_application_format("ünïcode");
    }));
}

#[test]
fn platform_format_identifier_is_passed_as_is() {
    let format = DataFormat::create_string_platform_format("text/html; charset=utf-8");
    assert_eq!(DataFormatKind::Platform, format.kind());
    assert_eq!("text/html; charset=utf-8", format.to_system_name("prefix."));

    assert!(panics(|| {
        DataFormat::create_bytes_platform_format("");
    }));
}

#[test]
fn from_system_name_recognizes_the_application_prefix() {
    let format = DataFormat::from_system_name::<String>("APP.prefix.my-format", "app.prefix.");
    assert_eq!(DataFormatKind::Application, format.kind());
    assert_eq!("my-format", format.identifier());

    // An invalid application identifier is a platform format.
    let format = DataFormat::from_system_name::<String>("app.prefix.my format", "app.prefix.");
    assert_eq!(DataFormatKind::Platform, format.kind());
    assert_eq!("app.prefix.my format", format.identifier());

    // The prefix alone is not an application format.
    let format = DataFormat::from_system_name::<String>("app.prefix.", "app.prefix.");
    assert_eq!(DataFormatKind::Platform, format.kind());

    let format = DataFormat::from_system_name::<Rc<[u8]>>("public.png", "app.prefix.");
    assert_eq!(DataFormatKind::Platform, format.kind());
    assert_eq!("public.png", format.identifier());

    // Round trip.
    let application = DataFormat::create_bytes_application_format("x-data");
    let round_trip = DataFormat::from_system_name::<Rc<[u8]>>(&application.to_system_name("com.app."), "com.app.");
    assert_eq!(application, round_trip);
}

// --- DataTransferItem / DataTransfer --------------------------------------------

#[test]
fn data_transfer_item_set_replaces_and_removes_formats() {
    let bytes = DataFormat::create_bytes_application_format("bytes");
    let item = DataTransferItem::new();
    assert!(item.formats().is_empty());

    item.set_text(Some("a"));
    assert_eq!(1, item.formats().len());

    item.set_text(Some("b"));
    assert_eq!(1, item.formats().len());
    assert_eq!(Some("b".to_string()), item.try_get_text());

    item.set(&bytes, Some(Rc::from(vec![1u8, 2, 3])));
    assert_eq!(2, item.formats().len());
    assert_eq!(item.formats()[0], DataFormat::text());
    assert_eq!(item.formats()[1], bytes);
    assert_eq!(Some(vec![1u8, 2, 3]), item.try_get_value(&bytes).map(|value| value.to_vec()));

    // Removing a format that is not there changes nothing.
    item.set(&DataFormat::bitmap(), None);
    assert_eq!(2, item.formats().len());

    item.set_text(None);
    assert_eq!(1, item.formats().len());
    assert!(item.try_get_text().is_none());
    assert!(!DataTransferItem::contains(&item, &DataFormat::text()));
    assert!(DataTransferItem::contains(&item, &bytes));

    item.set(&bytes, None);
    assert!(item.formats().is_empty());
}

#[test]
fn data_transfer_item_value_can_be_created_on_demand() {
    let calls = Rc::new(Cell::new(0));
    let item = {
        let calls = calls.clone();
        DataTransferItem::create_with(&DataFormat::text(), move || {
            calls.set(calls.get() + 1);
            Some(format!("call {}", calls.get()))
        })
    };

    assert_eq!(0, calls.get());
    assert_eq!(Some("call 1".to_string()), item.try_get_text());
    assert_eq!(Some("call 2".to_string()), item.try_get_text());

    let async_item: Rc<dyn IAsyncDataTransferItem> = item;
    assert_eq!(Some("call 3".to_string()), ready(async_item.try_get_text_async()));
}

#[test]
fn data_transfer_item_value_of_another_type_is_not_returned() {
    let format = DataFormat::create_in_process_format::<String>("shared");
    let same_format_other_type = DataFormat::create_in_process_format::<i32>("shared");
    let item = DataTransferItem::create(&format, Some("text".to_string()));

    assert!(item.try_get_raw(&same_format_other_type).is_some());
    assert_eq!(None, item.try_get_value(&same_format_other_type));
}

#[test]
fn data_transfer_formats_are_distinct_and_follow_the_items() {
    let custom = DataFormat::create_string_application_format("custom");
    let data_transfer = DataTransfer::new();
    assert!(data_transfer.formats().is_empty());
    assert!(data_transfer.items().is_empty());

    data_transfer.add(DataTransferItem::create_text(Some("first")));
    assert_eq!(1, data_transfer.formats().len());

    let second = DataTransferItem::create(&custom, Some("custom".to_string()));
    second.set_text(Some("second"));
    data_transfer.add(second);

    let formats = data_transfer.formats();
    assert_eq!(2, formats.len());
    assert_eq!(formats[0], DataFormat::text());
    assert_eq!(formats[1], custom);
    assert_eq!(2, data_transfer.items().len());

    let sync: Rc<dyn IDataTransfer> = data_transfer.clone();
    assert!(sync.contains(&custom));
    assert!(!sync.contains(&DataFormat::bitmap()));
    assert_eq!(2, sync.get_items(&DataFormat::text()).count());
    assert_eq!(1, sync.get_items(&custom).count());
    assert_eq!(Some("first".to_string()), sync.try_get_text());
    assert_eq!(Some(vec!["first".to_string(), "second".to_string()]), sync.try_get_values(&DataFormat::text()));
    assert_eq!(Some("custom".to_string()), sync.try_get_value(&custom));
    assert!(sync.try_get_bitmap().is_none());
    assert!(sync.try_get_values(&DataFormat::bitmap()).is_none());

    let asynchronous: Rc<dyn IAsyncDataTransfer> = data_transfer;
    assert!(asynchronous.contains(&custom));
    assert_eq!(2, asynchronous.get_items(&DataFormat::text()).count());
    assert_eq!(Some("first".to_string()), ready(asynchronous.try_get_text_async()));
    assert_eq!(
        Some(vec!["first".to_string(), "second".to_string()]),
        ready(asynchronous.try_get_values_async(&DataFormat::text()))
    );
    assert!(ready(asynchronous.try_get_bitmap_async()).is_none());
    assert!(ready(asynchronous.try_get_values_async(&DataFormat::bitmap())).is_none());
}

// --- PlatformDataTransferItem ---------------------------------------------------

struct TestPlatformDataTransferItem(Vec<DataFormat>);

impl PlatformDataTransferItemImpl for TestPlatformDataTransferItem {
    fn provide_formats(&self) -> Result<Vec<DataFormat>, ClipboardError> {
        Ok(self.0.clone())
    }

    fn try_get_raw_core(&self, format: &DataFormat) -> Result<Option<Rc<dyn Any>>, ClipboardError> {
        Ok(Some(Rc::new(format.clone())))
    }
}

fn is_format(value: Option<Rc<dyn Any>>, format: &DataFormat) -> bool {
    value.is_some_and(|value| value.downcast_ref::<DataFormat>() == Some(format))
}

#[test]
fn try_get_raw_should_return_none_when_format_is_unknown() {
    let format = DataFormat::create_bytes_application_format("test-format");
    let item = PlatformDataTransferItem::new(TestPlatformDataTransferItem(vec![]));

    let value = item.try_get_raw(&format);

    assert!(value.is_none());
}

#[test]
fn try_get_raw_should_return_expected_value_when_format_is_known() {
    let format = DataFormat::create_bytes_application_format("test-format");
    let item = PlatformDataTransferItem::new(TestPlatformDataTransferItem(vec![format.as_data_format().clone()]));

    let value = item.try_get_raw(&format);

    assert!(is_format(value, &format));
}

#[test]
fn try_get_raw_async_should_return_none_when_format_is_unknown() {
    let format = DataFormat::create_bytes_application_format("test-format");
    let item = PlatformDataTransferItem::new(TestPlatformDataTransferItem(vec![]));

    let value = ready(IAsyncDataTransferItem::try_get_raw_async(&*item, &format));

    assert!(value.is_none());
}

#[test]
fn try_get_raw_async_should_return_expected_value_when_format_is_known() {
    let format = DataFormat::create_bytes_application_format("test-format");
    let item = PlatformDataTransferItem::new(TestPlatformDataTransferItem(vec![format.as_data_format().clone()]));

    let value = ready(IAsyncDataTransferItem::try_get_raw_async(&*item, &format));

    assert!(is_format(value, &format));
}

#[test]
fn platform_data_transfer_item_create_holds_a_single_format() {
    let format = DataFormat::create_string_platform_format("text/plain");
    let item = PlatformDataTransferItem::create(&format, "value".to_string());

    assert_eq!(1, item.formats().len());
    assert!(item.contains(&format));
    assert_eq!(Some("value".to_string()), IDataTransferItem::try_get_raw(&*item, &format).and_then(|v| v.downcast_ref::<String>().cloned()));
    assert!(item.try_get_raw(&DataFormat::text()).is_none());
}

struct TestPlatformDataTransfer {
    provided: Rc<Cell<(u32, u32)>>,
    disposed: Rc<Cell<Option<(bool, bool)>>>,
}

impl PlatformDataTransferImpl for TestPlatformDataTransfer {
    fn provide_formats(&self) -> Result<Vec<DataFormat>, ClipboardError> {
        let (formats, items) = self.provided.get();
        self.provided.set((formats + 1, items));
        Ok(vec![DataFormat::text().into()])
    }

    fn provide_items(&self) -> Result<Vec<Rc<PlatformDataTransferItem>>, ClipboardError> {
        let (formats, items) = self.provided.get();
        self.provided.set((formats, items + 1));
        Ok(vec![PlatformDataTransferItem::create(&DataFormat::text(), "platform".to_string())])
    }

    fn dispose(&self, owner: &PlatformDataTransfer) {
        self.disposed.set(Some((owner.are_formats_initialized(), owner.are_items_initialized())));
    }
}

#[test]
fn platform_data_transfer_provides_formats_and_items_once() {
    let provided = Rc::new(Cell::new((0, 0)));
    let disposed = Rc::new(Cell::new(None));
    let data_transfer =
        PlatformDataTransfer::new(TestPlatformDataTransfer { provided: provided.clone(), disposed: disposed.clone() });

    assert!(!data_transfer.are_formats_initialized());
    assert!(!data_transfer.are_items_initialized());

    assert_eq!(1, data_transfer.formats().len());
    assert_eq!(1, data_transfer.formats().len());
    assert!(data_transfer.are_formats_initialized());
    assert!(!data_transfer.are_items_initialized());

    let sync: Rc<dyn IDataTransfer> = data_transfer.clone();
    assert_eq!(Some("platform".to_string()), sync.try_get_text());
    assert_eq!(Some("platform".to_string()), sync.try_get_text());
    assert_eq!((1, 1), provided.get());

    // It is its own asynchronous counterpart.
    let asynchronous = sync.clone().to_asynchronous();
    assert_eq!(Some("platform".to_string()), ready(asynchronous.try_get_text_async()));
    assert_eq!((1, 1), provided.get());

    sync.dispose();
    assert_eq!(Some((true, true)), disposed.get());
}

// --- sync / async adapters ------------------------------------------------------

/// A value that becomes available later: completes the futures created
/// from it when set.
#[derive(Clone, Default)]
struct Completion {
    value: Rc<RefCell<Option<Result<Rc<dyn Any>, ClipboardError>>>>,
    waker: Rc<RefCell<Option<Waker>>>,
}

impl Completion {
    fn set(&self, value: Rc<dyn Any>) {
        self.complete(Ok(value));
    }

    fn fail(&self, error: ClipboardError) {
        self.complete(Err(error));
    }

    fn complete(&self, value: Result<Rc<dyn Any>, ClipboardError>) {
        *self.value.borrow_mut() = Some(value);
        if let Some(waker) = self.waker.borrow_mut().take() {
            waker.wake();
        }
    }
}

impl Future for Completion {
    type Output = Result<Option<Rc<dyn Any>>, ClipboardError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.value.borrow().clone() {
            Some(value) => Poll::Ready(value.map(Some)),
            None => {
                *self.waker.borrow_mut() = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

struct AsyncOnlyItem {
    formats: Vec<DataFormat>,
    completion: Option<Completion>,
    requests: Cell<u32>,
}

impl IAsyncDataTransferItem for AsyncOnlyItem {
    fn formats(&self) -> Rc<[DataFormat]> {
        Rc::from(self.formats.clone())
    }

    fn try_get_raw_async(&self, format: &DataFormat) -> LocalBoxFuture<Result<Option<Rc<dyn Any>>, ClipboardError>> {
        self.requests.set(self.requests.get() + 1);
        if !self.formats.contains(format) {
            return Box::pin(std::future::ready(Ok(None)));
        }
        match &self.completion {
            Some(completion) => {
                let completion = completion.clone();
                Box::pin(async move { completion.await })
            }
            None => Box::pin(std::future::ready(Ok(Some(Rc::new("async text".to_string()) as Rc<dyn Any>)))),
        }
    }
}

struct AsyncOnlyDataTransfer {
    items: Vec<Rc<AsyncOnlyItem>>,
    disposed: Cell<u32>,
}

impl AsyncOnlyDataTransfer {
    fn new(items: Vec<Rc<AsyncOnlyItem>>) -> Rc<Self> {
        Rc::new(Self { items, disposed: Cell::new(0) })
    }

    fn text(completion: Option<Completion>) -> Rc<Self> {
        Self::new(vec![Rc::new(AsyncOnlyItem {
            formats: vec![DataFormat::text().into()],
            completion,
            requests: Cell::new(0),
        })])
    }
}

impl IDisposable for AsyncOnlyDataTransfer {
    fn dispose(&self) {
        self.disposed.set(self.disposed.get() + 1);
    }
}

impl IAsyncDataTransfer for AsyncOnlyDataTransfer {
    fn formats(&self) -> Rc<[DataFormat]> {
        self.items.iter().flat_map(|item| item.formats.clone()).collect()
    }

    fn items(&self) -> Rc<[Rc<dyn IAsyncDataTransferItem>]> {
        self.items.iter().map(|item| item.clone() as Rc<dyn IAsyncDataTransferItem>).collect()
    }
}

#[test]
fn to_synchronous_returns_the_object_itself_when_it_is_synchronous() {
    let data_transfer = DataTransfer::new();
    data_transfer.add(DataTransferItem::create_text(Some("text")));

    let asynchronous: Rc<dyn IAsyncDataTransfer> = data_transfer.clone();
    let sync = asynchronous.clone().to_synchronous("Test");
    assert!(std::ptr::addr_eq(Rc::as_ptr(&sync), Rc::as_ptr(&data_transfer)));

    let back = sync.to_asynchronous();
    assert!(std::ptr::addr_eq(Rc::as_ptr(&back), Rc::as_ptr(&data_transfer)));
}

#[test]
fn to_synchronous_wraps_asynchronous_only_data_transfers() {
    let data_transfer = AsyncOnlyDataTransfer::text(None);
    let asynchronous: Rc<dyn IAsyncDataTransfer> = data_transfer.clone();

    let sync = asynchronous.to_synchronous("Test");
    assert!(!std::ptr::addr_eq(Rc::as_ptr(&sync), Rc::as_ptr(&data_transfer)));
    assert_eq!(1, sync.formats().len());
    assert_eq!(1, sync.items().len());
    // The wrapped items are created once.
    assert!(Rc::ptr_eq(&sync.items()[0], &sync.items()[0]));
    assert_eq!(Some("async text".to_string()), sync.try_get_text());
    assert!(sync.try_get_bitmap().is_none());

    // The wrapper is both: its asynchronous side is the original.
    let back = sync.clone().to_asynchronous();
    assert!(std::ptr::addr_eq(Rc::as_ptr(&back), Rc::as_ptr(&sync)));
    assert!(std::ptr::addr_eq(Rc::as_ptr(&back.items()[0]), Rc::as_ptr(&data_transfer.items[0])));

    sync.dispose();
    assert_eq!(1, data_transfer.disposed.get());
}

#[test]
fn synchronous_wrapper_waits_on_the_dispatcher_for_pending_values() {
    let _scope = Dispatcher::unit_test_scope();
    let dispatcher = Dispatcher::ui_thread();
    assert!(dispatcher.supports_run_loops());

    let completion = Completion::default();
    let data_transfer: Rc<dyn IAsyncDataTransfer> = AsyncOnlyDataTransfer::text(Some(completion.clone()));
    let sync = data_transfer.to_synchronous("Test");

    // The value arrives from a dispatcher job: the wrapper has to keep the
    // dispatcher running while it waits.
    let ran = Arc::new(Mutex::new(false));
    {
        let ran = ran.clone();
        dispatcher.post(move || *ran.lock().unwrap() = true, DispatcherPriority::NORMAL);
    }

    // Complete from within the nested frame through a local task.
    let _task = dispatcher.invoke_async_task_local(move || async move {
        Dispatcher::yield_now().await;
        completion.set(Rc::new("late text".to_string()));
    });

    assert_eq!(Some("late text".to_string()), sync.try_get_text());
    assert!(*ran.lock().unwrap());
}

struct SyncOnlyDataTransfer {
    item: Rc<DataTransferItem>,
    disposed: Cell<u32>,
}

impl IDisposable for SyncOnlyDataTransfer {
    fn dispose(&self) {
        self.disposed.set(self.disposed.get() + 1);
    }
}

impl IDataTransfer for SyncOnlyDataTransfer {
    fn formats(&self) -> Rc<[DataFormat]> {
        self.item.formats()
    }

    fn items(&self) -> Rc<[Rc<dyn IDataTransferItem>]> {
        Rc::from(vec![self.item.clone() as Rc<dyn IDataTransferItem>])
    }
}

#[test]
fn to_asynchronous_wraps_synchronous_only_data_transfers() {
    let data_transfer =
        Rc::new(SyncOnlyDataTransfer { item: DataTransferItem::create_text(Some("sync text")), disposed: Cell::new(0) });
    let sync: Rc<dyn IDataTransfer> = data_transfer.clone();

    let asynchronous = sync.to_asynchronous();
    assert!(!std::ptr::addr_eq(Rc::as_ptr(&asynchronous), Rc::as_ptr(&data_transfer)));
    assert_eq!(1, asynchronous.formats().len());
    assert!(Rc::ptr_eq(&asynchronous.items()[0], &asynchronous.items()[0]));
    assert_eq!(Some("sync text".to_string()), ready(asynchronous.try_get_text_async()));

    let back = asynchronous.clone().to_synchronous("Test");
    assert!(std::ptr::addr_eq(Rc::as_ptr(&back), Rc::as_ptr(&asynchronous)));
    assert_eq!(Some("sync text".to_string()), back.try_get_text());

    asynchronous.dispose();
    assert_eq!(1, data_transfer.disposed.get());
}

#[test]
fn try_get_values_async_queries_the_items_one_after_the_other() {
    let first = Completion::default();
    let second = Completion::default();
    let items: Vec<Rc<AsyncOnlyItem>> = [&first, &second]
        .into_iter()
        .map(|completion| {
            Rc::new(AsyncOnlyItem {
                formats: vec![DataFormat::text().into()],
                completion: Some(completion.clone()),
                requests: Cell::new(0),
            })
        })
        .collect();
    let data_transfer = AsyncOnlyDataTransfer::new(items.clone());

    let mut values = data_transfer.try_get_values_async(&DataFormat::text());
    assert!(poll_once(&mut values).is_pending());
    assert_eq!((1, 0), (items[0].requests.get(), items[1].requests.get()));

    first.set(Rc::new("one".to_string()));
    assert!(poll_once(&mut values).is_pending());
    assert_eq!((1, 1), (items[0].requests.get(), items[1].requests.get()));

    second.set(Rc::new("two".to_string()));
    assert_eq!(Poll::Ready(Ok(Some(vec!["one".to_string(), "two".to_string()]))), poll_once(&mut values));
}

// --- Clipboard ------------------------------------------------------------------

#[derive(Default)]
struct TestClipboardImpl {
    owned: bool,
    flushable: bool,
    data: RefCell<Option<Rc<dyn IAsyncDataTransfer>>>,
    is_owner: Cell<bool>,
    calls: RefCell<Vec<&'static str>>,
    /// What every operation fails with, while set.
    failure: RefCell<Option<ClipboardError>>,
}

impl TestClipboardImpl {
    /// The outcome of an operation that would deliver `value`.
    fn outcome<T: 'static>(&self, value: impl FnOnce() -> T) -> LocalBoxFuture<Result<T, ClipboardError>> {
        let result = match self.failure.borrow().clone() {
            Some(error) => Err(error),
            None => Ok(value()),
        };
        Box::pin(std::future::ready(result))
    }
}

impl IClipboardImpl for TestClipboardImpl {
    fn try_get_data_async(&self) -> LocalBoxFuture<Result<Option<Rc<dyn IAsyncDataTransfer>>, ClipboardError>> {
        self.calls.borrow_mut().push("get");
        self.outcome(|| self.data.borrow().clone())
    }

    fn set_data_async(&self, data_transfer: Rc<dyn IAsyncDataTransfer>) -> LocalBoxFuture<Result<(), ClipboardError>> {
        self.calls.borrow_mut().push("set");
        self.outcome(|| {
            *self.data.borrow_mut() = Some(data_transfer);
            self.is_owner.set(true);
        })
    }

    fn clear_async(&self) -> LocalBoxFuture<Result<(), ClipboardError>> {
        self.calls.borrow_mut().push("clear");
        self.outcome(|| *self.data.borrow_mut() = None)
    }

    fn as_flushable_clipboard_impl(&self) -> Option<&dyn IFlushableClipboardImpl> {
        if self.flushable {
            Some(self)
        } else {
            None
        }
    }

    fn as_owned_clipboard_impl(&self) -> Option<&dyn IOwnedClipboardImpl> {
        if self.owned {
            Some(self)
        } else {
            None
        }
    }
}

impl IFlushableClipboardImpl for TestClipboardImpl {
    fn flush_async(&self) -> LocalBoxFuture<Result<(), ClipboardError>> {
        self.calls.borrow_mut().push("flush");
        self.outcome(|| ())
    }
}

impl IOwnedClipboardImpl for TestClipboardImpl {
    fn is_current_owner_async(&self) -> LocalBoxFuture<Result<bool, ClipboardError>> {
        self.calls.borrow_mut().push("owner");
        self.outcome(|| self.is_owner.get())
    }
}

fn clipboard(owned: bool, flushable: bool) -> (Rc<TestClipboardImpl>, Rc<Clipboard>) {
    let clipboard_impl = Rc::new(TestClipboardImpl { owned, flushable, ..Default::default() });
    let clipboard = Clipboard::new(clipboard_impl.clone());
    (clipboard_impl, clipboard)
}

#[test]
fn clipboard_text_round_trips_and_disposes_the_retrieved_data() {
    let (clipboard_impl, clipboard) = clipboard(false, false);

    ready(clipboard.set_text_async(Some("hello")));
    assert_eq!(vec!["set"], *clipboard_impl.calls.borrow());
    assert_eq!(Some("hello".to_string()), ready(clipboard.try_get_text_async()));
    assert_eq!(1, ready(clipboard.get_data_formats_async()).len());
    assert!(ready(clipboard.try_get_bitmap_async()).is_none());
    assert_eq!(Some(vec!["hello".to_string()]), ready(clipboard.try_get_values_async(&DataFormat::text())));

    // What the clipboard hands out is disposed after reading from it.
    let data_transfer = AsyncOnlyDataTransfer::text(None);
    *clipboard_impl.data.borrow_mut() = Some(data_transfer.clone());
    assert_eq!(Some("async text".to_string()), ready(clipboard.try_get_text_async()));
    assert_eq!(1, data_transfer.disposed.get());
    assert_eq!(1, ready(clipboard.get_data_formats_async()).len());
    assert_eq!(2, data_transfer.disposed.get());
    assert!(ready(clipboard.try_get_values_async(&DataFormat::bitmap())).is_none());
    assert_eq!(3, data_transfer.disposed.get());
}

#[test]
fn clipboard_null_values_clear_it() {
    let (clipboard_impl, clipboard) = clipboard(false, false);

    ready(clipboard.set_text_async(None));
    ready(clipboard.set_data_async(None));
    ready(clipboard.set_bitmap_async(None));
    ready(clipboard.set_values_async(&DataFormat::text(), None::<Vec<String>>));
    ready(clipboard.set_values_async(&DataFormat::text(), Some(Vec::<String>::new())));
    assert_eq!(vec!["clear"; 5], *clipboard_impl.calls.borrow());
    assert!(ready(clipboard.get_data_formats_async()).is_empty());
    assert!(ready(clipboard.try_get_text_async()).is_none());

    ready(clipboard.set_values_async(&DataFormat::text(), Some(vec!["a".to_string(), "b".to_string()])));
    assert_eq!(
        Some(vec!["a".to_string(), "b".to_string()]),
        ready(clipboard.try_get_values_async(&DataFormat::text()))
    );
}

#[test]
fn clipboard_flush_is_forwarded_to_flushable_implementations_only() {
    let (clipboard_impl, clipboard) = clipboard(false, false);
    ready(clipboard.flush_async());
    assert!(clipboard_impl.calls.borrow().is_empty());

    let (clipboard_impl, clipboard) = self::clipboard(false, true);
    ready(clipboard.flush_async());
    assert_eq!(vec!["flush"], *clipboard_impl.calls.borrow());
}

#[test]
fn clipboard_in_process_data_requires_an_owned_implementation() {
    let (clipboard_impl, clipboard) = clipboard(false, false);
    let data_transfer = AsyncOnlyDataTransfer::text(None);

    ready(clipboard.set_data_async(Some(data_transfer.clone())));
    assert!(ready(clipboard.try_get_in_process_data_async()).is_none());
    assert_eq!(vec!["set"], *clipboard_impl.calls.borrow());

    // Not kept, so not disposed when the clipboard is cleared.
    ready(clipboard.clear_async());
    assert_eq!(0, data_transfer.disposed.get());
}

#[test]
fn clipboard_in_process_data_is_returned_while_the_clipboard_is_owned() {
    let (clipboard_impl, clipboard) = clipboard(true, false);
    assert!(ready(clipboard.try_get_in_process_data_async()).is_none());
    assert!(clipboard_impl.calls.borrow().is_empty());

    let data_transfer = AsyncOnlyDataTransfer::text(None);
    ready(clipboard.set_data_async(Some(data_transfer.clone())));

    let in_process = ready(clipboard.try_get_in_process_data_async()).expect("the data set by this process");
    assert!(std::ptr::addr_eq(Rc::as_ptr(&in_process), Rc::as_ptr(&data_transfer)));

    // Another application took the clipboard.
    clipboard_impl.is_owner.set(false);
    assert!(ready(clipboard.try_get_in_process_data_async()).is_none());
    // It is forgotten for good.
    clipboard_impl.is_owner.set(true);
    assert!(ready(clipboard.try_get_in_process_data_async()).is_none());
    assert_eq!(0, data_transfer.disposed.get());
}

#[test]
fn clipboard_clear_disposes_the_last_owned_data() {
    let (_clipboard_impl, clipboard) = clipboard(true, false);
    let data_transfer = AsyncOnlyDataTransfer::text(None);

    ready(clipboard.set_data_async(Some(data_transfer.clone())));
    ready(clipboard.clear_async());
    assert_eq!(1, data_transfer.disposed.get());
    assert!(ready(clipboard.try_get_in_process_data_async()).is_none());

    ready(clipboard.clear_async());
    assert_eq!(1, data_transfer.disposed.get());
}

#[test]
fn platform_clipboard_manager_returns_the_clipboard_of_each_type() {
    let (_, default) = clipboard(false, false);
    let manager = PlatformClipboardManager::new(Some(default.clone()), None);

    let found = manager.try_get_clipboard(ClipboardType::Default).expect("the default clipboard");
    assert!(std::ptr::addr_eq(Rc::as_ptr(&found), Rc::as_ptr(&default)));
    assert!(manager.try_get_clipboard(ClipboardType::PrimarySelection).is_none());
}

// --- failures -------------------------------------------------------------------

/// One error of every kind.
fn clipboard_errors() -> Vec<ClipboardError> {
    vec![
        ClipboardError::timeout("Timeout opening clipboard."),
        ClipboardError::canceled("The operation was canceled."),
        ClipboardError::access_denied("Access is denied."),
        ClipboardError::platform(0x8004_01D0_u32 as i32, "OpenClipboard failed"),
        ClipboardError::other("Operation is not valid due to the current state of the object."),
    ]
}

#[test]
fn clipboard_reports_the_failure_of_the_implementation() {
    for error in clipboard_errors() {
        let (clipboard_impl, clipboard) = clipboard(true, true);
        *clipboard_impl.failure.borrow_mut() = Some(error.clone());

        assert_eq!(Err(error.clone()), ready_result(clipboard.clear_async()));
        assert_eq!(Err(error.clone()), ready_result(clipboard.set_data_async(None)));
        assert_eq!(Err(error.clone()), ready_result(clipboard.set_data_async(Some(DataTransfer::new()))));
        assert_eq!(Err(error.clone()), ready_result(clipboard.flush_async()));
        assert_eq!(Some(error.clone()), ready_result(clipboard.try_get_data_async()).err());
        assert_eq!(Some(error.clone()), ready_result(clipboard.try_get_in_process_data_async()).err());
        assert_eq!(vec!["clear", "clear", "set", "flush", "get", "owner"], *clipboard_impl.calls.borrow());
    }
}

#[test]
fn clipboard_extensions_report_the_failure_of_the_clipboard() {
    for error in clipboard_errors() {
        let (clipboard_impl, clipboard) = clipboard(false, false);
        *clipboard_impl.failure.borrow_mut() = Some(error.clone());

        assert_eq!(Err(error.clone()), ready_result(clipboard.set_text_async(Some("text"))));
        assert_eq!(Err(error.clone()), ready_result(clipboard.set_text_async(None)));
        assert_eq!(Err(error.clone()), ready_result(clipboard.set_bitmap_async(None)));
        assert_eq!(Err(error.clone()), ready_result(clipboard.set_value_async(&DataFormat::text(), Some("a".to_string()))));
        assert_eq!(
            Err(error.clone()),
            ready_result(clipboard.set_values_async(&DataFormat::text(), Some(vec!["a".to_string()])))
        );
        assert_eq!(Err(error.clone()), ready_result(clipboard.set_values_async(&DataFormat::text(), None::<Vec<String>>)));
        assert_eq!(Err(error.clone()), ready_result(clipboard.try_get_text_async()));
        assert_eq!(Some(error.clone()), ready_result(clipboard.try_get_bitmap_async()).err());
        assert_eq!(Err(error.clone()), ready_result(clipboard.try_get_value_async(&DataFormat::text())));
        assert_eq!(Err(error.clone()), ready_result(clipboard.try_get_values_async(&DataFormat::text())));
        assert_eq!(Some(error.clone()), ready_result(clipboard.get_data_formats_async()).err());
        assert_eq!(
            vec!["set", "clear", "clear", "set", "set", "clear", "get", "get", "get", "get", "get"],
            *clipboard_impl.calls.borrow()
        );
    }
}

#[test]
fn clipboard_keeps_the_in_process_data_when_the_ownership_query_fails() {
    let (clipboard_impl, clipboard) = clipboard(true, false);
    let data_transfer = AsyncOnlyDataTransfer::text(None);
    ready(clipboard.set_data_async(Some(data_transfer.clone())));

    let error = ClipboardError::timeout("Timeout opening clipboard.");
    *clipboard_impl.failure.borrow_mut() = Some(error.clone());
    assert_eq!(Some(error), ready_result(clipboard.try_get_in_process_data_async()).err());

    *clipboard_impl.failure.borrow_mut() = None;
    let in_process = ready(clipboard.try_get_in_process_data_async()).expect("the data set by this process");
    assert!(std::ptr::addr_eq(Rc::as_ptr(&in_process), Rc::as_ptr(&data_transfer)));
}

#[test]
fn clipboard_remembers_the_data_whose_placement_failed() {
    // The data transfer is kept before the implementation is asked to place
    // it, as in the reference.
    let (clipboard_impl, clipboard) = clipboard(true, false);
    let data_transfer = AsyncOnlyDataTransfer::text(None);

    *clipboard_impl.failure.borrow_mut() = Some(ClipboardError::access_denied("Access is denied."));
    assert!(ready_result(clipboard.set_data_async(Some(data_transfer.clone()))).is_err());

    *clipboard_impl.failure.borrow_mut() = None;
    // The implementation did not take the data, so it is not the owner.
    assert!(ready(clipboard.try_get_in_process_data_async()).is_none());
    assert_eq!(0, data_transfer.disposed.get());
}

/// An asynchronous data transfer whose formats, items, item formats or
/// values fail while the matching error is set.
#[derive(Default)]
struct FailingDataTransfer {
    formats_error: RefCell<Option<ClipboardError>>,
    items_error: RefCell<Option<ClipboardError>>,
    items: RefCell<Vec<Rc<FailingItem>>>,
    disposed: Cell<u32>,
}

#[derive(Default)]
struct FailingItem {
    formats_error: RefCell<Option<ClipboardError>>,
    value_error: RefCell<Option<ClipboardError>>,
    formats_requests: Cell<u32>,
    value_requests: Cell<u32>,
}

impl IAsyncDataTransferItem for FailingItem {
    fn formats(&self) -> Rc<[DataFormat]> {
        match self.try_formats() {
            Ok(formats) => formats,
            Err(error) => panic!("{error}"),
        }
    }

    fn try_formats(&self) -> Result<Rc<[DataFormat]>, ClipboardError> {
        self.formats_requests.set(self.formats_requests.get() + 1);
        match self.formats_error.borrow().clone() {
            Some(error) => Err(error),
            None => Ok(Rc::from(vec![DataFormat::text().into()])),
        }
    }

    fn try_get_raw_async(&self, _format: &DataFormat) -> LocalBoxFuture<Result<Option<Rc<dyn Any>>, ClipboardError>> {
        self.value_requests.set(self.value_requests.get() + 1);
        let result = match self.value_error.borrow().clone() {
            Some(error) => Err(error),
            None => Ok(Some(Rc::new("value".to_string()) as Rc<dyn Any>)),
        };
        Box::pin(std::future::ready(result))
    }
}

impl IDisposable for FailingDataTransfer {
    fn dispose(&self) {
        self.disposed.set(self.disposed.get() + 1);
    }
}

impl IAsyncDataTransfer for FailingDataTransfer {
    fn formats(&self) -> Rc<[DataFormat]> {
        match self.try_formats() {
            Ok(formats) => formats,
            Err(error) => panic!("{error}"),
        }
    }

    fn items(&self) -> Rc<[Rc<dyn IAsyncDataTransferItem>]> {
        match self.try_items() {
            Ok(items) => items,
            Err(error) => panic!("{error}"),
        }
    }

    fn try_formats(&self) -> Result<Rc<[DataFormat]>, ClipboardError> {
        match self.formats_error.borrow().clone() {
            Some(error) => Err(error),
            None => Ok(Rc::from(vec![DataFormat::text().into()])),
        }
    }

    fn try_items(&self) -> Result<Rc<[Rc<dyn IAsyncDataTransferItem>]>, ClipboardError> {
        match self.items_error.borrow().clone() {
            Some(error) => Err(error),
            None => Ok(self.items.borrow().iter().map(|item| item.clone() as Rc<dyn IAsyncDataTransferItem>).collect()),
        }
    }
}

fn failing_data_transfer(item_count: usize) -> Rc<FailingDataTransfer> {
    let data_transfer = Rc::new(FailingDataTransfer::default());
    *data_transfer.items.borrow_mut() = (0..item_count).map(|_| Rc::new(FailingItem::default())).collect();
    data_transfer
}

#[test]
fn clipboard_extensions_report_the_failure_of_the_data_and_dispose_it() {
    let (clipboard_impl, clipboard) = clipboard(false, false);
    let data_transfer = failing_data_transfer(1);
    *clipboard_impl.data.borrow_mut() = Some(data_transfer.clone());
    let error = ClipboardError::platform(0x8000_4005_u32 as i32, "GetFormats failed");

    // The formats of the data transfer.
    *data_transfer.formats_error.borrow_mut() = Some(error.clone());
    assert_eq!(Some(error.clone()), ready_result(clipboard.get_data_formats_async()).err());
    assert_eq!(1, data_transfer.disposed.get());
    *data_transfer.formats_error.borrow_mut() = None;
    assert_eq!(1, ready(clipboard.get_data_formats_async()).len());
    assert_eq!(2, data_transfer.disposed.get());

    // The items of the data transfer.
    *data_transfer.items_error.borrow_mut() = Some(error.clone());
    assert_eq!(Err(error.clone()), ready_result(clipboard.try_get_text_async()));
    assert_eq!(Err(error.clone()), ready_result(clipboard.try_get_values_async(&DataFormat::text())));
    assert_eq!(4, data_transfer.disposed.get());
    *data_transfer.items_error.borrow_mut() = None;

    // The formats of an item.
    let item = data_transfer.items.borrow()[0].clone();
    *item.formats_error.borrow_mut() = Some(error.clone());
    assert_eq!(Err(error.clone()), ready_result(clipboard.try_get_text_async()));
    assert_eq!(Err(error.clone()), ready_result(clipboard.try_get_values_async(&DataFormat::text())));
    assert_eq!(0, item.value_requests.get());
    assert_eq!(6, data_transfer.disposed.get());
    *item.formats_error.borrow_mut() = None;

    // The value of an item.
    *item.value_error.borrow_mut() = Some(error.clone());
    assert_eq!(Err(error.clone()), ready_result(clipboard.try_get_text_async()));
    assert_eq!(Err(error.clone()), ready_result(clipboard.try_get_values_async(&DataFormat::text())));
    assert_eq!(2, item.value_requests.get());
    assert_eq!(8, data_transfer.disposed.get());
    *item.value_error.borrow_mut() = None;

    assert_eq!(Some("value".to_string()), ready(clipboard.try_get_text_async()));
    assert_eq!(9, data_transfer.disposed.get());
}

#[test]
fn try_get_value_async_asks_no_item_after_the_first_supporting_the_format() {
    let data_transfer = failing_data_transfer(2);
    let items = data_transfer.items.borrow().clone();
    *items[1].formats_error.borrow_mut() = Some(ClipboardError::other("not asked"));

    assert_eq!(Some("value".to_string()), ready(data_transfer.try_get_text_async()));
    assert_eq!((1, 0), (items[0].formats_requests.get(), items[1].formats_requests.get()));
    assert_eq!((1, 0), (items[0].value_requests.get(), items[1].value_requests.get()));
}

#[test]
fn try_get_values_async_stops_at_the_first_failure() {
    let data_transfer = failing_data_transfer(3);
    let items = data_transfer.items.borrow().clone();
    let error = ClipboardError::timeout("The selection owner did not answer.");
    *items[1].value_error.borrow_mut() = Some(error.clone());

    assert_eq!(Err(error), ready_result(data_transfer.try_get_values_async(&DataFormat::text())));
    assert_eq!(
        (1, 1, 0),
        (items[0].value_requests.get(), items[1].value_requests.get(), items[2].value_requests.get())
    );
    assert_eq!(0, items[2].formats_requests.get());
}

#[test]
fn a_pending_value_can_fail_later() {
    let completion = Completion::default();
    let data_transfer = AsyncOnlyDataTransfer::text(Some(completion.clone()));
    let error = ClipboardError::canceled("The operation was canceled.");

    let mut text = data_transfer.try_get_text_async();
    assert!(poll_once(&mut text).is_pending());
    completion.fail(error.clone());
    assert_eq!(Poll::Ready(Err(error)), poll_once(&mut text));
}

#[test]
fn synchronous_wrapper_panics_with_the_failure_of_the_wrapped_item() {
    let data_transfer = failing_data_transfer(1);
    let item = data_transfer.items.borrow()[0].clone();
    let asynchronous: Rc<dyn IAsyncDataTransfer> = data_transfer;
    let sync = asynchronous.to_synchronous("Test");

    assert_eq!(Some("value".to_string()), sync.try_get_text());

    *item.value_error.borrow_mut() = Some(ClipboardError::timeout("The selection owner did not answer."));
    let payload = catch_unwind(AssertUnwindSafe(|| sync.try_get_text())).expect_err("the failure of the item");
    assert_eq!(Some("The selection owner did not answer."), payload.downcast_ref::<String>().map(String::as_str));

    // The asynchronous side of the wrapper still reports it.
    let back = sync.to_asynchronous();
    assert!(ready_result(back.try_get_text_async()).is_err());
}

/// A platform data transfer (and item) whose backend fails while the error
/// is set.
#[derive(Clone, Default)]
struct FailingBackend {
    error: Rc<RefCell<Option<ClipboardError>>>,
    calls: Rc<Cell<u32>>,
}

impl FailingBackend {
    fn call<T>(&self, value: T) -> Result<T, ClipboardError> {
        self.calls.set(self.calls.get() + 1);
        match self.error.borrow().clone() {
            Some(error) => Err(error),
            None => Ok(value),
        }
    }
}

impl PlatformDataTransferItemImpl for FailingBackend {
    fn provide_formats(&self) -> Result<Vec<DataFormat>, ClipboardError> {
        self.call(vec![DataFormat::text().into()])
    }

    fn try_get_raw_core(&self, _format: &DataFormat) -> Result<Option<Rc<dyn Any>>, ClipboardError> {
        self.call(Some(Rc::new("platform".to_string()) as Rc<dyn Any>))
    }
}

impl PlatformDataTransferImpl for FailingBackend {
    fn provide_formats(&self) -> Result<Vec<DataFormat>, ClipboardError> {
        self.call(vec![DataFormat::text().into()])
    }

    fn provide_items(&self) -> Result<Vec<Rc<PlatformDataTransferItem>>, ClipboardError> {
        self.call(vec![PlatformDataTransferItem::create(&DataFormat::text(), "platform".to_string())])
    }

    fn dispose(&self, _owner: &PlatformDataTransfer) {}
}

impl PlatformAsyncDataTransferItemImpl for FailingBackend {
    fn provide_formats(&self) -> Result<Vec<DataFormat>, ClipboardError> {
        self.call(vec![DataFormat::text().into()])
    }

    fn try_get_raw_core_async(&self, _format: &DataFormat) -> LocalBoxFuture<Result<Option<Rc<dyn Any>>, ClipboardError>> {
        Box::pin(std::future::ready(self.call(Some(Rc::new("platform".to_string()) as Rc<dyn Any>))))
    }
}

impl PlatformAsyncDataTransferImpl for FailingBackend {
    fn provide_formats(&self) -> Result<Vec<DataFormat>, ClipboardError> {
        self.call(vec![DataFormat::text().into()])
    }

    fn provide_items(&self) -> Result<Vec<Rc<dyn IAsyncDataTransferItem>>, ClipboardError> {
        self.call(vec![PlatformDataTransferItem::create(&DataFormat::text(), "platform".to_string())
            as Rc<dyn IAsyncDataTransferItem>])
    }

    fn dispose(&self, _owner: &PlatformAsyncDataTransfer) {}
}

fn backend_error() -> ClipboardError {
    ClipboardError::platform(0x8000_4005_u32 as i32, "Native call failed")
}

#[test]
fn platform_data_transfer_reports_the_failure_of_the_backend_and_asks_again() {
    let backend = FailingBackend::default();
    let data_transfer = PlatformDataTransfer::new(backend.clone());
    *backend.error.borrow_mut() = Some(backend_error());

    assert_eq!(Some(backend_error()), data_transfer.try_formats().err());
    assert_eq!(Some(backend_error()), data_transfer.try_items().err());
    assert_eq!(Some(backend_error()), IAsyncDataTransfer::try_formats(&*data_transfer).err());
    assert_eq!(Some(backend_error()), IAsyncDataTransfer::try_items(&*data_transfer).err());
    assert!(!data_transfer.are_formats_initialized());
    assert!(!data_transfer.are_items_initialized());
    assert!(panics(|| drop(data_transfer.formats())));
    assert!(panics(|| drop(data_transfer.items())));
    assert!(panics(|| drop(IDataTransfer::items(&*data_transfer))));
    assert!(panics(|| drop(IAsyncDataTransfer::items(&*data_transfer))));
    assert_eq!(Err(backend_error()), ready_result(data_transfer.try_get_text_async()));
    assert_eq!(9, backend.calls.get());

    // Nothing was kept: the backend is asked again, then once.
    *backend.error.borrow_mut() = None;
    assert_eq!(1, data_transfer.try_formats().expect("the formats").len());
    assert_eq!(1, data_transfer.formats().len());
    assert_eq!(1, IAsyncDataTransfer::try_items(&*data_transfer).expect("the items").len());
    assert_eq!(1, data_transfer.items().len());
    assert_eq!(11, backend.calls.get());
    assert_eq!(Some("platform".to_string()), ready(data_transfer.try_get_text_async()));
}

#[test]
fn platform_async_data_transfer_reports_the_failure_of_the_backend_and_asks_again() {
    let backend = FailingBackend::default();
    let data_transfer = PlatformAsyncDataTransfer::new(backend.clone());
    *backend.error.borrow_mut() = Some(backend_error());

    assert_eq!(Some(backend_error()), data_transfer.try_formats().err());
    assert_eq!(Some(backend_error()), data_transfer.try_items().err());
    assert_eq!(Some(backend_error()), IAsyncDataTransfer::try_formats(&*data_transfer).err());
    assert_eq!(Some(backend_error()), IAsyncDataTransfer::try_items(&*data_transfer).err());
    assert!(!data_transfer.are_formats_initialized());
    assert!(!data_transfer.are_items_initialized());
    assert!(panics(|| drop(data_transfer.formats())));
    assert!(panics(|| drop(data_transfer.items())));
    assert_eq!(6, backend.calls.get());

    *backend.error.borrow_mut() = None;
    assert_eq!(1, data_transfer.try_formats().expect("the formats").len());
    assert_eq!(1, data_transfer.formats().len());
    assert_eq!(1, data_transfer.try_items().expect("the items").len());
    assert_eq!(1, data_transfer.items().len());
    assert_eq!(8, backend.calls.get());
}

#[test]
fn platform_data_transfer_item_reports_the_failure_of_the_backend() {
    let backend = FailingBackend::default();
    let item = PlatformDataTransferItem::new(backend.clone());
    let format = DataFormat::text();
    *backend.error.borrow_mut() = Some(backend_error());

    // The formats.
    assert_eq!(Some(backend_error()), item.try_formats().err());
    assert_eq!(Some(backend_error()), item.try_get_raw_result(&format).err());
    assert_eq!(Some(backend_error()), ready_result(IAsyncDataTransferItem::try_get_raw_async(&*item, &format)).err());
    assert_eq!(Some(backend_error()), item.try_contains(&format).err());
    assert!(panics(|| drop(item.formats())));
    assert!(panics(|| drop(item.contains(&format))));
    assert!(panics(|| drop(item.try_get_raw(&format))));
    assert_eq!(7, backend.calls.get());

    // The value: the formats are kept once they were provided.
    *backend.error.borrow_mut() = None;
    assert!(item.contains(&format));
    *backend.error.borrow_mut() = Some(backend_error());
    assert_eq!(Some(backend_error()), item.try_get_raw_result(&format).err());
    assert_eq!(Err(backend_error()), ready_result(item.try_get_text_async()));
    let payload = catch_unwind(AssertUnwindSafe(|| IDataTransferItem::try_get_raw(&*item, &format)))
        .expect_err("the failure of the backend");
    assert_eq!(Some("Native call failed (0x80004005)"), payload.downcast_ref::<String>().map(String::as_str));
    assert_eq!(11, backend.calls.get());

    *backend.error.borrow_mut() = None;
    assert_eq!(Some("platform".to_string()), ready(item.try_get_text_async()));
    // A format the item does not support is not asked for.
    assert!(item.try_get_raw_result(&DataFormat::bitmap()).expect("no failure").is_none());
    assert_eq!(12, backend.calls.get());
}

#[test]
fn platform_async_data_transfer_item_reports_the_failure_of_the_backend() {
    let backend = FailingBackend::default();
    let item = PlatformAsyncDataTransferItem::new(backend.clone());
    let format = DataFormat::text();
    *backend.error.borrow_mut() = Some(backend_error());

    assert_eq!(Some(backend_error()), item.try_formats().err());
    assert_eq!(Some(backend_error()), ready_result(item.try_get_raw_async(&format)).err());
    assert_eq!(Some(backend_error()), IAsyncDataTransferItem::try_formats(&*item).err());
    assert!(panics(|| drop(item.formats())));
    assert!(panics(|| drop(item.contains(&format))));
    assert_eq!(5, backend.calls.get());

    *backend.error.borrow_mut() = None;
    assert!(item.contains(&format));
    *backend.error.borrow_mut() = Some(backend_error());
    assert_eq!(Err(backend_error()), ready_result(item.try_get_text_async()));
    assert_eq!(7, backend.calls.get());

    *backend.error.borrow_mut() = None;
    assert_eq!(Some("platform".to_string()), ready(item.try_get_text_async()));
    assert!(ready(item.try_get_raw_async(&DataFormat::bitmap())).is_none());
    assert_eq!(8, backend.calls.get());
}
