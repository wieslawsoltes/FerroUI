use super::super::{
    FloatSource,
    constant_source::ConstantReader,
    relative::{self, RelativeSource},
    syntax,
};
use crate::blocks::keep_comments;
use ferroui_base::{Matrix, media::immutable::ImmutableTransform};
use ferroui_designer_animation::{AnimationTransformList, AnimationValue};
use ferroui_designer_core::{SourceSpan, TextPatch};
use syn::{Expr, ExprCall, spanned::Spanned};

#[derive(Clone, Debug)]
pub(super) struct Leaf {
    span: SourceSpan,
    comments: String,
}

impl Leaf {
    pub(super) fn read(expression: &Expr) -> Option<Self> {
        Some(Self {
            span: syntax::span(expression),
            comments: keep_comments(&expression.span().source_text()?).ok()?,
        })
    }

    pub(super) fn plan(&self, source: String, patches: &mut Vec<TextPatch>) {
        patches.push(TextPatch {
            span: self.span,
            replacement: format!("{}{source}", self.comments),
        });
    }
}

pub(super) fn call<'a>(
    expression: &'a Expr,
    resolve: &impl Fn(&Expr) -> Option<String>,
    matches: impl FnOnce(&str) -> bool,
    count: usize,
) -> Option<&'a ExprCall> {
    let Expr::Call(call) = expression else {
        return None;
    };
    (call.attrs.is_empty() && call.args.len() == count && matches(&resolve(&call.func)?))
        .then_some(call)
}

pub(super) fn optional<'a>(
    expression: &'a Expr,
    resolve: &impl Fn(&Expr) -> Option<String>,
) -> Option<Option<&'a Expr>> {
    if resolve(expression).as_deref().is_some_and(|path| {
        matches!(
            path,
            "None" | "core::option::Option::None" | "std::option::Option::None"
        )
    }) {
        return Some(None);
    }
    let call = call(
        expression,
        resolve,
        |name| {
            matches!(
                name,
                "Some" | "core::option::Option::Some" | "std::option::Option::Some"
            )
        },
        1,
    )?;
    Some(call.args.first())
}

pub(super) fn rc<'a>(
    expression: &'a Expr,
    resolve: &impl Fn(&Expr) -> Option<String>,
) -> Option<&'a Expr> {
    call(
        expression,
        resolve,
        |path| matches!(path, "std::rc::Rc::new" | "alloc::rc::Rc::new"),
        1,
    )?
    .args
    .first()
}

#[derive(Clone, Debug)]
pub(super) enum GeometrySource {
    Number(FloatSource),
    Relative(RelativeSource),
    Default(Leaf),
}

impl GeometrySource {
    pub(super) fn read(
        expression: &Expr,
        default: &AnimationValue,
        resolve: &impl Fn(&Expr) -> Option<String>,
        constants: &ConstantReader<'_>,
    ) -> Option<(AnimationValue, Self)> {
        if matches!(default, AnimationValue::Number(_)) {
            let number = constants.float(expression)?;
            return Some((AnimationValue::Number(number.value), Self::Number(number)));
        }
        if let Some(operand) = optional(expression, resolve)? {
            if let Some((value, source)) = RelativeSource::read(operand, default.kind(), resolve, constants) {
                return Some((value, Self::Relative(source)));
            }
            let value = relative_constant(&resolve(operand)?)?;
            (value.kind() == default.kind())
                .then_some((value, Self::Default(Leaf::read(expression)?)))
        } else {
            Some((default.clone(), Self::Default(Leaf::read(expression)?)))
        }
    }

    pub(super) fn plan(
        &self,
        old: &AnimationValue,
        value: &AnimationValue,
        patches: &mut Vec<TextPatch>,
    ) -> Result<(), String> {
        if old == value {
            return Ok(());
        }
        match self {
            Self::Number(number) => {
                let AnimationValue::Number(value) = value else {
                    return Err("Choose a numeric angle".into());
                };
                number.plan(*value, patches);
            }
            Self::Relative(source) => source.plan(value, patches)?,
            Self::Default(leaf) => leaf.plan(geometry_literal(value)?, patches),
        }
        Ok(())
    }
}

fn relative_constant(path: &str) -> Option<AnimationValue> {
    use ferroui_base::{RelativePoint, RelativeScalar};
    use ferroui_designer_animation::{GradientDistance, GradientPoint};
    if let Some(name) = path.strip_prefix("ferroui_base::RelativePoint::") {
        let point = match name {
            "TOP_LEFT" => RelativePoint::TOP_LEFT,
            "CENTER" => RelativePoint::CENTER,
            "BOTTOM_RIGHT" => RelativePoint::BOTTOM_RIGHT,
            _ => return None,
        };
        return Some(GradientPoint::from(point).value());
    }
    let scalar = match path.strip_prefix("ferroui_base::RelativeScalar::")? {
        "BEGINNING" => RelativeScalar::BEGINNING,
        "MIDDLE" => RelativeScalar::MIDDLE,
        "END" => RelativeScalar::END,
        _ => return None,
    };
    Some(GradientDistance::from(scalar).value())
}

pub(super) fn geometry_literal(value: &AnimationValue) -> Result<String, String> {
    if let AnimationValue::Number(value) = value {
        return Ok(format!("{value:?}"));
    }
    relative::literal(value)
        .map(|literal| format!("Some({literal})"))
        .ok_or_else(|| "Choose a relative brush coordinate".into())
}

pub(super) fn transform(
    expression: &Expr,
    resolve: &impl Fn(&Expr) -> Option<String>,
    constants: &ConstantReader<'_>,
) -> Option<Option<AnimationTransformList>> {
    let Some(operand) = optional(expression, resolve)? else {
        return Some(None);
    };
    let operand = rc(operand, resolve)?;
    let call = call(
        operand,
        resolve,
        |path| path == "ferroui_base::media::immutable::ImmutableTransform::new",
        1,
    )?;
    let matrix = call.args.first()?;
    let matrix = if resolve(matrix).as_deref() == Some("ferroui_base::Matrix::IDENTITY") {
        Matrix::IDENTITY
    } else {
        let Expr::Call(call) = matrix else {
            return None;
        };
        let args = call
            .args
            .iter()
            .map(|expression| constants.float(expression))
            .map(|number| number.map(|number| number.value))
            .collect::<Option<Vec<_>>>()?;
        match (resolve(&call.func)?.as_str(), args.as_slice()) {
            ("ferroui_base::Matrix::new", &[a, b, c, d, e, f]) => Matrix::new(a, b, c, d, e, f),
            ("ferroui_base::Matrix::new_3x3", &[a, b, 0.0, c, d, 0.0, e, f, 1.0]) => {
                Matrix::new(a, b, c, d, e, f)
            }
            _ => return None,
        }
    };
    AnimationTransformList::from_native(Some(&ImmutableTransform::new(matrix)))
        .ok()
        .map(Some)
}

pub(super) fn transform_literal(list: &Option<AnimationTransformList>) -> String {
    let Some(list) = list else {
        return "None".into();
    };
    let matrix = list.to_native().value();
    let parts = [
        matrix.m11, matrix.m12, matrix.m21, matrix.m22, matrix.m31, matrix.m32,
    ]
    .iter()
    .map(|value| format!("{value:?}"))
    .collect::<Vec<_>>()
    .join(", ");
    format!(
        "Some(::std::rc::Rc::new(::ferroui_base::media::immutable::ImmutableTransform::new(::ferroui_base::Matrix::new({parts}))))"
    )
}
