use crate::browser_clipboard_data_transfer::BrowserClipboardDataTransfer;
use crate::browser_data_format_helper::to_browser_format;
use crate::browser_data_transfer_helper::{IReadableDataItems, JsReadableDataItems};
use crate::interop::completion_helper::PromiseError;
use crate::interop::input_helper;
use crate::windowing_platform::BrowserWindowingPlatform;
use ferroui_base::input::platform::{ClipboardError, IClipboardImpl};
use ferroui_base::input::{
    AsyncDataTransferItemExtensions, DataFormat, DataFormatKind, IAsyncDataTransfer, IAsyncDataTransferItem,
    LocalBoxFuture,
};
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::media::imaging::{BitmapEncoderOptions, PngBitmapEncoderOptions};
use std::rc::Rc;

/// A value of an item written to the clipboard.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum WriteableClipboardValue {
    String(String),
    Bytes(Vec<u8>),
}

/// An item written to the clipboard: its values by format string
/// (`WriteableClipboardItem` of the script module).
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct WriteableClipboardItem {
    pub(crate) data: Vec<(String, WriteableClipboardValue)>,
}

/// The items written to the clipboard (`WriteableClipboardSource` of the
/// script module).
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct WriteableClipboardSource {
    pub(crate) items: Vec<WriteableClipboardItem>,
}

/// What reading the clipboard of the page gives: an error, or the items.
pub(crate) struct ClipboardRead {
    pub(crate) error: Option<String>,
    pub(crate) items: Option<Rc<dyn IReadableDataItems>>,
}

/// The calls the clipboard makes into the page; replaced by a fake in the
/// tests.
pub(crate) trait IClipboardPage {
    fn is_clipboard_format_supported(&self, format: &str) -> bool;
    fn read_clipboard(&self) -> LocalBoxFuture<Result<ClipboardRead, PromiseError>>;
    fn write_clipboard(&self, source: Option<WriteableClipboardSource>) -> LocalBoxFuture<Result<String, PromiseError>>;
}

struct ClipboardPage;

impl IClipboardPage for ClipboardPage {
    fn is_clipboard_format_supported(&self, format: &str) -> bool {
        input_helper::is_clipboard_format_supported(format)
    }

    fn read_clipboard(&self) -> LocalBoxFuture<Result<ClipboardRead, PromiseError>> {
        Box::pin(async {
            let result = input_helper::read_clipboard_async(&BrowserWindowingPlatform::global_this()).await?;
            Ok(ClipboardRead { error: result.error, items: result.items.map(JsReadableDataItems::new) })
        })
    }

    fn write_clipboard(&self, source: Option<WriteableClipboardSource>) -> LocalBoxFuture<Result<String, PromiseError>> {
        // The objects of the script module are created from the items gathered by the clipboard.
        let js_source = source.map(|source| {
            let js_source = input_helper::create_writeable_clipboard_source();
            for item in &source.items {
                let js_item = input_helper::create_writeable_clipboard_item(&js_source);
                for (format, value) in &item.data {
                    match value {
                        WriteableClipboardValue::String(value) => {
                            input_helper::add_string_to_writeable_clipboard_item(&js_item, format, value)
                        }
                        WriteableClipboardValue::Bytes(value) => {
                            input_helper::add_bytes_to_writeable_clipboard_item(&js_item, format, value)
                        }
                    }
                }
            }
            js_source
        });

        Box::pin(async move {
            input_helper::write_clipboard_async(&BrowserWindowingPlatform::global_this(), js_source.as_ref()).await
        })
    }
}

/// The clipboard of the page: the asynchronous Clipboard API, and the
/// "paste" event in browsers without it.
pub struct ClipboardImpl {
    page: Rc<dyn IClipboardPage>,
}

impl Default for ClipboardImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl ClipboardImpl {
    /// Creates the clipboard of the page.
    pub fn new() -> Self {
        Self { page: Rc::new(ClipboardPage) }
    }

    #[cfg(test)]
    fn with_page(page: Rc<dyn IClipboardPage>) -> Self {
        Self { page }
    }

    async fn write_clipboard_async(
        page: &dyn IClipboardPage,
        source: Option<WriteableClipboardSource>,
    ) -> Result<(), ClipboardError> {
        let error = page.write_clipboard(source).await.map_err(to_clipboard_error)?;

        if error == "denied" {
            return Err(ClipboardError::access_denied("Write permission is not granted for clipboard"));
        }

        Ok(())
    }

    async fn try_add_item_async(
        page: &dyn IClipboardPage,
        data_transfer_item: &Rc<dyn IAsyncDataTransferItem>,
        source: &mut WriteableClipboardSource,
    ) -> Result<(), ClipboardError> {
        let mut writeable_item: Option<WriteableClipboardItem> = None;

        for format in data_transfer_item.try_formats()?.iter() {
            if format.kind() == DataFormatKind::InProcess {
                continue;
            }

            let format_string = to_browser_format(format);
            if !page.is_clipboard_format_supported(&format_string) {
                continue;
            }

            if DataFormat::text() == *format {
                let text = data_transfer_item.try_get_value_async(&DataFormat::text()).await?.unwrap_or_default();
                writeable_item
                    .get_or_insert_with(WriteableClipboardItem::default)
                    .data
                    .push((format_string, WriteableClipboardValue::String(text)));
                continue;
            }

            if DataFormat::bitmap() == *format {
                let bitmap = data_transfer_item.try_get_value_async(&DataFormat::bitmap()).await?;
                if let Some(bitmap) = bitmap {
                    let mut stream = Vec::new();
                    bitmap
                        .save(&mut stream, &BitmapEncoderOptions::Png(PngBitmapEncoderOptions::DEFAULT))
                        .map_err(|error| ClipboardError::other(error.to_string()))?;

                    writeable_item
                        .get_or_insert_with(WriteableClipboardItem::default)
                        .data
                        .push((format_string, WriteableClipboardValue::Bytes(stream)));
                }

                continue;
            }

            // The original tells string formats from byte formats by their data type, which the
            // formats of the framework do not carry: the type of the value decides.
            let value = data_transfer_item.try_get_raw_async(format).await?;
            match value {
                None => continue,
                Some(value) => {
                    if let Some(string_value) = value.downcast_ref::<String>() {
                        writeable_item
                            .get_or_insert_with(WriteableClipboardItem::default)
                            .data
                            .push((format_string, WriteableClipboardValue::String(string_value.clone())));
                        continue;
                    }

                    if let Some(bytes) = value.downcast_ref::<Rc<[u8]>>() {
                        writeable_item
                            .get_or_insert_with(WriteableClipboardItem::default)
                            .data
                            .push((format_string, WriteableClipboardValue::Bytes(bytes.to_vec())));
                        continue;
                    }
                }
            }

            // Note: the file format isn't supported, we can't put arbitrary files onto the
            // clipboard on the browser for security reasons.

            if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::BROWSER_PLATFORM) {
                logger.log(None, &format!("Unsupported data format {format}"));
            }
        }

        if let Some(writeable_item) = writeable_item {
            source.items.push(writeable_item);
        }

        Ok(())
    }
}

fn to_clipboard_error(error: PromiseError) -> ClipboardError {
    ClipboardError::other(error.to_string())
}

impl IClipboardImpl for ClipboardImpl {
    fn try_get_data_async(&self) -> LocalBoxFuture<Result<Option<Rc<dyn IAsyncDataTransfer>>, ClipboardError>> {
        let read = self.page.read_clipboard();
        Box::pin(async move {
            let result = read.await.map_err(to_clipboard_error)?;
            if result.error.as_deref() == Some("denied") {
                return Err(ClipboardError::access_denied("Read permission is not granted for clipboard"));
            }

            Ok(match result.items {
                Some(items) if items.len() > 0 => {
                    Some(BrowserClipboardDataTransfer::new(items) as Rc<dyn IAsyncDataTransfer>)
                }
                _ => None,
            })
        })
    }

    fn set_data_async(&self, data_transfer: Rc<dyn IAsyncDataTransfer>) -> LocalBoxFuture<Result<(), ClipboardError>> {
        let page = self.page.clone();
        Box::pin(async move {
            let mut source = WriteableClipboardSource::default();

            for data_transfer_item in data_transfer.try_items()?.iter() {
                Self::try_add_item_async(&*page, data_transfer_item, &mut source).await?;
            }

            Self::write_clipboard_async(&*page, Some(source)).await
        })
    }

    fn clear_async(&self) -> LocalBoxFuture<Result<(), ClipboardError>> {
        let page = self.page.clone();
        Box::pin(async move { Self::write_clipboard_async(&*page, None).await })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::browser_data_transfer_helper::tests::{FakeReadableDataItem, FakeReadableDataItems, FakeValue};
    use ferroui_base::input::platform::ClipboardErrorKind;
    use ferroui_base::input::{DataTransfer, DataTransferItem};
    use std::cell::RefCell;
    use std::future::Future;
    use std::task::{Context, Poll, Waker};

    /// Runs a future that completes without waiting.
    fn run<T>(future: impl Future<Output = T>) -> T {
        let mut future = std::pin::pin!(future);
        let mut context = Context::from_waker(Waker::noop());
        match future.as_mut().poll(&mut context) {
            Poll::Ready(value) => value,
            Poll::Pending => panic!("the future waits"),
        }
    }

    #[derive(Default)]
    struct FakePage {
        supported: Vec<&'static str>,
        read_error: Option<&'static str>,
        read_items: RefCell<Option<Vec<Rc<FakeReadableDataItem>>>>,
        reject: Option<&'static str>,
        write_answer: &'static str,
        written: RefCell<Vec<Option<WriteableClipboardSource>>>,
    }

    impl IClipboardPage for FakePage {
        fn is_clipboard_format_supported(&self, format: &str) -> bool {
            self.supported.contains(&format)
        }

        fn read_clipboard(&self) -> LocalBoxFuture<Result<ClipboardRead, PromiseError>> {
            if let Some(reason) = self.reject {
                return Box::pin(std::future::ready(Err(PromiseError::new("TypeError", reason))));
            }
            let items = self
                .read_items
                .borrow_mut()
                .take()
                .map(|items| Rc::new(FakeReadableDataItems(items)) as Rc<dyn IReadableDataItems>);
            Box::pin(std::future::ready(Ok(ClipboardRead { error: self.read_error.map(str::to_string), items })))
        }

        fn write_clipboard(
            &self,
            source: Option<WriteableClipboardSource>,
        ) -> LocalBoxFuture<Result<String, PromiseError>> {
            if let Some(reason) = self.reject {
                return Box::pin(std::future::ready(Err(PromiseError::new("TypeError", reason))));
            }
            self.written.borrow_mut().push(source);
            Box::pin(std::future::ready(Ok(self.write_answer.to_string())))
        }
    }

    fn page() -> FakePage {
        FakePage { supported: vec!["text/plain", "text/html", "image/png"], ..Default::default() }
    }

    fn string(value: &str) -> WriteableClipboardValue {
        WriteableClipboardValue::String(value.to_string())
    }

    #[test]
    fn reading_an_empty_clipboard_gives_no_data() {
        let page = Rc::new(page());
        *page.read_items.borrow_mut() = Some(Vec::new());
        let clipboard = ClipboardImpl::with_page(page.clone());
        assert!(run(clipboard.try_get_data_async()).unwrap().is_none());

        let clipboard = ClipboardImpl::with_page(page);
        assert!(run(clipboard.try_get_data_async()).unwrap().is_none());
    }

    #[test]
    fn a_denied_read_is_an_access_denied_error() {
        let page = Rc::new(FakePage { read_error: Some("denied"), ..page() });
        let error = run(ClipboardImpl::with_page(page).try_get_data_async()).err().expect("an error");
        assert_eq!(ClipboardErrorKind::AccessDenied, error.kind());
        assert_eq!("Read permission is not granted for clipboard", error.message());
    }

    #[test]
    fn a_failed_read_is_an_error_with_the_message_of_the_page() {
        let page = Rc::new(FakePage { reject: Some("no clipboard"), ..page() });
        let error = run(ClipboardImpl::with_page(page).try_get_data_async()).err().expect("an error");
        assert_eq!(ClipboardErrorKind::Other, error.kind());
        assert_eq!("TypeError: no clipboard", error.message());
    }

    #[test]
    fn the_items_of_the_clipboard_are_read_through_the_data_transfer() {
        let page = Rc::new(page());
        *page.read_items.borrow_mut() = Some(vec![
            FakeReadableDataItem::new(&["text/plain", "text/html"], &[
                ("text/plain", FakeValue::String("plain")),
                ("text/html", FakeValue::String("<b>html</b>")),
            ]),
            FakeReadableDataItem::new(&["text/plain"], &[("text/plain", FakeValue::String("second"))]),
        ]);
        let data = run(ClipboardImpl::with_page(page).try_get_data_async()).unwrap().expect("data");

        let formats = data.formats();
        assert_eq!(2, formats.len());
        assert!(DataFormat::text() == formats[0]);
        assert_eq!("text/html", formats[1].identifier());

        let items = data.items();
        assert_eq!(2, items.len());
        assert_eq!(Some("plain".to_string()), run(items[0].try_get_text_async()).unwrap());
        assert_eq!(Some("second".to_string()), run(items[1].try_get_text_async()).unwrap());
        let html: DataFormat = formats[1].clone();
        let value = run(items[0].try_get_raw_async(&html)).unwrap().expect("the html");
        assert_eq!(Some(&"<b>html</b>".to_string()), value.downcast_ref::<String>());
        // A format the item does not have is not read.
        assert_eq!(None, run(items[1].try_get_raw_async(&html)).unwrap().map(|_| ()));
    }

    #[test]
    fn values_of_other_formats_are_read_as_bytes() {
        let page = Rc::new(page());
        *page.read_items.borrow_mut() = Some(vec![FakeReadableDataItem::new(
            &["application/frn-fmt.custom"],
            &[("application/frn-fmt.custom", FakeValue::Bytes(vec![4, 5, 6]))],
        )]);
        let data = run(ClipboardImpl::with_page(page).try_get_data_async()).unwrap().expect("data");
        let custom = DataFormat::create_bytes_application_format("custom");
        let value = run(data.items()[0].try_get_value_async(&custom)).unwrap();
        assert_eq!(Some(vec![4u8, 5, 6]), value.map(|bytes| bytes.to_vec()));
    }

    #[test]
    fn a_failed_value_read_is_an_error() {
        let page = Rc::new(page());
        *page.read_items.borrow_mut() =
            Some(vec![FakeReadableDataItem::new(&["text/plain"], &[("text/plain", FakeValue::Rejected("gone"))])]);
        let data = run(ClipboardImpl::with_page(page).try_get_data_async()).unwrap().expect("data");
        let error = run(data.items()[0].try_get_text_async()).err().expect("an error");
        assert_eq!("Error: gone", error.message());
    }

    #[test]
    fn text_is_written_as_plain_text() {
        let page = Rc::new(page());
        let data = DataTransfer::new();
        data.add(DataTransferItem::create_text(Some("hello")));
        run(ClipboardImpl::with_page(page.clone()).set_data_async(data)).unwrap();

        let expected =
            WriteableClipboardSource { items: vec![WriteableClipboardItem { data: vec![("text/plain".into(), string("hello"))] }] };
        assert_eq!(vec![Some(expected)], *page.written.borrow());
    }

    #[test]
    fn each_item_with_supported_values_becomes_an_item_of_the_clipboard() {
        let page = Rc::new(page());
        let html = DataFormat::create_string_platform_format("text/html");
        let custom = DataFormat::create_bytes_application_format("custom");
        let local = DataFormat::create_in_process_format::<String>("local");

        let first = DataTransferItem::new();
        first.set(&DataFormat::text(), Some("text".to_string()));
        first.set(&html, Some("<i>html</i>".to_string()));
        first.set(&local, Some("in process".to_string()));
        let second = DataTransferItem::new();
        // Not supported by the clipboard of the page: no item.
        second.set(&custom, Some(Rc::<[u8]>::from(vec![1u8, 2])));
        let data = DataTransfer::new();
        data.add(first);
        data.add(second);

        run(ClipboardImpl::with_page(page.clone()).set_data_async(data)).unwrap();

        let expected = WriteableClipboardSource {
            items: vec![WriteableClipboardItem {
                data: vec![("text/plain".into(), string("text")), ("text/html".into(), string("<i>html</i>"))],
            }],
        };
        assert_eq!(vec![Some(expected)], *page.written.borrow());
    }

    #[test]
    fn byte_values_are_written_as_bytes() {
        let page = Rc::new(FakePage { supported: vec!["application/frn-fmt.custom"], ..page() });
        let custom = DataFormat::create_bytes_application_format("custom");
        let item = DataTransferItem::new();
        item.set(&custom, Some(Rc::<[u8]>::from(vec![1u8, 2, 3])));
        let data = DataTransfer::new();
        data.add(item);

        run(ClipboardImpl::with_page(page.clone()).set_data_async(data)).unwrap();

        let expected = WriteableClipboardSource {
            items: vec![WriteableClipboardItem {
                data: vec![("application/frn-fmt.custom".into(), WriteableClipboardValue::Bytes(vec![1, 2, 3]))],
            }],
        };
        assert_eq!(vec![Some(expected)], *page.written.borrow());
    }

    #[test]
    fn a_denied_write_is_an_access_denied_error() {
        let page = Rc::new(FakePage { write_answer: "denied", ..page() });
        let data = DataTransfer::new();
        data.add(DataTransferItem::create_text(Some("hello")));
        let error = run(ClipboardImpl::with_page(page).set_data_async(data)).err().expect("an error");
        assert_eq!(ClipboardErrorKind::AccessDenied, error.kind());
        assert_eq!("Write permission is not granted for clipboard", error.message());
    }

    #[test]
    fn clearing_writes_no_source() {
        let page = Rc::new(page());
        run(ClipboardImpl::with_page(page.clone()).clear_async()).unwrap();
        assert_eq!(vec![None], *page.written.borrow());
    }
}
