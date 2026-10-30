//! Portable ink transport through FerroUI's existing asynchronous clipboard.
use super::{InkError, InkServiceFuture, InkStrokeContainer};
use ferroui_base::input::platform::{ClipboardError, IClipboard};
use ferroui_base::input::{
    AsyncDataTransferExtensions, DataFormat, DataFormatOf, DataTransfer, DataTransferItem,
    IAsyncDataTransfer,
};
use ferroui_base::{Point, Rect};
use std::rc::Rc;

/// Shares portable ink between FerroUI applications on any clipboard backend.
/// Use the clipboard of the owning `TopLevel`; no global clipboard is assumed.
/// Native Windows Ink clipboard interoperability is supplied separately by
/// `IInkPlatformServices`, because its representation and API are different.
pub struct InkClipboard {
    clipboard: Rc<dyn IClipboard>,
}

impl InkClipboard {
    pub fn new(clipboard: Rc<dyn IClipboard>) -> Self {
        Self { clipboard }
    }

    /// A platform-independent application format, mapped by each backend to
    /// its native clipboard naming scheme.
    pub fn format() -> DataFormatOf<Rc<[u8]>> {
        DataFormat::create_bytes_application_format("org.ferroui.ink")
    }

    /// Captures selected strokes immediately. The clipboard owns the transfer
    /// after a successful write; subsequent edits cannot change its payload.
    pub fn copy_selected(&self, container: &InkStrokeContainer) -> InkServiceFuture<()> {
        if !container
            .get_strokes()
            .iter()
            .any(|stroke| stroke.selected())
        {
            return Box::pin(async { Ok(()) });
        }
        let bytes = container.copy_selected();
        let clipboard = self.clipboard.clone();
        Box::pin(async move {
            let transfer = DataTransfer::new();
            transfer.add(DataTransferItem::create(
                &Self::format(),
                Some(Rc::from(bytes?)),
            ));
            clipboard
                .set_data_async(Some(transfer))
                .await
                .map_err(platform)
        })
    }

    pub fn can_paste(&self) -> InkServiceFuture<bool> {
        let clipboard = self.clipboard.clone();
        Box::pin(async move {
            let Some(transfer) = clipboard.try_get_data_async().await.map_err(platform)? else {
                return Ok(false);
            };
            let transfer = TransferGuard(transfer);
            let format = Self::format();
            Ok(transfer
                .0
                .try_formats()
                .map_err(platform)?
                .iter()
                .any(|candidate| candidate == &*format))
        })
    }

    /// Appends fresh stroke identities, preserving existing ink if fetching or
    /// decoding fails. Selection and positioning use the container's atomic
    /// portable-paste implementation.
    pub fn paste(
        &self,
        container: Rc<InkStrokeContainer>,
        position: Point,
    ) -> InkServiceFuture<Rect> {
        let clipboard = self.clipboard.clone();
        Box::pin(async move {
            if !super::ink_point::valid_position(position) {
                return Err(InkError::InvalidPoint);
            }
            let Some(transfer) = clipboard.try_get_data_async().await.map_err(platform)? else {
                return Ok(Rect::default());
            };
            let transfer = TransferGuard(transfer);
            let Some(bytes) = transfer
                .0
                .try_get_value_async(&Self::format())
                .await
                .map_err(platform)?
            else {
                return Ok(Rect::default());
            };
            container.paste(&bytes, position)
        })
    }
}

// The clipboard read contract requires disposal, including when a future is
// canceled while an individual data item is still being fetched.
struct TransferGuard(Rc<dyn IAsyncDataTransfer>);
impl Drop for TransferGuard {
    fn drop(&mut self) {
        self.0.dispose();
    }
}

fn platform(error: ClipboardError) -> InkError {
    InkError::Platform(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{InkDrawingAttributes, InkPoint, InkStroke};
    use ferroui_base::input::{IAsyncDataTransferItem, LocalBoxFuture};
    use ferroui_base::reactive::IDisposable;
    use std::cell::{Cell, RefCell};
    use std::task::{Context, Poll, Waker};

    #[derive(Default)]
    struct MemoryClipboard {
        data: RefCell<Option<Rc<dyn IAsyncDataTransfer>>>,
    }
    impl IClipboard for MemoryClipboard {
        fn clear_async(&self) -> LocalBoxFuture<Result<(), ClipboardError>> {
            self.data.borrow_mut().take();
            Box::pin(async { Ok(()) })
        }
        fn set_data_async(
            &self,
            value: Option<Rc<dyn IAsyncDataTransfer>>,
        ) -> LocalBoxFuture<Result<(), ClipboardError>> {
            *self.data.borrow_mut() = value;
            Box::pin(async { Ok(()) })
        }
        fn flush_async(&self) -> LocalBoxFuture<Result<(), ClipboardError>> {
            Box::pin(async { Ok(()) })
        }
        fn try_get_data_async(
            &self,
        ) -> LocalBoxFuture<Result<Option<Rc<dyn IAsyncDataTransfer>>, ClipboardError>> {
            let data = self.data.borrow().clone();
            Box::pin(async { Ok(data) })
        }
        fn try_get_in_process_data_async(
            &self,
        ) -> LocalBoxFuture<Result<Option<Rc<dyn IAsyncDataTransfer>>, ClipboardError>> {
            self.try_get_data_async()
        }
    }
    struct TrackedTransfer {
        data: Rc<DataTransfer>,
        disposed: Rc<Cell<u32>>,
        fail: bool,
    }
    impl IDisposable for TrackedTransfer {
        fn dispose(&self) {
            self.disposed.set(self.disposed.get() + 1);
        }
    }
    impl IAsyncDataTransfer for TrackedTransfer {
        fn formats(&self) -> Rc<[DataFormat]> {
            self.data.formats()
        }
        fn items(&self) -> Rc<[Rc<dyn IAsyncDataTransferItem>]> {
            IAsyncDataTransfer::items(&*self.data)
        }
        fn try_items(&self) -> Result<Rc<[Rc<dyn IAsyncDataTransferItem>]>, ClipboardError> {
            if self.fail {
                Err(ClipboardError::access_denied("test failure"))
            } else {
                Ok(self.items())
            }
        }
    }
    fn ready<T>(mut future: InkServiceFuture<T>) -> Result<T, InkError> {
        match future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
        {
            Poll::Ready(result) => result,
            Poll::Pending => panic!("memory clipboard unexpectedly yielded"),
        }
    }
    fn stroke() -> Rc<InkStroke> {
        InkStroke::new(
            vec![InkPoint::new(Point::new(10.0, 20.0), 1.0).unwrap()],
            InkDrawingAttributes::default(),
        )
        .unwrap()
    }

    #[test]
    fn copies_selection_as_snapshot_and_pastes_fresh_positioned_strokes() {
        let clipboard = Rc::new(MemoryClipboard::default());
        let service = InkClipboard::new(clipboard);
        let container = InkStrokeContainer::new();
        let selected = stroke();
        selected.set_selected(true);
        container.add_strokes([selected.clone(), stroke()]).unwrap();
        let copy = service.copy_selected(&container);
        selected.set_selected(false);
        ready(copy).unwrap();
        assert!(ready(service.can_paste()).unwrap());
        let bounds = ready(service.paste(container.clone(), Point::new(100.0, 200.0))).unwrap();
        assert_eq!(bounds.position(), Point::new(100.0, 200.0));
        assert_eq!(container.len(), 3);
        assert_ne!(container.get_strokes()[2].id(), selected.id());
        assert!(container.get_strokes()[2].selected());
    }

    #[test]
    fn all_read_paths_dispose_transfer_and_failures_preserve_existing_ink() {
        let clipboard = Rc::new(MemoryClipboard::default());
        let service = InkClipboard::new(clipboard.clone());
        let container = InkStrokeContainer::new();
        let original = stroke();
        container.add_stroke(original.clone()).unwrap();
        let disposed = Rc::new(Cell::new(0));
        for fail in [false, true] {
            let data = DataTransfer::new();
            data.add(DataTransferItem::create(
                &InkClipboard::format(),
                Some(Rc::from(&b"invalid ink"[..])),
            ));
            *clipboard.data.borrow_mut() = Some(Rc::new(TrackedTransfer {
                data,
                disposed: disposed.clone(),
                fail,
            }));
            assert!(ready(service.can_paste()).unwrap());
            assert!(ready(service.paste(container.clone(), Point::default())).is_err());
            assert_eq!(container.len(), 1);
            assert!(Rc::ptr_eq(&container.get_strokes()[0], &original));
        }
        assert_eq!(disposed.get(), 4);
    }
}
