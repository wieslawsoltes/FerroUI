use super::{AnimationBrush, AnimationGradient, AnimationGradientStop, BrushForm, GradientShape};
use crate::{AnimationError, AnimationTransformList, AnimationValue, Result};
use ferroui_base::animation::{Animatable, animators::GradientBrushAnimator};
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{
    Color, ConicGradientBrush, GradientBrush, GradientStop, IBrush, LinearGradientBrush,
    RadialGradientBrush,
};
use ferroui_base::{BoxedValue, FerroProperty, FerroPropertyRegistry, Ref};
use std::{any::TypeId, rc::Rc};

impl AnimationBrush {
    pub fn is_property(property: &FerroProperty) -> bool {
        property.property_type() == TypeId::of::<Option<Rc<dyn IBrush>>>()
    }

    pub fn base_value(target: &Animatable, property: &'static FerroProperty) -> Option<Self> {
        if !FerroPropertyRegistry::instance().is_registered_for(target, property) {
            return None;
        }
        let property = property.as_styled::<Option<Rc<dyn IBrush>>>()?;
        let brush = target.get_base_value(property).flatten()?;
        Self::from_native(&*brush).ok()
    }

    pub fn from_boxed(value: &BoxedValue) -> Option<Result<Self>> {
        let brush = value.downcast_ref::<Option<Rc<dyn IBrush>>>()?.as_deref()?;
        Some(Self::from_native(brush))
    }

    pub fn sampled_value(value: &BoxedValue) -> std::result::Result<AnimationValue, String> {
        Self::from_boxed(value)
            .ok_or("The native sampler did not produce a brush at this time")?
            .map(AnimationValue::Brush)
            .map_err(|error| error.to_string())
    }

    pub fn from_native(brush: &dyn IBrush) -> Result<Self> {
        let opacity = brush.opacity();
        if let Some(solid) = brush.as_solid_color_brush() {
            let argb = argb(solid.color());
            let brush = Self::Solid {
                argb,
                opacity,
                form: if opacity == 1.0 {
                    BrushForm::Literal
                } else {
                    BrushForm::Object
                },
            };
            brush.validate()?;
            return Ok(brush);
        }
        let gradient = brush.as_gradient_brush().ok_or_else(|| {
            AnimationError("Choose a native linear, radial, or conic gradient brush".into())
        })?;
        let shape = if let Some(linear) = brush.as_linear_gradient_brush() {
            GradientShape::Linear {
                start: linear.start_point().into(),
                end: linear.end_point().into(),
            }
        } else if let Some(radial) = brush.as_radial_gradient_brush() {
            GradientShape::Radial {
                center: radial.center().into(),
                origin: radial.gradient_origin().into(),
                radius_x: radial.radius_x().into(),
                radius_y: radial.radius_y().into(),
            }
        } else if let Some(conic) = brush.as_conic_gradient_brush() {
            GradientShape::Conic {
                center: conic.center().into(),
                angle: conic.angle(),
            }
        } else {
            return Err(AnimationError(
                "This gradient has an application-specific shape".into(),
            ));
        };
        let value = AnimationGradient {
            shape,
            opacity,
            spread: gradient.spread_method().into(),
            stops: gradient
                .gradient_stops()
                .iter()
                .map(|stop| AnimationGradientStop {
                    offset: stop.offset(),
                    argb: argb(stop.color()),
                })
                .collect(),
            transform_origin: brush.transform_origin().into(),
            transform: brush
                .transform()
                .as_deref()
                .map(|transform| AnimationTransformList::from_native(Some(transform)))
                .transpose()?,
            relative_transform: brush
                .relative_transform()
                .as_deref()
                .map(|transform| AnimationTransformList::from_native(Some(transform)))
                .transpose()?,
        };
        value.validate()?;
        Ok(Self::gradient(value))
    }

    pub fn to_native(&self) -> Rc<dyn IBrush> {
        match self {
            Self::Gradient { value, .. } => value.native(),
            &Self::Solid { argb, opacity, .. } => {
                Rc::new(ImmutableSolidColorBrush::with_opacity(color(argb), opacity))
            }
        }
    }

    pub(crate) fn interpolate(&self, next: &Self, position: f64) -> Result<Self> {
        if !(matches!(self, Self::Gradient { .. }) && matches!(next, Self::Gradient { .. })) {
            return Err(AnimationError(
                "Sample mixed brush tracks with the native animation clock".into(),
            ));
        }
        let brush = GradientBrushAnimator::interpolate_core(
            position,
            &Some(self.to_native()),
            &Some(next.to_native()),
        )
        .ok_or_else(|| AnimationError("The native animator produced an empty brush".into()))?;
        Self::from_native(&*brush)
    }
}

impl AnimationGradient {
    fn native(&self) -> Rc<dyn IBrush> {
        let brush: Ref<GradientBrush> = match self.shape {
            GradientShape::Linear { start, end } => {
                let brush = LinearGradientBrush::new();
                brush.set_start_point(start.native());
                brush.set_end_point(end.native());
                brush.upcast()
            }
            GradientShape::Radial {
                center,
                origin,
                radius_x,
                radius_y,
            } => {
                let brush = RadialGradientBrush::new();
                brush.set_center(center.native());
                brush.set_gradient_origin(origin.native());
                brush.set_radius_x(radius_x.native());
                brush.set_radius_y(radius_y.native());
                brush.upcast()
            }
            GradientShape::Conic { center, angle } => {
                let brush = ConicGradientBrush::new();
                brush.set_center(center.native());
                brush.set_angle(angle);
                brush.upcast()
            }
        };
        brush.set_opacity(self.opacity);
        brush.set_spread_method(self.spread.into());
        brush.set_transform_origin(self.transform_origin.native());
        brush.set_transform(
            self.transform
                .as_ref()
                .and_then(AnimationTransformList::to_transform),
        );
        brush.set_relative_transform(
            self.relative_transform
                .as_ref()
                .and_then(AnimationTransformList::to_transform),
        );
        let stops = brush.gradient_stops();
        for stop in &self.stops {
            stops.add(GradientStop::with_color_and_offset(
                color(stop.argb),
                stop.offset,
            ));
        }
        brush.into()
    }
}

fn argb(color: Color) -> [u8; 4] {
    [color.a, color.r, color.g, color.b]
}

fn color([a, r, g, b]: [u8; 4]) -> Color {
    Color::from_argb(a, r, g, b)
}
