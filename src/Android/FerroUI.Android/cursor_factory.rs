use ferroui_base::input::StandardCursorType;
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::platform::{ICursorFactory, ICursorImpl};
use ferroui_base::PixelPoint;
use std::any::Any;
use std::rc::Rc;

/// The cursor factory of a platform that shows no cursor of the
/// application: every cursor is the same empty cursor.
#[derive(Default)]
pub struct CursorFactory;

struct CursorImpl;

thread_local! {
    static ZERO_CURSOR: Rc<CursorImpl> = Rc::new(CursorImpl);
}

impl CursorImpl {
    fn zero_cursor() -> Rc<dyn ICursorImpl> {
        ZERO_CURSOR.with(|cursor| cursor.clone())
    }
}

impl ICursorImpl for CursorImpl {
    fn dispose(&self) {}

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl ICursorFactory for CursorFactory {
    fn get_cursor(&self, _cursor_type: StandardCursorType) -> Rc<dyn ICursorImpl> {
        CursorImpl::zero_cursor()
    }

    fn create_cursor(&self, _cursor: &Bitmap, _hot_spot: PixelPoint) -> Rc<dyn ICursorImpl> {
        CursorImpl::zero_cursor()
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of the factory.
    use super::*;

    #[test]
    fn every_standard_cursor_is_the_same_cursor() {
        let factory = CursorFactory;
        let arrow = factory.get_cursor(StandardCursorType::Arrow);
        let hand = factory.get_cursor(StandardCursorType::Hand);

        assert!(std::ptr::addr_eq(Rc::as_ptr(&arrow), Rc::as_ptr(&hand)));
        arrow.dispose();
    }
}
