use super::box_shadows::BoxShadowsSource;
use super::brush::BrushSource;
use super::color::{packed_argument, ColorSource};
use super::effect::EffectSource;
use super::relative::RelativeSource;
use super::transform_list::TransformListSource;
use super::{syntax, FloatSource};
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
    Numbers(Vec<FloatSource>),
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
            let (value, source) = RelativeSource::read(expression, kind)?;
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
        let parts = if kind == ValueKind::Number
            || matches!(kind, ValueKind::CornerRadius | ValueKind::Thickness)
                && !matches!(expression, Expr::Call(_))
        {
            vec![syntax::float(expression)?]
        } else {
            let Expr::Call(call) = expression else {
                return None;
            };
            let path = syntax::path(&call.func)?;
            let (owner, method) = path.rsplit_once("::")?;
            let (crate_name, type_name) = owner.split_once("::")?;
            if crate_name != "ferroui_base"
                || Self::from_type(&format!("::{owner}"))?.kind != kind
                || type_name.contains("::")
            {
                return None;
            }
            let count = match (method, kind) {
                ("uniform", ValueKind::CornerRadius | ValueKind::Thickness) => 1,
                ("top_bottom", ValueKind::CornerRadius) => 2,
                ("new", ValueKind::Point | ValueKind::Vector | ValueKind::Size) => 2,
                ("new", ValueKind::CornerRadius | ValueKind::Thickness | ValueKind::Rect) => 4,
                _ => return None,
            };
            if call.args.len() != count {
                return None;
            }
            call.args
                .iter()
                .map(syntax::float)
                .collect::<Option<Vec<_>>>()?
        };
        let mut values = parts.iter().map(|part| part.value).collect::<Vec<_>>();
        if let (ValueKind::CornerRadius, [top, bottom]) = (kind, values.as_slice()) {
            values = vec![*top, *top, *bottom, *bottom];
        }
        let text = values
            .iter()
            .map(f64::to_string)
            .collect::<Vec<_>>()
            .join(",");
        Some(ValueSource {
            value: AnimationValue::parse(&text, kind).ok()?,
            source: ValueSyntax::Numbers(parts),
        })
    }

    pub(super) fn read_keyframe(self, expression: &Expr, project_effects: bool) -> Option<ValueSource> {
        if self.brush {
            if let Some((value, source)) = BrushSource::read(expression, &syntax::path) {
                value.validate().ok()?;
                return Some(ValueSource { value, source: ValueSyntax::Brush(Box::new(source)) });
            }
        }
        if self.kind == ValueKind::Effect {
            return self.read_effect(expression, syntax::path, project_effects);
        }
        self.read(expression)
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
        let parts = match &self.source {
            ValueSyntax::Color(source) => return source.plan(&self.value, &value, patches),
            ValueSyntax::Relative(source) => return source.plan(&value, patches),
            ValueSyntax::Effect(_) => return Err("The key requires an effect".into()),
            ValueSyntax::BoxShadows(source) => {
                let (AnimationValue::BoxShadows(previous), AnimationValue::BoxShadows(next)) =
                    (&self.value, value)
                else {
                    return Err("The key requires box shadows".into());
                };
                return source.plan(previous, &next, patches);
            }
            ValueSyntax::TransformList(source) => {
                if self.value != value {
                    source.plan(&value.to_xaml(), patches);
                }
                return Ok(());
            }
            ValueSyntax::Brush(source) => {
                let (AnimationValue::Brush(old), AnimationValue::Brush(next)) = (&self.value, value) else {
                    return Err("The key requires a brush".into());
                };
                return source.plan(old, &next, patches);
            }
            ValueSyntax::Numbers(parts) => parts,
        };
        let values = numeric_parts(&value)?;
        let components = match (parts.len(), values.as_slice()) {
            (1, [a, b, c, d]) if a == b && a == c && a == d => vec![*a],
            (2, [a, b, c, d]) if kind == ValueKind::CornerRadius && a == b && c == d => {
                vec![*a, *c]
            }
            (count, _) if count == values.len() => values,
            _ => {
                return Err(
                    "Enter values that fit the Rust constructor, or change its shape in source"
                        .into(),
                );
            }
        };
        for (part, value) in parts.iter().zip(components) {
            part.plan(value, patches);
        }
        Ok(())
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
