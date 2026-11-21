//! The cursor factory of a platform without cursors (the port of
//! `Stubs.cs`).

use ferroui_base::input::StandardCursorType;
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::platform::{ICursorFactory, ICursorImpl};
use ferroui_base::PixelPoint;
use std::rc::Rc;

#[derive(Default)]
pub(crate) struct CursorFactoryStub;

struct CursorStub;

impl ICursorImpl for CursorStub {
    fn dispose(&self) {}

}

impl ICursorFactory for CursorFactoryStub {
    fn get_cursor(&self, _cursor_type: StandardCursorType) -> Rc<dyn ICursorImpl> {
        Rc::new(CursorStub)
    }

    fn create_cursor(&self, _cursor: &Bitmap, _hot_spot: PixelPoint) -> Rc<dyn ICursorImpl> {
        Rc::new(CursorStub)
    }
}
