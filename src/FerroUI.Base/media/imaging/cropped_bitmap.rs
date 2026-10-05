use crate::media::ref_adapter::RefAdapter;
use crate::media::{DrawingContext, IAffectsRender, IImage};
use crate::reactive::{Disposable, IDisposable};
use crate::utilities::HandlerList;
use crate::{
    ferro_class, ferro_property, instantiate, FerroObject, FerroObjectImpl, FerroProperty,
    FerroPropertyChangedEventArgs, PixelRect, Rect, Ref, Size, StyledProperty, Vector,
};
use std::any::Any;
use std::rc::Rc;

/// Crops a bitmap.
#[repr(C)]
pub struct CroppedBitmap {
    base: FerroObject,
    invalidated: HandlerList<dyn Fn()>,
}

ferro_class! {
    CroppedBitmap: FerroObject, virtuals CroppedBitmapImpl: FerroObjectImpl {
        /// Releases the source bitmap.
        fn dispose(this);
    }
}
crate::ferro_class_info!(CroppedBitmap { new: CroppedBitmap::new, interfaces: [std::rc::Rc<dyn crate::media::IImage>] });

impl FerroObjectImpl for CroppedBitmap {}

impl CroppedBitmapImpl for CroppedBitmap {
    fn dispose(this: &Self) {
        if let Some(source) = this.source() {
            if let Some(bitmap) = source.as_bitmap() {
                bitmap.dispose();
            }
        }
    }
}

crate::ferro_properties! { impl CroppedBitmap {
    ferro_property!(
        /// Defines the `Source` property.
        pub fn source_property() -> StyledProperty<Option<Rc<dyn IImage>>> {
            FerroProperty::register::<CroppedBitmap, _>("Source", None)
        }
    );

    ferro_property!(
        /// Defines the `SourceRect` property.
        pub fn source_rect_property() -> StyledProperty<PixelRect> {
            FerroProperty::register::<CroppedBitmap, _>("SourceRect", PixelRect::default())
        }
    );
} }

impl CroppedBitmap {
    fn static_constructor() {
        Self::source_rect_property().changed().add_class_handler::<CroppedBitmap>(|x, e| x.source_rect_changed(e));
        Self::source_property().changed().add_class_handler::<CroppedBitmap>(|x, e| x.source_changed(e));
    }

    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: FerroObject::construct(), invalidated: HandlerList::new() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates a cropped bitmap showing `source_rect` of `source`.
    pub fn with_source(source: Rc<dyn IImage>, source_rect: PixelRect) -> Ref<Self> {
        let result = Self::new();
        result.set_source(Some(source));
        result.set_source_rect(source_rect);
        result
    }

    /// Raised when the image changes.
    pub fn invalidated(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.invalidated.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.invalidated.remove(token);
            }
        })
    }

    fn raise_invalidated(&self) {
        if self.invalidated.is_empty() {
            return;
        }
        for (_, handler) in self.invalidated.snapshot().iter() {
            handler();
        }
    }

    /// The source for the bitmap.
    pub fn source(&self) -> Option<Rc<dyn IImage>> {
        self.get_value(Self::source_property())
    }

    pub fn set_source(&self, value: Option<Rc<dyn IImage>>) {
        self.set_value(Self::source_property(), value)
    }

    /// The rectangular area that the bitmap is cropped to.
    pub fn source_rect(&self) -> PixelRect {
        self.get_value(Self::source_rect_property())
    }

    pub fn set_source_rect(&self, value: PixelRect) {
        self.set_value(Self::source_rect_property(), value)
    }

    fn source_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        let Some(new_value) = e.get_new_value::<Option<Rc<dyn IImage>>>() else { return };
        if new_value.as_bitmap().is_none() {
            panic!("Only IBitmap supported as source");
        }
        self.raise_invalidated();
    }

    fn source_rect_changed(&self, _e: &FerroPropertyChangedEventArgs<'_>) {
        self.raise_invalidated();
    }

    /// The size of the image, in device independent pixels.
    pub fn size(&self) -> Size {
        let Some(source) = self.source() else { return Size::default() };
        let Some(bmp) = source.as_bitmap() else { return Size::default() };
        let source_rect = self.source_rect();
        if source_rect.width == 0 && source_rect.height == 0 {
            return source.size();
        }
        source_rect.size().to_size_with_dpi_vector(bmp.dpi())
    }

    /// Draws the cropped part of the source bitmap: `source_rect` is
    /// relative to the top left corner of the crop rectangle.
    pub fn draw(&self, context: &mut DrawingContext<'_>, source_rect: Rect, dest_rect: Rect) {
        let Some(source) = self.source() else { return };
        let Some(bmp) = source.as_bitmap() else { return };
        let top_left = self.source_rect().top_left().to_point_with_dpi_vector(bmp.dpi());
        source.draw(context, source_rect.translate(Vector::new(top_left.x, top_left.y)), dest_rect);
    }
}

impl IImage for RefAdapter<CroppedBitmap> {
    fn size(&self) -> Size {
        self.0.size()
    }

    fn draw(&self, context: &mut DrawingContext<'_>, source_rect: Rect, dest_rect: Rect) {
        self.0.draw(context, source_rect, dest_rect)
    }

    fn as_any(&self) -> &dyn Any {
        &*self.0
    }

    fn as_object(&self) -> Option<&FerroObject> {
        Some(self.object())
    }

    fn as_affects_render(&self) -> Option<&dyn IAffectsRender> {
        Some(&*self.0)
    }

    fn reference_id(&self) -> *const () {
        RefAdapter::reference_id(self)
    }
}

impl From<Ref<CroppedBitmap>> for Rc<dyn IImage> {
    #[inline]
    fn from(value: Ref<CroppedBitmap>) -> Self {
        Rc::new(RefAdapter(value))
    }
}

impl From<&Ref<CroppedBitmap>> for Rc<dyn IImage> {
    #[inline]
    fn from(value: &Ref<CroppedBitmap>) -> Self {
        Rc::new(RefAdapter(value.clone()))
    }
}
