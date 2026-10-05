use crate::media::immutable::ImmutableImageBrush;
use crate::media::ref_adapter::RefAdapter;
use crate::media::{BrushImpl, IImageBrushSource, IImmutableBrush, TileBrush};
use crate::{ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObjectImpl, FerroProperty, Ref, StyledProperty};
use std::rc::Rc;

/// Paints an area with an image.
#[repr(C)]
pub struct ImageBrush {
    base: TileBrush,
}

ferro_class!(ImageBrush: TileBrush);
crate::ferro_class_info!(ImageBrush { new: ImageBrush::new });
ferro_impl_classes!(ImageBrush: FerroObjectImpl, BrushImpl);

crate::ferro_properties! { impl ImageBrush {
    ferro_property!(pub fn source_property() -> StyledProperty<Option<Rc<dyn IImageBrushSource>>> {
        FerroProperty::register::<ImageBrush, _>("Source", None)
    });
} }

impl ImageBrush {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: TileBrush::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates a brush that draws `source`.
    pub fn with_source(source: Option<Rc<dyn IImageBrushSource>>) -> Ref<Self> {
        let result = Self::new();
        result.set_source(source);
        result
    }

    /// The image to draw.
    pub fn source(&self) -> Option<Rc<dyn IImageBrushSource>> {
        self.get_value(Self::source_property())
    }

    pub fn set_source(&self, value: Option<Rc<dyn IImageBrushSource>>) {
        self.set_value(Self::source_property(), value)
    }

    /// Creates an immutable clone of the brush.
    pub fn to_immutable(&self) -> Rc<dyn IImmutableBrush> {
        Rc::new(ImmutableImageBrush::from_brush(&RefAdapter(self.to_ref())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::{AlignmentX, BrushExtensions, IBrush, Stretch, TileMode};
    use crate::platform::IBitmapImpl;
    use crate::utilities::RefCounted;
    use crate::RelativeRect;
    use std::cell::Cell;

    struct MockSource;

    impl IImageBrushSource for MockSource {
        fn bitmap(&self) -> Option<&RefCounted<dyn IBitmapImpl>> {
            None
        }
    }

    #[test]
    fn changing_source_raises_invalidated() {
        let bitmap1: Rc<dyn IImageBrushSource> = Rc::new(MockSource);
        let bitmap2: Rc<dyn IImageBrushSource> = Rc::new(MockSource);
        let target = ImageBrush::with_source(Some(bitmap1));

        let raised = Rc::new(Cell::new(false));
        let r = raised.clone();
        target.invalidated(move || r.set(true));
        target.set_source(Some(bitmap2));
        assert!(raised.get());
    }

    // --- not from upstream ---

    #[test]
    fn setting_the_same_source_does_not_raise_invalidated() {
        let bitmap: Rc<dyn IImageBrushSource> = Rc::new(MockSource);
        let target = ImageBrush::with_source(Some(bitmap.clone()));
        let raised = Rc::new(Cell::new(false));
        let r = raised.clone();
        target.invalidated(move || r.set(true));
        target.set_source(Some(bitmap));
        assert!(!raised.get());
    }

    #[test]
    fn defaults_interfaces_and_immutable_copy() {
        let source: Rc<dyn IImageBrushSource> = Rc::new(MockSource);
        let target = ImageBrush::with_source(Some(source.clone()));
        assert_eq!(AlignmentX::Center, target.alignment_x());
        assert_eq!(RelativeRect::FILL, target.destination_rect());
        assert_eq!(RelativeRect::FILL, target.source_rect());
        assert_eq!(Stretch::Uniform, target.stretch());
        assert_eq!(TileMode::None, target.tile_mode());

        target.set_tile_mode(TileMode::FlipXY);
        target.set_opacity(0.5);

        let brush: Rc<dyn IBrush> = target.clone().into();
        assert!(brush.as_solid_color_brush().is_none());
        assert_eq!(TileMode::FlipXY, brush.as_tile_brush().unwrap().tile_mode());
        assert!(*brush.as_image_brush().unwrap().source().unwrap() == *source);
        assert!(brush.as_mutable_brush().is_some());
        assert!(brush.as_scene_brush().is_none());

        let immutable = BrushExtensions::to_immutable(&brush);
        target.set_tile_mode(TileMode::Tile);
        let image = immutable.as_image_brush().unwrap();
        assert_eq!(TileMode::FlipXY, image.tile_mode());
        assert_eq!(0.5, image.opacity());
        assert_eq!(Stretch::Uniform, image.stretch());
        assert!(*image.source().unwrap() == *source);
        assert!(immutable.as_mutable_brush().is_none());
    }
}
