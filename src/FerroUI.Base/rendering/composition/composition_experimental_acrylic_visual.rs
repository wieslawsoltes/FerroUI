use super::visual::CompositionVisualKind;
use super::{CompositionDrawListVisual, CompositionVisual, Compositor};
use crate::media::ImmutableExperimentalAcrylicMaterial;
use crate::{CornerRadius, Visual};
use std::ops::Deref;
use std::rc::Rc;

/// A draw list visual that draws an acrylic rectangle below its content.
#[derive(Clone)]
pub struct CompositionExperimentalAcrylicVisual(CompositionDrawListVisual);

impl Deref for CompositionExperimentalAcrylicVisual {
    type Target = CompositionDrawListVisual;

    fn deref(&self) -> &CompositionDrawListVisual {
        &self.0
    }
}

impl CompositionExperimentalAcrylicVisual {
    pub fn new(compositor: &Rc<Compositor>, visual: &Visual) -> CompositionExperimentalAcrylicVisual {
        CompositionExperimentalAcrylicVisual(CompositionDrawListVisual::create(compositor, visual, true))
    }

    /// The handle of `visual` as an acrylic visual, if it is one.
    pub fn from_visual(visual: &Rc<CompositionVisual>) -> Option<CompositionExperimentalAcrylicVisual> {
        match &visual.kind {
            CompositionVisualKind::DrawList(data) if data.acrylic.is_some() => {
                CompositionDrawListVisual::from_visual(visual).map(CompositionExperimentalAcrylicVisual)
            }
            _ => None,
        }
    }

    /// The draw list visual this visual is.
    pub fn as_draw_list_visual(&self) -> &CompositionDrawListVisual {
        &self.0
    }

    fn props(&self) -> &super::generated::CompositionExperimentalAcrylicVisualProps {
        match &self.0.kind {
            CompositionVisualKind::DrawList(data) => match &data.acrylic {
                Some(props) => props,
                None => unreachable!("an acrylic visual handle wraps an acrylic visual"),
            },
            _ => unreachable!("an acrylic visual handle wraps an acrylic visual"),
        }
    }

    pub fn material(&self) -> ImmutableExperimentalAcrylicMaterial {
        self.props().material()
    }

    pub fn set_material(&self, value: ImmutableExperimentalAcrylicMaterial) {
        let visual: &CompositionVisual = &self.0;
        self.props().set_material(visual, value)
    }

    pub fn corner_radius(&self) -> CornerRadius {
        self.props().corner_radius()
    }

    pub fn set_corner_radius(&self, value: CornerRadius) {
        let visual: &CompositionVisual = &self.0;
        self.props().set_corner_radius(visual, value)
    }
}
