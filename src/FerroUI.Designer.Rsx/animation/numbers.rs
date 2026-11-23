use super::{FloatSource, ValueShape, syntax};
use super::constant_source::ConstantReader;
use ferroui_designer_animation::{AnimationValue, ValueKind};
use ferroui_designer_core::{SourceSpan, TextPatch};
use syn::Expr;

#[derive(Clone, Debug)]
pub(super) struct Numbers {
    parts: Vec<FloatSource>,
    shorthand: Option<Shorthand>,
}

#[derive(Clone, Debug)]
struct Shorthand {
    fields: [usize; 4],
    ends: Vec<usize>,
    constructor: Constructor,
}

#[derive(Clone, Debug)]
enum Constructor {
    Call(SourceSpan),
    Scalar { at: usize, name: &'static str },
}

impl Numbers {
    pub(super) fn read(
        expression: &Expr,
        kind: ValueKind,
        resolve: &impl Fn(&Expr) -> Option<String>,
        constants: &ConstantReader<'_>,
    ) -> Option<(AnimationValue, Self)> {
        let uniform = [0; 4];
        let (args, shorthand) = if kind == ValueKind::Number {
            (vec![expression], None)
        } else if let Expr::Call(call) = expression {
            let path = resolve(&call.func)?;
            let (owner, method) = path.rsplit_once("::")?;
            if !call.attrs.is_empty()
                || ValueShape::from_type(&format!("::{owner}"))?.kind() != kind
            {
                return None;
            }
            let (count, fields) = match (method, kind) {
                ("uniform", ValueKind::CornerRadius | ValueKind::Thickness) => (1, Some(uniform)),
                ("top_bottom", ValueKind::CornerRadius) => (2, Some([0, 0, 1, 1])),
                ("symmetric", ValueKind::Thickness) => (2, Some([0, 1, 0, 1])),
                ("new", ValueKind::Point | ValueKind::Vector | ValueKind::Size) => (2, None),
                ("new", ValueKind::CornerRadius | ValueKind::Thickness | ValueKind::Rect) => {
                    (4, None)
                }
                _ => return None,
            };
            if call.args.len() != count {
                return None;
            }
            let shorthand = if let Some(fields) = fields {
                let Expr::Path(factory) = call.func.as_ref() else {
                    return None;
                };
                Some(Shorthand {
                    fields,
                    ends: call.args.iter().map(|arg| syntax::span(arg).end).collect(),
                    constructor: Constructor::Call(syntax::span(
                        &factory.path.segments.last()?.ident,
                    )),
                })
            } else {
                None
            };
            (call.args.iter().collect(), shorthand)
        } else {
            let name = match kind {
                ValueKind::CornerRadius => "CornerRadius",
                ValueKind::Thickness => "Thickness",
                _ => return None,
            };
            let span = syntax::span(expression);
            (
                vec![expression],
                Some(Shorthand {
                    fields: uniform,
                    ends: vec![span.end],
                    constructor: Constructor::Scalar {
                        at: span.start,
                        name,
                    },
                }),
            )
        };
        let parts = args
            .iter()
            .map(|expr| constants.float(expr))
            .collect::<Option<Vec<_>>>()?;
        let values = match &shorthand {
            Some(short) => short.fields.map(|part| parts[part].value).to_vec(),
            None => parts.iter().map(|part| part.value).collect(),
        };
        let text = values
            .iter()
            .map(f64::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let value = AnimationValue::parse(&text, kind).ok()?;
        Some((value, Self { parts, shorthand }))
    }

    pub(super) fn plan(&self, next: &[f64], patches: &mut Vec<TextPatch>) -> Result<(), String> {
        if let Some(short) = &self.shorthand {
            let fields: &[f64; 4] = next.try_into().map_err(|_| "Choose four geometry values")?;
            short.plan(&self.parts, fields, patches);
            return Ok(());
        }
        if self.parts.len() != next.len() {
            return Err("Keep the Rust keyframe's numeric value type".into());
        }
        for (part, next) in self.parts.iter().zip(next) {
            part.plan(*next, patches);
        }
        Ok(())
    }
}

impl Shorthand {
    fn plan(&self, args: &[FloatSource], next: &[f64; 4], patches: &mut Vec<TextPatch>) {
        // The first field of each original argument keeps its spelling and comments.
        let retained = (0..args.len())
            .map(|arg| {
                self.fields
                    .iter()
                    .position(|&field| field == arg)
                    .expect("each shorthand argument has a field")
            })
            .collect::<Vec<_>>();
        let expand = self
            .fields
            .iter()
            .zip(next)
            .any(|(&arg, &value)| next[retained[arg]] != value);
        if expand {
            let (span, replacement) = match self.constructor {
                Constructor::Call(name) => (name, "new".into()),
                Constructor::Scalar { at, name } => (
                    SourceSpan::new(at, at),
                    format!("::ferroui_base::{name}::new("),
                ),
            };
            patches.push(TextPatch { span, replacement });
        }
        for (i, (part, &field)) in args.iter().zip(&retained).enumerate() {
            part.plan(next[field], patches);
            if !expand {
                continue;
            }
            let end = retained.get(i + 1).copied().unwrap_or(next.len());
            let mut replacement = next[field + 1..end]
                .iter()
                .map(|value| format!(", {value:?}"))
                .collect::<String>();
            if matches!(self.constructor, Constructor::Scalar { .. }) {
                replacement.push(')');
            }
            if !replacement.is_empty() {
                let at = self.ends[i];
                patches.push(TextPatch {
                    span: SourceSpan::new(at, at),
                    replacement,
                });
            }
        }
    }
}
