/// Determines insertion affinity at an anchor's current position.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AnchorMovementType {
    #[default]
    Default,
    BeforeInsertion,
    AfterInsertion,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum OffsetChangeMappingType {
    #[default]
    Normal,
    RemoveAndInsert,
    CharacterReplace,
    KeepAnchorBeforeInsertion,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct OffsetChangeMapEntry {
    pub offset: usize,
    pub removal_length: usize,
    pub insertion_length: usize,
    pub removal_never_causes_anchor_deletion: bool,
    pub default_anchor_movement_is_before_insertion: bool,
}

impl OffsetChangeMapEntry {
    pub const fn new(offset: usize, removal_length: usize, insertion_length: usize) -> Self {
        Self {
            offset,
            removal_length,
            insertion_length,
            removal_never_causes_anchor_deletion: false,
            default_anchor_movement_is_before_insertion: false,
        }
    }

    pub fn get_new_offset(&self, old_offset: usize, movement: AnchorMovementType) -> usize {
        if self.removal_length != 0 || old_offset != self.offset {
            if old_offset <= self.offset {
                return old_offset;
            }
            if old_offset >= self.offset.saturating_add(self.removal_length) {
                return old_offset
                    .saturating_sub(self.removal_length)
                    .saturating_add(self.insertion_length);
            }
        }
        match movement {
            AnchorMovementType::BeforeInsertion => self.offset,
            AnchorMovementType::AfterInsertion => self.offset.saturating_add(self.insertion_length),
            AnchorMovementType::Default if self.default_anchor_movement_is_before_insertion => {
                self.offset
            }
            AnchorMovementType::Default => self.offset.saturating_add(self.insertion_length),
        }
    }

    pub fn deletes_anchor(&self, offset: usize) -> bool {
        !self.removal_never_causes_anchor_deletion
            && offset > self.offset
            && offset < self.offset.saturating_add(self.removal_length)
    }
}

/// Immutable, shareable explanation of the position changes in one edit.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OffsetChangeMap {
    entries: Vec<OffsetChangeMapEntry>,
}

impl OffsetChangeMap {
    pub fn new(entries: Vec<OffsetChangeMapEntry>) -> Self {
        Self { entries }
    }
    pub fn empty() -> Self {
        Self::default()
    }
    pub fn from_single_element(entry: OffsetChangeMapEntry) -> Self {
        Self::new(vec![entry])
    }
    pub fn entries(&self) -> &[OffsetChangeMapEntry] {
        &self.entries
    }

    pub fn for_change(
        offset: usize,
        removed: usize,
        inserted: usize,
        mapping: OffsetChangeMappingType,
    ) -> Self {
        let entry = OffsetChangeMapEntry::new(offset, removed, inserted);
        match mapping {
            OffsetChangeMappingType::Normal => Self::from_single_element(entry),
            OffsetChangeMappingType::KeepAnchorBeforeInsertion => {
                Self::from_single_element(OffsetChangeMapEntry {
                    default_anchor_movement_is_before_insertion: true,
                    ..entry
                })
            }
            OffsetChangeMappingType::RemoveAndInsert => Self::new(vec![
                OffsetChangeMapEntry::new(offset, removed, 0),
                OffsetChangeMapEntry::new(offset, 0, inserted),
            ]),
            OffsetChangeMappingType::CharacterReplace => {
                if inserted == 0 || removed == 0 {
                    Self::from_single_element(entry)
                } else if inserted == removed {
                    Self::empty()
                } else if inserted < removed {
                    Self::from_single_element(OffsetChangeMapEntry {
                        removal_never_causes_anchor_deletion: true,
                        ..OffsetChangeMapEntry::new(
                            offset.saturating_add(inserted),
                            removed - inserted,
                            0,
                        )
                    })
                } else if removed == 0 {
                    Self::from_single_element(entry)
                } else {
                    Self::from_single_element(OffsetChangeMapEntry::new(
                        offset.saturating_add(removed - 1),
                        1,
                        inserted - removed + 1,
                    ))
                }
            }
        }
    }

    pub fn get_new_offset(&self, offset: usize, movement: AnchorMovementType) -> usize {
        self.entries.iter().fold(offset, |offset, entry| {
            entry.get_new_offset(offset, movement)
        })
    }

    pub fn is_valid_for_document_change(
        &self,
        offset: usize,
        removed: usize,
        inserted: usize,
    ) -> bool {
        let Some(mut end) = offset.checked_add(removed) else {
            return false;
        };
        for entry in &self.entries {
            if entry.offset < offset
                || entry
                    .offset
                    .checked_add(entry.removal_length)
                    .is_none_or(|v| v > end)
            {
                return false;
            }
            let Some(next) = end
                .checked_sub(entry.removal_length)
                .and_then(|v| v.checked_add(entry.insertion_length))
            else {
                return false;
            };
            end = next;
        }
        offset.checked_add(inserted) == Some(end)
    }

    pub fn invert(&self) -> Self {
        Self::new(
            self.entries
                .iter()
                .rev()
                .map(|entry| {
                    OffsetChangeMapEntry::new(
                        entry.offset,
                        entry.insertion_length,
                        entry.removal_length,
                    )
                })
                .collect(),
        )
    }
}
