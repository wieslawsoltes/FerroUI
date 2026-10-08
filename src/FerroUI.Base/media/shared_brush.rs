//! The form of an immutable brush and of an immutable pen that is sent to
//! the render thread.
//!
//! A brush object belongs to the UI thread: the handles its contract hands
//! out (its transforms, its gradient stops, the source of an image) are
//! `Rc`. What the render thread draws with is a copy of the values of an
//! immutable brush, [`SharedBrush`], which is `Send + Sync` and implements
//! the same contracts, building the handles its accessors return on demand.
//!
//! A shared brush is a counted handle to its values, and its identity
//! ([`IBrush::reference_id`]) is that of the values: every handle made from
//! it, on either thread, is the same brush to the caches keyed by a brush.
//! An immutable brush creates its shared form once and keeps it.

use crate::media::immutable::{ImmutableDashStyle, ImmutableGradientStop, ImmutablePen, ImmutableTransform};
use crate::media::{
    AlignmentX, AlignmentY, Color, GradientSpreadMethod, IBrush, IConicGradientBrush, IDashStyle, IGradientBrush,
    IGradientStop, IImageBrush, IImageBrushSource, IImmutableBrush, ILinearGradientBrush, IPen, IRadialGradientBrush,
    ISolidColorBrush, ITileBrush, ITransform, PenLineCap, PenLineJoin, Stretch, TileMode,
};
use crate::platform::SharedBitmapImpl;
use crate::utilities::RefCounted;
use crate::{Matrix, RelativePoint, RelativeRect, RelativeScalar};
use std::any::Any;
use std::rc::Rc;
use std::sync::Arc;

/// What a brush paints with.
enum Kind {
    Solid(Color),
    Linear { gradient: Gradient, start_point: RelativePoint, end_point: RelativePoint },
    Radial {
        gradient: Gradient,
        center: RelativePoint,
        gradient_origin: RelativePoint,
        radius_x: RelativeScalar,
        radius_y: RelativeScalar,
    },
    Conic { gradient: Gradient, center: RelativePoint, angle: f64 },
    Image { tile: Tile, bitmap: Option<RefCounted<SharedBitmapImpl>> },
}

struct Gradient {
    stops: Vec<(f64, Color)>,
    spread_method: GradientSpreadMethod,
}

struct Tile {
    alignment_x: AlignmentX,
    alignment_y: AlignmentY,
    destination_rect: RelativeRect,
    source_rect: RelativeRect,
    stretch: Stretch,
    tile_mode: TileMode,
}

struct BrushValues {
    opacity: f64,
    transform: Option<Matrix>,
    transform_origin: RelativePoint,
    relative_transform: Option<Matrix>,
    kind: Kind,
}

/// The values of an immutable brush, shared between the UI thread and the
/// render thread. See the module documentation.
#[derive(Clone)]
pub struct SharedBrush(Arc<BrushValues>);

impl SharedBrush {
    /// Copies the values of `brush`. `None` for a brush that is none of the
    /// kinds a render backend draws from values (a solid colour, a gradient,
    /// an image).
    pub fn from_brush(brush: &dyn IBrush) -> Option<SharedBrush> {
        fn gradient(brush: &dyn IGradientBrush) -> Gradient {
            Gradient {
                stops: brush.gradient_stops().iter().map(|stop| (stop.offset(), stop.color())).collect(),
                spread_method: brush.spread_method(),
            }
        }

        let kind = if let Some(solid) = brush.as_solid_color_brush() {
            Kind::Solid(solid.color())
        } else if let Some(linear) = brush.as_linear_gradient_brush() {
            Kind::Linear { gradient: gradient(linear), start_point: linear.start_point(), end_point: linear.end_point() }
        } else if let Some(radial) = brush.as_radial_gradient_brush() {
            Kind::Radial {
                gradient: gradient(radial),
                center: radial.center(),
                gradient_origin: radial.gradient_origin(),
                radius_x: radial.radius_x(),
                radius_y: radial.radius_y(),
            }
        } else if let Some(conic) = brush.as_conic_gradient_brush() {
            Kind::Conic { gradient: gradient(conic), center: conic.center(), angle: conic.angle() }
        } else if let Some(image) = brush.as_image_brush() {
            Kind::Image {
                tile: Tile {
                    alignment_x: image.alignment_x(),
                    alignment_y: image.alignment_y(),
                    destination_rect: image.destination_rect(),
                    source_rect: image.source_rect(),
                    stretch: image.stretch(),
                    tile_mode: image.tile_mode(),
                },
                bitmap: image
                    .source()
                    .and_then(|source| source.bitmap().filter(|bitmap| bitmap.is_alive()).map(RefCounted::clone_ref)),
            }
        } else {
            return None;
        };

        Some(SharedBrush(Arc::new(BrushValues {
            opacity: brush.opacity(),
            transform: brush.transform().map(|transform| transform.value()),
            transform_origin: brush.transform_origin(),
            relative_transform: brush.relative_transform().map(|transform| transform.value()),
            kind,
        })))
    }

    fn gradient(&self) -> Option<&Gradient> {
        match &self.0.kind {
            Kind::Linear { gradient, .. } | Kind::Radial { gradient, .. } | Kind::Conic { gradient, .. } => {
                Some(gradient)
            }
            _ => None,
        }
    }

    fn tile(&self) -> &Tile {
        match &self.0.kind {
            Kind::Image { tile, .. } => tile,
            _ => unreachable!("the tile view is given for an image only"),
        }
    }
}

fn transform_handle(value: Option<Matrix>) -> Option<Rc<dyn ITransform>> {
    value.map(|value| Rc::new(ImmutableTransform::new(value)) as Rc<dyn ITransform>)
}

impl IBrush for SharedBrush {
    fn opacity(&self) -> f64 {
        self.0.opacity
    }

    fn transform(&self) -> Option<Rc<dyn ITransform>> {
        transform_handle(self.0.transform)
    }

    fn transform_origin(&self) -> RelativePoint {
        self.0.transform_origin
    }

    fn relative_transform(&self) -> Option<Rc<dyn ITransform>> {
        transform_handle(self.0.relative_transform)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_solid_color_brush(&self) -> Option<&dyn ISolidColorBrush> {
        matches!(self.0.kind, Kind::Solid(_)).then_some(self as &dyn ISolidColorBrush)
    }

    fn as_gradient_brush(&self) -> Option<&dyn IGradientBrush> {
        self.gradient().map(|_| self as &dyn IGradientBrush)
    }

    fn as_linear_gradient_brush(&self) -> Option<&dyn ILinearGradientBrush> {
        matches!(self.0.kind, Kind::Linear { .. }).then_some(self as &dyn ILinearGradientBrush)
    }

    fn as_radial_gradient_brush(&self) -> Option<&dyn IRadialGradientBrush> {
        matches!(self.0.kind, Kind::Radial { .. }).then_some(self as &dyn IRadialGradientBrush)
    }

    fn as_conic_gradient_brush(&self) -> Option<&dyn IConicGradientBrush> {
        matches!(self.0.kind, Kind::Conic { .. }).then_some(self as &dyn IConicGradientBrush)
    }

    fn as_tile_brush(&self) -> Option<&dyn ITileBrush> {
        matches!(self.0.kind, Kind::Image { .. }).then_some(self as &dyn ITileBrush)
    }

    fn as_image_brush(&self) -> Option<&dyn IImageBrush> {
        matches!(self.0.kind, Kind::Image { .. }).then_some(self as &dyn IImageBrush)
    }

    fn as_immutable_brush(&self) -> Option<&dyn IImmutableBrush> {
        Some(self)
    }

    fn into_immutable_brush(self: Rc<Self>) -> Option<Rc<dyn IImmutableBrush>> {
        Some(self)
    }

    fn to_shared(&self) -> Option<SharedBrush> {
        Some(self.clone())
    }

    fn reference_id(&self) -> *const () {
        Arc::as_ptr(&self.0) as *const ()
    }

    /// The same values, or equal solid colours (immutable solid colour
    /// brushes compare structurally; their transforms by value here).
    fn equals(&self, other: &dyn IBrush) -> bool {
        if self.reference_id() == other.reference_id() {
            return true;
        }
        // An immutable solid colour brush of either form.
        let (Kind::Solid(color), Some(solid), true) =
            (&self.0.kind, other.as_solid_color_brush(), other.as_immutable_brush().is_some())
        else {
            return false;
        };
        solid.color() == *color
            && self.0.opacity == other.opacity()
            && self.0.transform == other.transform().map(|transform| transform.value())
            && self.0.relative_transform == other.relative_transform().map(|transform| transform.value())
    }
}

impl IImmutableBrush for SharedBrush {}

impl ISolidColorBrush for SharedBrush {
    fn color(&self) -> Color {
        match self.0.kind {
            Kind::Solid(color) => color,
            _ => unreachable!("the solid colour view is given for a solid colour only"),
        }
    }
}

impl IGradientBrush for SharedBrush {
    fn gradient_stops(&self) -> Vec<Rc<dyn IGradientStop>> {
        let gradient = self.gradient().expect("the gradient view is given for a gradient only");
        gradient
            .stops
            .iter()
            .map(|(offset, color)| Rc::new(ImmutableGradientStop::new(*offset, *color)) as Rc<dyn IGradientStop>)
            .collect()
    }

    fn spread_method(&self) -> GradientSpreadMethod {
        self.gradient().expect("the gradient view is given for a gradient only").spread_method
    }
}

impl ILinearGradientBrush for SharedBrush {
    fn start_point(&self) -> RelativePoint {
        match self.0.kind {
            Kind::Linear { start_point, .. } => start_point,
            _ => unreachable!("the linear gradient view is given for a linear gradient only"),
        }
    }

    fn end_point(&self) -> RelativePoint {
        match self.0.kind {
            Kind::Linear { end_point, .. } => end_point,
            _ => unreachable!("the linear gradient view is given for a linear gradient only"),
        }
    }
}

impl IRadialGradientBrush for SharedBrush {
    fn center(&self) -> RelativePoint {
        match self.0.kind {
            Kind::Radial { center, .. } => center,
            _ => unreachable!("the radial gradient view is given for a radial gradient only"),
        }
    }

    fn gradient_origin(&self) -> RelativePoint {
        match self.0.kind {
            Kind::Radial { gradient_origin, .. } => gradient_origin,
            _ => unreachable!("the radial gradient view is given for a radial gradient only"),
        }
    }

    fn radius_x(&self) -> RelativeScalar {
        match self.0.kind {
            Kind::Radial { radius_x, .. } => radius_x,
            _ => unreachable!("the radial gradient view is given for a radial gradient only"),
        }
    }

    fn radius_y(&self) -> RelativeScalar {
        match self.0.kind {
            Kind::Radial { radius_y, .. } => radius_y,
            _ => unreachable!("the radial gradient view is given for a radial gradient only"),
        }
    }
}

impl IConicGradientBrush for SharedBrush {
    fn center(&self) -> RelativePoint {
        match self.0.kind {
            Kind::Conic { center, .. } => center,
            _ => unreachable!("the conic gradient view is given for a conic gradient only"),
        }
    }

    fn angle(&self) -> f64 {
        match self.0.kind {
            Kind::Conic { angle, .. } => angle,
            _ => unreachable!("the conic gradient view is given for a conic gradient only"),
        }
    }
}

impl ITileBrush for SharedBrush {
    fn alignment_x(&self) -> AlignmentX {
        self.tile().alignment_x
    }

    fn alignment_y(&self) -> AlignmentY {
        self.tile().alignment_y
    }

    fn destination_rect(&self) -> RelativeRect {
        self.tile().destination_rect
    }

    fn source_rect(&self) -> RelativeRect {
        self.tile().source_rect
    }

    fn stretch(&self) -> Stretch {
        self.tile().stretch
    }

    fn tile_mode(&self) -> TileMode {
        self.tile().tile_mode
    }
}

/// The source of a shared image brush: a counted reference to the bitmap.
struct SharedImageSource(RefCounted<SharedBitmapImpl>);

impl IImageBrushSource for SharedImageSource {
    fn bitmap(&self) -> Option<&RefCounted<SharedBitmapImpl>> {
        Some(&self.0)
    }
}

impl IImageBrush for SharedBrush {
    fn source(&self) -> Option<Rc<dyn IImageBrushSource>> {
        match &self.0.kind {
            Kind::Image { bitmap: Some(bitmap), .. } if bitmap.is_alive() => {
                Some(Rc::new(SharedImageSource(bitmap.clone_ref())) as Rc<dyn IImageBrushSource>)
            }
            _ => None,
        }
    }
}

struct PenValues {
    brush: Option<SharedBrush>,
    thickness: f64,
    dashes: Option<(Vec<f64>, f64)>,
    line_cap: PenLineCap,
    line_join: PenLineJoin,
    miter_limit: f64,
}

/// The values of an immutable pen, shared between the UI thread and the
/// render thread. See the module documentation.
#[derive(Clone)]
pub struct SharedPen(Arc<PenValues>);

impl SharedPen {
    /// Copies the values of `pen`. `None` when its brush has no shared form.
    pub fn from_pen(pen: &dyn IPen) -> Option<SharedPen> {
        let brush = match pen.brush() {
            Some(brush) => Some(shared_brush_of(&*brush)?),
            None => None,
        };

        Some(SharedPen(Arc::new(PenValues {
            brush,
            thickness: pen.thickness(),
            dashes: pen.dash_style().map(|style| (style.dashes().unwrap_or_default(), style.offset())),
            line_cap: pen.line_cap(),
            line_join: pen.line_join(),
            miter_limit: pen.miter_limit(),
        })))
    }
}

impl IPen for SharedPen {
    fn brush(&self) -> Option<Rc<dyn IBrush>> {
        self.0.brush.clone().map(|brush| Rc::new(brush) as Rc<dyn IBrush>)
    }

    fn dash_style(&self) -> Option<Rc<dyn IDashStyle>> {
        self.0
            .dashes
            .as_ref()
            .map(|(dashes, offset)| Rc::new(ImmutableDashStyle::new(Some(dashes), *offset)) as Rc<dyn IDashStyle>)
    }

    fn line_cap(&self) -> PenLineCap {
        self.0.line_cap
    }

    fn line_join(&self) -> PenLineJoin {
        self.0.line_join
    }

    fn miter_limit(&self) -> f64 {
        self.0.miter_limit
    }

    fn thickness(&self) -> f64 {
        self.0.thickness
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_immutable_pen(self: Rc<Self>) -> Rc<ImmutablePen> {
        let brush = self.0.brush.clone().map(|brush| Rc::new(brush) as Rc<dyn IImmutableBrush>);
        let dash_style =
            self.0.dashes.as_ref().map(|(dashes, offset)| Rc::new(ImmutableDashStyle::new(Some(dashes), *offset)));
        Rc::new(ImmutablePen::new(
            brush,
            self.0.thickness,
            dash_style,
            self.0.line_cap,
            self.0.line_join,
            self.0.miter_limit,
        ))
    }

    fn to_shared(&self) -> Option<SharedPen> {
        Some(self.clone())
    }

    fn reference_id(&self) -> *const () {
        Arc::as_ptr(&self.0) as *const ()
    }

    /// The same values, or structurally equal ones (immutable pens compare
    /// structurally against any pen).
    fn equals(&self, other: &dyn IPen) -> bool {
        if self.reference_id() == other.reference_id() {
            return true;
        }

        let brushes_equal = match (self.brush(), other.brush()) {
            (None, None) => true,
            (Some(brush), Some(other)) => brush.equals(&*other),
            _ => false,
        };
        let dashes_equal = match (&self.0.dashes, other.dash_style()) {
            (None, None) => true,
            (Some((dashes, offset)), Some(other)) => {
                other.dashes().unwrap_or_default() == *dashes && other.offset() == *offset
            }
            _ => false,
        };

        brushes_equal
            && dashes_equal
            && self.0.thickness == other.thickness()
            && self.0.line_cap == other.line_cap()
            && self.0.line_join == other.line_join()
            && self.0.miter_limit == other.miter_limit()
    }
}

impl std::fmt::Debug for SharedBrush {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self as &dyn IBrush, f)
    }
}

impl std::fmt::Debug for SharedPen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self as &dyn IPen, f)
    }
}

/// The shared form of a brush: the one the brush keeps, or a copy of its
/// values made for the occasion when it keeps none (a brush type that only
/// implements the contract).
pub fn shared_brush_of(brush: &dyn IBrush) -> Option<SharedBrush> {
    brush.to_shared().or_else(|| SharedBrush::from_brush(brush))
}

/// The shared form of a pen; see [`shared_brush_of`].
pub fn shared_pen_of(pen: &dyn IPen) -> Option<SharedPen> {
    pen.to_shared().or_else(|| SharedPen::from_pen(pen))
}

/// Compile-time proof of what the module promises.
const _: fn() = || {
    fn shared<T: Send + Sync>() {}
    shared::<SharedBrush>();
    shared::<SharedPen>();
};

#[cfg(test)]
mod tests {
    // Not from upstream.
    use super::*;
    use crate::media::immutable::{ImmutableLinearGradientBrush, ImmutableSolidColorBrush};
    use crate::media::Colors;

    #[test]
    fn an_immutable_brush_keeps_one_shared_form() {
        let brush = ImmutableSolidColorBrush::with_opacity(Colors::RED, 0.5);

        let first = brush.to_shared().unwrap();
        let second = brush.to_shared().unwrap();

        assert_eq!(first.reference_id(), second.reference_id());
        assert_eq!(Colors::RED, first.as_solid_color_brush().unwrap().color());
        assert_eq!(0.5, IBrush::opacity(&first));
        assert!(first.as_gradient_brush().is_none());
        assert!(first.equals(&shared_brush_of(&ImmutableSolidColorBrush::with_opacity(Colors::RED, 0.5)).unwrap()));
        assert!(!first.equals(&shared_brush_of(&ImmutableSolidColorBrush::new(Colors::RED)).unwrap()));
    }

    #[test]
    fn a_gradient_is_copied_with_its_stops() {
        let stops = [ImmutableGradientStop::new(0.0, Colors::RED), ImmutableGradientStop::new(1.0, Colors::BLUE)];
        let brush = ImmutableLinearGradientBrush::from_stops(&stops);

        let shared = brush.to_shared().unwrap();

        let linear = shared.as_linear_gradient_brush().unwrap();
        assert_eq!(brush.start_point(), linear.start_point());
        assert_eq!(brush.end_point(), linear.end_point());
        let copied: Vec<(f64, Color)> =
            linear.gradient_stops().iter().map(|stop| (stop.offset(), stop.color())).collect();
        assert_eq!(vec![(0.0, Colors::RED), (1.0, Colors::BLUE)], copied);
        assert!(shared.as_solid_color_brush().is_none());
    }

    #[test]
    fn a_pen_is_shared_with_its_brush_and_dashes() {
        let brush: Rc<dyn IImmutableBrush> = Rc::new(ImmutableSolidColorBrush::new(Colors::GREEN));
        let dashes = Rc::new(ImmutableDashStyle::new(Some(&[2.0, 1.0]), 3.0));
        let pen = ImmutablePen::new(Some(brush.clone()), 4.0, Some(dashes), PenLineCap::Round, PenLineJoin::Bevel, 7.0);

        let shared = pen.to_shared().unwrap();

        assert_eq!(shared.reference_id(), pen.to_shared().unwrap().reference_id());
        assert_eq!(4.0, shared.thickness());
        assert_eq!(PenLineCap::Round, shared.line_cap());
        assert_eq!(PenLineJoin::Bevel, shared.line_join());
        assert_eq!(7.0, shared.miter_limit());
        assert_eq!(Some(vec![2.0, 1.0]), shared.dash_style().unwrap().dashes());
        assert_eq!(3.0, shared.dash_style().unwrap().offset());
        // The brush of the pen is the shared form of the pen's brush, each
        // time it is asked for.
        let expected = brush.to_shared().unwrap();
        assert_eq!(expected.reference_id(), shared.brush().unwrap().reference_id());
        assert_eq!(expected.reference_id(), shared.brush().unwrap().reference_id());
        assert!(shared.equals(&pen));
    }
}
