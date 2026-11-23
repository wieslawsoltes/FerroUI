//! Source spans for literal shadow lists. Keep the original Rust constructors.

#[path = "box_shadow_layer.rs"]
mod layer;

use super::{number_source::FloatReader, syntax::span};
use super::text_source::TextSource;
use ferroui_base::media::BoxShadows;
use ferroui_designer_animation::{AnimationBoxShadows, AnimationValue};
use ferroui_designer_core::TextPatch;
use layer::ShadowLayer;
use syn::{Expr, ExprCall};

const CLASS: &str = "ferroui_base::media::BoxShadows";
const MODULE_CLASS: &str = "ferroui_base::media::box_shadows::BoxShadows";
const FACTORIES: &[&str] = &["ferroui_base::media::BoxShadows::parse", "ferroui_base::media::box_shadows::BoxShadows::parse"];

#[derive(Clone, Debug)]
pub(super) struct BoxShadowsSource(ListSyntax);

#[derive(Clone, Debug)]
enum ListSyntax {
    Text(TextSource),
    Layers(Vec<ShadowLayer>),
}

impl BoxShadowsSource {
    pub(super) fn read(
        expression: &Expr,
        resolve: &impl Fn(&Expr) -> Option<String>,
        numbers: &FloatReader<'_>,
    ) -> Option<(AnimationBoxShadows, Self)> {
        if let Some((value, source)) = TextSource::parse_call(expression, resolve, FACTORIES) {
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
                let first = ShadowLayer::read(first, resolve, numbers)?;
                (BoxShadows::new(first.value.into()), vec![first])
            }
            ("with_rest", [first, Expr::Reference(rest)]) => {
                let Expr::Array(array) = rest.expr.as_ref() else {
                    return None;
                };
                if rest.mutability.is_some() || !rest.attrs.is_empty() || !array.attrs.is_empty() {
                    return None;
                }
                let first = ShadowLayer::read(first, resolve, numbers)?;
                let rest = array
                    .elems
                    .iter()
                    .map(|layer| ShadowLayer::read(layer, resolve, numbers))
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
