use super::{AnimationPrograms, ID_MARKER, NAME_MARKER, core_error};
use crate::{AnimationError, AnimationProgram, Result, Timeline, require};
use ferroui_designer_core::{Document, NodeId, NodeKind, TextPatch};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum AnimationClipEdit {
    Create {
        owner: NodeId,
        name: String,
        selector: String,
        duration: f64,
    },
    Rename {
        program: NodeId,
        name: String,
    },
    Duplicate {
        program: NodeId,
        name: String,
    },
    Remove {
        program: NodeId,
    },
}

impl AnimationClipEdit {
    pub fn program(&self) -> Option<NodeId> {
        match self {
            Self::Create { .. } => None,
            Self::Rename { program, .. }
            | Self::Duplicate { program, .. }
            | Self::Remove { program } => Some(*program),
        }
    }
}

pub struct AnimationClipPlan {
    pub patches: Vec<TextPatch>,
    pub selected_key: Option<String>,
}

impl AnimationPrograms {
    /// Plan a whole clip edit through the document transaction used by Studio.
    pub fn plan_clip_edit(
        document: &Document,
        command: &AnimationClipEdit,
    ) -> Result<AnimationClipPlan> {
        let edits = match command {
            AnimationClipEdit::Create {
                owner,
                name,
                selector,
                duration,
            } => Self::create(document, *owner, name, selector, &Timeline::new(*duration)?)?,
            AnimationClipEdit::Rename { program, name } => Self::rename(document, *program, name)?,
            AnimationClipEdit::Duplicate { program, name } => {
                Self::duplicate(document, *program, name)?
            }
            AnimationClipEdit::Remove { program } => Self::remove(document, *program)?,
        };
        let mut staged = document.clone();
        let change = staged
            .apply("Edit animation clip", edits)
            .map_err(core_error)?;
        let selected_key = if matches!(command, AnimationClipEdit::Remove { .. }) {
            None
        } else {
            let mut matches = Self::scan_hosted(&staged)
                .into_iter()
                .filter(|clip| match command {
                    AnimationClipEdit::Rename { program, .. } => clip.id == *program,
                    _ => change.inserted.contains(&clip.id),
                });
            let key = matches
                .next()
                .and_then(|clip| clip.source_key)
                .ok_or_else(|| AnimationError("The edited clip has no source identity".into()))?;
            require(
                matches.next().is_none(),
                "The edited clip identity is ambiguous",
            )?;
            Some(key)
        };
        let mut patches = change.patches;
        if document
            .source()
            .split_once('\n')
            .is_some_and(|(line, _)| line.ends_with('\r'))
        {
            for patch in &mut patches {
                patch.replacement = patch
                    .replacement
                    .replace("\r\n", "\n")
                    .replace('\n', "\r\n");
            }
        }
        Ok(AnimationClipPlan {
            patches,
            selected_key,
        })
    }

    /// Recognize a legacy rename and its undo across an identity stamp. Compare
    /// the exact source outside the declaration and the payload inside it.
    pub fn is_annotation_only_rename(
        before: &Document,
        program: &AnimationProgram,
        after: &Document,
        replacement: &AnimationProgram,
    ) -> bool {
        program.name != replacement.name
            && program.selector == replacement.selector
            && (program.source_key.is_none() || replacement.source_key.is_none())
            && before.source()[..program.span.start] == after.source()[..replacement.span.start]
            && before.source()[program.span.end..] == after.source()[replacement.span.end..]
            && without_annotations(before, program) == without_annotations(after, replacement)
    }
}

fn without_annotations(document: &Document, program: &AnimationProgram) -> String {
    let mut source = document.source()[program.span.range()].to_owned();
    let Some(node) = document.ast().node(program.id) else {
        return source;
    };
    let mut comments = node
        .children
        .iter()
        .filter_map(|id| document.ast().node(*id))
        .filter(|comment| {
            comment.kind == NodeKind::Comment
                && comment.text.as_deref().is_some_and(|text| {
                    let text = text.trim();
                    text.starts_with(NAME_MARKER) || text.starts_with(ID_MARKER)
                })
        })
        .map(|comment| comment.span)
        .collect::<Vec<_>>();
    comments.sort_unstable_by_key(|span| std::cmp::Reverse(span.start));
    for comment in comments {
        source.replace_range(
            comment.start - program.span.start..comment.end - program.span.start,
            "",
        );
    }
    if !node.self_closing && source.ends_with(&format!("></{}>", node.name)) {
        source.truncate(source.rfind("></").expect("checked closing tag"));
        source.push_str("/>");
    }
    source
}
