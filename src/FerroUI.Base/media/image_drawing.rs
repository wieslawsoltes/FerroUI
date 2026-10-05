use crate::media::{Drawing, DrawingContext, DrawingImpl, IImage};
use crate::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObjectImpl, FerroProperty, Rect, Ref,
    StyledProperty,
};
use std::rc::Rc;

/// Draws an image within a region defined by a [`Rect`].
#[repr(C)]
pub struct ImageDrawing {
    base: Drawing,
}

ferro_class!(ImageDrawing: Drawing);
crate::ferro_class_info!(ImageDrawing { new: ImageDrawing::new });
ferro_impl_classes!(ImageDrawing: FerroObjectImpl);

impl DrawingImpl for ImageDrawing {
    fn draw_core(this: &Self, context: &mut DrawingContext) {
        let image_source = this.image_source();
        let rect = this.rect();

        if let Some(image_source) = image_source {
            if rect.width != 0.0 || rect.height != 0.0 {
                context.draw_image(&*image_source, rect);
            }
        }
    }

    fn get_bounds(this: &Self) -> Rect {
        this.rect()
    }
}

crate::ferro_properties! { impl ImageDrawing {
    ferro_property!(
        /// Defines the `ImageSource` property.
        pub fn image_source_property() -> StyledProperty<Option<Rc<dyn IImage>>> {
            FerroProperty::register::<ImageDrawing, _>("ImageSource", None)
        }
    );

    ferro_property!(
        /// Defines the `Rect` property.
        pub fn rect_property() -> StyledProperty<Rect> {
            FerroProperty::register::<ImageDrawing, _>("Rect", Rect::default())
        }
    );
} }

impl ImageDrawing {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: Drawing::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The source of the image.
    pub fn image_source(&self) -> Option<Rc<dyn IImage>> {
        self.get_value(Self::image_source_property())
    }

    pub fn set_image_source(&self, value: Option<Rc<dyn IImage>>) {
        self.set_value(Self::image_source_property(), value)
    }

    /// The region in which the image is drawn.
    pub fn rect(&self) -> Rect {
        self.get_value(Self::rect_property())
    }

    pub fn set_rect(&self, value: Rect) {
        self.set_value(Self::rect_property(), value)
    }
}
