//! The WinUI members of the gradient brushes (phase 2, not from upstream):
//! the brush transform, `ExtendMode`, `InterpolationSpace`, `MappingMode`,
//! `ColorStops`, the `Vector2` points of the linear brush and the ellipse of
//! the radial brush, mapped onto the members of the upstream brushes.

use super::composition_brush::{CompositionBrush, CompositionBrushKind};
use super::{CompositionGradientBrush, CompositionLinearGradientBrush, CompositionRadialGradientBrush};
use crate::collections::FerroList;
use crate::media::immutable::ImmutableTransform;
use crate::media::{GradientSpreadMethod, IGradientStop, ITransform};
use crate::rendering::composition::animations::CompositionColorSpace;
use crate::rendering::composition::{CompositionGradientExtendMode, CompositionMappingMode};
use crate::{Matrix, RelativePoint, RelativeScalar, RelativeUnit, Vector};
use std::rc::Rc;

/// The unit the WinUI members of a gradient brush set its points in.
fn unit_of(mode: CompositionMappingMode) -> RelativeUnit {
    match mode {
        CompositionMappingMode::Absolute => RelativeUnit::Absolute,
        CompositionMappingMode::Relative => RelativeUnit::Relative,
    }
}

fn point_of(value: Vector, unit: RelativeUnit) -> RelativePoint {
    RelativePoint::new(value.x, value.y, unit)
}

fn vector_of(point: RelativePoint) -> Vector {
    Vector::new(point.point.x, point.point.y)
}

/// Runs `$body` with `$props` bound to the generated properties of the
/// concrete gradient class of `$brush` (a `&CompositionBrush`).
macro_rules! with_gradient_props {
    ($brush:expr, |$props:ident| $body:expr) => {
        match &$brush.kind {
            CompositionBrushKind::LinearGradient(_, $props) => $body,
            CompositionBrushKind::RadialGradient(_, $props) => $body,
            CompositionBrushKind::ConicGradient(_, $props) => $body,
            _ => unreachable!("a gradient brush handle wraps a gradient brush"),
        }
    };
}

/// The value members of the brush transform, animatable under their WinUI
/// names.
macro_rules! gradient_transform_members {
    ($($(#[$meta:meta])* $get:ident / $set:ident : $ty:ty;)*) => {
        impl CompositionGradientBrush {$(
            $(#[$meta])*
            pub fn $get(&self) -> $ty {
                let brush: &CompositionBrush = self;
                with_gradient_props!(brush, |props| props.$get())
            }

            pub fn $set(&self, value: $ty) {
                let brush: &CompositionBrush = self;
                with_gradient_props!(brush, |props| props.$set(brush, value))
            }
        )*}
    };
}

gradient_transform_members! {
    /// The point of the gradient, relative to the size of the painted area,
    /// that is placed at `offset` (animatable as `AnchorPoint`).
    anchor_point / set_anchor_point: Vector;
    /// The point the gradient is scaled and rotated about, in the
    /// coordinates of the painted area (animatable as `CenterPoint`).
    center_point / set_center_point: Vector;
    /// The offset of the gradient (animatable as `Offset`).
    offset / set_offset: Vector;
    /// The rotation of the gradient about its centre point, in radians
    /// (animatable as `RotationAngle`).
    rotation_angle / set_rotation_angle: f32;
    /// The scale of the gradient about its centre point (default `(1, 1)`;
    /// animatable as `Scale`).
    scale / set_scale: Vector;
    /// The transform matrix of the gradient (WinUI `Matrix3x2`; animatable
    /// as `TransformMatrix`).
    transform_matrix / set_transform_matrix: Matrix;
}

impl CompositionGradientBrush {
    fn gradient_brush(&self) -> &CompositionBrush {
        self
    }

    fn mapping_cell(&self) -> &std::cell::Cell<CompositionMappingMode> {
        match &self.gradient_brush().kind {
            CompositionBrushKind::LinearGradient(data, _)
            | CompositionBrushKind::RadialGradient(data, _)
            | CompositionBrushKind::ConicGradient(data, _) => &data.mapping_mode,
            _ => unreachable!("a gradient brush handle wraps a gradient brush"),
        }
    }

    fn interpolation_cell(&self) -> &std::cell::Cell<CompositionColorSpace> {
        match &self.gradient_brush().kind {
            CompositionBrushKind::LinearGradient(data, _)
            | CompositionBrushKind::RadialGradient(data, _)
            | CompositionBrushKind::ConicGradient(data, _) => &data.interpolation_space,
            _ => unreachable!("a gradient brush handle wraps a gradient brush"),
        }
    }

    /// The rotation in degrees: a view of
    /// [`rotation_angle`](Self::rotation_angle) (not an animation target).
    pub fn rotation_angle_in_degrees(&self) -> f32 {
        self.rotation_angle().to_degrees()
    }

    pub fn set_rotation_angle_in_degrees(&self, value: f32) {
        self.set_rotation_angle(value.to_radians())
    }

    /// How the gradient continues outside its stops: a view of
    /// [`spread_method`](Self::spread_method) (`Clamp` is `Pad`, `Wrap` is
    /// `Repeat`, `Mirror` is `Reflect`).
    pub fn extend_mode(&self) -> CompositionGradientExtendMode {
        match self.spread_method() {
            GradientSpreadMethod::Pad => CompositionGradientExtendMode::Clamp,
            GradientSpreadMethod::Repeat => CompositionGradientExtendMode::Wrap,
            GradientSpreadMethod::Reflect => CompositionGradientExtendMode::Mirror,
        }
    }

    pub fn set_extend_mode(&self, value: CompositionGradientExtendMode) {
        self.set_spread_method(match value {
            CompositionGradientExtendMode::Clamp => GradientSpreadMethod::Pad,
            CompositionGradientExtendMode::Wrap => GradientSpreadMethod::Repeat,
            CompositionGradientExtendMode::Mirror => GradientSpreadMethod::Reflect,
        })
    }

    /// The colour space the stops are interpolated in.
    ///
    /// Deviation (winui-composition.md): stored only; the drawing contexts
    /// interpolate in RGB.
    pub fn interpolation_space(&self) -> CompositionColorSpace {
        self.interpolation_cell().get()
    }

    pub fn set_interpolation_space(&self, value: CompositionColorSpace) {
        self.interpolation_cell().set(value)
    }

    /// How the points of the WinUI members are measured (default
    /// `Relative`). Changing it keeps the numbers of the points and changes
    /// their unit, as WinUI reads the same numbers in the new mode.
    pub fn mapping_mode(&self) -> CompositionMappingMode {
        self.mapping_cell().get()
    }

    pub fn set_mapping_mode(&self, value: CompositionMappingMode) {
        if self.mapping_cell().replace(value) == value {
            return;
        }
        let unit = unit_of(value);
        let brush = self.gradient_brush();
        match &brush.kind {
            CompositionBrushKind::LinearGradient(_, props) => {
                props.set_start_point(brush, point_of(vector_of(props.start_point()), unit));
                props.set_end_point(brush, point_of(vector_of(props.end_point()), unit));
            }
            CompositionBrushKind::RadialGradient(_, props) => {
                props.set_center(brush, point_of(vector_of(props.center()), unit));
                props.set_gradient_origin(brush, point_of(vector_of(props.gradient_origin()), unit));
                props.set_radius_x(brush, RelativeScalar::new(props.radius_x().scalar, unit));
                props.set_radius_y(brush, RelativeScalar::new(props.radius_y().scalar, unit));
            }
            CompositionBrushKind::ConicGradient(_, props) => {
                props.set_center(brush, point_of(vector_of(props.center()), unit));
            }
            _ => {}
        }
    }

    /// `ColorStops`: the WinUI name of
    /// [`gradient_stops`](Self::gradient_stops), with the same rule: a change
    /// of the list itself is shipped when the list is assigned again
    /// ([`set_color_stops`](Self::set_color_stops)).
    pub fn color_stops(&self) -> FerroList<Rc<dyn IGradientStop>> {
        self.gradient_stops()
    }

    pub fn set_color_stops(&self, value: FerroList<Rc<dyn IGradientStop>>) {
        self.set_gradient_stops(value)
    }
}

impl CompositionLinearGradientBrush {
    /// `StartPoint` as a WinUI `Vector2`, in the unit of the mapping mode
    /// (default `(0, 0)`). The upstream member is
    /// [`start_point`](Self::start_point).
    ///
    /// Deviation (winui-composition.md): animations target the upstream
    /// `StartPoint` (a relative point), not a `Vector2`.
    pub fn start_point_vector(&self) -> Vector {
        vector_of(self.start_point())
    }

    pub fn set_start_point_vector(&self, value: Vector) {
        self.set_start_point(point_of(value, unit_of(self.mapping_mode())))
    }

    /// `EndPoint` as a WinUI `Vector2`, in the unit of the mapping mode.
    ///
    /// Deviation (winui-composition.md): the default is the one of the
    /// upstream brush, `(0, 0)`, not the `(1, 0)` of WinUI.
    pub fn end_point_vector(&self) -> Vector {
        vector_of(self.end_point())
    }

    pub fn set_end_point_vector(&self, value: Vector) {
        self.set_end_point(point_of(value, unit_of(self.mapping_mode())))
    }
}

impl CompositionRadialGradientBrush {
    /// `EllipseCenter`: the centre of the ellipse, the upstream
    /// [`center`](Self::center), in the unit of the mapping mode.
    pub fn ellipse_center(&self) -> Vector {
        vector_of(self.center())
    }

    /// Sets the centre; the gradient origin moves with it, keeping
    /// [`gradient_origin_offset`](Self::gradient_origin_offset).
    pub fn set_ellipse_center(&self, value: Vector) {
        let offset = self.gradient_origin_offset();
        let unit = unit_of(self.mapping_mode());
        self.set_center(point_of(value, unit));
        self.set_gradient_origin(point_of(value + offset, unit));
    }

    /// `EllipseRadius`: the radii of the ellipse, the upstream
    /// [`radius_x`](Self::radius_x) and [`radius_y`](Self::radius_y), in the
    /// unit of the mapping mode.
    pub fn ellipse_radius(&self) -> Vector {
        Vector::new(self.radius_x().scalar, self.radius_y().scalar)
    }

    pub fn set_ellipse_radius(&self, value: Vector) {
        let unit = unit_of(self.mapping_mode());
        self.set_radius_x(RelativeScalar::new(value.x, unit));
        self.set_radius_y(RelativeScalar::new(value.y, unit));
    }

    /// `GradientOriginOffset`: where the gradient starts relative to the
    /// centre of the ellipse; the upstream
    /// [`gradient_origin`](Self::gradient_origin) is the centre plus this
    /// offset.
    pub fn gradient_origin_offset(&self) -> Vector {
        vector_of(self.gradient_origin()) - vector_of(self.center())
    }

    pub fn set_gradient_origin_offset(&self, value: Vector) {
        let unit = unit_of(self.mapping_mode());
        self.set_gradient_origin(point_of(self.ellipse_center() + value, unit));
    }
}

// --- the server side ---------------------------------------------------------------

/// The WinUI brush transform of a server-side composition brush that is a
/// brush of the drawing context: the drawing context applies it through the
/// `Transform` and `RelativeTransform` the brush reports.
pub trait IServerBrushWinUiTransform {
    /// The brush transform without its anchor point (`CenterPoint`,
    /// `Offset`, `RotationAngle`, `Scale`, `TransformMatrix`), in the
    /// coordinates of the painted area; `None` for the identity.
    fn winui_transform(&self) -> Option<Matrix> {
        None
    }

    /// The anchor point, relative to the size of the painted area.
    fn winui_anchor_point(&self) -> Vector {
        Vector::default()
    }
}

/// The `Transform` of a brush with its WinUI brush transform: the upstream
/// transform, then the WinUI one.
///
/// Deviation (winui-composition.md): both are applied about the upstream
/// `TransformOrigin` (by default the top left corner of the painted area,
/// which is where WinUI applies its brush transform).
pub(crate) fn with_winui_transform(
    upstream: Option<Rc<dyn ITransform>>,
    winui: Option<Matrix>,
) -> Option<Rc<dyn ITransform>> {
    match (upstream, winui) {
        (upstream, None) => upstream,
        (None, Some(winui)) => Some(Rc::new(ImmutableTransform::new(winui))),
        (Some(upstream), Some(winui)) => Some(Rc::new(ImmutableTransform::new(upstream.value() * winui))),
    }
}

/// The `RelativeTransform` of a brush with its WinUI anchor point: the
/// anchor point is a translation by its negation in the unit square of the
/// painted area, applied before the absolute transform, as WinUI applies it
/// first.
pub(crate) fn with_winui_anchor(upstream: Option<Rc<dyn ITransform>>, anchor: Vector) -> Option<Rc<dyn ITransform>> {
    if anchor == Vector::default() {
        return upstream;
    }
    let translation = Matrix::create_translation(-anchor.x, -anchor.y);
    match upstream {
        None => Some(Rc::new(ImmutableTransform::new(translation))),
        Some(upstream) => Some(Rc::new(ImmutableTransform::new(upstream.value() * translation))),
    }
}
