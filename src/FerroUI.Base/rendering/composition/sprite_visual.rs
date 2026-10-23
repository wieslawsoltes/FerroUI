use super::generated::{SpriteVisualHooks, SpriteVisualProps};
use super::server::ServerSpriteVisual;
use super::visual::CompositionVisualKind;
use super::{CompositionBrush, CompositionVisual, Compositor, DropShadow, ICompositionObject};
use std::ops::Deref;
use std::rc::Rc;

impl SpriteVisualHooks for CompositionVisual {}

/// A visual that paints its size with a composition brush and casts a
/// shadow (WinUI `Microsoft.UI.Composition.SpriteVisual`): the shadow of its
/// rectangle, then the brush over `(0, 0, size)`, then its children.
///
/// The brush is any [`CompositionBrush`]: the colour and gradient brushes
/// fill the rectangle, a
/// [`CompositionSurfaceBrush`](super::CompositionSurfaceBrush) lays out its
/// surface in it. A change of the brush or of the shadow, an animation
/// included, redraws the visual.
///
/// Unlike the other visuals of the compositor, a sprite visual does not
/// clip to its bounds when it is created (`clip_to_bounds` is `false`), as
/// a WinUI visual: its shadow and its children may extend beyond its size.
#[derive(Clone)]
pub struct SpriteVisual(Rc<CompositionVisual>);

impl Deref for SpriteVisual {
    type Target = Rc<CompositionVisual>;

    fn deref(&self) -> &Rc<CompositionVisual> {
        &self.0
    }
}

impl SpriteVisual {
    pub(crate) fn new(compositor: &Rc<Compositor>) -> SpriteVisual {
        let visual = CompositionVisual::create(
            compositor,
            CompositionVisualKind::Sprite(SpriteVisualProps::new()),
            || Box::new(ServerSpriteVisual::new()),
        );
        // A WinUI visual has no implicit clip.
        visual.set_clip_to_bounds(false);
        SpriteVisual(visual)
    }

    /// The handle of `visual` as a sprite visual, if it is one.
    pub fn from_visual(visual: &Rc<CompositionVisual>) -> Option<SpriteVisual> {
        matches!(visual.kind, CompositionVisualKind::Sprite(_)).then(|| SpriteVisual(visual.clone()))
    }

    /// The visual as a composition visual.
    pub fn visual(&self) -> &Rc<CompositionVisual> {
        &self.0
    }

    fn props(&self) -> &SpriteVisualProps {
        match &self.0.kind {
            CompositionVisualKind::Sprite(props) => props,
            _ => unreachable!("a sprite visual handle wraps a sprite visual"),
        }
    }

    /// The brush the visual paints its size with.
    pub fn brush(&self) -> Option<Rc<CompositionBrush>> {
        self.props().brush().and_then(|brush| brush.into_any_rc().downcast::<CompositionBrush>().ok())
    }

    pub fn set_brush(&self, value: Option<Rc<CompositionBrush>>) {
        self.props().set_brush(&*self.0, value.map(|brush| brush as Rc<dyn ICompositionObject>))
    }

    /// The shadow of the visual.
    pub fn shadow(&self) -> Option<Rc<DropShadow>> {
        self.props().shadow().and_then(|shadow| shadow.into_any_rc().downcast::<DropShadow>().ok())
    }

    pub fn set_shadow(&self, value: Option<Rc<DropShadow>>) {
        self.props().set_shadow(&*self.0, value.map(|shadow| shadow as Rc<dyn ICompositionObject>))
    }
}
