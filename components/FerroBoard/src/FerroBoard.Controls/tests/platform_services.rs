#[path = "support/headless.rs"]
mod headless;
#[path = "../../../tests/support/local_future.rs"]
mod native_future;
use ferroboard_controls::services::*;
use ferroboard_core::{clipboard::IClipboardService, settings::FeedbackKind, text::Utf16Text};
use ferroui_base::{
    Ref, Visual,
    input::{
        DataFormat, DataTransfer, DataTransferItem, IAsyncDataTransfer, IAsyncDataTransferItem,
        LocalBoxFuture,
        platform::{ClipboardError, ClipboardErrorKind, IClipboard},
    },
};
use ferroui_controls::{Control, TextBox, Window};
use ferroui_system::IDisposable;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
#[derive(Default)]
struct Backend {
    data: RefCell<Option<Rc<dyn IAsyncDataTransfer>>>,
    failure: RefCell<Option<ClipboardError>>,
    calls: Rc<RefCell<Vec<&'static str>>>,
    gate: RefCell<Option<native_future::Completion<()>>>,
}
impl IClipboard for Backend {
    fn clear_async(&self) -> LocalBoxFuture<Result<(), ClipboardError>> {
        panic!("source service must not clear for null text")
    }
    fn flush_async(&self) -> LocalBoxFuture<Result<(), ClipboardError>> {
        panic!("source service does not flush")
    }
    fn set_data_async(
        &self,
        data: Option<Rc<dyn IAsyncDataTransfer>>,
    ) -> LocalBoxFuture<Result<(), ClipboardError>> {
        self.calls.borrow_mut().push("set");
        let gate = self.gate.borrow().clone();
        let error = self.failure.borrow().clone();
        let calls = self.calls.clone();
        if error.is_none() {
            *self.data.borrow_mut() = data;
        }
        Box::pin(async move {
            calls.borrow_mut().push("write-poll");
            if let Some(gate) = gate {
                gate.future().await.unwrap();
            }
            calls.borrow_mut().push("write-finish");
            error.map_or(Ok(()), Err)
        })
    }
    fn try_get_data_async(
        &self,
    ) -> LocalBoxFuture<Result<Option<Rc<dyn IAsyncDataTransfer>>, ClipboardError>> {
        self.calls.borrow_mut().push("get");
        let value = self.data.borrow().clone();
        let error = self.failure.borrow().clone();
        let gate = self.gate.borrow().clone();
        Box::pin(async move {
            if let Some(gate) = gate {
                gate.future().await.unwrap();
            }
            error.map_or(Ok(value), Err)
        })
    }
    fn try_get_in_process_data_async(
        &self,
    ) -> LocalBoxFuture<Result<Option<Rc<dyn IAsyncDataTransfer>>, ClipboardError>> {
        panic!("not source GetContent")
    }
}
struct Transfer {
    data: Rc<DataTransfer>,
    disposed: Cell<i32>,
    fail_items: bool,
}
impl IDisposable for Transfer {
    fn dispose(&self) {
        self.disposed.set(self.disposed.get() + 1);
    }
}
impl IAsyncDataTransfer for Transfer {
    fn formats(&self) -> Rc<[DataFormat]> {
        self.data.formats()
    }
    fn items(&self) -> Rc<[Rc<dyn IAsyncDataTransferItem>]> {
        IAsyncDataTransfer::items(self.data.as_ref())
    }
    fn try_items(&self) -> Result<Rc<[Rc<dyn IAsyncDataTransferItem>]>, ClipboardError> {
        if self.fail_items {
            Err(ClipboardError::access_denied("items"))
        } else {
            Ok(self.items())
        }
    }
}
fn transfer(text: Option<&str>, fail_items: bool) -> Rc<Transfer> {
    let data = DataTransfer::new();
    if let Some(text) = text {
        data.add(DataTransferItem::create_text(Some(text)));
    }
    Rc::new(Transfer {
        data,
        disposed: Cell::new(0),
        fail_items,
    })
}
#[test]
fn clipboard_roundtrip_denial_null_encoding_and_disposal() {
    headless::run(|| {
        let backend = Rc::new(Backend::default());
        let resolve = backend.clone();
        let service = SystemClipboardService::with_resolver(move || Some(resolve.clone()));
        for text in ["", "hello 😀", "a\r\nb\rאב", "👩🏽‍🚀"] {
            let write = service.set_text_async(Some(text.into()));
            assert!(native_future::ready(write).is_ok());
            assert_eq!(
                native_future::ready(service.get_text_async()).unwrap(),
                Some(text.into())
            );
        }
        let before = backend.data.borrow().clone().unwrap();
        let calls = backend.calls.borrow().len();
        assert!(native_future::ready(service.set_text_async(None)).is_ok());
        assert!(Rc::ptr_eq(&before, backend.data.borrow().as_ref().unwrap()));
        assert_eq!(calls, backend.calls.borrow().len());
        let invalid = Utf16Text::from_units(vec![0xD800]);
        let write = service.set_text_async(Some(invalid));
        assert_eq!(
            native_future::ready(write).unwrap_err().to_string(),
            "UnpairedSurrogate"
        );
        assert_eq!(calls, backend.calls.borrow().len());
        for kind in [
            ClipboardErrorKind::Timeout,
            ClipboardErrorKind::Canceled,
            ClipboardErrorKind::AccessDenied,
            ClipboardErrorKind::Platform,
            ClipboardErrorKind::Other,
        ] {
            *backend.failure.borrow_mut() = Some(ClipboardError::from_kind(kind));
            assert!(
                native_future::ready(service.get_text_async())
                    .unwrap()
                    .is_none()
            );
            assert!(native_future::ready(service.set_text_async(Some("ignored".into()))).is_ok());
        }
        *backend.failure.borrow_mut() = None;
        for (text, fail) in [(None, false), (Some("x"), false), (Some("x"), true)] {
            let data = transfer(text, fail);
            *backend.data.borrow_mut() = Some(data.clone());
            let result = native_future::ready(service.get_text_async()).unwrap();
            assert_eq!(
                result,
                if fail {
                    None
                } else {
                    text.map(Utf16Text::from)
                }
            );
            assert_eq!(data.disposed.get(), 1);
        }
        let missing = SystemClipboardService::with_resolver(|| None);
        assert!(
            native_future::ready(missing.get_text_async())
                .unwrap()
                .is_none()
        );
        assert!(native_future::ready(missing.set_text_async(Some("x".into()))).is_ok());
    });
}
#[test]
fn clipboard_pending_read_and_write_keep_source_completed_write_order() {
    headless::run(|| {
        let backend = Rc::new(Backend::default());
        let resolve = backend.clone();
        let service = SystemClipboardService::with_resolver(move || Some(resolve.clone()));
        let gate = native_future::Completion::default();
        *backend.gate.borrow_mut() = Some(gate.clone());
        let write = service.set_text_async(Some("pending".into()));
        assert!(native_future::ready(write).is_ok());
        assert_eq!(*backend.calls.borrow(), ["set", "write-poll"]);
        let mut read = service.get_text_async();
        assert!(native_future::poll(read.as_mut()).is_pending());
        drop(service);
        gate.complete(());
        ferroui_base::threading::Dispatcher::current_dispatcher().run_jobs(None);
        assert_eq!(native_future::ready(read).unwrap(), Some("pending".into()));
        assert_eq!(
            *backend.calls.borrow(),
            ["set", "write-poll", "get", "write-finish"]
        );
    });
}
#[test]
fn clipboard_actual_headless_locator_and_visual_root_resolution() {
    headless::run(|| {
        let service = SystemClipboardService::new();
        native_future::ready(service.set_text_async(Some("local 😀".into()))).unwrap();
        assert_eq!(
            native_future::ready(service.get_text_async()).unwrap(),
            Some("local 😀".into())
        );
        let input = TextBox::new();
        let visual: Ref<Visual> = input.clone().upcast();
        let scoped = SystemClipboardService::for_visual(visual);
        assert_eq!(native_future::ready(scoped.get_text_async()).unwrap(), None);
        let window = Window::new();
        window.set_content(Some(Control::boxed(input)));
        window.show();
        native_future::ready(scoped.set_text_async(Some("window".into()))).unwrap();
        assert_eq!(
            native_future::ready(scoped.get_text_async()).unwrap(),
            Some("window".into())
        );
        window.set_content(None);
        assert_eq!(native_future::ready(scoped.get_text_async()).unwrap(), None);
        window.close();
        for duration in [i32::MIN, -1, 0, 20, i32::MAX] {
            assert_eq!(
                KeyboardHaptics::perform(FeedbackKind::LongPress, duration),
                Ok(())
            );
        }
    });
}
#[test]
fn unchanged_source_platform_service_outcomes() {
    headless::run(|| {
        let corpus = include_str!("../../../tests/fixtures/platform-services.jsonl");
        for line in corpus.lines() {
            let row: serde_json::Value = serde_json::from_str(line).unwrap();
            match row["op"].as_str().unwrap() {
                "read" => {
                    let scenario = row["scenario"].as_str().unwrap();
                    let backend = Rc::new(Backend::default());
                    if scenario != "missing" {
                        let data = transfer(
                            if scenario == "no-text" {
                                None
                            } else if scenario == "empty" {
                                Some("")
                            } else {
                                Some("hello 😀")
                            },
                            scenario == "contains-failed",
                        );
                        *backend.data.borrow_mut() = Some(data);
                    }
                    if matches!(scenario, "read-denied" | "get-failed") {
                        *backend.failure.borrow_mut() =
                            Some(ClipboardError::access_denied("source fault"));
                    }
                    let resolve = backend.clone();
                    let service =
                        SystemClipboardService::with_resolver(move || Some(resolve.clone()));
                    let task = service.get_text_async();
                    let result = native_future::ready(task);
                    assert_eq!(result.is_ok(), row["completed"].as_bool().unwrap());
                    let text = result.unwrap().map(|s| s.units().to_vec());
                    assert_eq!(
                        serde_json::to_value(text).unwrap(),
                        row["text"],
                        "{scenario}"
                    );
                }
                "write" => {
                    let scenario = row["scenario"].as_str().unwrap();
                    let backend = Rc::new(Backend::default());
                    *backend.data.borrow_mut() = Some(transfer(Some("old"), false));
                    if matches!(scenario, "write-denied" | "package-failed") {
                        *backend.failure.borrow_mut() =
                            Some(ClipboardError::access_denied("source fault"));
                    }
                    let resolve = backend.clone();
                    let service =
                        SystemClipboardService::with_resolver(move || Some(resolve.clone()));
                    let task = service.set_text_async(if scenario == "null" {
                        None
                    } else {
                        Some(
                            if scenario == "empty" {
                                ""
                            } else {
                                "hello 😀"
                            }
                            .into(),
                        )
                    });
                    assert_eq!(
                        native_future::ready(task).is_ok(),
                        row["completed"].as_bool().unwrap()
                    );
                    *backend.failure.borrow_mut() = None;
                    let text = native_future::ready(service.get_text_async())
                        .unwrap()
                        .map(|s| s.units().to_vec());
                    assert_eq!(
                        serde_json::to_value(text).unwrap(),
                        row["text"],
                        "{scenario}"
                    );
                }
                "pending-before" | "pending-after" => {} // Actual pending/fault transitions covered in adjacent native test.
                "desktop-haptics" => {
                    let kind = match row["kind"].as_str().unwrap() {
                        "KeyPress" => FeedbackKind::KeyPress,
                        "LongPress" => FeedbackKind::LongPress,
                        "GestureStep" => FeedbackKind::GestureStep,
                        _ => panic!(),
                    };
                    KeyboardHaptics::perform(kind, row["duration"].as_i64().unwrap() as i32)
                        .unwrap();
                }
                op => panic!("{op}"),
            }
        }
    });
}

#[test]
fn native_future_api_preserves_platform_errors_and_disposes_read_transfers() {
    headless::run(|| {
        fn ready<T>(future: impl std::future::Future<Output = T>) -> T {
            let mut future = std::pin::pin!(future);
            match future
                .as_mut()
                .poll(&mut std::task::Context::from_waker(std::task::Waker::noop()))
            {
                std::task::Poll::Ready(value) => value,
                std::task::Poll::Pending => panic!("test clipboard must be ready"),
            }
        }
        let backend = Rc::new(Backend::default());
        let resolve = backend.clone();
        let service = SystemClipboardService::with_resolver(move || Some(resolve.clone()));
        ready(service.write_text("native 😀")).unwrap();
        assert_eq!(
            ready(service.read_text()).unwrap().as_deref(),
            Some("native 😀")
        );
        *backend.failure.borrow_mut() = Some(ClipboardError::access_denied("native denial"));
        assert_eq!(
            ready(service.read_text()).unwrap_err().kind(),
            ClipboardErrorKind::AccessDenied
        );
        assert_eq!(
            ready(service.write_text("denied")).unwrap_err().kind(),
            ClipboardErrorKind::AccessDenied
        );
        *backend.failure.borrow_mut() = None;
        let data = transfer(Some("owned"), false);
        *backend.data.borrow_mut() = Some(data.clone());
        assert_eq!(
            ready(service.read_text()).unwrap().as_deref(),
            Some("owned")
        );
        assert_eq!(data.disposed.get(), 1);
        let missing = SystemClipboardService::with_resolver(|| None);
        assert_eq!(
            ready(missing.read_text()).unwrap_err().kind(),
            ClipboardErrorKind::Platform
        );
        assert_eq!(
            ready(missing.write_text("x")).unwrap_err().kind(),
            ClipboardErrorKind::Platform
        );
    });
}

#[test]
fn native_haptic_device_preserves_duration_and_unavailable_result() {
    struct Device {
        durations: Rc<RefCell<Vec<i32>>>,
        available: bool,
    }
    impl HapticDevice for Device {
        fn vibrate(&self, duration: i32) -> bool {
            self.durations.borrow_mut().push(duration);
            self.available
        }
    }
    let durations = Rc::new(RefCell::new(Vec::new()));
    let available = KeyboardHapticService::new(Rc::new(Device {
        durations: durations.clone(),
        available: true,
    }));
    assert!(available.perform(FeedbackKind::LongPress, 12));
    assert!(available.perform(FeedbackKind::LongPress, i32::MAX));
    let denied = KeyboardHapticService::new(Rc::new(Device {
        durations: durations.clone(),
        available: false,
    }));
    assert!(!denied.perform(FeedbackKind::LongPress, 0));
    assert_eq!(*durations.borrow(), vec![24, -2, 0]);
}
