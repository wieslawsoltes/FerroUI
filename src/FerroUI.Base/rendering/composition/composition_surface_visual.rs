use super::generated::{CompositionSurfaceVisualHooks, CompositionSurfaceVisualProps};
use super::server::ServerCompositionSurfaceVisual;
use super::visual::CompositionVisualKind;
use super::{CompositionSurface, CompositionVisual, Compositor, ICompositionObject};
use std::ops::Deref;
use std::rc::Rc;

impl CompositionSurfaceVisualHooks for CompositionVisual {}

/// A visual that shows a composition surface.
#[derive(Clone)]
pub struct CompositionSurfaceVisual(Rc<CompositionVisual>);

impl Deref for CompositionSurfaceVisual {
    type Target = Rc<CompositionVisual>;

    fn deref(&self) -> &Rc<CompositionVisual> {
        &self.0
    }
}

impl CompositionSurfaceVisual {
    pub(crate) fn new(compositor: &Rc<Compositor>) -> CompositionSurfaceVisual {
        CompositionSurfaceVisual(CompositionVisual::create(
            compositor,
            CompositionVisualKind::Surface(CompositionSurfaceVisualProps::new()),
            || Box::new(ServerCompositionSurfaceVisual::new()),
        ))
    }

    /// The handle of `visual` as a surface visual, if it is one.
    pub fn from_visual(visual: &Rc<CompositionVisual>) -> Option<CompositionSurfaceVisual> {
        matches!(visual.kind, CompositionVisualKind::Surface(_)).then(|| CompositionSurfaceVisual(visual.clone()))
    }

    fn props(&self) -> &CompositionSurfaceVisualProps {
        match &self.0.kind {
            CompositionVisualKind::Surface(props) => props,
            _ => unreachable!("a surface visual handle wraps a surface visual"),
        }
    }

    pub fn surface(&self) -> Option<Rc<CompositionSurface>> {
        self.props().surface().and_then(|o| o.into_any_rc().downcast::<CompositionSurface>().ok())
    }

    pub fn set_surface(&self, value: Option<Rc<CompositionSurface>>) {
        self.props().set_surface(&*self.0, value.map(|v| v as Rc<dyn ICompositionObject>))
    }
}
