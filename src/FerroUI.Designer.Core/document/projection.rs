use super::{parse_snapshot, ChangeSet, DesignerError, Document};
use crate::NodeId;
use std::sync::Arc;

impl Document {
    /// Install a checked projection using identities from a staged document in
    /// the same lineage. Locations are element starts in the new markup.
    /// An exact history snapshot can be installed without additional locations.
    pub fn apply_projection(
        &mut self,
        expected_revision: u64,
        label: impl Into<String>,
        source: String,
        basis: &Self,
        locations: impl IntoIterator<Item = (NodeId, usize)>,
    ) -> Result<ChangeSet, DesignerError> {
        self.check_revision(Some(expected_revision))?;
        if !Arc::ptr_eq(&self.lineage, &basis.lineage) || !basis.is_valid() {
            return Err(DesignerError::InvalidEdit(
                "The projected edit belongs to another document".into(),
            ));
        }
        let pins: Vec<_> = locations
            .into_iter()
            .map(|(id, start)| (start, id))
            .collect();
        if pins.is_empty() && source != basis.source() {
            return Err(DesignerError::InvalidEdit(
                "The projected source has no verified element locations".into(),
            ));
        }
        let mut next_id = self.next_id.max(basis.next_id);
        let snapshot = parse_snapshot(source, &basis.current, &mut next_id, &pins)?;
        if pins.iter().any(|(start, id)| {
            !snapshot
                .ast
                .node(*id)
                .is_some_and(|node| node.is_element() && node.span.start == *start)
        }) {
            return Err(DesignerError::InvalidEdit(
                "The projected controls changed before the edit".into(),
            ));
        }
        let change = self.commit(label.into(), snapshot)?;
        self.next_id = next_id;
        Ok(change)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Edit;

    #[test]
    fn a_projection_retains_the_staged_order_and_rejects_foreign_identities() {
        let mut document = Document::new("<Grid><Button/><Button/></Grid>").unwrap();
        let original = document.snapshot_without_history();
        let root = document.ast().root();
        let children = document.ast().element_children(root);
        let mut basis = document.snapshot_without_history();
        basis
            .apply(
                "Move",
                vec![Edit::MoveNode {
                    node: children[0],
                    parent: root,
                    index: 1,
                }],
            )
            .unwrap();
        let source = "<Grid>\n  <Button/>\n  <Button/>\n</Grid>";
        let starts: Vec<_> = source
            .match_indices("<Button")
            .map(|(start, _)| start)
            .collect();
        let revision = document.revision();
        let result = document
            .apply_projection(
                revision,
                "Move",
                source.into(),
                &basis,
                [
                    (root, 0),
                    (children[1], starts[0]),
                    (children[0], starts[1]),
                ],
            )
            .unwrap();
        assert_eq!(document.revision(), revision + 1);
        assert!(result.inserted.iter().all(|&id| !document.ast().node(id).unwrap().is_element()));
        assert!(result.removed.is_empty());
        assert_eq!(
            document.ast().element_children(root),
            [children[1], children[0]]
        );
        document
            .apply_projection(
                document.revision(),
                "Undo",
                original.source().into(),
                &original,
                [],
            )
            .unwrap();
        assert_eq!(document.ast().element_children(root), children);
        let foreign = Document::new(source).unwrap();
        assert!(document
            .apply_projection(document.revision(), "Bad", source.into(), &foreign, [])
            .is_err());
        assert_eq!(document.ast().element_children(root), children);
    }
}
