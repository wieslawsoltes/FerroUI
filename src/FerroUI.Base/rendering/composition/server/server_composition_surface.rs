use super::IServerObject;
use crate::platform::{IBitmapImpl, IDrawingContextImpl};
use crate::{Rect, Size};
use std::cell::RefCell;
use std::rc::Rc;

/// The `Changed` delegate of a server-side surface: the callbacks of the
/// visuals that show it.
#[derive(Default)]
pub struct ServerCompositionSurfaceChanged {
    handlers: RefCell<Vec<(usize, Rc<dyn Fn()>)>>,
}

impl ServerCompositionSurfaceChanged {
    pub fn new() -> Self {
        Self::default()
    }

    /// `Changed += handler`; `key` identifies the subscriber.
    pub fn add(&self, key: usize, handler: Rc<dyn Fn()>) {
        self.handlers.borrow_mut().push((key, handler));
    }

    /// `Changed -= handler`.
    pub fn remove(&self, key: usize) {
        let mut handlers = self.handlers.borrow_mut();
        if let Some(index) = handlers.iter().rposition(|(k, _)| *k == key) {
            handlers.remove(index);
        }
    }

    /// `Changed?.Invoke()`.
    pub fn invoke(&self) {
        let handlers: Vec<_> = self.handlers.borrow().iter().map(|(_, h)| h.clone()).collect();
        for handler in handlers {
            handler();
        }
    }
}

/// Server-side counterpart of a composition surface: something that
/// provides a bitmap to the surface visuals that show it.
///
/// The WinUI port adds the surfaces whose content is not a bitmap (a
/// visual surface renders a visual subtree): what a surface visual or a
/// surface brush shows goes through [`natural_size`](Self::natural_size)
/// and [`draw`](Self::draw), whose defaults use the bitmap.
pub trait IServerCompositionSurface: IServerObject {
    /// The current content of the surface.
    fn bitmap(&self) -> Option<std::sync::Arc<crate::platform::SharedBitmapImpl>>;

    /// Raised when the content changed.
    fn changed(&self) -> &ServerCompositionSurfaceChanged;

    /// The size of the content in device independent pixels (the natural
    /// size of WinUI's surfaces), which a surface brush lays out; `None`
    /// while the surface has no content. The default is the size of the
    /// bitmap at its DPI.
    fn natural_size(&self) -> Option<Size> {
        let bitmap = self.bitmap()?;
        let size = bitmap.pixel_size().to_size_with_dpi_vector(bitmap.dpi());
        (size.width > 0.0 && size.height > 0.0).then_some(size)
    }

    /// Draws the whole content of the surface into `dest`, in the
    /// coordinates of `canvas`. The default draws the bitmap.
    fn draw(&self, canvas: &mut dyn IDrawingContextImpl, dest: Rect, opacity: f64) {
        let Some(bitmap) = self.bitmap() else { return };
        canvas.draw_bitmap(&*bitmap, opacity, Rect::from_size(bitmap.pixel_size().to_size(1.0)), dest);
    }
}
