use crate::media::imaging::IBitmap;
use crate::media::{DrawingContext, IAffectsRender};
use crate::{FerroObject, Rect, Size};
use std::any::Any;

/// Represents a bitmap or drawing which can be drawn.
///
/// Implemented by the bitmap types and, through an adapter, by the image
/// classes (cropped bitmaps and drawing images): a handle of such a class
/// converts to `Rc<dyn IImage>` with `into()`. The `as_*` members replace
/// interface casts (`image as IBitmap`).
pub trait IImage: 'static {
    /// The size of the image, in device independent pixels.
    fn size(&self) -> Size;

    /// Draws the image to a drawing context: the part of the image inside
    /// `source_rect` is drawn into `dest_rect`.
    fn draw(&self, context: &mut DrawingContext<'_>, source_rect: Rect, dest_rect: Rect);

    /// The implementing value, for downcasts to concrete image types. For
    /// image classes use [`as_object`](Self::as_object).
    fn as_any(&self) -> &dyn Any;

    /// The object behind the image when it is an image class.
    fn as_object(&self) -> Option<&FerroObject> {
        None
    }

    /// The image viewed as a bitmap, when it is one.
    fn as_bitmap(&self) -> Option<&dyn IBitmap> {
        None
    }

    /// The image viewed as [`IAffectsRender`], when it can change.
    fn as_affects_render(&self) -> Option<&dyn IAffectsRender> {
        None
    }

    /// The identity of the image, used for reference equality.
    #[doc(hidden)]
    fn reference_id(&self) -> *const () {
        self as *const Self as *const ()
    }
}

/// Images compare by reference.
impl PartialEq for dyn IImage {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.reference_id() == other.reference_id()
    }
}

impl std::fmt::Debug for dyn IImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "IImage({})", self.size())
    }
}
