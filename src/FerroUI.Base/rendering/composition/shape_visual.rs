use super::generated::{ShapeVisualHooks, ShapeVisualProps};
use super::server::ServerShapeVisual;
use super::visual::CompositionVisualKind;
use super::{
    CompositionShapeCollection, CompositionViewBox, CompositionVisual, Compositor, ICompositionObject,
};
use std::ops::Deref;
use std::rc::Rc;

impl ShapeVisualHooks for CompositionVisual {}

/// A visual that draws shapes (WinUI `Microsoft.UI.Composition.ShapeVisual`):
/// its [`shapes`](Self::shapes) in order, through its
/// [`view_box`](Self::view_box) when it has one, then its children.
///
/// The shapes are clipped to the size of the visual (so a shape visual
/// without a size shows no shapes, as in WinUI); the visual itself does not
/// clip its children (`clip_to_bounds` is `false`, as for a sprite visual).
/// A change of a shape, of its geometry or brushes, of the collections or
/// of the view box, an animation included, redraws the visual.
#[derive(Clone)]
pub struct ShapeVisual(Rc<CompositionVisual>);

impl Deref for ShapeVisual {
    type Target = Rc<CompositionVisual>;

    fn deref(&self) -> &Rc<CompositionVisual> {
        &self.0
    }
}

impl ShapeVisual {
    pub(crate) fn new(compositor: &Rc<Compositor>) -> ShapeVisual {
        let shapes = CompositionShapeCollection::new(compositor);
        let visual = CompositionVisual::create(
            compositor,
            CompositionVisualKind::Shape(ShapeVisualProps::new()),
            || Box::new(ServerShapeVisual::new()),
        );
        // A WinUI visual has no implicit clip.
        visual.set_clip_to_bounds(false);
        let visual = ShapeVisual(visual);
        visual.props().set_shapes(&*visual.0, Some(shapes as Rc<dyn ICompositionObject>));
        visual
    }

    /// The handle of `visual` as a shape visual, if it is one.
    pub fn from_visual(visual: &Rc<CompositionVisual>) -> Option<ShapeVisual> {
        matches!(visual.kind, CompositionVisualKind::Shape(_)).then(|| ShapeVisual(visual.clone()))
    }

    /// The visual as a composition visual.
    pub fn visual(&self) -> &Rc<CompositionVisual> {
        &self.0
    }

    fn props(&self) -> &ShapeVisualProps {
        match &self.0.kind {
            CompositionVisualKind::Shape(props) => props,
            _ => unreachable!("a shape visual handle wraps a shape visual"),
        }
    }

    /// The shapes the visual draws, in order.
    pub fn shapes(&self) -> Rc<CompositionShapeCollection> {
        self.props()
            .shapes()
            .and_then(|shapes| shapes.into_any_rc().downcast::<CompositionShapeCollection>().ok())
            .expect("a shape visual has a shape collection")
    }

    /// The area of the shape coordinates the visual shows.
    pub fn view_box(&self) -> Option<Rc<CompositionViewBox>> {
        self.props().view_box().and_then(|view_box| view_box.into_any_rc().downcast::<CompositionViewBox>().ok())
    }

    pub fn set_view_box(&self, value: Option<Rc<CompositionViewBox>>) {
        self.props().set_view_box(&*self.0, value.map(|view_box| view_box as Rc<dyn ICompositionObject>))
    }
}
