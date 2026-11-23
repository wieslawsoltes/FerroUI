use super::{TextSource, constructor, span};
use crate::animation::{FloatSource, color::ColorSource, number_source::FloatReader};
use ferroui_base::media::BoxShadow;
use ferroui_designer_animation::{AnimationBoxShadow, AnimationValue};
use ferroui_designer_core::{SourceSpan, TextPatch};
use syn::{Expr, ExprStruct, Lit, Member};

const CLASS: &str = "ferroui_base::media::BoxShadow";
const MODULE_CLASS: &str = "ferroui_base::media::box_shadow::BoxShadow";
const FACTORIES: &[&str] = &["ferroui_base::media::BoxShadow::parse", "ferroui_base::media::box_shadow::BoxShadow::parse"];
const NUMBERS: [&str; 4] = ["offset_x", "offset_y", "blur", "spread"];

#[derive(Clone, Debug)]
pub(super) struct ShadowLayer {
    pub(super) value: AnimationBoxShadow,
    source: LayerSyntax,
}

#[derive(Clone, Debug)]
enum LayerSyntax {
    Text(TextSource),
    Fields(Box<Fields>),
}

#[derive(Clone, Debug, Default)]
struct Fields {
    numbers: [Option<FloatSource>; 4],
    color: Option<ColorSource>,
    inset: Option<SourceSpan>,
    // New explicit fields can be inserted immediately before the update base.
    defaults: Option<SourceSpan>,
}

impl ShadowLayer {
    pub(super) fn read(
        expr: &Expr,
        resolve: &impl Fn(&Expr) -> Option<String>,
        numbers: &FloatReader<'_>,
    ) -> Option<Self> {
        if let Some((value, source)) = TextSource::parse_call(expr, resolve, FACTORIES) {
            let value = BoxShadow::parse(&value).ok()?.into();
            return Some(Self {
                value,
                source: LayerSyntax::Text(source),
            });
        }
        let Expr::Struct(layer) = expr else {
            return None;
        };
        let (value, fields) = Fields::read(layer, resolve, numbers)?;
        Some(Self {
            value,
            source: LayerSyntax::Fields(Box::new(fields)),
        })
    }

    pub(super) fn plan(
        &self,
        next: AnimationBoxShadow,
        patches: &mut Vec<TextPatch>,
    ) -> Result<(), String> {
        if self.value == next {
            return Ok(());
        }
        match &self.source {
            LayerSyntax::Text(source) => source.plan(&BoxShadow::from(next).to_string(), patches),
            LayerSyntax::Fields(fields) => fields.plan(self.value, next, patches)?,
        }
        Ok(())
    }
}

impl Fields {
    fn read(
        layer: &ExprStruct,
        resolve: &impl Fn(&Expr) -> Option<String>,
        reader: &FloatReader<'_>,
    ) -> Option<(AnimationBoxShadow, Self)> {
        if !layer.attrs.is_empty() || layer.qself.is_some() {
            return None;
        }
        let owner = Expr::Path(syn::ExprPath {
            attrs: vec![],
            qself: None,
            path: layer.path.clone(),
        });
        if !matches!(resolve(&owner)?.as_str(), CLASS | MODULE_CLASS) {
            return None;
        }
        let defaults = match (&layer.rest, &layer.dot2_token) {
            (None, None) if layer.fields.len() == 6 => None,
            (Some(base), Some(dots)) => {
                let Expr::Call(call) = base.as_ref() else {
                    return None;
                };
                let (method, args) = constructor(call, resolve, CLASS, MODULE_CLASS)?;
                if method != "default" || !args.is_empty() {
                    return None;
                }
                let start = span(dots).start;
                Some(SourceSpan { start, end: start })
            }
            _ => return None,
        };
        let mut fields = Self {
            defaults,
            ..Self::default()
        };
        let mut value = AnimationBoxShadow::from(BoxShadow::default());
        let numbers = [
            &mut value.offset_x,
            &mut value.offset_y,
            &mut value.blur,
            &mut value.spread,
        ];
        for field in &layer.fields {
            let Member::Named(name) = &field.member else {
                return None;
            };
            if !field.attrs.is_empty() || field.colon_token.is_none() {
                return None;
            }
            let name = name.to_string();
            match name.as_str() {
                "color" if fields.color.is_none() => {
                    let (color, source) = ColorSource::read_with(&field.expr, false, resolve)?;
                    value.argb = color.argb8()?;
                    fields.color = Some(source);
                }
                "is_inset" if fields.inset.is_none() => {
                    let Expr::Lit(literal) = &field.expr else {
                        return None;
                    };
                    let Lit::Bool(inset) = &literal.lit else {
                        return None;
                    };
                    if !literal.attrs.is_empty() {
                        return None;
                    }
                    value.inset = inset.value;
                    fields.inset = Some(span(inset));
                }
                name => {
                    let index = NUMBERS.iter().position(|candidate| *candidate == name)?;
                    let number = reader.read(&field.expr)?;
                    *numbers[index] = number.value;
                    if fields.numbers[index].replace(number).is_some() {
                        return None;
                    }
                }
            }
        }
        Some((value, fields))
    }

    fn plan(
        &self,
        before: AnimationBoxShadow,
        next: AnimationBoxShadow,
        patches: &mut Vec<TextPatch>,
    ) -> Result<(), String> {
        let mut added = String::new();
        let previous_numbers = [before.offset_x, before.offset_y, before.blur, before.spread];
        let next_numbers = [next.offset_x, next.offset_y, next.blur, next.spread];
        for (index, name) in NUMBERS.into_iter().enumerate() {
            if let Some(source) = &self.numbers[index] {
                source.plan(next_numbers[index], patches);
            } else if previous_numbers[index] != next_numbers[index] {
                added.push_str(&format!("{name}: {:?}, ", next_numbers[index]));
            }
        }
        if before.argb != next.argb {
            let before = AnimationValue::from_argb8(before.argb);
            let color = AnimationValue::from_argb8(next.argb);
            if let Some(source) = &self.color {
                source.plan(&before, &color, patches)?;
            } else {
                let [a, r, g, b] = next.argb;
                added.push_str(&format!(
                    "color: ::ferroui_base::media::Color::from_argb({a}, {r}, {g}, {b}), "
                ));
            }
        }
        if before.inset != next.inset {
            if let Some(span) = self.inset {
                patches.push(TextPatch {
                    span,
                    replacement: next.inset.to_string(),
                });
            } else {
                added.push_str(&format!("is_inset: {}, ", next.inset));
            }
        }
        if !added.is_empty() {
            let span = self
                .defaults
                .ok_or("The Rust shadow fields no longer match the key")?;
            patches.push(TextPatch {
                span,
                replacement: added,
            });
        }
        Ok(())
    }
}
