use crate::media::immutable::{ImmutableGradientStop, ImmutableTransform};
use crate::media::{GradientBrush, GradientSpreadMethod, IGradientStop, ITransform, TransformExtensions};
use crate::RelativePoint;
use std::rc::Rc;

/// The data shared by all immutable gradient brushes.
#[derive(Clone)]
pub struct ImmutableGradientBrush {
    gradient_stops: Vec<Rc<dyn IGradientStop>>,
    opacity: f64,
    transform: Option<Rc<dyn ITransform>>,
    transform_origin: RelativePoint,
    spread_method: GradientSpreadMethod,
    relative_transform: Option<Rc<dyn ITransform>>,
}

impl ImmutableGradientBrush {
    /// Creates the gradient brush data. `transform_origin` defaults to the
    /// top left corner.
    pub fn new(
        gradient_stops: &[ImmutableGradientStop],
        opacity: f64,
        transform: Option<Rc<ImmutableTransform>>,
        transform_origin: Option<RelativePoint>,
        spread_method: GradientSpreadMethod,
        relative_transform: Option<Rc<ImmutableTransform>>,
    ) -> Self {
        Self {
            gradient_stops: gradient_stops.iter().map(|stop| Rc::new(*stop) as Rc<dyn IGradientStop>).collect(),
            opacity,
            transform: transform.map(|t| t as Rc<dyn ITransform>),
            transform_origin: transform_origin.unwrap_or(RelativePoint::TOP_LEFT),
            spread_method,
            relative_transform: relative_transform.map(|t| t as Rc<dyn ITransform>),
        }
    }

    /// Creates an immutable copy of the gradient data of `source`.
    pub fn from_brush(source: &GradientBrush) -> Self {
        Self::new(
            &source.gradient_stops().to_immutable(),
            source.opacity(),
            source.transform().map(|t| Rc::new(TransformExtensions::to_immutable(&*t))),
            Some(source.transform_origin()),
            source.spread_method(),
            source.relative_transform().map(|t| Rc::new(TransformExtensions::to_immutable(&*t))),
        )
    }

    /// The brush's gradient stops.
    #[inline]
    pub fn gradient_stops(&self) -> &[Rc<dyn IGradientStop>] {
        &self.gradient_stops
    }

    /// The opacity of the brush.
    #[inline]
    pub fn opacity(&self) -> f64 {
        self.opacity
    }

    /// The transform of the brush.
    #[inline]
    pub fn transform(&self) -> Option<Rc<dyn ITransform>> {
        self.transform.clone()
    }

    /// The transform origin of the brush.
    #[inline]
    pub fn transform_origin(&self) -> RelativePoint {
        self.transform_origin
    }

    /// The brush's spread method.
    #[inline]
    pub fn spread_method(&self) -> GradientSpreadMethod {
        self.spread_method
    }

    /// The transform of the brush, relative to the bounds of the painted
    /// area.
    #[inline]
    pub fn relative_transform(&self) -> Option<Rc<dyn ITransform>> {
        self.relative_transform.clone()
    }
}

/// Implements the brush interfaces of an immutable gradient brush whose
/// `base` field is an [`ImmutableGradientBrush`].
macro_rules! immutable_gradient_brush_interfaces {
    ($name:ty, $as_fn:ident, $iface:path) => {
        impl ::std::ops::Deref for $name {
            type Target = $crate::media::immutable::ImmutableGradientBrush;
            #[inline]
            fn deref(&self) -> &Self::Target {
                &self.base
            }
        }

        impl $crate::media::IBrush for $name {
            #[inline]
            fn opacity(&self) -> f64 {
                self.base.opacity()
            }

            fn transform(&self) -> Option<::std::rc::Rc<dyn $crate::media::ITransform>> {
                self.base.transform()
            }

            #[inline]
            fn transform_origin(&self) -> $crate::RelativePoint {
                self.base.transform_origin()
            }

            fn relative_transform(&self) -> Option<::std::rc::Rc<dyn $crate::media::ITransform>> {
                self.base.relative_transform()
            }

            fn as_immutable_brush(&self) -> Option<&dyn $crate::media::IImmutableBrush> {
                Some(self)
            }

            fn as_any(&self) -> &dyn ::std::any::Any {
                self
            }

            fn as_gradient_brush(&self) -> Option<&dyn $crate::media::IGradientBrush> {
                Some(self)
            }

            fn $as_fn(&self) -> Option<&dyn $iface> {
                Some(self)
            }

            fn into_immutable_brush(
                self: ::std::rc::Rc<Self>,
            ) -> Option<::std::rc::Rc<dyn $crate::media::IImmutableBrush>> {
                Some(self)
            }
        }

        impl $crate::media::IGradientBrush for $name {
            fn gradient_stops(&self) -> Vec<::std::rc::Rc<dyn $crate::media::IGradientStop>> {
                self.base.gradient_stops().to_vec()
            }

            #[inline]
            fn spread_method(&self) -> $crate::media::GradientSpreadMethod {
                self.base.spread_method()
            }
        }

        impl $crate::media::IImmutableBrush for $name {}
    };
}

pub(crate) use immutable_gradient_brush_interfaces;
