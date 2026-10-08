use crate::media::ref_adapter::RefAdapter;
use crate::media::{Brush, ITileBrush, ImageBrush};
use crate::platform::IBitmapImpl;
use crate::utilities::RefCounted;
use crate::{ObjectType, Upcast};
use std::rc::Rc;

/// Paints an area with an image.
pub trait IImageBrush: ITileBrush {
    /// The image to draw.
    fn source(&self) -> Option<Rc<dyn IImageBrushSource>>;
}

/// An image that can be the source of an [`IImageBrush`].
pub trait IImageBrushSource: 'static {
    /// The counted reference to the platform bitmap, while it is alive.
    fn bitmap(&self) -> Option<&RefCounted<crate::platform::SharedBitmapImpl>>;

    /// The platform bitmap, while it is alive.
    fn get_bitmap(&self) -> Option<std::sync::Arc<crate::platform::SharedBitmapImpl>> {
        self.bitmap().map(|bitmap| bitmap.item())
    }

    /// The identity of the source, used for reference equality.
    #[doc(hidden)]
    fn reference_id(&self) -> *const () {
        self as *const Self as *const ()
    }
}

/// Image brush sources compare by reference.
impl PartialEq for dyn IImageBrushSource {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.reference_id() == other.reference_id()
    }
}

impl std::fmt::Debug for dyn IImageBrushSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("IImageBrushSource")
    }
}

impl<T: ObjectType + Upcast<Brush>> IImageBrush for RefAdapter<T> {
    fn source(&self) -> Option<Rc<dyn IImageBrushSource>> {
        self.class::<ImageBrush>().source()
    }
}
