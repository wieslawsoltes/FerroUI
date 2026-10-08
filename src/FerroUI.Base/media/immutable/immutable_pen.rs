use crate::media::immutable::{ImmutableDashStyle, ImmutableSolidColorBrush};
use crate::media::{IBrush, IDashStyle, IImmutableBrush, IPen, PenLineCap, PenLineJoin};
use std::any::Any;
use std::rc::Rc;

/// Describes how a stroke is drawn.
#[derive(Clone, Debug)]
pub struct ImmutablePen {
    brush: Option<Rc<dyn IBrush>>,
    thickness: f64,
    dash_style: Option<Rc<dyn IDashStyle>>,
    line_cap: PenLineCap,
    line_join: PenLineJoin,
    miter_limit: f64,
    /// The form sent to the render thread, created on first use.
    shared: std::sync::OnceLock<Option<crate::media::SharedPen>>,
}

impl ImmutablePen {
    /// Creates a pen.
    pub fn new(
        brush: Option<Rc<dyn IImmutableBrush>>,
        thickness: f64,
        dash_style: Option<Rc<ImmutableDashStyle>>,
        line_cap: PenLineCap,
        line_join: PenLineJoin,
        miter_limit: f64,
    ) -> Self {
        Self {
            brush: brush.map(|b| b as Rc<dyn IBrush>),
            thickness,
            line_cap,
            line_join,
            miter_limit,
            dash_style: dash_style.map(|d| d as Rc<dyn IDashStyle>),
            shared: std::sync::OnceLock::new(),
        }
    }

    /// Creates a solid pen with the given brush and thickness: no dashes,
    /// flat caps, miter joins and a miter limit of 10.
    pub fn with_brush(brush: Option<Rc<dyn IImmutableBrush>>, thickness: f64) -> Self {
        Self::new(brush, thickness, None, PenLineCap::Flat, PenLineJoin::Miter, 10.0)
    }

    /// Creates a solid pen with the color given as an `0xAARRGGBB` value.
    pub fn from_uint32(color: u32, thickness: f64) -> Self {
        Self::with_brush(Some(Rc::new(ImmutableSolidColorBrush::from_uint32(color))), thickness)
    }

    /// The brush used to draw the stroke.
    #[inline]
    pub fn brush(&self) -> Option<Rc<dyn IBrush>> {
        self.brush.clone()
    }

    /// The stroke thickness.
    #[inline]
    pub fn thickness(&self) -> f64 {
        self.thickness
    }

    /// The dash style for the stroke.
    #[inline]
    pub fn dash_style(&self) -> Option<Rc<dyn IDashStyle>> {
        self.dash_style.clone()
    }

    /// The type of shape to use on both ends of a line.
    #[inline]
    pub fn line_cap(&self) -> PenLineCap {
        self.line_cap
    }

    /// A value describing how to join consecutive line or curve segments.
    #[inline]
    pub fn line_join(&self) -> PenLineJoin {
        self.line_join
    }

    /// The limit of the ratio of the miter length to half this pen's
    /// thickness.
    #[inline]
    pub fn miter_limit(&self) -> f64 {
        self.miter_limit
    }
}

impl IPen for ImmutablePen {
    fn brush(&self) -> Option<Rc<dyn IBrush>> {
        self.brush.clone()
    }

    fn dash_style(&self) -> Option<Rc<dyn IDashStyle>> {
        self.dash_style.clone()
    }

    #[inline]
    fn line_cap(&self) -> PenLineCap {
        self.line_cap
    }

    #[inline]
    fn line_join(&self) -> PenLineJoin {
        self.line_join
    }

    #[inline]
    fn miter_limit(&self) -> f64 {
        self.miter_limit
    }

    #[inline]
    fn thickness(&self) -> f64 {
        self.thickness
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_immutable_pen(self: Rc<Self>) -> Rc<ImmutablePen> {
        self
    }

    fn to_shared(&self) -> Option<crate::media::SharedPen> {
        self.shared.get_or_init(|| crate::media::SharedPen::from_pen(self)).clone()
    }

    fn equals(&self, other: &dyn IPen) -> bool {
        if self.reference_id() == other.reference_id() {
            return true;
        }

        self.brush == other.brush()
            && self.thickness == other.thickness()
            && self.dash_style == other.dash_style()
            && self.line_cap == other.line_cap()
            && self.line_join == other.line_join()
            && self.miter_limit == other.miter_limit()
    }
}

impl PartialEq for ImmutablePen {
    fn eq(&self, other: &Self) -> bool {
        IPen::equals(self, other)
    }
}
