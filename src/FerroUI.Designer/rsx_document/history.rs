use super::{invalid, RustDocument};
use ferroui_designer_core::{ChangeSet, DesignerError, Document, NodeId};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone)]
pub(super) struct RustRevision {
    source: Arc<str>,
    projection: Option<(usize, Document)>,
}

pub(super) enum ProjectedIdentity<'a> {
    Edit {
        expected: &'a str,
        basis: &'a Document,
        locations: &'a BTreeMap<NodeId, usize>,
    },
    History(&'a Document),
}

impl ProjectedIdentity<'_> {
    pub(super) fn apply(
        self,
        candidate: &mut Document,
        label: &str,
        markup: String,
    ) -> Result<ChangeSet, DesignerError> {
        match self {
            Self::Edit {
                expected,
                basis,
                locations,
            } if expected == markup => candidate.apply_projection(
                candidate.revision(),
                label,
                markup,
                basis,
                locations.iter().map(|(&node, &start)| (node, start)),
            ),
            Self::Edit { .. } => Err(invalid(
                "The checked Rust projection changed before applying the edit",
            )),
            Self::History(basis) if markup == basis.source() => {
                candidate.apply_projection(candidate.revision(), label, markup, basis, [])
            }
            Self::History(_) => update_markup(candidate, label, markup),
        }
    }
}

pub(super) fn update_markup(
    candidate: &mut Document,
    label: &str,
    markup: String,
) -> Result<ChangeSet, DesignerError> {
    let update = candidate.try_update_source(label, markup)?;
    if !update.valid {
        return Err(invalid("The RSX projection produced invalid markup"));
    }
    Ok(update.change)
}

impl RustDocument {
    pub(super) fn history_revision(&self, document: &Document) -> RustRevision {
        RustRevision {
            source: self.source.clone(),
            projection: self
                .map()
                .map(|map| (map.invocation.start, document.snapshot_without_history())),
        }
    }

    pub(crate) fn restore_history(
        &mut self,
        document: &mut Document,
        redo: bool,
    ) -> Result<ChangeSet, DesignerError> {
        let mut history = self.history.clone();
        let (label, revision) = if redo {
            history.redo().ok_or(DesignerError::NothingToRedo)?
        } else {
            history.undo().ok_or(DesignerError::NothingToUndo)?
        };
        let (caret, anchor_replaced) =
            super::carry_caret(&self.source, &revision.source, self.caret);
        let projection = revision
            .projection
            .as_ref()
            .filter(|(start, _)| *start == caret)
            .map(|(_, basis)| ProjectedIdentity::History(basis));
        let update =
            self.project_into(document, revision.source, caret, &label, false, projection)?;
        self.scope_changed |= anchor_replaced;
        self.history = history;
        Ok(update.change)
    }
}
