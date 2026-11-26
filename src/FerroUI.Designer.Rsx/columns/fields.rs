mod properties;

use crate::{
    animation::syntax::{method_suffix, path, span},
    blocks::keep_comments,
    item_templates::TemplateSource,
    literal::{self, Literal},
};
use ferroui_designer_core::{SourceSpan, TextPatch};
use ferroui_designer_tools::{ColumnField, ColumnKind, GridLength};
use syn::{Expr, ExprMethodCall, Lit};

const LENGTH: &str = "ferroui_controls::GridLength";
const UNIT: &str = "ferroui_controls::GridUnitType";

#[derive(Clone, Copy)]
enum ValueCodec {
    Text,
    Length,
    Boolean,
    Choice(&'static str),
    BindingPath,
    Expression,
}

#[derive(Clone, Copy)]
struct Property {
    name: &'static str,
    setter: &'static str,
    encoding: ValueCodec,
}

use ValueCodec::*;

pub(super) struct SourceField {
    property: Property,
    pub(super) value: Option<String>,
    argument: SourceSpan,
    method: SourceSpan,
    pub(super) template: Option<TemplateSource>,
}

impl SourceField {
    pub(super) fn read(
        kind: ColumnKind,
        method: &ExprMethodCall,
        scope: Option<crate::local_rust::RustSourceScope<'_>>,
    ) -> Option<Self> {
        let property = properties::of(kind).find(|property| method.method == property.setter)?;
        if method.args.len() != 1 {
            return None;
        }
        let expr = method.args.first()?;
        let value = property
            .encoding
            .read(expr, kind.field_choices(property.name));
        let argument = span(expr);
        let method = method_suffix(method);
        let template = matches!(property.name, "CellTemplate" | "CellEditingTemplate")
            .then(|| TemplateSource::read(expr, Some(method), scope));
        Some(Self {
            property,
            value,
            argument,
            method,
            template,
        })
    }

    pub(super) fn name(&self) -> &'static str {
        self.property.name
    }

    pub(super) fn render_value(&self) -> Option<String> {
        let value = self.value.as_ref()?;
        Some(match self.property.encoding {
            Text => literal::text_value(value),
            BindingPath => reflection_binding(value),
            _ => value.clone(),
        })
    }

    pub(super) fn describe(&self, field: &mut ColumnField) {
        field.value = self
            .value
            .clone()
            .or_else(|| Some("Rust expression".into()));
        field.editable = self.value.is_some();
        field.can_clear = field.editable;
        field.source = Some(self.argument);
    }
}

pub(super) fn change(
    source: &str,
    kind: ColumnKind,
    authored: Option<&SourceField>,
    end: usize,
    field: &ColumnField,
    value: Option<&str>,
) -> Result<Option<TextPatch>, String> {
    let Some(value) = value else {
        return match authored {
            None => Ok(None),
            Some(authored) if field.can_clear => Ok(Some(TextPatch {
                span: authored.method,
                replacement: keep_comments(&source[authored.method.range()])?,
            })),
            Some(_) => Err("Reveal this Rust expression to edit it in source".into()),
        };
    };
    if !field.editable {
        return Err("Reveal this Rust expression to edit it in source".into());
    }
    if value.len() > 4096 || !column_text(value) {
        return Err("Enter a single-line column value of up to 4096 bytes".into());
    }
    let property = properties::of(kind)
        .find(|property| property.name == field.name)
        .ok_or("Choose a mapped column property")?;
    let argument = property.encoding.rust(value, field.choices)?;
    if let Some(authored) = authored {
        if authored.value
            == property.encoding.read(
                &syn::parse_str::<Expr>(&argument).map_err(|error| error.to_string())?,
                field.choices,
            )
        {
            return Ok(None);
        }
        return Ok(Some(TextPatch {
            span: authored.argument,
            replacement: format!(
                "{}{argument}",
                keep_comments(&source[authored.argument.range()])?
            ),
        }));
    }
    Ok(Some(TextPatch {
        span: SourceSpan::new(end, end),
        replacement: format!(".{}({argument})", property.setter),
    }))
}

impl ValueCodec {
    fn read(self, expression: &Expr, choices: &[&str]) -> Option<String> {
        match self {
            Text | BindingPath => {
                let Expr::Lit(literal) = expression else {
                    return None;
                };
                let Lit::Str(text) = &literal.lit else {
                    return None;
                };
                let value = text.value();
                (if matches!(self, Text) {
                    column_text(&value)
                } else {
                    binding_path(&value)
                })
                .then_some(value)
            }
            Length => width(expression).map(GridLength::to_xaml),
            Boolean => Literal::Bool
                .read_rust(expression)
                .map(|value| if value == "true" { "True" } else { "False" }.into()),
            Choice(rust_type) => {
                let name = path(expression)?;
                let value = name.strip_prefix(&format!("{rust_type}::"))?;
                choices.contains(&value).then(|| value.into())
            }
            Expression => None,
        }
    }

    fn rust(self, value: &str, choices: &[&str]) -> Result<String, String> {
        match self {
            Text => Ok(format!("{value:?}")),
            Length => Ok(
                match GridLength::parse(value).map_err(|error| error.to_string())? {
                    GridLength::Auto => format!("::{LENGTH}::AUTO"),
                    GridLength::Pixels(pixels) => format!("::{LENGTH}::from_pixels({pixels:?})"),
                    GridLength::Star(stars) => {
                        format!("::{LENGTH}::new({stars:?}, ::{UNIT}::Star)")
                    }
                },
            ),
            Boolean => Literal::Bool
                .rust(value)
                .ok_or_else(|| "Choose True or False".into()),
            Choice(rust_type) => choices
                .iter()
                .find(|choice| choice.eq_ignore_ascii_case(value.trim()))
                .map(|choice| format!("::{rust_type}::{choice}"))
                .ok_or_else(|| "Choose a supported column value".into()),
            BindingPath if binding_path(value.trim()) => Ok(format!("{:?}", value.trim())),
            BindingPath => Err("Enter a row property path such as Account.Name".into()),
            Expression => Err("Edit this Rust expression in source".into()),
        }
    }
}

pub(super) fn reflection_binding(value: &str) -> String {
    if value.is_empty() {
        "{ReflectionBinding}".into()
    } else {
        format!("{{ReflectionBinding {value}}}")
    }
}

fn width(expression: &Expr) -> Option<GridLength> {
    let width = match expression {
        Expr::Path(_) => match path(expression)?.strip_prefix(&format!("{LENGTH}::"))? {
            "AUTO" => GridLength::Auto,
            "STAR" => GridLength::Star(1.0),
            _ => return None,
        },
        Expr::Call(call) => {
            let method = path(&call.func)?;
            let method = method.strip_prefix(&format!("{LENGTH}::"))?;
            let args = call.args.iter().collect::<Vec<_>>();
            let number = |expr| Literal::Number.read_rust(expr)?.parse::<f64>().ok();
            match (method, args.as_slice()) {
                ("auto", []) => GridLength::Auto,
                ("star", []) => GridLength::Star(1.0),
                ("from_pixels", [value]) => GridLength::Pixels(number(value)?),
                ("new", [value, unit]) => match path(unit)?.strip_prefix(&format!("{UNIT}::"))? {
                    "Auto" if number(value)? >= 0.0 => GridLength::Auto,
                    "Pixel" => GridLength::Pixels(number(value)?),
                    "Star" => GridLength::Star(number(value)?),
                    _ => return None,
                },
                _ => return None,
            }
        }
        _ => return None,
    };
    width.validate().ok()?;
    Some(width)
}

fn binding_path(value: &str) -> bool {
    value.is_empty()
        || value.split('.').all(|component| {
            let mut chars = component.chars();
            chars.next().is_some_and(|c| c.is_alphabetic() || c == '_')
                && chars.all(|c| c.is_alphanumeric() || c == '_')
        })
}

pub(super) fn column_text(value: &str) -> bool {
    !value
        .chars()
        .any(|c| c.is_control() || matches!(c, '\u{FFFE}' | '\u{FFFF}'))
}
