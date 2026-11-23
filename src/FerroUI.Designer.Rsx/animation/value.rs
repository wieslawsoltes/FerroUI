use super::box_shadows::BoxShadowsSource;
use super::brush::BrushSource;
use super::color::{packed_argument, ColorSource};
use super::effect::EffectSource;
use super::numbers::Numbers;
use super::relative::RelativeSource;
use super::transform_list::TransformListSource;
use super::syntax;
use crate::local_rust::{LocalRust, Value};
use ferroui_designer_animation::{AnimationValue, ValueKind};
use ferroui_designer_core::TextPatch;
use syn::Expr;

#[derive(Clone, Copy, Debug)]
pub(crate) struct ValueShape {
    kind: ValueKind,
    brush: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct ValueSource {
    pub(crate) value: AnimationValue,
    source: ValueSyntax,
}

#[derive(Clone, Debug)]
enum ValueSyntax {
    Numbers(Numbers),
    Color(ColorSource),
    Relative(RelativeSource),
    Effect(EffectSource),
    BoxShadows(BoxShadowsSource),
    TransformList(TransformListSource),
    Brush(Box<BrushSource>),
}

impl ValueShape {
    pub(crate) const NUMBER: Self = Self::typed(ValueKind::Number);
    pub(crate) const CORNER_RADIUS: Self = Self::typed(ValueKind::CornerRadius);
    pub(crate) const RELATIVE_POINT: Self = Self::typed(ValueKind::RelativePoint);
    pub(crate) const EFFECT: Self = Self::typed(ValueKind::Effect);
    pub(crate) const BOX_SHADOWS: Self = Self::typed(ValueKind::BoxShadows);
    pub(crate) const TRANSFORM_LIST: Self = Self::typed(ValueKind::TransformList);
    pub(crate) const SOLID_BRUSH: Self = Self {
        kind: ValueKind::Color,
        brush: true,
    };

    const fn typed(kind: ValueKind) -> Self {
        Self { kind, brush: false }
    }

    pub(crate) fn kind(self) -> ValueKind {
        self.kind
    }

    pub(super) fn is_brush(self) -> bool {
        self.brush
    }

    pub(crate) fn from_type(path: &str) -> Option<Self> {
        if path == "f64" {
            return Some(Self::NUMBER);
        }
        let name = path.strip_prefix("::ferroui_base::")?;
        Some(Self::typed(match name {
            "media::Color" | "media::color::Color" => ValueKind::Color,
            "media::BoxShadows" | "media::box_shadows::BoxShadows" => ValueKind::BoxShadows,
            "CornerRadius" | "corner_radius::CornerRadius" => ValueKind::CornerRadius,
            "Thickness" | "thickness::Thickness" => ValueKind::Thickness,
            "Point" | "point::Point" => ValueKind::Point,
            "Vector" | "vector::Vector" => ValueKind::Vector,
            "Size" | "size::Size" => ValueKind::Size,
            "Rect" | "rect::Rect" => ValueKind::Rect,
            "RelativePoint" | "relative_point::RelativePoint" => ValueKind::RelativePoint,
            "RelativeScalar" | "relative_scalar::RelativeScalar" => ValueKind::RelativeScalar,
            _ => return None,
        }))
    }

    pub(crate) fn read(self, expression: &Expr) -> Option<ValueSource> {
        self.read_with(expression, syntax::path)
    }

    pub(crate) fn read_with(
        self,
        expression: &Expr,
        resolve: impl Fn(&Expr) -> Option<String>,
    ) -> Option<ValueSource> {
        let kind = self.kind;
        if kind == ValueKind::Color {
            return ColorSource::read_with(expression, self.brush, resolve)
                .map(ValueSource::color);
        }
        if matches!(kind, ValueKind::RelativePoint | ValueKind::RelativeScalar) {
            let (value, source) = RelativeSource::read(expression, kind, &resolve)?;
            return Some(ValueSource { value, source: ValueSyntax::Relative(source) });
        }
        if kind == ValueKind::Effect {
            return self.read_effect(expression, resolve, false);
        }
        if kind == ValueKind::BoxShadows {
            let (value, source) = BoxShadowsSource::read(expression, &resolve)?;
            return Some(ValueSource {
                value: AnimationValue::BoxShadows(value),
                source: ValueSyntax::BoxShadows(source),
            });
        }
        if kind == ValueKind::TransformList {
            let (value, source) = TransformListSource::read(expression, &resolve)?;
            return Some(ValueSource {
                value: AnimationValue::TransformList(value),
                source: ValueSyntax::TransformList(source),
            });
        }
        let (value, source) = Numbers::read(expression, kind, &resolve)?;
        Some(ValueSource { value, source: ValueSyntax::Numbers(source) })
    }

    pub(super) fn read_keyframe(
        self,
        expression: &Expr,
        project_effects: bool,
        resolve: &impl Fn(&Expr) -> Option<String>,
        shared_stops: &impl Fn(&Expr) -> Option<Vec<ferroui_designer_animation::AnimationGradientStop>>,
    ) -> Option<ValueSource> {
        if self.brush {
            if let Some((value, source)) = BrushSource::read(expression, resolve, shared_stops) {
                value.validate().ok()?;
                return Some(ValueSource { value, source: ValueSyntax::Brush(Box::new(source)) });
            }
        }
        if self.kind == ValueKind::Effect {
            return self.read_effect(expression, resolve, project_effects);
        }
        self.read_with(expression, resolve)
    }

    fn read_effect(self, expression: &Expr, resolve: impl Fn(&Expr) -> Option<String>, project_effects: bool) -> Option<ValueSource> {
        let (value, source) = EffectSource::read(expression, &resolve, project_effects)?;
        Some(ValueSource { value: AnimationValue::Effect(value), source: ValueSyntax::Effect(source) })
    }
}

impl ValueSource {
    pub(crate) fn read_resource_color<'a>(
        expression: &'a Expr,
        bindings: &'a LocalRust,
    ) -> Option<(Self, Value<'a>)> {
        let initializer = bindings.value(expression)?;
        let resolve = |path: &Expr| bindings.value_path(&initializer, path);
        let packed = match initializer.expression {
            Expr::Call(call) => resolve(&call.func).and_then(|factory| packed_argument(call, &factory)),
            _ => None,
        };
        if let Some(argument) = packed {
            let operand = bindings.operand(&initializer, argument)?;
            let value = Self::color(ColorSource::read_packed(operand.expression)?);
            if operand.constant.is_none() {
                return Some((value, initializer));
            }
            return matches!(
                bindings.value_type_path(&operand).as_deref(),
                Some("u32" | "core::primitive::u32" | "std::primitive::u32")
            )
            .then_some((value, operand));
        }
        let value = Self::color(ColorSource::read_with(initializer.expression, false, resolve)?);
        Some((value, initializer))
    }

    fn color((value, source): (AnimationValue, ColorSource)) -> Self {
        Self {
            value,
            source: ValueSyntax::Color(source),
        }
    }

    pub(crate) fn plan(&self, text: &str, patches: &mut Vec<TextPatch>) -> Result<(), String> {
        if let (ValueSyntax::Effect(source), AnimationValue::Effect(old)) = (&self.source, &self.value) {
            return source.plan(old, text, patches);
        }
        let kind = self.value.kind();
        let value = AnimationValue::parse_native_literal(text, kind).map_err(|error| error.to_string())?;
        match &self.source {
            ValueSyntax::Color(source) => source.plan(&self.value, &value, patches),
            ValueSyntax::Relative(source) => source.plan(&value, patches),
            ValueSyntax::Effect(_) => Err("The key requires an effect".into()),
            ValueSyntax::BoxShadows(source) => {
                let (AnimationValue::BoxShadows(previous), AnimationValue::BoxShadows(next)) =
                    (&self.value, value)
                else {
                    return Err("The key requires box shadows".into());
                };
                source.plan(previous, &next, patches)
            }
            ValueSyntax::TransformList(source) => {
                if self.value != value {
                    source.plan(&value.to_xaml(), patches);
                }
                Ok(())
            }
            ValueSyntax::Brush(source) => {
                let (AnimationValue::Brush(old), AnimationValue::Brush(next)) = (&self.value, value) else {
                    return Err("The key requires a brush".into());
                };
                source.plan(old, &next, patches)
            }
            ValueSyntax::Numbers(numbers) => numbers.plan(&numeric_parts(&value)?, patches),
        }
    }
}

pub(crate) fn rust_literal(value: &AnimationValue) -> Result<String, String> {
    if let AnimationValue::Brush(brush) = value {
        value.validate().map_err(|error| error.to_string())?;
        return super::brush::literal(brush);
    }
    if let AnimationValue::Effect(effect) = value {
        return super::effect_constructor::literal(effect);
    }
    if let AnimationValue::BoxShadows(shadows) = value {
        value.validate().map_err(|error| error.to_string())?;
        return Ok(format!(
            "::ferroui_base::media::BoxShadows::parse({:?}).unwrap()",
            shadows.to_xaml(),
        ));
    }
    if let AnimationValue::TransformList(list) = value {
        value.validate().map_err(|error| error.to_string())?;
        return Ok(format!("{:?}", list.to_xaml()));
    }
    if let AnimationValue::Number(number) = value {
        return Ok(format!("{number:?}"));
    }
    if let Some([a, r, g, b]) = value.argb8() {
        return Ok(format!(
            "::ferroui_base::media::Color::from_argb({a}, {r}, {g}, {b})"
        ));
    }
    if let Some(relative) = super::relative::literal(value) {
        return Ok(relative);
    }
    let name = match value.kind() {
        ValueKind::CornerRadius => "CornerRadius",
        ValueKind::Thickness => "Thickness",
        ValueKind::Point => "Point",
        ValueKind::Vector => "Vector",
        ValueKind::Size => "Size",
        ValueKind::Rect => "Rect",
        _ => return Err("The key value has no typed Rust builder representation".into()),
    };
    let args = numeric_parts(value)?
        .iter()
        .map(|part| format!("{part:?}"))
        .collect::<Vec<_>>()
        .join(", ");
    Ok(format!("::ferroui_base::{name}::new({args})"))
}

fn numeric_parts(value: &AnimationValue) -> Result<Vec<f64>, String> {
    value
        .to_xaml()
        .split(',')
        .map(str::parse::<f64>)
        .collect::<Result<_, _>>()
        .map_err(|_| "The key value has no numeric Rust representation".into())
}
