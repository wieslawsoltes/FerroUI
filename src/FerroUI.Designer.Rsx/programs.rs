//! Source transactions on the inline styles of an accepted RSX invocation.
use crate::source_array::ArraySlot;

use crate::RsxProjection;
use crate::animation::program_comment::{AnimationAttachment, annotation_text};
use crate::animation::{RsxAnimation, syntax};
use crate::blocks::keep_comments;
use crate::source_edits::line_prefix;
use ferroui_designer_animation::{AnimationProgramIdentity, Timeline};
use ferroui_designer_core::{SourceSpan, TextPatch};

pub enum RsxAnimationEdit<'a> {
    Rename(&'a str),
    Duplicate(&'a str),
    Remove,
}

/// A checked patch in original Rust coordinates and the stable identity to select after it.
pub struct RsxAnimationPlan {
    pub patches: Vec<TextPatch>,
    pub selected_key: Option<String>,
}

impl RsxProjection<'_> {
    pub fn can_create_animation(&self, owner: usize) -> bool {
        self.nodes
            .get(owner)
            .is_some_and(|node| !matches!(node.style_slot, ArraySlot::Unavailable))
    }

    pub fn create_animation(
        &self,
        owner: usize,
        name: &str,
        selector: &str,
        duration: f64,
    ) -> Result<RsxAnimationPlan, String> {
        let node = self
            .nodes
            .get(owner)
            .ok_or("Choose a mapped Rust control to own the animation")?;
        if !syntax::valid_selector(selector) {
            return Err("Enter a native style selector".into());
        }
        Timeline::new(duration).map_err(|error| error.to_string())?;
        let millis = (duration * 1000.0).round();
        if millis == 0.0 || millis / 1000.0 != duration {
            return Err("Use a positive duration in whole milliseconds for Rust animations".into());
        }
        let identity = identity(name, None)?;
        let label = annotation_text(&identity);
        let patches = node.style_slot.insert(self.source, node.open, "styles", |indent, eol| format!(
            "ferroui_view::style({selector:?}).animation({eol}{indent}    {label}{eol}{indent}    ferroui_view::animation(ferroui_view::ms({millis:.0})){eol}{indent})"
        ))?;
        self.check_animation_plan(patches, Some(identity.key()), 1)
    }

    pub fn edit_animation(
        &self,
        declaration: SourceSpan,
        edit: RsxAnimationEdit<'_>,
    ) -> Result<RsxAnimationPlan, String> {
        let attachment = self
            .clips()
            .find(|clip| clip.declaration == declaration)
            .and_then(|clip| clip.attachment.as_ref())
            .ok_or("Choose an animation declared inline on a mapped Rust style")?;
        match edit {
            RsxAnimationEdit::Remove => {
                let patches = remove_attachment(self.source, attachment)?;
                self.check_animation_plan(patches, None, -1)
            }
            RsxAnimationEdit::Rename(name) => {
                let key = attachment
                    .identity()
                    .map(AnimationProgramIdentity::key)
                    .filter(|key| {
                        self.clips()
                            .filter(|clip| clip_key(clip).as_ref() == Some(key))
                            .count()
                            == 1
                    });
                let identity = identity(name, key.as_deref())?;
                let patch = attachment.name_patch(&identity);
                self.check_animation_plan(vec![patch], Some(identity.key()), 0)
            }
            RsxAnimationEdit::Duplicate(name) => {
                let identity = identity(name, None)?;
                let method = attachment.method;
                let mut copied = self.source[method.range()].to_owned();
                let patch = attachment.name_patch(&identity);
                copied.replace_range(
                    patch.span.start - method.start..patch.span.end - method.start,
                    &patch.replacement,
                );
                let prefix = line_prefix(self.source, Some(method.start));
                let patch = TextPatch {
                    span: SourceSpan::new(method.end, method.end),
                    replacement: format!("{prefix}{copied}"),
                };
                self.check_animation_plan(vec![patch], Some(identity.key()), 1)
            }
        }
    }

    fn clips(&self) -> impl Iterator<Item = &RsxAnimation> {
        self.styles
            .values()
            .flatten()
            .flat_map(|style| &style.clips)
    }

    fn check_animation_plan(
        &self,
        mut patches: Vec<TextPatch>,
        selected_key: Option<String>,
        change: isize,
    ) -> Result<RsxAnimationPlan, String> {
        patches.retain(|patch| {
            self.source.get(patch.span.range()) != Some(patch.replacement.as_str())
        });
        let candidate = TextPatch::apply_all(self.source, patches.clone())
            .map_err(|error| error.to_string())?;
        let draft = self.catalog.project(&candidate, self.invocation.start)?;
        if draft.tree != self.tree
            || self.clips().count().checked_add_signed(change) != Some(draft.clips().count())
            || selected_key.as_ref().is_some_and(|key| {
                draft
                    .clips()
                    .filter(|clip| clip_key(clip).as_ref() == Some(key))
                    .count()
                    != 1
            })
        {
            return Err("The animation change does not round-trip in the active Rust view".into());
        }
        Ok(RsxAnimationPlan {
            patches,
            selected_key,
        })
    }
}

fn identity(name: &str, key: Option<&str>) -> Result<AnimationProgramIdentity, String> {
    AnimationProgramIdentity::new(name, key).map_err(|error| error.to_string())
}

fn clip_key(clip: &RsxAnimation) -> Option<String> {
    Some(clip.attachment.as_ref()?.identity()?.key())
}

fn remove_attachment(
    source: &str,
    attachment: &AnimationAttachment,
) -> Result<Vec<TextPatch>, String> {
    let (mut span, comma) = attachment.lone_style.unwrap_or((attachment.method, None));
    let mut removed = source[span.range()].to_owned();
    if let Some((annotation, _)) = &attachment.annotation {
        removed.replace_range(
            annotation.start - span.start..annotation.end - span.start,
            "",
        );
    }
    let replacement = keep_comments(&removed)?;
    if replacement.is_empty() {
        let prefix = line_prefix(source, Some(span.start));
        span.start -= prefix.len();
    }
    let mut patches = vec![TextPatch { span, replacement }];
    if let Some(comma) = comma {
        patches.push(TextPatch {
            span: comma,
            replacement: String::new(),
        });
    }
    Ok(patches)
}
