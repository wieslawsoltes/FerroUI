use crate::{document::DesignDocument, edits::editor_edits, protocol::EditResult};
use ferroui_designer::LayerViewState;
use ferroui_designer_core::{Document, SourceSpan, TextPatch};
use ferroui_designer_language::Range;
use ferroui_designer_rsx::RsxChildEdit;
use serde::Serialize;

/// Layer locators use native markup byte offsets. Only checked surviving
/// controls participate in the forward and reverse maps.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LayerChange {
    before_stamp: String,
    after_stamp: String,
    block: Option<Range>,
    positions: Vec<[usize; 2]>,
}

pub(crate) fn xaml(
    design: &DesignDocument<'_>,
    draft: &Document,
    mut patches: Vec<TextPatch>,
) -> Result<EditResult, String> {
    let paired = || {
        design.xaml.ast().elements().filter_map(|old| {
            draft
                .ast()
                .node(old.id)
                .filter(|node| node.is_element())
                .map(|new| (old, new))
        })
    };
    if patches.is_empty() && design.source == draft.source() {
        patches = reorder_patches(
            design.source,
            paired().map(|(old, new)| (old.name_span, new.name_span)),
        );
    }
    let mut result = editor_edits(design, patches)?;
    if !result.edits.is_empty() {
        result.layer_change = Some(LayerChange {
            before_stamp: LayerViewState::stamp_source(design.source),
            after_stamp: LayerViewState::stamp_source(draft.source()),
            block: None,
            positions: paired()
                .map(|(old, new)| [old.span.start, new.span.start])
                .collect(),
        });
    }
    Ok(result)
}

pub(crate) fn rust(design: &DesignDocument<'_>, plan: RsxChildEdit) -> Result<EditResult, String> {
    let projection = design.rsx.as_ref().ok_or("Open the current RSX view")?;
    rust_in_block(design, plan, design.range(projection.invocation)?)
}

pub(crate) fn rust_in_block(
    design: &DesignDocument<'_>,
    plan: RsxChildEdit,
    block: Range,
) -> Result<EditResult, String> {
    let patches = if plan.authored_source == design.source {
        reorder_patches(
            design.source,
            design.xaml.ast().elements().filter_map(|old| {
                let (_, rust) = design.rsx_node(old.id)?;
                Some((rust.name, *plan.controls.get(&old.id)?))
            }),
        )
    } else {
        plan.patches
    };
    let mut result = editor_edits(design, patches)?;
    if !result.edits.is_empty() {
        result.layer_change = Some(LayerChange {
            before_stamp: LayerViewState::stamp_source(design.xaml.source()),
            after_stamp: LayerViewState::stamp_source(&plan.projected_source),
            block: Some(block),
            positions: design
                .xaml
                .ast()
                .elements()
                .filter_map(|old| Some([old.span.start, *plan.projected_starts.get(&old.id)?]))
                .collect(),
        });
    }
    Ok(result)
}

/// Reordering identical controls can leave the buffer byte-for-byte the same.
/// Keep a surviving token between two disjoint edit ranges so the editor can
/// record the checked reorder as one real Undo operation.
fn reorder_patches(
    source: &str,
    pairs: impl Iterator<Item = (SourceSpan, SourceSpan)>,
) -> Vec<TextPatch> {
    let Some((before, after)) = pairs
        .filter(|(before, after)| {
            before.start != after.start && before.len() == after.len() && !before.is_empty()
        })
        .find(|(before, after)| source[before.range()] == source[after.range()])
    else {
        return Vec::new();
    };
    let prefix = TextPatch::between(&source[..before.start], &source[..after.start]);
    let suffix =
        TextPatch::between(&source[before.end..], &source[after.end..]).map(|mut patch| {
            patch.span.start += before.end;
            patch.span.end += before.end;
            patch
        });
    prefix.into_iter().chain(suffix).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_designer_core::Edit;

    #[test]
    fn identical_markup_moves_use_disjoint_source_edits_and_keep_identity() {
        let source = "<!-- 🧭 -->\r\n<Grid><Button Content='🧭' /><Button Content='🧭' /></Grid>";
        let before = Document::new(source).unwrap();
        let mut after = before.snapshot_without_history();
        let root = before.ast().root();
        let children = before.ast().element_children(root);
        after
            .apply(
                "Move",
                vec![Edit::MoveNode {
                    node: children[0],
                    parent: root,
                    index: 1,
                }],
            )
            .unwrap();
        assert_eq!(before.source(), after.source());
        let pairs = before
            .ast()
            .elements()
            .map(|old| (old.name_span, after.ast().node(old.id).unwrap().name_span));
        let patches = reorder_patches(source, pairs);
        assert_eq!(patches.len(), 2);
        assert!(patches[0].span.end < patches[1].span.start);
        assert_eq!(TextPatch::apply_all(source, patches).unwrap(), source);
    }
}
