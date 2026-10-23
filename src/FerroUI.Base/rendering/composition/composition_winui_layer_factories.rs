//! The factories of the WinUI layer and redirect visuals, the nine-grid and
//! mask brushes and the colour gradient stop (`Compositor.Create*` of
//! `Microsoft.UI.Composition`, phase 2).

use super::{
    CompositionGradientStop, CompositionMaskBrush, CompositionNineGridBrush, CompositionVisual, Compositor,
    LayerVisual, RedirectVisual,
};
use crate::media::Color;
use std::rc::Rc;

impl Compositor {
    /// `CreateLayerVisual`: a container visual whose children are composed
    /// into one layer.
    pub fn create_layer_visual(&self) -> LayerVisual {
        LayerVisual::new(&self.this_handle())
    }

    /// `CreateRedirectVisual`: a visual that shows the subtree of another
    /// visual, without a source.
    pub fn create_redirect_visual(&self) -> RedirectVisual {
        RedirectVisual::new(&self.this_handle())
    }

    /// `CreateRedirectVisual(source)`: a visual that shows the subtree of
    /// `source`.
    pub fn create_redirect_visual_with_source_visual(&self, source: Option<Rc<CompositionVisual>>) -> RedirectVisual {
        RedirectVisual::with_source(&self.this_handle(), source)
    }

    /// `CreateNineGridBrush`: a brush that paints another brush as a
    /// nine-grid.
    pub fn create_nine_grid_brush(&self) -> CompositionNineGridBrush {
        CompositionNineGridBrush::new(&self.this_handle())
    }

    /// `CreateMaskBrush`: a brush that paints a brush through the alpha of
    /// another.
    pub fn create_mask_brush(&self) -> CompositionMaskBrush {
        CompositionMaskBrush::new(&self.this_handle())
    }

    /// `CreateColorGradientStop(offset, color)`: the WinUI name of
    /// [`create_gradient_stop_with`](Self::create_gradient_stop_with).
    pub fn create_color_gradient_stop(&self, offset: f32, color: Color) -> Rc<CompositionGradientStop> {
        self.create_gradient_stop_with(offset as f64, color)
    }

    /// `CreateColorGradientStop()`: the WinUI name of
    /// [`create_gradient_stop`](Self::create_gradient_stop).
    pub fn create_color_gradient_stop_default(&self) -> Rc<CompositionGradientStop> {
        self.create_gradient_stop()
    }
}
