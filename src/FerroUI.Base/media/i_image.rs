use crate::media::imaging::IBitmap;
use crate::media::{DrawingContext, IAffectsRender};
use crate::platform::PlatformColorValues;
use crate::reactive::Subscription;
use crate::styling::ThemeVariant;
use crate::{FerroObject, Rect, Size};
use std::any::Any;
use std::error::Error;
use std::rc::Rc;

/// The outcome of a load performed by an image source.
pub type ImageLoadResult = Result<(), Rc<dyn Error>>;

/// Receives an image load outcome on the UI thread.
pub type ImageLoadHandler = Rc<dyn Fn(&ImageLoadResult)>;

/// The context of the control displaying a logical image.
#[derive(Clone, Debug)]
pub struct ImageVariantContext {
    /// Device pixels per device-independent pixel.
    pub scaling: f64,
    /// The effective theme variant of the control.
    pub theme: Option<ThemeVariant>,
    /// The system palette and contrast preference.
    pub platform_colors: PlatformColorValues,
}

/// A concrete image chosen by a logical image source.
pub struct ImageScaleVariant {
    /// Identifies this choice within the logical source's variants.
    pub id: usize,
    /// The image to measure and draw, with its own load notification.
    pub image: Rc<dyn IImage>,
}

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

    /// Subscribes to the pending load, if any. The callback runs at most once,
    /// on the UI thread. A source that has already announced its result does
    /// not replay it, and an image supplied ready to draw has no load to report.
    fn subscribe_load(&self, _handler: ImageLoadHandler) -> Option<Subscription> {
        None
    }

    /// Selects a concrete image for a display scale. A source with variants
    /// returns its initial choice when `current` is absent and returns
    /// `None` when the selection is unchanged. Ordinary images return `None`.
    fn resolve_for_scaling(&self, _scaling: f64, _current: Option<usize>) -> Option<ImageScaleVariant> {
        None
    }

    /// Selects an image for the control's current display, theme and palette.
    /// An adaptive source returns its initial choice when `current` is absent
    /// and returns `None` when the selected variant is unchanged.
    fn resolve_for_context(
        &self,
        context: &ImageVariantContext,
        current: Option<usize>,
    ) -> Option<ImageScaleVariant> {
        self.resolve_for_scaling(context.scaling, current)
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
