use super::{IActiveElement, SnippetError, SnippetSession, WeakSnippetSession};
use crate::document::{
    AnchorMovementType, AnchorSegment, DocumentEdit, DocumentError, SimpleSegment,
};

/// A named snippet marker whose text can be filled or replaced by an extension.
/// Its range remains tracked after replacement, including an initially empty
/// range and supplementary UTF-16 characters.
#[derive(Clone)]
pub struct AnchorElement {
    name: String,
    range: AnchorSegment,
    session: WeakSnippetSession,
}

impl AnchorElement {
    pub(crate) fn new(
        session: &SnippetSession,
        name: String,
        offset: usize,
    ) -> Result<Self, SnippetError> {
        let range = AnchorSegment::new(&session.document(), offset, 0)?;
        range
            .start
            .set_movement_type(AnchorMovementType::BeforeInsertion);
        range
            .end
            .set_movement_type(AnchorMovementType::BeforeInsertion);
        Ok(Self {
            name,
            range,
            session: session.downgrade(),
        })
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn segment(&self) -> Option<SimpleSegment> {
        self.range.segment().ok()
    }
    pub fn text(&self) -> Result<String, SnippetError> {
        let range = self.range.segment()?;
        Ok(self
            .range
            .start
            .document()
            .get_text(range.offset, range.length)?)
    }
    pub fn set_text(&self, text: &str) -> Result<(), SnippetError> {
        let range = self.range.segment()?;
        let document = self.range.start.document();
        let version = document.version();
        let edit = DocumentEdit::new(range.offset, range.length, text);
        if let Some(session) = self.session.upgrade() {
            session.validate_edits(std::slice::from_ref(&edit))?;
        }
        if document.version() != version {
            return Err(DocumentError::ReentrantChange.into());
        }
        self.range
            .end
            .set_movement_type(AnchorMovementType::AfterInsertion);
        struct Restore<'a>(&'a AnchorElement);
        impl Drop for Restore<'_> {
            fn drop(&mut self) {
                self.0
                    .range
                    .end
                    .set_movement_type(AnchorMovementType::BeforeInsertion);
            }
        }
        let _restore = Restore(self);
        document.replace(range.offset, range.length, text)?;
        Ok(())
    }
}

impl IActiveElement for AnchorElement {
    fn is_editable(&self) -> bool {
        false
    }
    fn segment(&self) -> Option<SimpleSegment> {
        self.segment()
    }
}
