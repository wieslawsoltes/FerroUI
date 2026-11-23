use super::super::{
    color::{ColorSource, rust_literal as color_literal},
    constant_source::{ConstantReader, value_in_scope},
    syntax, FloatSource,
};
use super::operand::Leaf;
use crate::blocks::keep_comments;
use crate::local_rust::RustSourceScope;
use ferroui_designer_animation::{AnimationGradientStop, AnimationValue};
use ferroui_designer_core::{SourceSpan, TextPatch};
use syn::{spanned::Spanned, Expr, ExprArray};

const STOP: &str = "::ferroui_base::media::immutable::ImmutableGradientStop::new";

#[derive(Clone, Debug)]
pub(super) enum StopsSource {
    Inline(InlineStops),
    Shared(Leaf),
}

#[derive(Clone, Debug)]
pub(super) struct InlineStops {
    stops: Vec<StopSource>,
    array: SourceSpan,
    text: String,
    end: usize,
}

#[derive(Clone, Debug)]
struct StopSource {
    span: SourceSpan,
    offset: FloatSource,
    color: ColorSource,
}

impl StopsSource {
    pub(super) fn read(
        expression: &Expr,
        resolve: &impl Fn(&Expr) -> Option<String>,
        shared: &impl Fn(&Expr) -> Option<Vec<AnimationGradientStop>>,
        constants: &ConstantReader<'_>,
    ) -> Option<(Vec<AnimationGradientStop>, Self)> {
        if let Some(array) = borrowed(expression).and_then(array) {
            let (values, source) = InlineStops::read(array, resolve, constants)?;
            return Some((values, Self::Inline(source)));
        }
        Some((shared(expression)?, Self::Shared(Leaf::read(expression)?)))
    }

    pub(super) fn plan(
        &self,
        original: &[AnimationGradientStop],
        next: &[AnimationGradientStop],
        patches: &mut Vec<TextPatch>,
    ) -> Result<(), String> {
        match self {
            Self::Inline(source) => source.plan(original, next, patches),
            Self::Shared(source) => {
                if original != next {
                    source.plan(literal(next), patches);
                }
                Ok(())
            }
        }
    }
}

pub(in crate::animation) fn shared_key(expression: &Expr) -> Option<String> {
    match borrowed(expression) {
        Some(path) => Some(format!("&{}", syntax::authored_path(path)?)),
        None => syntax::authored_path(syntax::unparen(expression)),
    }
}

pub(in crate::animation) fn shared_values(
    expression: &Expr,
    scope: RustSourceScope<'_>,
) -> Option<Vec<AnimationGradientStop>> {
    let reference = borrowed(expression);
    let path = reference.unwrap_or_else(|| syntax::unparen(expression));
    let (initializer, scope) = scope.constant_initializer(path)?;
    let array = if reference.is_some() {
        array(initializer)?
    } else {
        array(borrowed(initializer)?)?
    };
    InlineStops::read(
        array,
        &|path| scope.path(path),
        &ConstantReader(&|path, ty| value_in_scope(path, ty, scope)),
    )
    .map(|(values, _)| values)
}

fn borrowed(expression: &Expr) -> Option<&Expr> {
    let Expr::Reference(reference) = syntax::unparen(expression) else {
        return None;
    };
    (reference.attrs.is_empty() && reference.mutability.is_none())
        .then(|| syntax::unparen(&reference.expr))
}

fn array(expression: &Expr) -> Option<&ExprArray> {
    let Expr::Array(array) = syntax::unparen(expression) else {
        return None;
    };
    array.attrs.is_empty().then_some(array)
}

impl InlineStops {
    fn read(
        array: &ExprArray,
        resolve: &impl Fn(&Expr) -> Option<String>,
        constants: &ConstantReader<'_>,
    ) -> Option<(Vec<AnimationGradientStop>, Self)> {
        let mut values = Vec::with_capacity(array.elems.len());
        let mut stops = Vec::with_capacity(array.elems.len());
        for expr in &array.elems {
            let Expr::Call(call) = expr else { return None };
            if !call.attrs.is_empty()
                || call.args.len() != 2
                || resolve(&call.func)?.as_str() != STOP.trim_start_matches("::")
            {
                return None;
            }
            let offset = constants.float(&call.args[0])?;
            let (color, source) = ColorSource::read(&call.args[1], false, resolve, constants)?;
            values.push(AnimationGradientStop {
                offset: offset.value,
                argb: color.argb8()?,
            });
            stops.push(StopSource {
                span: syntax::span(expr),
                offset,
                color: source,
            });
        }
        let result = Self {
            stops,
            array: syntax::span(array),
            text: array.span().source_text()?,
            end: array.bracket_token.span.close().byte_range().start,
        };
        Some((values, result))
    }

    fn plan(
        &self,
        original: &[AnimationGradientStop],
        next: &[AnimationGradientStop],
        patches: &mut Vec<TextPatch>,
    ) -> Result<(), String> {
        for ((source, original), next) in self.stops.iter().zip(original).zip(next) {
            source.offset.plan(next.offset, patches);
            source.color.plan(
                &AnimationValue::from_argb8(original.argb),
                &AnimationValue::from_argb8(next.argb),
                patches,
            )?;
        }
        if next.len() < self.stops.len() {
            let span = SourceSpan {
                start: self.stops[next.len()].span.start,
                end: self.end,
            };
            let text = self
                .text
                .get(span.start - self.array.start..span.end - self.array.start)
                .ok_or("The authored gradient stop range is unavailable")?;
            patches.push(TextPatch {
                span,
                replacement: keep_comments(text)?,
            });
        } else if next.len() > self.stops.len() {
            let at = self.stops.last().map_or(self.end, |stop| stop.span.end);
            let prefix = if self.stops.is_empty() { "" } else { ", " };
            let additions = next[self.stops.len()..]
                .iter()
                .map(stop_literal)
                .collect::<Vec<_>>();
            patches.push(TextPatch {
                span: SourceSpan { start: at, end: at },
                replacement: format!("{prefix}{}", additions.join(", ")),
            });
        }
        Ok(())
    }
}

fn stop_literal(stop: &AnimationGradientStop) -> String {
    format!("{STOP}({:?}, {})", stop.offset, color_literal(stop.argb))
}

pub(super) fn literal(stops: &[AnimationGradientStop]) -> String {
    let stops = stops.iter().map(stop_literal).collect::<Vec<_>>();
    format!("&[{}]", stops.join(", "))
}
