//! Plans in the original Rust buffer, validated by reprojecting the invocation.
use super::{string_literal, syntax::validate_selector, Deletion, RsxStyle};
use crate::{blocks::keep_comments, source_edits::line_prefix, RsxProjection};
use ferroui_designer_core::{SourceSpan, TextPatch};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum RsxStyleEdit {
    Create {
        owner: usize,
        selector: String,
    },
    CreateNested {
        style: usize,
        selector: String,
    },
    Selector {
        style: usize,
        selector: String,
    },
    Setter {
        style: usize,
        setter: usize,
        value: Option<String>,
    },
    AddSetter {
        style: usize,
        getter: String,
        value: String,
    },
    Remove {
        style: usize,
    },
}

impl RsxProjection<'_> {
    pub fn plan_style(&self, edit: &RsxStyleEdit) -> Result<Vec<TextPatch>, String> {
        let mut count = self.style_definitions().len();
        let mut patches = match edit {
            RsxStyleEdit::Create { owner, selector } => {
                let owner = self
                    .nodes
                    .get(*owner)
                    .ok_or("Choose a mapped Rust style owner")?;
                let selector = selector.trim();
                validate_selector(selector, false)?;
                count += 1;
                owner
                    .style_slot
                    .insert(self.source, owner.open, "styles", |_, _| {
                        style_factory(selector)
                    })?
            }
            RsxStyleEdit::CreateNested { style, selector } => {
                let style = self.style_at(*style)?;
                let selector = selector.trim();
                validate_selector(selector, true)?;
                count += 1;
                vec![insert(
                    style.declaration.end,
                    format!(".nested({})", style_factory(selector)),
                )]
            }
            RsxStyleEdit::Selector { style, selector } => {
                let style = self.style_at(*style)?;
                let selector = selector.trim();
                validate_selector(selector, style.deletion.span != style.declaration)?;
                if selector == style.selector {
                    return Ok(Vec::new());
                }
                let token = style.selector_span;
                vec![TextPatch {
                    span: token,
                    replacement: string_literal(&self.source[token.range()], selector),
                }]
            }
            RsxStyleEdit::Setter {
                style,
                setter,
                value,
            } => {
                let source = self
                    .style_at(*style)?
                    .setters
                    .iter()
                    .find(|member| member.declaration.start == *setter)
                    .ok_or("Choose an authored style setter")?;
                let (field, authored) = source
                    .field
                    .as_ref()
                    .zip(source.value.as_ref())
                    .ok_or("Open the Rust source to change this setter expression")?;
                if let Some(value) = value {
                    if *value == authored.text() {
                        return Ok(Vec::new());
                    }
                    authored.plan(field, self.source, value)?
                } else {
                    self.remove_style_part(&Deletion {
                        span: source.declaration,
                        comma: None,
                    })?
                }
            }
            RsxStyleEdit::AddSetter {
                style,
                getter,
                value,
            } => {
                let style = self.style_at(*style)?;
                let field = self
                    .catalog
                    .style_field(getter.trim_start_matches("::"))
                    .ok_or("Choose a registered writable Rust style property")?;
                if style
                    .setters
                    .iter()
                    .filter_map(|setter| setter.field.as_ref())
                    .any(|existing| existing.name == field.name && existing.owner == field.owner)
                {
                    return Err("Edit the existing setter for this registered property".into());
                }
                let getter = if field.getter.starts_with("crate::") {
                    field.getter.clone()
                } else {
                    format!("::{}", field.getter)
                };
                let value = field.rust(value)?;
                vec![insert(
                    style.declaration.end,
                    format!(".set({getter}(), {value})"),
                )]
            }
            RsxStyleEdit::Remove { style } => {
                let style = self.style_at(*style)?;
                if !style.can_remove() {
                    return Err(
                        "Reveal the style in Rust to remove its unmapped expressions".into(),
                    );
                }
                style.visit(&mut |_| count -= 1);
                self.remove_style_part(&style.deletion)?
            }
        };
        patches.retain(|patch| self.source.get(patch.span.range()) != Some(&patch.replacement));
        let source = TextPatch::apply_all(self.source, patches.clone())
            .map_err(|error| error.to_string())?;
        let draft = self.catalog.project(&source, self.invocation.start)?;
        if draft.invocation.start != self.invocation.start
            || draft.tree != self.tree
            || draft.style_definitions().len() != count
            || draft.style_issues.len() > self.style_issues.len()
            || draft.animation_issues.len() > self.animation_issues.len()
        {
            return Err("The style change does not round-trip in the active Rust view".into());
        }
        Ok(patches)
    }

    fn style_at(&self, node: usize) -> Result<&RsxStyle, String> {
        self.style_definitions()
            .into_iter()
            .find_map(|(_, style)| (style.declaration.start == node).then_some(style))
            .ok_or_else(|| "Choose a style from the current Rust invocation".into())
    }

    fn remove_style_part(&self, deletion: &Deletion) -> Result<Vec<TextPatch>, String> {
        let mut span = deletion.span;
        let replacement = keep_comments(&self.source[span.range()])?;
        if replacement.is_empty() {
            span.start -= line_prefix(self.source, Some(span.start)).len();
        }
        let mut patches = vec![TextPatch { span, replacement }];
        if let Some(span) = deletion.comma {
            patches.push(TextPatch {
                span,
                replacement: String::new(),
            });
        }
        Ok(patches)
    }
}

fn style_factory(selector: &str) -> String {
    format!("::ferroui_view::style({selector:?})")
}

fn insert(at: usize, replacement: String) -> TextPatch {
    TextPatch {
        span: SourceSpan::new(at, at),
        replacement,
    }
}
