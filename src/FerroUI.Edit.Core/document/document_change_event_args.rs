use super::{
    AnchorMovementType, DocumentError, OffsetChangeMap, OffsetChangeMappingType, TextSnapshot,
};

/// A lossless immutable edit description. All lengths and offsets are UTF-16.
#[derive(Clone, Debug)]
pub struct DocumentChangeEventArgs {
    pub offset: usize,
    pub removed_text: TextSnapshot,
    pub inserted_text: TextSnapshot,
    pub offset_change_map: OffsetChangeMap,
}

/// Generic source change. Its boundary affinity differs from a normal document
/// replacement: both replacement endpoints obey the supplied movement type.
#[derive(Clone, Debug)]
pub struct TextChangeEventArgs {
    pub offset: usize,
    pub removed_text: TextSnapshot,
    pub inserted_text: TextSnapshot,
}

impl TextChangeEventArgs {
    pub fn new(offset: usize, removed_text: &str, inserted_text: &str) -> Self {
        Self {
            offset,
            removed_text: TextSnapshot::from_text(removed_text),
            inserted_text: TextSnapshot::from_text(inserted_text),
        }
    }
    pub fn removal_length(&self) -> usize {
        self.removed_text.text_length()
    }
    pub fn insertion_length(&self) -> usize {
        self.inserted_text.text_length()
    }
    pub fn get_new_offset(&self, offset: usize, movement: AnchorMovementType) -> usize {
        if offset >= self.offset && offset <= self.offset.saturating_add(self.removal_length()) {
            if movement == AnchorMovementType::BeforeInsertion {
                self.offset
            } else {
                self.offset.saturating_add(self.insertion_length())
            }
        } else if offset > self.offset {
            offset
                .saturating_sub(self.removal_length())
                .saturating_add(self.insertion_length())
        } else {
            offset
        }
    }
    pub fn invert(&self) -> Self {
        Self {
            offset: self.offset,
            removed_text: self.inserted_text.clone(),
            inserted_text: self.removed_text.clone(),
        }
    }
}

impl From<TextChangeEventArgs> for DocumentChangeEventArgs {
    fn from(change: TextChangeEventArgs) -> Self {
        let map = OffsetChangeMap::for_change(
            change.offset,
            change.removal_length(),
            change.insertion_length(),
            OffsetChangeMappingType::RemoveAndInsert,
        );
        Self {
            offset: change.offset,
            removed_text: change.removed_text,
            inserted_text: change.inserted_text,
            offset_change_map: map,
        }
    }
}

impl DocumentChangeEventArgs {
    pub fn new(offset: usize, removed_text: &str, inserted_text: &str) -> Self {
        let removed_text = TextSnapshot::from_text(removed_text);
        let inserted_text = TextSnapshot::from_text(inserted_text);
        let map = OffsetChangeMap::for_change(
            offset,
            removed_text.text_length(),
            inserted_text.text_length(),
            OffsetChangeMappingType::Normal,
        );
        Self {
            offset,
            removed_text,
            inserted_text,
            offset_change_map: map,
        }
    }

    pub fn with_map(
        offset: usize,
        removed_text: TextSnapshot,
        inserted_text: TextSnapshot,
        offset_change_map: OffsetChangeMap,
    ) -> Result<Self, DocumentError> {
        if !offset_change_map.is_valid_for_document_change(
            offset,
            removed_text.text_length(),
            inserted_text.text_length(),
        ) {
            return Err(DocumentError::InvalidOffsetMap);
        }
        // Change fragments must not retain document versions: a version owns
        // its outgoing change, so retaining it here would form an Arc cycle.
        let removed_text = TextSnapshot {
            rope: removed_text.rope,
            version: None,
        };
        let inserted_text = TextSnapshot {
            rope: inserted_text.rope,
            version: None,
        };
        Ok(Self {
            offset,
            removed_text,
            inserted_text,
            offset_change_map,
        })
    }

    pub fn removal_length(&self) -> usize {
        self.removed_text.text_length()
    }
    pub fn insertion_length(&self) -> usize {
        self.inserted_text.text_length()
    }
    pub fn get_new_offset(&self, offset: usize, movement: AnchorMovementType) -> usize {
        self.offset_change_map.get_new_offset(offset, movement)
    }
    pub fn invert(&self) -> Self {
        Self {
            offset: self.offset,
            removed_text: self.inserted_text.clone(),
            inserted_text: self.removed_text.clone(),
            offset_change_map: self.offset_change_map.invert(),
        }
    }
}
