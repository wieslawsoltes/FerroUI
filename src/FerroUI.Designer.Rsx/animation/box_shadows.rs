//! Source spans for literal shadow lists. Keep the original Rust constructors.

#[path = "box_shadow_layer.rs"]
mod layer;

use super::syntax::span;
use crate::literal::string_literal;
use ferroui_base::media::BoxShadows;
use ferroui_designer_animation::{AnimationBoxShadows, AnimationValue};
use ferroui_designer_core::{SourceSpan, TextPatch};
use layer::ShadowLayer;
use syn::{Expr, ExprCall, Lit};

const CLASS: &str = "ferroui_base::media::BoxShadows";
const MODULE_CLASS: &str = "ferroui_base::media::box_shadows::BoxShadows";

#[derive(Clone, Debug)]
pub(super) struct BoxShadowsSource(ListSyntax);

#[derive(Clone, Debug)]
enum ListSyntax {
    Text(ShadowText),
    Layers(Vec<ShadowLayer>),
}

impl BoxShadowsSource {
    pub(super) fn read(
        expression: &Expr,
        resolve: &impl Fn(&Expr) -> Option<String>,
    ) -> Option<(AnimationBoxShadows, Self)> {
        if let Some((value, source)) = ShadowText::read(expression, resolve, CLASS, MODULE_CLASS) {
            return Some((
                AnimationBoxShadows::parse(&value).ok()?,
                Self(ListSyntax::Text(source)),
            ));
        }
        let Expr::Call(call) = expression else {
            return None;
        };
        let (method, args) = constructor(call, resolve, CLASS, MODULE_CLASS)?;
        let (value, layers) = match (method.as_str(), args.as_slice()) {
            ("default", []) => (BoxShadows::default(), Vec::new()),
            ("new", [first]) => {
                let first = ShadowLayer::read(first, resolve)?;
                (BoxShadows::new(first.value.into()), vec![first])
            }
            ("with_rest", [first, Expr::Reference(rest)]) => {
                let Expr::Array(array) = rest.expr.as_ref() else {
                    return None;
                };
                if rest.mutability.is_some() || !rest.attrs.is_empty() || !array.attrs.is_empty() {
                    return None;
                }
                let first = ShadowLayer::read(first, resolve)?;
                let rest = array
                    .elems
                    .iter()
                    .map(|layer| ShadowLayer::read(layer, resolve))
                    .collect::<Option<Vec<_>>>()?;
                let values = rest
                    .iter()
                    .map(|layer| layer.value.into())
                    .collect::<Vec<_>>();
                let value = BoxShadows::with_rest(first.value.into(), &values);
                (value, std::iter::once(first).chain(rest).collect())
            }
            _ => return None,
        };
        let value = AnimationBoxShadows::from_native(&value);
        AnimationValue::BoxShadows(value.clone()).validate().ok()?;
        Some((value, Self(ListSyntax::Layers(layers))))
    }

    pub(super) fn plan(
        &self,
        previous: &AnimationBoxShadows,
        value: &AnimationBoxShadows,
        patches: &mut Vec<TextPatch>,
    ) -> Result<(), String> {
        if previous == value {
            return Ok(());
        }
        match &self.0 {
            ListSyntax::Text(source) => source.plan(&value.to_xaml(), patches),
            ListSyntax::Layers(layers) => {
                if layers.len() != value.layers().len() {
                    return Err("Change the number of shadow layers in Rust or use a BoxShadows::parse literal".into());
                }
                for (source, &layer) in layers.iter().zip(value.layers()) {
                    source.plan(layer, patches)?;
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
struct ShadowText {
    literal: SourceSpan,
    spelling: String,
}

impl ShadowText {
    fn read(
        expression: &Expr,
        resolve: &impl Fn(&Expr) -> Option<String>,
        class: &str,
        module_class: &str,
    ) -> Option<(String, Self)> {
        let call = match expression {
            Expr::MethodCall(method)
                if method.attrs.is_empty()
                    && method.turbofish.is_none()
                    && ((method.method == "unwrap" && method.args.is_empty())
                        || (method.method == "expect" && method.args.len() == 1)) =>
            {
                method.receiver.as_ref()
            }
            Expr::Try(operation) if operation.attrs.is_empty() => operation.expr.as_ref(),
            _ => return None,
        };
        let Expr::Call(call) = call else { return None };
        let (method, args) = constructor(call, resolve, class, module_class)?;
        let [Expr::Lit(literal)] = args.as_slice() else {
            return None;
        };
        let Lit::Str(text) = &literal.lit else {
            return None;
        };
        if method != "parse" || !literal.attrs.is_empty() {
            return None;
        }
        Some((
            text.value(),
            Self {
                literal: span(text),
                spelling: text.token().to_string(),
            },
        ))
    }

    fn plan(&self, text: &str, patches: &mut Vec<TextPatch>) {
        patches.push(TextPatch {
            span: self.literal,
            replacement: string_literal(&self.spelling, text),
        });
    }
}

fn constructor<'a>(
    call: &'a ExprCall,
    resolve: &impl Fn(&Expr) -> Option<String>,
    class: &str,
    module_class: &str,
) -> Option<(String, Vec<&'a Expr>)> {
    if !call.attrs.is_empty() {
        return None;
    }
    let path = resolve(&call.func)?;
    let (owner, method) = path.rsplit_once("::")?;
    (owner == class || owner == module_class)
        .then(|| (method.to_owned(), call.args.iter().collect()))
}
