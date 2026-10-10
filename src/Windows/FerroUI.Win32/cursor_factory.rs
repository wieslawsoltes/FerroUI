//! Mouse cursors: the cursors of the system by their resource identifiers.

use ferroui_base::input::StandardCursorType;

/// The resource identifier of the system cursor for a standard cursor type
/// (0 for no cursor).
pub fn cursor_resource_id(cursor_type: StandardCursorType) -> i32 {
    match cursor_type {
        StandardCursorType::None => 0,
        StandardCursorType::AppStarting => 32650,
        StandardCursorType::Arrow => 32512,
        StandardCursorType::Cross => 32515,
        StandardCursorType::Hand => 32649,
        StandardCursorType::Help => 32651,
        StandardCursorType::Ibeam => 32513,
        StandardCursorType::No => 32648,
        StandardCursorType::SizeAll => 32646,
        StandardCursorType::UpArrow => 32516,
        StandardCursorType::SizeNorthSouth => 32645,
        StandardCursorType::SizeWestEast => 32644,
        StandardCursorType::Wait => 32514,
        //Same as SizeNorthSouth
        StandardCursorType::TopSide => 32645,
        StandardCursorType::BottomSide => 32645,
        //Same as SizeWestEast
        StandardCursorType::LeftSide => 32644,
        StandardCursorType::RightSide => 32644,
        //Using SizeNorthWestSouthEast
        StandardCursorType::TopLeftCorner => 32642,
        StandardCursorType::BottomRightCorner => 32642,
        //Using SizeNorthEastSouthWest
        StandardCursorType::TopRightCorner => 32643,
        StandardCursorType::BottomLeftCorner => 32643,

        // Fallback, should have been loaded from ole32.dll
        StandardCursorType::DragMove => 32516,
        StandardCursorType::DragCopy => 32516,
        StandardCursorType::DragLink => 32516,
    }
}

#[cfg(windows)]
pub use imp::{CursorFactory, CursorImpl};

#[cfg(windows)]
mod imp {
    use super::*;
    use crate::interop::unmanaged_methods::{get_module_handle, load_cursor};
    use crate::interop::win32_icon::Win32Icon;
    use crate::platform_constants::PlatformConstants;
    use ferroui_base::media::imaging::Bitmap;
    use ferroui_base::platform::{ICursorFactory, ICursorImpl};
    use ferroui_base::PixelPoint;
    use std::any::Any;
    use std::cell::{Cell, RefCell};
    use std::collections::HashMap;
    use std::rc::Rc;

    /// Creates the cursors of the Windows backend.
    pub struct CursorFactory {
        cache: RefCell<HashMap<StandardCursorType, Rc<CursorImpl>>>,
    }

    impl CursorFactory {
        /// The cursor factory of the calling thread.
        pub fn instance() -> Rc<CursorFactory> {
            thread_local! {
                static INSTANCE: Rc<CursorFactory> = Rc::new(CursorFactory::new());
            }
            INSTANCE.with(Rc::clone)
        }

        fn new() -> Self {
            let factory = Self { cache: RefCell::new(HashMap::new()) };
            factory.load_module_cursor(StandardCursorType::DragMove, "ole32.dll", 2);
            factory.load_module_cursor(StandardCursorType::DragCopy, "ole32.dll", 3);
            factory.load_module_cursor(StandardCursorType::DragLink, "ole32.dll", 4);
            factory
        }

        fn load_module_cursor(&self, cursor_type: StandardCursorType, module: &str, id: i32) {
            let mh = get_module_handle(Some(module));
            if mh != 0 {
                let cursor = load_cursor(mh, id);
                if cursor != 0 {
                    self.cache.borrow_mut().insert(cursor_type, Rc::new(CursorImpl::new(cursor)));
                }
            }
        }
    }

    impl ICursorFactory for CursorFactory {
        fn get_cursor(&self, cursor_type: StandardCursorType) -> Rc<dyn ICursorImpl> {
            if let Some(cursor) = self.cache.borrow().get(&cursor_type) {
                return cursor.clone();
            }

            let cursor = Rc::new(CursorImpl::new(load_cursor(0, cursor_resource_id(cursor_type))));
            self.cache.borrow_mut().insert(cursor_type, cursor.clone());
            cursor
        }

        fn create_cursor(&self, cursor: &Bitmap, hot_spot: PixelPoint) -> Rc<dyn ICursorImpl> {
            Rc::new(CursorImpl::from_icon(Win32Icon::from_bitmap(cursor, hot_spot)))
        }
    }

    /// A cursor of the system.
    pub struct CursorImpl {
        handle: Cell<isize>,
        icon: RefCell<Option<Win32Icon>>,
    }

    impl CursorImpl {
        /// Wraps a cursor handle the system owns.
        pub fn new(handle: isize) -> Self {
            Self { handle: Cell::new(handle), icon: RefCell::new(None) }
        }

        /// A cursor made from a bitmap, which owns its icon.
        pub(crate) fn from_icon(icon: Win32Icon) -> Self {
            Self { handle: Cell::new(icon.handle()), icon: RefCell::new(Some(icon)) }
        }

        /// The cursor handle.
        pub fn handle(&self) -> isize {
            self.handle.get()
        }

        /// The descriptor of the handle.
        pub fn handle_descriptor(&self) -> &'static str {
            PlatformConstants::CURSOR_HANDLE_TYPE
        }
    }

    impl ICursorImpl for CursorImpl {
        /// A cursor of the system is shared and owned by the system:
        /// nothing is released. A cursor made from a bitmap owns its icon,
        /// which is destroyed.
        fn dispose(&self) {
            if let Some(icon) = self.icon.borrow_mut().take() {
                icon.dispose();
                self.handle.set(0);
            }
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_standard_cursors_are_the_cursors_of_the_system() {
        assert_eq!(cursor_resource_id(StandardCursorType::Arrow), 32512);
        assert_eq!(cursor_resource_id(StandardCursorType::Ibeam), 32513);
        assert_eq!(cursor_resource_id(StandardCursorType::Wait), 32514);
        assert_eq!(cursor_resource_id(StandardCursorType::Cross), 32515);
        assert_eq!(cursor_resource_id(StandardCursorType::UpArrow), 32516);
        assert_eq!(cursor_resource_id(StandardCursorType::SizeAll), 32646);
        assert_eq!(cursor_resource_id(StandardCursorType::No), 32648);
        assert_eq!(cursor_resource_id(StandardCursorType::Hand), 32649);
        assert_eq!(cursor_resource_id(StandardCursorType::AppStarting), 32650);
        assert_eq!(cursor_resource_id(StandardCursorType::Help), 32651);
        assert_eq!(cursor_resource_id(StandardCursorType::None), 0);
    }

    #[test]
    fn the_sides_and_corners_share_the_sizing_cursors() {
        let north_south = cursor_resource_id(StandardCursorType::SizeNorthSouth);
        assert_eq!(cursor_resource_id(StandardCursorType::TopSide), north_south);
        assert_eq!(cursor_resource_id(StandardCursorType::BottomSide), north_south);

        let west_east = cursor_resource_id(StandardCursorType::SizeWestEast);
        assert_eq!(cursor_resource_id(StandardCursorType::LeftSide), west_east);
        assert_eq!(cursor_resource_id(StandardCursorType::RightSide), west_east);

        assert_eq!(cursor_resource_id(StandardCursorType::TopLeftCorner), 32642);
        assert_eq!(cursor_resource_id(StandardCursorType::BottomRightCorner), 32642);
        assert_eq!(cursor_resource_id(StandardCursorType::TopRightCorner), 32643);
        assert_eq!(cursor_resource_id(StandardCursorType::BottomLeftCorner), 32643);
    }

    #[test]
    fn the_drag_cursors_fall_back_to_the_up_arrow() {
        for cursor_type in [StandardCursorType::DragMove, StandardCursorType::DragCopy, StandardCursorType::DragLink] {
            assert_eq!(cursor_resource_id(cursor_type), 32516);
        }
    }
}
