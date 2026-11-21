use crate::{
    animation::syntax::{method_suffix, path, span},
    blocks::keep_comments,
    literal::{self, Literal},
};
use ferroui_designer_core::{SourceSpan, TextPatch};
use ferroui_designer_tools::{ColumnField, GridLength};
use syn::{Expr, ExprMethodCall, Lit};

const ALIGNMENT: &str = "ferroui_base::layout::HorizontalAlignment";
const LENGTH: &str = "ferroui_controls::GridLength";
const UNIT: &str = "ferroui_controls::GridUnitType";

#[derive(Clone, Copy)]
enum ValueCodec {
    Text,
    Length,
    Boolean,
    Alignment,
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
const PROPERTIES: &[Property] = &[
    Property {
        name: "Header",
        setter: "header",
        encoding: Text,
    },
    Property {
        name: "Width",
        setter: "width",
        encoding: Length,
    },
    Property {
        name: "IsVisible",
        setter: "is_visible",
        encoding: Boolean,
    },
    Property {
        name: "CanUserResize",
        setter: "can_user_resize",
        encoding: Boolean,
    },
    Property {
        name: "HorizontalContentAlignment",
        setter: "horizontal_content_alignment",
        encoding: Alignment,
    },
    Property {
        name: "Binding",
        setter: "binding_path",
        encoding: BindingPath,
    },
    Property {
        name: "Binding",
        setter: "binding",
        encoding: Expression,
    },
    Property {
        name: "CellTemplate",
        setter: "cell_template",
        encoding: Expression,
    },
    Property {
        name: "HeaderTemplate",
        setter: "header_template",
        encoding: Expression,
    },
    Property {
        name: "CellTheme",
        setter: "cell_theme",
        encoding: Expression,
    },
    Property {
        name: "HeaderTheme",
        setter: "header_theme",
        encoding: Expression,
    },
];

pub(super) struct SourceField {
    property: Property,
    pub(super) value: Option<String>,
    argument: SourceSpan,
    method: SourceSpan,
}

impl SourceField {
    pub(super) fn read(method: &ExprMethodCall) -> Option<Self> {
        let property = *PROPERTIES
            .iter()
            .find(|property| method.method == property.setter)?;
        if method.args.len() != 1 {
            return None;
        }
        let expr = method.args.first()?;
        let value = property.encoding.read(expr);
        let argument = span(expr);
        let method = method_suffix(method);
        Some(Self {
            property,
            value,
            argument,
            method,
        })
    }

    pub(super) fn name(&self) -> &'static str {
        self.property.name
    }

    pub(super) fn render_value(&self) -> Option<String> {
        let value = self.value.as_ref()?;
        Some(match self.property.encoding {
            Text => literal::text_value(value),
            BindingPath => format!("{{ReflectionBinding {value}}}"),
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
    authored: Option<&SourceField>,
    end: usize,
    field: &ColumnField,
    value: Option<&str>,
) -> Result<Option<TextPatch>, String> {
    let Some(value) = value else {
        return match authored {
            None => Ok(None),
            Some(field) if field.value.is_some() => Ok(Some(TextPatch {
                span: field.method,
                replacement: keep_comments(&source[field.method.range()])?,
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
    let property = PROPERTIES
        .iter()
        .find(|property| property.name == field.name)
        .ok_or("Choose a mapped column property")?;
    let argument = property.encoding.rust(value, field.choices)?;
    if let Some(authored) = authored {
        if authored.value
            == property
                .encoding
                .read(&syn::parse_str::<Expr>(&argument).map_err(|error| error.to_string())?)
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
    fn read(self, expression: &Expr) -> Option<String> {
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
            Alignment => {
                let name = path(expression)?;
                let value = name.strip_prefix(&format!("{ALIGNMENT}::"))?;
                matches!(value, "Left" | "Center" | "Right" | "Stretch").then(|| value.into())
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
            Alignment => choices
                .iter()
                .find(|choice| choice.eq_ignore_ascii_case(value.trim()))
                .map(|choice| format!("::{ALIGNMENT}::{choice}"))
                .ok_or_else(|| "Choose a column alignment".into()),
            BindingPath if binding_path(value.trim()) => Ok(format!("{:?}", value.trim())),
            BindingPath => Err("Enter a row property path such as Account.Name".into()),
            Expression => Err("Edit this Rust expression in source".into()),
        }
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
    value.split('.').all(|component| {
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
