use ferroui_base::input::StandardCursorType;
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::platform::{ICursorFactory, ICursorImpl};
use ferroui_base::PixelPoint;
use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// The cursor factory of Android: the pointer icons of the system for the standard cursors,
/// and icons made of bitmaps (`PointerIcon`, API 24; below it every cursor shows the default
/// pointer).
#[derive(Default)]
pub struct CursorFactory {
    cache: RefCell<HashMap<StandardCursorType, Rc<dyn ICursorImpl>>>,
}

/// A cursor: its `android.view.PointerIcon`, `None` where the system has none.
pub(crate) struct CursorImpl {
    #[cfg(target_os = "android")]
    pub(crate) pointer_icon: Option<crate::interop::java::JavaObject>,
}

/// The `PointerIcon` type of a standard cursor (the `TYPE_*` constants of the class).
pub(crate) fn pointer_icon_type(cursor_type: StandardCursorType) -> i32 {
    const TYPE_NULL: i32 = 0;
    const TYPE_ARROW: i32 = 1000;
    const TYPE_HAND: i32 = 1002;
    const TYPE_HELP: i32 = 1003;
    const TYPE_WAIT: i32 = 1004;
    const TYPE_CROSSHAIR: i32 = 1007;
    const TYPE_TEXT: i32 = 1008;
    const TYPE_ALIAS: i32 = 1010;
    const TYPE_COPY: i32 = 1011;
    const TYPE_NO_DROP: i32 = 1012;
    const TYPE_ALL_SCROLL: i32 = 1013;
    const TYPE_HORIZONTAL_DOUBLE_ARROW: i32 = 1014;
    const TYPE_VERTICAL_DOUBLE_ARROW: i32 = 1015;
    const TYPE_TOP_RIGHT_DIAGONAL_DOUBLE_ARROW: i32 = 1016;
    const TYPE_TOP_LEFT_DIAGONAL_DOUBLE_ARROW: i32 = 1017;
    const TYPE_GRABBING: i32 = 1021;

    use StandardCursorType as C;
    match cursor_type {
        C::Arrow => TYPE_ARROW,
        C::Cross => TYPE_CROSSHAIR,
        C::Hand => TYPE_HAND,
        C::Ibeam => TYPE_TEXT,
        C::Wait | C::AppStarting => TYPE_WAIT,
        C::Help => TYPE_HELP,
        C::No => TYPE_NO_DROP,
        C::SizeAll => TYPE_ALL_SCROLL,
        C::SizeNorthSouth | C::TopSide | C::BottomSide => TYPE_VERTICAL_DOUBLE_ARROW,
        C::SizeWestEast | C::LeftSide | C::RightSide => TYPE_HORIZONTAL_DOUBLE_ARROW,
        C::TopLeftCorner | C::BottomRightCorner => TYPE_TOP_LEFT_DIAGONAL_DOUBLE_ARROW,
        C::TopRightCorner | C::BottomLeftCorner => TYPE_TOP_RIGHT_DIAGONAL_DOUBLE_ARROW,
        C::DragMove => TYPE_GRABBING,
        C::DragCopy => TYPE_COPY,
        C::DragLink => TYPE_ALIAS,
        C::None => TYPE_NULL,
        _ => TYPE_ARROW,
    }
}

impl CursorImpl {
    #[cfg(target_os = "android")]
    fn system(cursor_type: StandardCursorType) -> Self {
        use crate::ferro_android_application::FerroAndroidApplication;
        use crate::interop::java::{call_static_object, JavaClass, JavaValue};
        use crate::interop::natives::PLATFORM_HELPER;

        let pointer_icon = FerroAndroidApplication::try_context().and_then(|context| {
            call_static_object(
                &JavaClass::find(PLATFORM_HELPER),
                "systemPointerIcon",
                "(Landroid/content/Context;I)Ljava/lang/Object;",
                &[JavaValue::Object(Some(&context)), JavaValue::Int(pointer_icon_type(cursor_type))],
            )
        });
        Self { pointer_icon: pointer_icon.map(|icon| icon.to_global()) }
    }

    #[cfg(not(target_os = "android"))]
    fn system(_cursor_type: StandardCursorType) -> Self {
        Self {}
    }

    #[cfg(target_os = "android")]
    fn from_bitmap(cursor: &Bitmap, hot_spot: PixelPoint) -> Self {
        use crate::interop::java::{call_static_object, new_int_array, JavaClass, JavaValue};
        use crate::interop::natives::PLATFORM_HELPER;

        let size = cursor.pixel_size();
        let pixels = argb_pixels(cursor);
        let pixels = new_int_array(&pixels);
        let pointer_icon = call_static_object(
            &JavaClass::find(PLATFORM_HELPER),
            "bitmapPointerIcon",
            "([IIIII)Ljava/lang/Object;",
            &[
                JavaValue::Object(Some(&pixels)),
                JavaValue::Int(size.width),
                JavaValue::Int(size.height),
                JavaValue::Int(hot_spot.x),
                JavaValue::Int(hot_spot.y),
            ],
        );
        Self { pointer_icon: pointer_icon.map(|icon| icon.to_global()) }
    }

    #[cfg(not(target_os = "android"))]
    fn from_bitmap(_cursor: &Bitmap, _hot_spot: PixelPoint) -> Self {
        Self {}
    }
}

/// The pixels of a bitmap as the colors of `Bitmap.createBitmap(int[], ...)`: unpremultiplied
/// `0xAARRGGBB`, row by row.
#[cfg(target_os = "android")]
fn argb_pixels(cursor: &Bitmap) -> Vec<i32> {
    use ferroui_base::platform::{AlphaFormat, PixelFormat, RetainedFramebuffer};
    use ferroui_base::Vector;

    let size = cursor.pixel_size();
    let framebuffer = RetainedFramebuffer::new(size, PixelFormat::BGRA8888, AlphaFormat::Unpremul);
    let locked = framebuffer.lock(Vector::new(96.0, 96.0), |_| {});
    cursor.copy_pixels_to_framebuffer(&*locked);
    let (width, height, row_bytes) =
        (size.width.max(0) as usize, size.height.max(0) as usize, locked.row_bytes().max(1) as usize);
    let mut pixels = Vec::with_capacity(width * height);
    locked.with_data(&mut |data| {
        for row in data.chunks_exact(row_bytes).take(height) {
            for pixel in row[..width * 4].chunks_exact(4) {
                let [b, g, r, a] = [pixel[0], pixel[1], pixel[2], pixel[3]];
                pixels.push(i32::from_be_bytes([a, r, g, b]));
            }
        }
    });
    locked.dispose();
    framebuffer.dispose();
    pixels
}

impl ICursorImpl for CursorImpl {
    fn dispose(&self) {}

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl ICursorFactory for CursorFactory {
    fn get_cursor(&self, cursor_type: StandardCursorType) -> Rc<dyn ICursorImpl> {
        self.cache
            .borrow_mut()
            .entry(cursor_type)
            .or_insert_with(|| Rc::new(CursorImpl::system(cursor_type)))
            .clone()
    }

    fn create_cursor(&self, cursor: &Bitmap, hot_spot: PixelPoint) -> Rc<dyn ICursorImpl> {
        Rc::new(CursorImpl::from_bitmap(cursor, hot_spot))
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of the factory.
    use super::*;

    #[test]
    fn standard_cursors_are_cached_per_type() {
        let factory = CursorFactory::default();
        let arrow = factory.get_cursor(StandardCursorType::Arrow);
        let hand = factory.get_cursor(StandardCursorType::Hand);

        assert!(std::ptr::addr_eq(Rc::as_ptr(&arrow), Rc::as_ptr(&factory.get_cursor(StandardCursorType::Arrow))));
        assert!(!std::ptr::addr_eq(Rc::as_ptr(&arrow), Rc::as_ptr(&hand)));
        arrow.dispose();
    }

    #[test]
    fn standard_cursors_map_to_the_pointer_icons_of_the_system() {
        assert_eq!(1000, pointer_icon_type(StandardCursorType::Arrow));
        assert_eq!(1008, pointer_icon_type(StandardCursorType::Ibeam));
        assert_eq!(1015, pointer_icon_type(StandardCursorType::TopSide));
        assert_eq!(1017, pointer_icon_type(StandardCursorType::BottomRightCorner));
        assert_eq!(1016, pointer_icon_type(StandardCursorType::BottomLeftCorner));
        assert_eq!(0, pointer_icon_type(StandardCursorType::None));
    }
}
