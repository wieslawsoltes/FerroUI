use crate::interop::InputSystemCursorShape;
use ferroui_base::input::StandardCursorType;

#[cfg(windows)]
pub(crate) use imp::{WinUICursorFactory, WinUICursorImpl};

#[cfg(windows)]
mod imp {
    use super::map_shape;
    use crate::interop::win_rt::factory;
    use crate::interop::{IInputCursor, IInputSystemCursorStatics, InputSystemCursorShape};
    use ferroui_base::input::StandardCursorType;
    use ferroui_base::media::imaging::Bitmap;
    use ferroui_base::platform::{ICursorFactory, ICursorImpl};
    use ferroui_base::PixelPoint;
    use ferroui_microcom::ComPtr;
    use std::any::Any;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    /// Wraps a WinUI `InputCursor` so it can flow through the framework's
    /// platform-agnostic cursor APIs.
    pub(crate) struct WinUICursorImpl {
        cursor: Option<ComPtr<IInputCursor>>,
    }

    impl WinUICursorImpl {
        pub(crate) fn new(cursor: Option<ComPtr<IInputCursor>>) -> WinUICursorImpl {
            WinUICursorImpl { cursor }
        }

        pub(crate) fn cursor(&self) -> Option<ComPtr<IInputCursor>> {
            self.cursor.clone()
        }
    }

    impl ICursorImpl for WinUICursorImpl {
        fn dispose(&self) {
            // InputSystemCursor instances are cached — never disposed.
        }

    }

    /// `InputSystemCursor.Create(shape)`, as an `InputCursor`; none when the
    /// system refuses.
    fn create_system_cursor(shape: InputSystemCursorShape) -> Option<ComPtr<IInputCursor>> {
        let statics = factory::<IInputSystemCursorStatics>("Microsoft.UI.Input.InputSystemCursor").ok()?;
        statics.create(shape).ok().flatten().and_then(|cursor| cursor.cast::<IInputCursor>().ok())
    }

    pub(crate) struct WinUICursorFactory {
        // The reference's dictionary is concurrent; the cursors of WinUI are
        // objects of the UI thread, and so is this factory (one per thread).
        cache: RefCell<HashMap<StandardCursorType, Rc<WinUICursorImpl>>>,
    }

    ferroui_base::ferro_thread_local! {
        static INSTANCE: Rc<WinUICursorFactory> = Rc::new(WinUICursorFactory::new());
    }

    impl WinUICursorFactory {
        pub(crate) fn instance() -> Rc<WinUICursorFactory> {
            INSTANCE.with(Rc::clone)
        }

        fn new() -> WinUICursorFactory {
            WinUICursorFactory { cache: RefCell::new(HashMap::new()) }
        }

        fn get_or_add(&self, cursor_type: StandardCursorType, create: impl FnOnce() -> WinUICursorImpl) -> Rc<WinUICursorImpl> {
            if let Some(cursor) = self.cache.borrow().get(&cursor_type) {
                return cursor.clone();
            }
            let cursor = Rc::new(create());
            self.cache.borrow_mut().entry(cursor_type).or_insert(cursor).clone()
        }
    }

    impl ICursorFactory for WinUICursorFactory {
        fn get_cursor(&self, cursor_type: StandardCursorType) -> Rc<dyn ICursorImpl> {
            self.get_or_add(cursor_type, || {
                let shape = map_shape(cursor_type);
                let cursor = shape.and_then(create_system_cursor);
                WinUICursorImpl::new(cursor)
            })
        }

        fn create_cursor(&self, _cursor: &Bitmap, _hot_spot: PixelPoint) -> Rc<dyn ICursorImpl> {
            // WinUI 3 exposes no public API to build an InputCursor from an
            // in-memory image (only InputSystemCursor.Create(shape) and
            // InputDesktopResourceCursor.CreateFromResource(fileName) are
            // available). Fall back to the default arrow so the caller still
            // gets a usable cursor.

            // The reference leaves a note to do custom bitmap cursors here.
            self.get_or_add(StandardCursorType::Arrow, || {
                WinUICursorImpl::new(create_system_cursor(InputSystemCursorShape::Arrow))
            })
        }
    }
}

pub(crate) fn map_shape(t: StandardCursorType) -> Option<InputSystemCursorShape> {
    Some(match t {
        StandardCursorType::None => return None,
        StandardCursorType::Arrow => InputSystemCursorShape::Arrow,
        StandardCursorType::Ibeam => InputSystemCursorShape::IBeam,
        StandardCursorType::Wait => InputSystemCursorShape::Wait,
        StandardCursorType::AppStarting => InputSystemCursorShape::Wait,
        StandardCursorType::Cross => InputSystemCursorShape::Cross,
        StandardCursorType::Help => InputSystemCursorShape::Help,
        StandardCursorType::Hand => InputSystemCursorShape::Hand,
        StandardCursorType::No => InputSystemCursorShape::UniversalNo,
        StandardCursorType::SizeAll => InputSystemCursorShape::SizeAll,
        StandardCursorType::SizeNorthSouth => InputSystemCursorShape::SizeNorthSouth,
        StandardCursorType::SizeWestEast => InputSystemCursorShape::SizeWestEast,
        StandardCursorType::UpArrow => InputSystemCursorShape::UpArrow,
        StandardCursorType::TopSide => InputSystemCursorShape::SizeNorthSouth,
        StandardCursorType::BottomSide => InputSystemCursorShape::SizeNorthSouth,
        StandardCursorType::LeftSide => InputSystemCursorShape::SizeWestEast,
        StandardCursorType::RightSide => InputSystemCursorShape::SizeWestEast,
        StandardCursorType::TopLeftCorner => InputSystemCursorShape::SizeNorthwestSoutheast,
        StandardCursorType::BottomRightCorner => InputSystemCursorShape::SizeNorthwestSoutheast,
        StandardCursorType::TopRightCorner => InputSystemCursorShape::SizeNortheastSouthwest,
        StandardCursorType::BottomLeftCorner => InputSystemCursorShape::SizeNortheastSouthwest,
        // No built-in drag cursors — fall back to Arrow; OLE drag operations on
        // Win32 traditionally use ole32.dll resources we can't reach from here.
        StandardCursorType::DragMove => InputSystemCursorShape::Arrow,
        StandardCursorType::DragCopy => InputSystemCursorShape::Arrow,
        StandardCursorType::DragLink => InputSystemCursorShape::Arrow,
    })
}

#[cfg(test)]
mod tests {
    // Not from upstream: the reference has no tests of the integration.
    use super::*;

    #[test]
    fn the_shapes_of_the_standard_cursors() {
        assert_eq!(None, map_shape(StandardCursorType::None));
        assert_eq!(Some(InputSystemCursorShape::IBeam), map_shape(StandardCursorType::Ibeam));
        assert_eq!(Some(InputSystemCursorShape::Wait), map_shape(StandardCursorType::AppStarting));
        assert_eq!(Some(InputSystemCursorShape::UniversalNo), map_shape(StandardCursorType::No));
        assert_eq!(Some(InputSystemCursorShape::SizeNorthwestSoutheast), map_shape(StandardCursorType::TopLeftCorner));
        assert_eq!(Some(InputSystemCursorShape::SizeNortheastSouthwest), map_shape(StandardCursorType::BottomLeftCorner));
        assert_eq!(Some(InputSystemCursorShape::Arrow), map_shape(StandardCursorType::DragLink));
        // The values of the system (Microsoft.UI.Input.InputSystemCursorShape).
        assert_eq!(0, InputSystemCursorShape::Arrow.0);
        assert_eq!(1, InputSystemCursorShape::Cross.0);
        assert_eq!(4, InputSystemCursorShape::Help.0);
        assert_eq!(13, InputSystemCursorShape::Wait.0);
    }
}
