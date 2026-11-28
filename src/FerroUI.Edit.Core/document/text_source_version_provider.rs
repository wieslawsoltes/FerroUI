use super::{AnchorMovementType, DocumentChangeEventArgs, DocumentError};
use std::cmp::Ordering;
use std::sync::{Arc, OnceLock};

struct VersionNode {
    document: Arc<()>,
    revision: u64,
    next: OnceLock<VersionStep>,
}

struct VersionStep {
    change: DocumentChangeEventArgs,
    version: DocumentVersion,
}

impl Drop for VersionNode {
    fn drop(&mut self) {
        // A long-lived snapshot can retain millions of edits. Reclaim the
        // uniquely owned suffix iteratively instead of overflowing the stack.
        let mut next = self.next.take();
        while let Some(step) = next {
            match Arc::try_unwrap(step.version.0) {
                Ok(mut node) => next = node.next.take(),
                Err(_) => break,
            }
        }
    }
}

/// Persistent version checkpoint. Holding an old checkpoint retains only the
/// subsequent edits needed to translate positions; unused history is reclaimed.
#[derive(Clone)]
pub struct DocumentVersion(Arc<VersionNode>);

impl std::fmt::Debug for DocumentVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("DocumentVersion")
            .field(&self.revision())
            .finish()
    }
}

impl PartialEq for DocumentVersion {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for DocumentVersion {}

impl DocumentVersion {
    pub(crate) fn initial() -> Self {
        Self(Arc::new(VersionNode {
            document: Arc::new(()),
            revision: 0,
            next: OnceLock::new(),
        }))
    }
    pub fn revision(&self) -> u64 {
        self.0.revision
    }
    pub fn belongs_to_same_document_as(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0.document, &other.0.document)
    }

    pub fn compare_age(&self, other: &Self) -> Result<Ordering, DocumentError> {
        if !self.belongs_to_same_document_as(other) {
            return Err(DocumentError::DifferentDocument);
        }
        Ok(self.revision().cmp(&other.revision()))
    }

    pub(crate) fn append(&self, change: DocumentChangeEventArgs) -> Result<Self, DocumentError> {
        let change = DocumentChangeEventArgs::with_map(
            change.offset,
            change.removed_text,
            change.inserted_text,
            change.offset_change_map,
        )?;
        let revision = self
            .revision()
            .checked_add(1)
            .ok_or(DocumentError::VersionOverflow)?;
        let version = Self(Arc::new(VersionNode {
            document: self.0.document.clone(),
            revision,
            next: OnceLock::new(),
        }));
        self.0
            .next
            .set(VersionStep {
                change,
                version: version.clone(),
            })
            .map_err(|_| DocumentError::InvalidVersion)?;
        Ok(version)
    }

    pub fn get_changes_to(
        &self,
        other: &Self,
    ) -> Result<Vec<DocumentChangeEventArgs>, DocumentError> {
        let order = self.compare_age(other)?;
        if order == Ordering::Greater {
            return Ok(other
                .get_changes_to(self)?
                .iter()
                .rev()
                .map(DocumentChangeEventArgs::invert)
                .collect());
        }
        let mut changes = Vec::new();
        let mut current = self.clone();
        while current != *other {
            let step = current.0.next.get().ok_or(DocumentError::InvalidVersion)?;
            changes.push(step.change.clone());
            current = step.version.clone();
        }
        Ok(changes)
    }

    pub fn move_offset_to(
        &self,
        other: &Self,
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

/// Version provider for text sources outside a document.
#[derive(Debug)]
pub struct TextSourceVersionProvider {
    current: DocumentVersion,
}

impl Default for TextSourceVersionProvider {
    fn default() -> Self {
        Self::new()
    }
}
impl TextSourceVersionProvider {
    pub fn new() -> Self {
        Self {
            current: DocumentVersion::initial(),
        }
    }
    pub fn current_version(&self) -> DocumentVersion {
        self.current.clone()
    }
    pub fn append_change(
        &mut self,
        change: impl Into<DocumentChangeEventArgs>,
    ) -> Result<(), DocumentError> {
        self.current = self.current.append(change.into())?;
        Ok(())
    }
}
