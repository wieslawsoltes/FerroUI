//! Tests of the data transfer of the backend through the whole stack: a
//! window as a drop target and a bitmap on the clipboard. Not a port:
//! upstream's tests of this area (`OleVirtualFileDataTests`) test types
//! that are internal to the backend and are tests of its crate
//! (`ole_tests.rs`); these two need what only this crate sets up, a
//! window of the window class and the render backend.

use crate::TestCase;

pub fn tests(cases: &mut Vec<TestCase>) {
    #[cfg(windows)]
    {
        cases.push(TestCase::new(
            "ole_data_tests::a_shown_window_is_registered_as_a_drop_target".to_string(),
            windows::a_shown_window_is_registered_as_a_drop_target,
        ));
        cases.push(TestCase::new(
            "ole_data_tests::a_bitmap_round_trips_through_the_clipboard".to_string(),
            windows::a_bitmap_round_trips_through_the_clipboard,
        ));
    }
    #[cfg(not(windows))]
    let _ = cases;
}

#[cfg(windows)]
mod windows {
    use crate::unmanaged_methods::get_prop;
    use crate::window_extensions::when_loaded;
    use crate::WindowGuard;
    use ferroui_base::input::platform::IClipboardImpl;
    use ferroui_base::input::{
        DataFormat, DataTransfer, DataTransferExtensions, DataTransferItem, IAsyncDataTransfer, IDataTransfer,
    };
    use ferroui_base::logging::LogArea;
    use ferroui_base::media::imaging::Bitmap;
    use ferroui_base::platform::{AlphaFormat, PixelFormat};
    use ferroui_base::reactive::IDisposable;
    use ferroui_base::{FerroLocator, LocatorExtensions, PixelRect, PixelSize, Vector};
    use ferroui_controls::Window;
    use std::future::Future;
    use std::pin::Pin;
    use std::rc::Rc;
    use std::task::{Context, Poll, Waker};

    /// The value of a future of the clipboard, which is ready at once
    /// unless another process holds the clipboard open.
    fn ready<T>(mut future: Pin<Box<dyn Future<Output = T>>>) -> T {
        match future.as_mut().poll(&mut Context::from_waker(Waker::noop())) {
            Poll::Ready(value) => value,
            Poll::Pending => panic!("the clipboard is held open by another process"),
        }
    }

    /// OLE keeps the drop target of a window as a property of the window.
    pub fn a_shown_window_is_registered_as_a_drop_target() {
        let window = Window::new();
        window.set_width(200.0);
        window.set_height(200.0);
        let _guard = WindowGuard(window.clone());
        window.show();
        when_loaded(&window);

        let hwnd = window.try_get_platform_handle().expect("a shown window has a handle").handle();
        assert_ne!(get_prop(hwnd, "OleDropTargetInterface"), 0, "the window has no drop target registered with OLE");
    }

    pub fn a_bitmap_round_trips_through_the_clipboard() {
        // Four by three pixels, each of its own colour, blue first.
        let (width, height) = (4, 3);
        let mut pixels = Vec::new();
        for y in 0..height {
            for x in 0..width {
                pixels.extend_from_slice(&[(x * 60) as u8, (y * 100) as u8, (x * 20 + y * 30 + 15) as u8, 255]);
            }
        }
        let bitmap = Rc::new(Bitmap::from_pixels(
            PixelFormat::BGRA8888,
            AlphaFormat::Opaque,
            &pixels,
            PixelSize::new(width, height),
            Vector::new(96.0, 96.0),
            width * 4,
        ));

        let item = DataTransferItem::new();
        item.set_bitmap(Some(bitmap));
        let data_transfer = DataTransfer::new();
        data_transfer.add(item);
        let data_transfer: Rc<dyn IDataTransfer> = data_transfer;

        let clipboard = FerroLocator::current().get_required_service::<dyn IClipboardImpl>();
        ready(clipboard.set_data_async(data_transfer.to_asynchronous())).expect("the bitmap is put on the clipboard");
        let read: Rc<dyn IAsyncDataTransfer> =
            ready(clipboard.try_get_data_async()).expect("the clipboard is read").expect("the clipboard has data");
        let read = read.to_synchronous(LogArea::WIN32_PLATFORM);

        let formats = read.formats();
        assert!(formats.contains(&DataFormat::bitmap()), "the formats have no bitmap: {formats:?}");
        // The formats a bitmap is offered in, as the system names them.
        for name in ["image/png", "PNG", "CF_DIB", "CF_DIBV5", "CF_BITMAP"] {
            assert!(formats.iter().any(|format| format.identifier() == name), "no format {name}: {formats:?}");
        }

        let read_bitmap = read.try_get_bitmap().expect("the clipboard has a bitmap");
        assert_eq!(read_bitmap.pixel_size(), PixelSize::new(width, height));
        let format = read_bitmap.format().expect("the bitmap that was read has a pixel format");
        let mut read_pixels = vec![0u8; pixels.len()];
        read_bitmap.copy_pixels(PixelRect::new(0, 0, width, height), &mut read_pixels, width * 4);
        if format == PixelFormat::RGBA8888 {
            for pixel in read_pixels.chunks_exact_mut(4) {
                pixel.swap(0, 2);
            }
        } else {
            assert_eq!(format, PixelFormat::BGRA8888);
        }
        assert_eq!(read_pixels, pixels, "the pixels of the bitmap changed on the way");

        read.dispose();
        ready(clipboard.clear_async()).expect("the clipboard is cleared");
    }
}
