use crate::media::immutable::immutable_tile_brush::immutable_tile_brush_interfaces;
use crate::media::immutable::ImmutableTileBrush;
use crate::media::ref_adapter::RefAdapter;
use crate::media::{Brush, DrawingBrush, IImmutableBrush, ITileBrush, VisualBrush};
use crate::platform::IDrawingContextImpl;
use crate::{Matrix, ObjectType, Rect, Upcast};
use std::rc::Rc;

/// A tile brush whose content is a recorded scene.
pub trait ISceneBrush: ITileBrush {
    /// Records the current content of the brush.
    fn create_content(&self) -> Option<Rc<dyn ISceneBrushContent>>;
}

/// The scene brush classes ([`VisualBrush`], [`DrawingBrush`]) behind a
/// class handle.
impl<T: ObjectType + Upcast<Brush>> ISceneBrush for RefAdapter<T> {
    fn create_content(&self) -> Option<Rc<dyn ISceneBrushContent>> {
        match self.object().downcast_ref::<VisualBrush>() {
            Some(visual_brush) => visual_brush.create_content(),
            None => self.class::<DrawingBrush>().create_content(),
        }
    }
}

/// The recorded content of an [`ISceneBrush`].
pub trait ISceneBrushContent: IImmutableBrush {
    /// The tile brush parameters the content is drawn with.
    fn brush(&self) -> Rc<dyn ITileBrush>;

    /// The bounds of the content.
    fn rect(&self) -> Rect;

    /// Replays the content into a drawing context.
    fn render(&self, context: &mut dyn IDrawingContextImpl, transform: Option<Matrix>);

    /// Whether the content should be rasterized at the resolution it is
    /// displayed at.
    fn use_scalable_rasterization(&self) -> bool;

    /// Releases the content.
    fn dispose(&self);
}

/// The immutable tile brush parameters of a scene brush.
#[derive(Clone)]
pub struct ImmutableSceneBrush {
    base: ImmutableTileBrush,
}

immutable_tile_brush_interfaces!(ImmutableSceneBrush);

impl ImmutableSceneBrush {
    /// Creates an immutable copy of the tile brush parameters of `source`.
    pub fn new(source: &dyn ITileBrush) -> Self {
        Self { base: ImmutableTileBrush::from_brush(source) }
    }
}
