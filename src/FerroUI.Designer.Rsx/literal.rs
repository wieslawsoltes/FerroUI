use crate::animation::{ValueShape, rust_literal};
use ferroui_designer_animation::{AnimationValue, ValueKind};
use ferroui_designer_core::TextPatch;
use ferroui_designer_language::PropertyInfo;
use syn::{Expr, Lit, UnOp};

pub(crate) fn string_literal(original: &str, value: &str) -> String {
    if let Some((prefix, _)) = original.split_once('"') {
        if let Some(hashes) = prefix.strip_prefix('r') {
            if hashes.chars().all(|character| character == '#')
                && !value.contains(&format!("\"{hashes}"))
                && !value.contains('\r')
            {
                return format!("{prefix}\"{value}\"{hashes}");
            }
        }
    }
    format!("{value:?}")
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Literal {
    Text,
    Bool,
    Integer,
    Number,
    CornerRadius,
    BoxShadows,
    Thickness,
    EmbeddedImage,
    Enum {
        module: &'static str,
        name: String,
        variants: Vec<String>,
    },
}

impl Literal {
    pub(crate) fn enumeration(module: &'static str, property: &PropertyInfo) -> Self {
        Self::Enum {
            module,
            name: property.type_name.clone(),
            variants: property.values.clone(),
        }
    }

    /// XAML lexical spelling (the leading `{}` escapes markup-extension syntax).
    pub(crate) fn read_rust(&self, expr: &Expr) -> Option<String> {
        let literal = match expr {
            Expr::Lit(value) => Some(&value.lit),
            _ => None,
        };
        match self {
            Self::Text => match literal? {
                Lit::Str(value) => Some(text_value(&value.value())),
                _ => None,
            },
            Self::Bool => match literal? {
                Lit::Bool(value) => Some(value.value.to_string()),
                _ => None,
            },
            Self::Integer => Some(integer(expr)?.to_string()),
            Self::Number => Some(number(expr)?.to_string()),
            Self::CornerRadius => read_radius(expr),
            Self::BoxShadows => Some(ValueShape::BOX_SHADOWS.read(expr)?.value.to_xaml()),
            Self::EmbeddedImage => crate::image_source::read(expr),
            Self::Thickness => match expr {
                Expr::Tuple(tuple) if matches!(tuple.elems.len(), 2 | 4) => Some(
                    tuple
                        .elems
                        .iter()
                        .map(|value| number(value).map(|n| n.to_string()))
                        .collect::<Option<Vec<_>>>()?
                        .join(","),
                ),
                _ => Some(number(expr)?.to_string()),
            },
            Self::Enum {
                module,
                name,
                variants,
            } => {
                let Expr::Path(path) = expr else { return None };
                if path.qself.is_some()
                    || path.path.segments.iter().any(|s| !s.arguments.is_empty())
                {
                    return None;
                }
                let segments: Vec<_> = path
                    .path
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect();
                let (value, prefix) = segments.split_last()?;
                let owner = prefix.join("::");
                ((owner == *name || owner == format!("{module}::{name}"))
                    && variants.contains(value))
                .then(|| value.clone())
            }
        }
    }

    pub(crate) fn rust(&self, value: &str) -> Option<String> {
        let code = match self {
            Self::Text => {
                let text = value
                    .strip_prefix("{}")
                    .or_else(|| (!value.starts_with('{')).then_some(value))?;
                format!("{text:?}")
            }
            Self::Bool => match value.trim().to_ascii_lowercase().as_str() {
                "true" => "true".into(),
                "false" => "false".into(),
                _ => return None,
            },
            Self::Integer => value.trim().parse::<i32>().ok()?.to_string(),
            Self::Number => format!("{:?}", parse_number(value.trim())?),
            Self::CornerRadius => rust_radius(value)?,
            Self::BoxShadows => rust_literal(&AnimationValue::parse(value, ValueKind::BoxShadows).ok()?).ok()?,
            Self::EmbeddedImage => crate::image_source::rust(value)?,
            Self::Thickness => {
                let values = value
                    .split(|c: char| c == ',' || c.is_whitespace())
                    .filter(|v| !v.is_empty())
                    .map(parse_number)
                    .collect::<Option<Vec<_>>>()?;
                match values.as_slice() {
                    [one] => format!("{one:?}"),
                    [a, b] => format!("({a:?}, {b:?})"),
                    [a, b, c, d] => format!("({a:?}, {b:?}, {c:?}, {d:?})"),
                    _ => return None,
                }
            }
            Self::Enum {
                module,
                name,
                variants,
            } => {
                let variant = variants
                    .iter()
                    .find(|item| item.eq_ignore_ascii_case(value.trim()))?;
                format!("::{module}::{name}::{variant}")
            }
        };
        Some(code)
    }

    pub(crate) fn normalize(&self, value: &str) -> Option<String> {
        self.read_rust(&syn::parse_str(&self.rust(value)?).ok()?)
    }
}

pub(crate) fn edit_box_shadows(original: &str, value: &str) -> Result<String, String> {
    let expression = syn::parse_str(original).map_err(|error| error.to_string())?;
    let source = ValueShape::BOX_SHADOWS.read(&expression).ok_or("Edit a literal box shadow list")?;
    let mut patches = Vec::new();
    source.plan(value, &mut patches)?;
    TextPatch::apply_all(original, patches).map_err(|error| error.to_string())
}

pub(crate) fn text_value(text: &str) -> String {
    if text.starts_with('{') {
        format!("{{}}{text}")
    } else {
        text.into()
    }
}

fn numeric_literal(expr: &Expr) -> Option<(bool, &Lit)> {
    match expr {
        Expr::Unary(unary) if matches!(unary.op, UnOp::Neg(_)) => match &*unary.expr {
            Expr::Lit(value) => Some((true, &value.lit)),
            _ => None,
        },
        Expr::Lit(value) => Some((false, &value.lit)),
        _ => None,
    }
}

fn integer(expr: &Expr) -> Option<i32> {
    let (negative, Lit::Int(lit)) = numeric_literal(expr)? else {
        return None;
    };
    if !matches!(lit.suffix(), "" | "i32") {
        return None;
    }
    let value: i64 = lit.base10_parse().ok()?;
    i32::try_from(if negative { -value } else { value }).ok()
}

fn number(expr: &Expr) -> Option<f64> {
    let (negative, lit) = numeric_literal(expr)?;
    let mut value = match lit {
        Lit::Int(lit) if matches!(lit.suffix(), "" | "i32" | "u32" | "i64" | "u64") => {
            lit.base10_parse::<u64>().ok()? as f64
        }
        Lit::Float(lit) if matches!(lit.suffix(), "" | "f64") => lit.base10_parse::<f64>().ok()?,
        Lit::Float(lit) if lit.suffix() == "f32" => f64::from(lit.base10_parse::<f32>().ok()?),
        _ => return None,
    };
    if negative {
        value = -value;
    }
    value.is_finite().then_some(value)
}

fn parse_number(value: &str) -> Option<f64> {
    let value: f64 = value.parse().ok()?;
    value.is_finite().then_some(value)
}

fn read_radius(expr: &Expr) -> Option<String> {
    if let Some(radius) = number(expr) {
        return Some(radius.to_string());
    }
    if let Expr::Tuple(tuple) = expr {
        if !tuple.attrs.is_empty() || !matches!(tuple.elems.len(), 2 | 4) {
            return None;
        }
        let values = tuple.elems.iter().map(|value| number(value).map(|value| value.to_string()))
            .collect::<Option<Vec<_>>>()?;
        return Some(values.join(","));
    }
    let Expr::Call(call) = expr else { return None };
    let Expr::Path(function) = call.func.as_ref() else {
        return None;
    };
    if !call.attrs.is_empty() || !function.attrs.is_empty() || function.path.leading_colon.is_none()
    {
        return None;
    }
    ValueShape::CORNER_RADIUS
        .read(expr)
        .map(|source| source.value.to_xaml())
}

fn rust_radius(value: &str) -> Option<String> {
    // Keep existing scalar declarations compatible with the Rust builder conversion.
    if let Some(radius) = parse_number(value.trim()) {
        return Some(format!("{radius:?}"));
    }
    match AnimationValue::parse(value, ValueKind::CornerRadius).ok()? {
        AnimationValue::CornerRadius {
            top_left,
            top_right,
            bottom_right,
            bottom_left,
        } if top_left == top_right && top_left == bottom_right && top_left == bottom_left => {
            Some(format!("{top_left:?}"))
        }
        AnimationValue::CornerRadius {
            top_left, top_right, bottom_right, bottom_left,
        } if top_left == top_right && bottom_right == bottom_left => {
            Some(format!("({top_left:?}, {bottom_right:?})"))
        }
        AnimationValue::CornerRadius { top_left, top_right, bottom_right, bottom_left } => {
            Some(format!("({top_left:?}, {top_right:?}, {bottom_right:?}, {bottom_left:?})"))
        }
        _ => None,
    }
}
