use super::IDrawingContextImpl;
use crate::media::effects::IEffect;
use crate::Rect;

/// A drawing context implementation that can apply bitmap effects.
pub trait IDrawingContextImplWithEffects: IDrawingContextImpl {
    /// Pushes an effect: everything drawn until the matching
    /// [`pop_effect`](Self::pop_effect) is drawn through it. `clip_rect`
    /// bounds the output of the effect, when known.
    fn push_effect(&mut self, clip_rect: Option<Rect>, effect: &dyn IEffect);

    /// Pops the latest pushed effect.
    fn pop_effect(&mut self);
}
