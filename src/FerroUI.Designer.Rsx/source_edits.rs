use crate::{RsxProjection, catalog};
use ferroui_designer_core::{SourceSpan, TextPatch};
use ferroui_view_syntax::snake_case;

#[derive(Clone, Copy, Debug)]
pub struct RsxEvent {
    pub value: SourceSpan,
    pub(crate) field: Option<SourceSpan>,
}

impl RsxEvent {
    pub fn editable(self) -> bool {
        self.field.is_some()
    }
}

pub fn event_setter(name: &str) -> String {
    format!("on_{}", snake_case(name))
}

impl RsxProjection<'_> {
    pub fn edit_property(
        &self,
        index: usize,
        name: &str,
        value: Option<&str>,
    ) -> Result<TextPatch, String> {
        let node = self.nodes.get(index).ok_or("Unknown RSX element")?;
        if node.expressions.contains_key(name) {
            return Err(format!("{name} is a Rust expression; edit it in source"));
        }
        if let Some(value) = value {
            if node.properties.contains_key(name) {
                return self.set_property(index, name, value);
            }
            let replacement = catalog::property(&node.class, name)
                .and_then(|kind| kind.rust(value))
                .ok_or_else(|| format!("{value:?} is outside the static RSX values for {name}"))?;
            return Ok(insert_field(
                self.source,
                node.open,
                &format!("{}: {replacement}", catalog::setter(name)),
            ));
        }
        let span = node
            .fields
            .get(name)
            .ok_or("Select an authored literal property")?;
        Ok(remove_field(self.source, *span))
    }

    pub fn set_event(
        &self,
        index: usize,
        name: &str,
        value: Option<&str>,
    ) -> Result<TextPatch, String> {
        let node = self.nodes.get(index).ok_or("Unknown RSX element")?;
        let setter = event_setter(name);
        let existing = node.events.get(&setter);
        if existing.is_some_and(|event| !event.editable()) {
            return Err("Edit this Rust handler in source to preserve its builder call".into());
        }
        if let Some(value) = value {
            let value = value.trim();
            syn::parse_str::<syn::Expr>(value)
                .map_err(|error| format!("Invalid handler expression: {error}"))?;
            return Ok(if let Some(event) = existing {
                TextPatch {
                    span: event.value,
                    replacement: value.into(),
                }
            } else {
                insert_field(self.source, node.open, &format!("{setter}: {value}"))
            });
        }
        let span = existing
            .and_then(|event| event.field)
            .ok_or("Select an authored Rust handler")?;
        Ok(remove_field(self.source, span))
    }
}

fn insert_field(source: &str, offset: usize, field: &str) -> TextPatch {
    let following = &source[offset..];
    let rest = following.trim_start_matches([' ', '\t']);
    let replacement = if rest.starts_with(['\n', '\r']) {
        let eol = if rest.starts_with("\r\n") {
            "\r\n"
        } else {
            "\n"
        };
        let indent = line_indent(source, offset);
        format!("{eol}{indent}    {field},")
    } else {
        format!(" {field},{}", if following == rest { " " } else { "" })
    };
    TextPatch {
        span: SourceSpan::new(offset, offset),
        replacement,
    }
}

pub(crate) fn remove_field(source: &str, span: SourceSpan) -> TextPatch {
    let following = &source[span.end..];
    let rest = following.trim_start_matches([' ', '\t']);
    let end = if rest.starts_with(',') {
        span.end + following.len() - rest.len() + 1
    } else {
        span.end
    };
    let before = source[..span.start].rsplit('\n').next().unwrap_or("");
    let after = &source[end..];
    let suffix = after.trim_start_matches([' ', '\t']);
    let span = if before.trim().is_empty() && suffix.starts_with(['\n', '\r']) {
        SourceSpan::new(
            span.start - before.len(),
            end + after.len() - suffix.len() + if suffix.starts_with("\r\n") { 2 } else { 1 },
        )
    } else {
        SourceSpan::new(span.start, end)
    };
    TextPatch {
        span,
        replacement: String::new(),
    }
}

pub(crate) fn line_indent(source: &str, offset: usize) -> &str {
    let line = source[..offset].rsplit('\n').next().unwrap_or("");
    &line[..line.len() - line.trim_start_matches([' ', '\t']).len()]
}
