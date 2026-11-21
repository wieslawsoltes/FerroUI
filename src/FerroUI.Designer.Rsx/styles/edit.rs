//! Plans in the original Rust buffer, validated by reprojecting the invocation.
use super::{string_literal, syntax::validate_selector, Deletion, RsxStyle, RsxStyleField};
use crate::{blocks::keep_comments, source_edits::line_prefix, RsxProjection};
use ferroui_designer_core::{SourceSpan, TextPatch};
use ferroui_designer_tools::component_palette::PaletteLayer;
use serde::Deserialize;

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResourceColorMode {
    #[default]
    Literal,
    Declaration,
    Local,
}

pub struct RsxStylePlan {
    pub patches: Vec<TextPatch>,
    pub shared_color: Option<SourceSpan>,
}

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
    SetResource {
        style: usize,
        key: String,
        setter: Option<usize>,
        getter: Option<String>,
    },
    AddColorResource {
        style: usize,
        key: String,
        value: String,
        layer: Option<PaletteLayer>,
    },
    ColorResource {
        style: usize,
        resource: usize,
        value: String,
        #[serde(default)]
        mode: ResourceColorMode,
    },
    Remove {
        style: usize,
    },
}

impl RsxStyleEdit {
    pub fn target(&self) -> usize {
        match self {
            Self::Create { owner, .. } => *owner,
            Self::CreateNested { style, .. }
            | Self::Selector { style, .. }
            | Self::Setter { style, .. }
            | Self::AddSetter { style, .. }
            | Self::SetResource { style, .. }
            | Self::AddColorResource { style, .. }
            | Self::ColorResource { style, .. }
            | Self::Remove { style } => *style,
        }
    }
}

impl RsxProjection<'_> {
    pub fn plan_style(&self, edit: &RsxStyleEdit) -> Result<Vec<TextPatch>, String> {
        self.plan_style_source(edit).map(|plan| plan.patches)
    }

    pub fn plan_style_source(&self, edit: &RsxStyleEdit) -> Result<RsxStylePlan, String> {
        let mut count = self.style_definitions().len();
        let mut shared_color = None;
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
                    return Ok(RsxStylePlan { patches: Vec::new(), shared_color: None });
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
                        return Ok(RsxStylePlan { patches: Vec::new(), shared_color: None });
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
                let field = self.new_style_field(style, getter)?;
                let getter = qualified_getter(field);
                let value = field.rust(value)?;
                vec![insert(
                    style.declaration.end,
                    format!(".set({getter}(), {value})"),
                )]
            }
            RsxStyleEdit::SetResource { style, key, setter, getter } => {
                self.plan_resource_setter(self.style_at(*style)?, key, *setter, getter.as_deref())?
            }
            RsxStyleEdit::AddColorResource { style, key, value, layer } => {
                self.add_style_color(self.style_at(*style)?, key, value, layer.unwrap_or(PaletteLayer::Shared))?
            }
            RsxStyleEdit::ColorResource { style, resource, value, mode } => {
                let plan = self.change_style_color(self.style_at(*style)?, *resource, value, *mode)?;
                shared_color = plan.shared_color;
                plan.patches
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
        let start = self.invocation_start_after(&patches)?;
        let draft = self.catalog.project(&source, start)?;
        if draft.invocation.start != start
            || draft.tree != self.tree
            || draft.style_definitions().len() != count
            || draft.style_issues.len() > self.style_issues.len()
            || draft.animation_issues.len() > self.animation_issues.len()
        {
            return Err("The style change does not round-trip in the active Rust view".into());
        }
        Ok(RsxStylePlan { patches, shared_color })
    }

    fn style_at(&self, node: usize) -> Result<&RsxStyle, String> {
        self.style_definitions()
            .into_iter()
            .find_map(|(_, style)| (style.declaration.start == node).then_some(style))
            .ok_or_else(|| "Choose a style from the current Rust invocation".into())
    }

    pub(super) fn new_style_field(&self, style: &RsxStyle, getter: &str) -> Result<&RsxStyleField, String> {
        let field = self.catalog.style_field(getter.trim_start_matches("::"))
            .filter(|field| field.insertable)
            .ok_or("Choose a registered writable Rust style property")?;
        if style.has_setter(field) {
            return Err("Edit the existing setter for this registered property".into());
        }
        Ok(field)
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

pub(super) fn qualified_getter(field: &RsxStyleField) -> String {
    if field.getter.starts_with("crate::") { field.getter.clone() } else { format!("::{}", field.getter) }
}

pub(super) fn insert(at: usize, replacement: String) -> TextPatch {
    TextPatch {
        span: SourceSpan::new(at, at),
        replacement,
    }
}
