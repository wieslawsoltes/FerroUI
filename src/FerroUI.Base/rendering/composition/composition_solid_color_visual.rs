use super::generated::{CompositionSolidColorVisualHooks, CompositionSolidColorVisualProps};
use super::server::ServerCompositionSolidColorVisual;
use super::visual::CompositionVisualKind;
use super::{CompositionVisual, Compositor};
use crate::media::Color;
use std::ops::Deref;
use std::rc::Rc;

impl CompositionSolidColorVisualHooks for CompositionVisual {}

/// A visual that fills its size with a color.
#[derive(Clone)]
pub struct CompositionSolidColorVisual(Rc<CompositionVisual>);

impl Deref for CompositionSolidColorVisual {
    type Target = Rc<CompositionVisual>;

    fn deref(&self) -> &Rc<CompositionVisual> {
        &self.0
    }
}

impl CompositionSolidColorVisual {
    pub(crate) fn new(compositor: &Rc<Compositor>) -> CompositionSolidColorVisual {
        CompositionSolidColorVisual(CompositionVisual::create(
            compositor,
            CompositionVisualKind::SolidColor(CompositionSolidColorVisualProps::new()),
            || Box::new(ServerCompositionSolidColorVisual::new()),
        ))
    }

    /// The handle of `visual` as a solid color visual, if it is one.
    pub fn from_visual(visual: &Rc<CompositionVisual>) -> Option<CompositionSolidColorVisual> {
        matches!(visual.kind, CompositionVisualKind::SolidColor(_)).then(|| CompositionSolidColorVisual(visual.clone()))
    }

    fn props(&self) -> &CompositionSolidColorVisualProps {
        match &self.0.kind {
            CompositionVisualKind::SolidColor(props) => props,
            _ => unreachable!("a solid color visual handle wraps a solid color visual"),
        }
    }

    pub fn color(&self) -> Color {
        self.props().color()
    }

    pub fn set_color(&self, value: Color) {
        self.props().set_color(&*self.0, value)
    }
}
