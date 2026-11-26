//! `IClipboard::content_changed` on top of a platform clipboard.

use super::{Clipboard, ClipboardError, IClipboard, IClipboardChangeImpl, IClipboardImpl};
use crate::input::{IAsyncDataTransfer, LocalBoxFuture};
use crate::reactive::{Disposable, Subscription};
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

/// A platform clipboard whose operations complete at once; it reports changes when `observes`.
struct FakeClipboard {
    observes: bool,
    handler: Rc<RefCell<Option<Rc<dyn Fn()>>>>,
    subscriptions: Rc<Cell<u32>>,
}

impl FakeClipboard {
    fn new(observes: bool) -> Rc<FakeClipboard> {
        Rc::new(FakeClipboard { observes, handler: Rc::default(), subscriptions: Rc::default() })
    }

    /// Another application changes the clipboard.
    fn change_from_outside(&self) {
        let handler = self.handler.borrow().clone();
        if let Some(handler) = handler {
            handler();
        }
    }
}

impl IClipboardImpl for FakeClipboard {
    fn try_get_data_async(&self) -> LocalBoxFuture<Result<Option<Rc<dyn IAsyncDataTransfer>>, ClipboardError>> {
        Box::pin(std::future::ready(Ok(None)))
    }

    fn set_data_async(&self, _data_transfer: Rc<dyn IAsyncDataTransfer>) -> LocalBoxFuture<Result<(), ClipboardError>> {
        Box::pin(std::future::ready(Ok(())))
    }

    fn clear_async(&self) -> LocalBoxFuture<Result<(), ClipboardError>> {
        Box::pin(std::future::ready(Ok(())))
    }

    fn as_clipboard_change_impl(&self) -> Option<&dyn IClipboardChangeImpl> {
        self.observes.then_some(self as &dyn IClipboardChangeImpl)
    }
}

impl IClipboardChangeImpl for FakeClipboard {
    fn subscribe_content_changed(&self, handler: Rc<dyn Fn()>) -> Subscription {
        self.subscriptions.set(self.subscriptions.get() + 1);
        self.handler.replace(Some(handler));
        let slot = self.handler.clone();
        let subscriptions = self.subscriptions.clone();
        Subscription::new(Disposable::create(move || {
            subscriptions.set(subscriptions.get() - 1);
            slot.replace(None);
        }))
    }
}

fn complete(mut operation: LocalBoxFuture<Result<(), ClipboardError>>) -> Result<(), ClipboardError> {
    match operation.as_mut().poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(result) => result,
        Poll::Pending => panic!("the fake clipboard completes at once"),
    }
}

fn counter() -> (Rc<Cell<u32>>, Rc<dyn Fn()>) {
    let count = Rc::new(Cell::new(0));
    let handler = {
        let count = count.clone();
        Rc::new(move || count.set(count.get() + 1)) as Rc<dyn Fn()>
    };
    (count, handler)
}

#[test]
fn without_platform_changes_the_clipboard_reports_its_own() {
    let platform = FakeClipboard::new(false);
    let clipboard = Clipboard::new(platform.clone());
    assert!(!clipboard.is_content_changed_supported());
    let (count, handler) = counter();
    let subscription = clipboard.content_changed(handler);

    complete(clipboard.clear_async()).unwrap();
    assert_eq!(1, count.get());
    drop(subscription);
    complete(clipboard.clear_async()).unwrap();
    assert_eq!(1, count.get(), "a dropped subscription is not called");
}

#[test]
fn the_platform_is_observed_only_while_there_is_a_handler() {
    let platform = FakeClipboard::new(true);
    let clipboard = Clipboard::new(platform.clone());
    assert!(clipboard.is_content_changed_supported());
    assert_eq!(0, platform.subscriptions.get(), "nothing is observed before the application asks");

    let (first_count, first) = counter();
    let (second_count, second) = counter();
    let first = clipboard.content_changed(first);
    let second = clipboard.content_changed(second);
    assert_eq!(1, platform.subscriptions.get(), "one platform subscription for every handler");

    platform.change_from_outside();
    assert_eq!((1, 1), (first_count.get(), second_count.get()));
    // The platform reports this clipboard's own changes too: they are not reported twice.
    complete(clipboard.clear_async()).unwrap();
    assert_eq!((1, 1), (first_count.get(), second_count.get()));

    drop(first);
    assert_eq!(1, platform.subscriptions.get());
    drop(second);
    assert_eq!(0, platform.subscriptions.get(), "the platform stops when the last handler goes");
}

mod history {
    use super::*;
    use crate::input::platform::{
        ClipboardContentOptions, ClipboardHistoryItemsResult, ClipboardHistoryItemsResultStatus, IClipboardHistoryImpl,
    };
    use crate::input::DataTransfer;

    /// A platform clipboard with a history; it records the options of the last copy.
    #[derive(Default)]
    struct HistoryClipboard {
        options: Cell<Option<ClipboardContentOptions>>,
    }

    impl IClipboardImpl for HistoryClipboard {
        fn try_get_data_async(&self) -> LocalBoxFuture<Result<Option<Rc<dyn IAsyncDataTransfer>>, ClipboardError>> {
            Box::pin(std::future::ready(Ok(None)))
        }
        fn set_data_async(&self, _data_transfer: Rc<dyn IAsyncDataTransfer>) -> LocalBoxFuture<Result<(), ClipboardError>> {
            Box::pin(std::future::ready(Ok(())))
        }
        fn clear_async(&self) -> LocalBoxFuture<Result<(), ClipboardError>> {
            Box::pin(std::future::ready(Ok(())))
        }
        fn as_clipboard_history_impl(&self) -> Option<&dyn IClipboardHistoryImpl> {
            Some(self)
        }
    }

    impl IClipboardHistoryImpl for HistoryClipboard {
        fn is_history_enabled(&self) -> bool {
            true
        }
        fn is_roaming_enabled(&self) -> bool {
            false
        }
        fn set_data_with_options_async(
            &self,
            _data_transfer: Rc<dyn IAsyncDataTransfer>,
            options: ClipboardContentOptions,
        ) -> LocalBoxFuture<Result<(), ClipboardError>> {
            self.options.set(Some(options));
            Box::pin(std::future::ready(Ok(())))
        }
        fn get_history_items_async(&self) -> LocalBoxFuture<Result<ClipboardHistoryItemsResult, ClipboardError>> {
            Box::pin(std::future::ready(Ok(ClipboardHistoryItemsResult::empty(
                ClipboardHistoryItemsResultStatus::ClipboardHistoryDisabled,
            ))))
        }
    }

    fn history_of(clipboard: &Rc<Clipboard>) -> ClipboardHistoryItemsResultStatus {
        let mut read = clipboard.get_history_items_async();
        match read.as_mut().poll(&mut Context::from_waker(Waker::noop())) {
            Poll::Ready(result) => result.unwrap().status,
            Poll::Pending => panic!("the fake clipboard completes at once"),
        }
    }

    #[test]
    fn a_platform_without_history_says_so_and_copies_without_the_options() {
        let platform = FakeClipboard::new(false);
        let clipboard = Clipboard::new(platform);
        assert!(!clipboard.is_history_enabled() && !clipboard.is_roaming_enabled());
        assert_eq!(ClipboardHistoryItemsResultStatus::NotSupported, history_of(&clipboard));
        let options = ClipboardContentOptions { is_allowed_in_history: false, is_roaming_allowed: false };
        complete(clipboard.set_data_with_options_async(Some(DataTransfer::new()), options)).unwrap();
    }

    #[test]
    fn a_platform_with_history_gets_the_options_and_answers_the_history() {
        let platform = Rc::new(HistoryClipboard::default());
        let clipboard = Clipboard::new(platform.clone());
        assert!(clipboard.is_history_enabled());
        assert!(!clipboard.is_roaming_enabled());
        let options = ClipboardContentOptions { is_allowed_in_history: false, is_roaming_allowed: true };
        complete(clipboard.set_data_with_options_async(Some(DataTransfer::new()), options)).unwrap();
        assert_eq!(Some(options), platform.options.get());
        assert_eq!(ClipboardHistoryItemsResultStatus::ClipboardHistoryDisabled, history_of(&clipboard));
        assert_eq!(ClipboardContentOptions { is_allowed_in_history: true, is_roaming_allowed: true }, ClipboardContentOptions::default());
    }
}
