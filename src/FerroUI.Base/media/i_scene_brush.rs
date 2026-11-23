use crate::media::immutable::immutable_tile_brush::immutable_tile_brush_interfaces;
use crate::media::immutable::ImmutableTileBrush;
use crate::media::ref_adapter::RefAdapter;
use crate::media::{Brush, DrawingBrush, IImmutableBrush, ITileBrush, VisualBrush};
use crate::platform::IDrawingContextImpl;
use crate::reactive::Subscription;
use crate::{Matrix, ObjectType, Rect, Upcast};
use std::rc::Rc;

/// A tile brush whose content is a recorded scene.
pub trait ISceneBrush: ITileBrush {
    /// Records the current content of the brush.
    fn create_content(&self) -> Option<Rc<dyn ISceneBrushContent>>;

    /// Calls `handler` when what the brush draws changes without a change of a property of the
    /// brush itself: the drawing of a `DrawingBrush` invalidated (a change of a 3D material's
    /// content, `docs/porting/media3d.md`, section 3.18), until the returned subscription is
    /// dropped or disposed. `None` when the brush raises no such change (by default, and for a
    /// `VisualBrush`, whose visual is followed by the 2D compositor).
    fn subscribe_content_changed(&self, handler: Rc<dyn Fn()>) -> Option<Subscription> {
        let _ = handler;
        None
    }

    /// The visual a `VisualBrush` paints with. `None` for any other scene brush.
    fn scene_visual(&self) -> Option<crate::Ref<crate::Visual>> {
        None
    }
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

    fn subscribe_content_changed(&self, handler: Rc<dyn Fn()>) -> Option<Subscription> {
        let drawing = self.object().downcast_ref::<DrawingBrush>()?.drawing()?;
        Some(drawing.invalidated(move || handler()))
    }

    fn scene_visual(&self) -> Option<crate::Ref<crate::Visual>> {
        self.object().downcast_ref::<VisualBrush>()?.visual()
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
