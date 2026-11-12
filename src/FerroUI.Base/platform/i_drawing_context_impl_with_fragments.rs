use super::IDrawingContextImpl;
use std::any::Any;
use std::rc::Rc;

/// What a drawing context kept of a part of a frame: the form the backend
/// draws it in, made once and drawn again in later frames without the
/// commands that drew it. Only the backend that made a fragment knows what
/// it holds.
pub trait IDrawingFragment: Any {
    /// Lets the backend recover its concrete type.
    fn as_any(&self) -> &dyn Any;
}

/// When a drawing context wants a fragment made of a drawing of the
/// compositor that it has no fragment of, or whose fragment it refused to
/// draw ([`IDrawingContextImplWithFragments::fragment_keeping`]).
///
/// The compositor keeps the answer apart from the backend: when fragments are
/// made is one policy for every backend, and a backend only says which case
/// is its own.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum FragmentKeeping {
    /// Not in this frame: the context draws the fragments it made before and
    /// makes none. What it refuses now it may draw again in a later frame,
    /// so nothing the compositor keeps changes.
    None,
    /// With every such drawing: a fragment costs the backend little more
    /// than the drawing does.
    #[default]
    Each,
    /// Once the drawing was drawn before as it is now: making a fragment
    /// costs the backend more than the drawing does, so a drawing that
    /// changes with every frame gets none. A drawing whose fragment the
    /// context refuses is drawn from its commands for a number of frames
    /// that doubles with every refusal, before the next fragment is made.
    Settled,
}

/// A drawing context that can keep what is drawn between two calls as a
/// fragment and draw it again in a later frame.
///
/// The compositor keeps the fragment of the content of a visual and draws
/// it again while the content does not change. Drawing a fragment again
/// gives the pixels the commands would give, or the backend refuses to:
/// it decides from the state of the context (the transform, the opacity,
/// the options) whether it can.
pub trait IDrawingContextImplWithFragments: IDrawingContextImpl {
    /// Starts a fragment: what is drawn until the matching
    /// [`end_fragment`](Self::end_fragment) is drawn as usual and also
    /// kept. Fragments do not nest; pushes and pops in between are
    /// balanced.
    fn begin_fragment(&mut self);

    /// Ends the fragment that was started last. What was drawn stays drawn.
    /// Returns the fragment, or `None` when what was drawn cannot be drawn
    /// again from a fragment.
    fn end_fragment(&mut self) -> Option<Rc<dyn IDrawingFragment>>;

    /// Draws a fragment that this context's backend made in an earlier
    /// frame, when doing so at the current state gives the pixels the
    /// commands it was made of would give. Returns `false` without drawing
    /// anything otherwise.
    fn draw_fragment(&mut self, fragment: &Rc<dyn IDrawingFragment>) -> bool;

    /// When the compositor has fragments made by this context, as the
    /// context is now.
    fn fragment_keeping(&mut self) -> FragmentKeeping {
        FragmentKeeping::Each
    }
}
