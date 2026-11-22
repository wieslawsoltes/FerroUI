use super::{AnchorMovementType, DocumentChangeEventArgs, DocumentError, DocumentVersion};
use std::any::Any;
use std::cmp::Ordering;
use std::sync::Arc;

/// Thread-safe checkpoint contract independent of retaining the text snapshot.
/// Change values preserve custom document offset maps in both directions.
pub trait ITextSourceVersion: Any + Send + Sync {
    fn belongs_to_same_document_as(&self, other: &dyn ITextSourceVersion) -> bool;
    fn compare_age(&self, other: &dyn ITextSourceVersion) -> Result<Ordering, DocumentError>;
    fn get_changes_to(
        &self,
        other: &dyn ITextSourceVersion,
    ) -> Result<Vec<DocumentChangeEventArgs>, DocumentError>;
    fn move_offset_to(
        &self,
        other: &dyn ITextSourceVersion,
        offset: usize,
        movement: AnchorMovementType,
    ) -> Result<usize, DocumentError> {
        Ok(self
            .get_changes_to(other)?
            .iter()
            .fold(offset, |offset, change| {
                change.get_new_offset(offset, movement)
            }))
    }
}

impl DocumentVersion {
    pub fn as_text_source_version(&self) -> Arc<dyn ITextSourceVersion> {
        Arc::new(self.clone())
    }
}

fn checkpoint(version: &dyn ITextSourceVersion) -> Result<&DocumentVersion, DocumentError> {
    let value: &dyn Any = version;
    value.downcast_ref().ok_or(DocumentError::DifferentDocument)
}

impl ITextSourceVersion for DocumentVersion {
    fn belongs_to_same_document_as(&self, other: &dyn ITextSourceVersion) -> bool {
        checkpoint(other)
            .is_ok_and(|other| DocumentVersion::belongs_to_same_document_as(self, other))
    }
    fn compare_age(&self, other: &dyn ITextSourceVersion) -> Result<Ordering, DocumentError> {
        DocumentVersion::compare_age(self, checkpoint(other)?)
    }
    fn get_changes_to(
        &self,
        other: &dyn ITextSourceVersion,
    ) -> Result<Vec<DocumentChangeEventArgs>, DocumentError> {
        DocumentVersion::get_changes_to(self, checkpoint(other)?)
    }
}
