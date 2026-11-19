use super::{syntax, FloatSource};
use ferroui_designer_animation::{AnimationValue, ValueKind};
use ferroui_designer_core::TextPatch;
use syn::Expr;

#[derive(Clone, Copy)]
pub(crate) struct ValueShape(ValueKind);

#[derive(Clone, Debug)]
pub(super) struct ValueSource {
    pub(super) value: AnimationValue,
    parts: Vec<FloatSource>,
}

impl ValueShape {
    pub(crate) const NUMBER: Self = Self(ValueKind::Number);
    pub(crate) const CORNER_RADIUS: Self = Self(ValueKind::CornerRadius);

    pub(crate) fn from_type(path: &str) -> Option<Self> {
        if path == "f64" {
            return Some(Self::NUMBER);
        }
        let name = path.strip_prefix("::ferroui_base::")?;
        Some(Self(match name {
            "CornerRadius" | "corner_radius::CornerRadius" => ValueKind::CornerRadius,
            "Thickness" | "thickness::Thickness" => ValueKind::Thickness,
            "Point" | "point::Point" => ValueKind::Point,
            "Vector" | "vector::Vector" => ValueKind::Vector,
            "Size" | "size::Size" => ValueKind::Size,
            "Rect" | "rect::Rect" => ValueKind::Rect,
            _ => return None,
        }))
    }

    pub(super) fn read(self, expression: &Expr) -> Option<ValueSource> {
        let kind = self.0;
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
                || Self::from_type(&format!("::{owner}"))?.0 != kind
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
            parts,
        })
    }
}

impl ValueSource {
    pub(super) fn plan(&self, text: &str, patches: &mut Vec<TextPatch>) -> Result<(), String> {
        let kind = self.value.kind();
        let value = AnimationValue::parse(text, kind).map_err(|error| error.to_string())?;
        let values = value
            .to_xaml()
            .split(',')
            .map(str::parse::<f64>)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| "The key value has no numeric Rust representation")?;
        let components = match (self.parts.len(), values.as_slice()) {
            (1, [a, b, c, d]) if a == b && a == c && a == d => vec![*a],
            (2, [a, b, c, d]) if kind == ValueKind::CornerRadius && a == b && c == d => {
                vec![*a, *c]
            }
            (count, _) if count == values.len() => values,
            _ => {
                return Err(
                    "Enter values that fit the Rust constructor, or change its shape in source"
                        .into(),
                )
            }
        };
        for (part, value) in self.parts.iter().zip(components) {
            part.plan(value, patches);
        }
        Ok(())
    }
}
