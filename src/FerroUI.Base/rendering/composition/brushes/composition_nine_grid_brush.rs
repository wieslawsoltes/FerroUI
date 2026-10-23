//! The WinUI `CompositionNineGridBrush` and `CompositionMaskBrush` (phase 2,
//! not from upstream): composition brushes painted by the visuals that show
//! them, as the surface brush.

use super::composition_brush::{CompositionBrush, CompositionBrushKind};
use super::{ServerCompositionMaskBrush, ServerCompositionNineGridBrush};
use crate::rendering::composition::generated::{CompositionMaskBrushProps, CompositionNineGridBrushProps};
use crate::rendering::composition::{Compositor, ICompositionObject};
use std::ops::Deref;
use std::rc::Rc;

/// A brush that paints another brush stretched as a nine-grid (WinUI
/// `Microsoft.UI.Composition.CompositionNineGridBrush`): the source is cut
/// by its four insets into corners, edges and a centre; the corners keep
/// their size (the insets times the inset scales), the edges stretch along
/// one axis, the centre along both.
///
/// The source is a colour or gradient brush (the nine regions of the area
/// are filled with it, the frame only when the centre is hollow) or a
/// [`CompositionSurfaceBrush`](super::CompositionSurfaceBrush), whose
/// surface is cut at its natural size: the insets are in the units of the
/// natural size of the surface. The insets and inset scales are animatable.
///
/// Deviation (winui-composition.md): the stretch, alignment and transform
/// of a surface brush source are not applied; its surface is cut as it is.
#[derive(Clone)]
pub struct CompositionNineGridBrush(Rc<CompositionBrush>);

impl Deref for CompositionNineGridBrush {
    type Target = Rc<CompositionBrush>;

    fn deref(&self) -> &Rc<CompositionBrush> {
        &self.0
    }
}

macro_rules! nine_grid_scalar {
    ($($(#[$meta:meta])* $get:ident / $set:ident;)*) => {
        impl CompositionNineGridBrush {$(
            $(#[$meta])*
            pub fn $get(&self) -> f32 {
                self.props().$get()
            }

            pub fn $set(&self, value: f32) {
                self.props().$set(&*self.0, value)
            }
        )*}
    };
}

nine_grid_scalar! {
    /// The width of the left column of the source (animatable as
    /// `LeftInset`).
    left_inset / set_left_inset;
    /// The height of the top row of the source (animatable as `TopInset`).
    top_inset / set_top_inset;
    /// The width of the right column of the source (animatable as
    /// `RightInset`).
    right_inset / set_right_inset;
    /// The height of the bottom row of the source (animatable as
    /// `BottomInset`).
    bottom_inset / set_bottom_inset;
    /// The scale of the left inset in the painted area (default 1;
    /// animatable as `LeftInsetScale`).
    left_inset_scale / set_left_inset_scale;
    /// The scale of the top inset in the painted area (default 1).
    top_inset_scale / set_top_inset_scale;
    /// The scale of the right inset in the painted area (default 1).
    right_inset_scale / set_right_inset_scale;
    /// The scale of the bottom inset in the painted area (default 1).
    bottom_inset_scale / set_bottom_inset_scale;
}

impl CompositionNineGridBrush {
    pub(crate) fn new(compositor: &Rc<Compositor>) -> CompositionNineGridBrush {
        let brush = CompositionBrush::create(
            compositor,
            CompositionBrushKind::NineGrid(CompositionNineGridBrushProps::new()),
            |c| ServerCompositionNineGridBrush::new(c),
        );
        brush.initialize_defaults();
        CompositionNineGridBrush(brush)
    }

    /// The handle of `brush` as a nine-grid brush, if it is one.
    pub fn from_brush(brush: &Rc<CompositionBrush>) -> Option<CompositionNineGridBrush> {
        matches!(brush.kind, CompositionBrushKind::NineGrid(_)).then(|| CompositionNineGridBrush(brush.clone()))
    }

    /// The generated properties of the brush.
    pub fn props(&self) -> &CompositionNineGridBrushProps {
        match &self.0.kind {
            CompositionBrushKind::NineGrid(props) => props,
            _ => unreachable!("a nine-grid brush handle wraps a nine-grid brush"),
        }
    }

    /// The brush that is cut into nine regions.
    pub fn source(&self) -> Option<Rc<CompositionBrush>> {
        self.props().source().and_then(|brush| brush.into_any_rc().downcast::<CompositionBrush>().ok())
    }

    pub fn set_source(&self, value: Option<Rc<CompositionBrush>>) {
        self.props().set_source(&*self.0, value.map(|brush| brush as Rc<dyn ICompositionObject>))
    }

    /// Whether the centre region is left unpainted.
    pub fn is_center_hollow(&self) -> bool {
        self.props().is_center_hollow()
    }

    pub fn set_is_center_hollow(&self, value: bool) {
        self.props().set_is_center_hollow(&*self.0, value)
    }

    /// `SetInsets(inset)`: the four insets.
    pub fn set_insets(&self, inset: f32) {
        self.set_insets_with(inset, inset, inset, inset);
    }

    /// `SetInsets(left, top, right, bottom)`.
    pub fn set_insets_with(&self, left: f32, top: f32, right: f32, bottom: f32) {
        self.set_left_inset(left);
        self.set_top_inset(top);
        self.set_right_inset(right);
        self.set_bottom_inset(bottom);
    }

    /// `SetInsetScales(scale)`: the four inset scales.
    pub fn set_inset_scales(&self, scale: f32) {
        self.set_inset_scales_with(scale, scale, scale, scale);
    }

    /// `SetInsetScales(left, top, right, bottom)`.
    pub fn set_inset_scales_with(&self, left: f32, top: f32, right: f32, bottom: f32) {
        self.set_left_inset_scale(left);
        self.set_top_inset_scale(top);
        self.set_right_inset_scale(right);
        self.set_bottom_inset_scale(bottom);
    }
}

/// A brush that paints a source brush through the alpha of a mask brush
/// (WinUI `Microsoft.UI.Composition.CompositionMaskBrush`).
///
/// A colour or gradient mask is the opacity mask of the drawing context. A
/// surface (or other composition) brush as the mask is painted over the
/// source in a layer with the destination-in blending of bitmaps.
///
/// Deviation (winui-composition.md): a mask that is not a colour or gradient
/// brush masks through the bitmaps it draws only (a loaded image surface);
/// shapes drawn by a visual surface used as a mask do not mask.
#[derive(Clone)]
pub struct CompositionMaskBrush(Rc<CompositionBrush>);

impl Deref for CompositionMaskBrush {
    type Target = Rc<CompositionBrush>;

    fn deref(&self) -> &Rc<CompositionBrush> {
        &self.0
    }
}

impl CompositionMaskBrush {
    pub(crate) fn new(compositor: &Rc<Compositor>) -> CompositionMaskBrush {
        let brush = CompositionBrush::create(
            compositor,
            CompositionBrushKind::Mask(CompositionMaskBrushProps::new()),
            |c| ServerCompositionMaskBrush::new(c),
        );
        brush.initialize_defaults();
        CompositionMaskBrush(brush)
    }

    /// The handle of `brush` as a mask brush, if it is one.
    pub fn from_brush(brush: &Rc<CompositionBrush>) -> Option<CompositionMaskBrush> {
        matches!(brush.kind, CompositionBrushKind::Mask(_)).then(|| CompositionMaskBrush(brush.clone()))
    }

    /// The generated properties of the brush.
    pub fn props(&self) -> &CompositionMaskBrushProps {
        match &self.0.kind {
            CompositionBrushKind::Mask(props) => props,
            _ => unreachable!("a mask brush handle wraps a mask brush"),
        }
    }

    /// The brush that is painted.
    pub fn source(&self) -> Option<Rc<CompositionBrush>> {
        self.props().source().and_then(|brush| brush.into_any_rc().downcast::<CompositionBrush>().ok())
    }

    pub fn set_source(&self, value: Option<Rc<CompositionBrush>>) {
        self.props().set_source(&*self.0, value.map(|brush| brush as Rc<dyn ICompositionObject>))
    }

    /// The brush whose alpha masks the source.
    pub fn mask(&self) -> Option<Rc<CompositionBrush>> {
        self.props().mask().and_then(|brush| brush.into_any_rc().downcast::<CompositionBrush>().ok())
    }

    pub fn set_mask(&self, value: Option<Rc<CompositionBrush>>) {
        self.props().set_mask(&*self.0, value.map(|brush| brush as Rc<dyn ICompositionObject>))
    }
}
