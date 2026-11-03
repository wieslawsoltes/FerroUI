//! Delta undo units (#439; finding S8 of the review of #243, decision D9 of
//! `docs/porting/winui-rich-text.md`). An edit of the editor is recorded as the blocks it
//! replaced and the blocks that took their place, not as snapshots of the whole document: the
//! document keeps a journal of the blocks each edit being recorded changes, and an undo or a redo
//! installs the blocks of one side in place of those of the other, as the structural edits of F1c
//! install the paragraphs they change.

use super::{
    block_snapshot, common_paragraphs, BlockReplacement, DocumentChangeKind, DocumentError,
    RichTextDocument,
};
use crate::documents::{Block, DocumentLinkField, DocumentSnapshot, Paragraph, ParagraphSnapshot};
use ferroui_base::Ref;
use std::cell::Cell;
use std::ops::Range;
use std::rc::Rc;

/// The link fields of a document and its next field identity.
type Fields = (Vec<DocumentLinkField>, u64);

/// An open journal: what the blocks of the document were before the edit being recorded changed
/// them (S8).
pub(super) struct JournalFrame {
    id: u64,
    /// The blocks the edit changed so far, or those it was about to change when it began; `None`
    /// while there are none.
    blocks: Option<ChangedBlocks>,
    /// The link fields and the next field identity of the document when the edit began.
    fields: Fields,
    /// The edit changed the tree in a way the journal cannot follow: a write that went around the
    /// document's own edits, a change of a block the journal holds no snapshot of, or of blocks
    /// apart from those it holds.
    lost: bool,
}

/// Consecutive blocks an edit changed: the `current` blocks of the tree from `start` hold the
/// place of the blocks `before` holds the snapshots of.
struct ChangedBlocks {
    start: usize,
    before: Vec<ParagraphSnapshot>,
    current: usize,
}

impl JournalFrame {
    /// Notes that the `removed` blocks from `start` of the tree are replaced by `inserted`
    /// blocks; `before` are the snapshots of the removed blocks when the change has them.
    fn note(
        &mut self,
        start: usize,
        removed: usize,
        inserted: usize,
        before: Option<&[ParagraphSnapshot]>,
    ) {
        if self.lost {
            return;
        }
        if self.blocks.is_none() {
            match before {
                Some(before) => {
                    self.blocks = Some(ChangedBlocks {
                        start,
                        before: before.to_vec(),
                        current: inserted,
                    });
                }
                None => self.lost = true,
            }
            return;
        }
        let Some(blocks) = self.blocks.as_mut() else {
            return;
        };
        let end = blocks.start + blocks.current;
        if start >= blocks.start && start + removed <= end {
            blocks.current = blocks.current - removed + inserted;
            return;
        }
        // Blocks outside those the journal holds join them when the change has their snapshots
        // and they adjoin or overlap the blocks it holds.
        let adjoins = start <= end && start + removed >= blocks.start;
        let before = match before {
            Some(before) if adjoins && before.len() == removed => before,
            _ => {
                self.lost = true;
                return;
            }
        };
        let first = start.min(blocks.start);
        let last = (start + removed).max(end);
        let mut original = Vec::new();
        if start < blocks.start {
            original.extend_from_slice(&before[..blocks.start - start]);
        }
        original.append(&mut blocks.before);
        if start + removed > end {
            original.extend_from_slice(&before[end - start..]);
        }
        blocks.before = original;
        blocks.current = last - first - removed + inserted;
        blocks.start = first;
    }
}

/// The journal of the blocks one edit changes (S8), open from `RichTextDocument::begin_journal`
/// until it is finished or dropped.
pub(crate) struct BlockJournal {
    document: RichTextDocument,
    id: u64,
}

impl BlockJournal {
    /// Closes the journal and returns what it holds.
    fn finish(self) -> Option<JournalFrame> {
        self.close()
    }

    fn close(&self) -> Option<JournalFrame> {
        let mut journals = self.document.0.journals.borrow_mut();
        let index = journals.iter().position(|frame| frame.id == self.id)?;
        Some(journals.remove(index))
    }
}

impl Drop for BlockJournal {
    fn drop(&mut self) {
        // The snapshots of the journal are dropped once the journals are no longer borrowed.
        let frame = self.close();
        drop(frame);
    }
}

/// Marks the changes of the tree made while it lives as changes the document notes in the
/// journals itself.
pub(super) struct Journaling<'a>(&'a Cell<usize>);

impl Drop for Journaling<'_> {
    fn drop(&mut self) {
        self.0.set(self.0.get() - 1);
    }
}

/// The document side of an undo unit (S8, decision D9): the blocks from `start` before the edit
/// and after it. Each side is a `DocumentSnapshot` of those blocks with the link fields and the
/// next field identity of the document. A whole delta holds the whole document on each side: the
/// unit of an edit of a document with link fields or with blocks that are not paragraphs (which
/// F1c edits through a snapshot of the document), or of a composition.
#[derive(Clone)]
pub(crate) struct DocumentDelta {
    start: usize,
    before: Rc<DocumentSnapshot>,
    after: Rc<DocumentSnapshot>,
    whole: bool,
}

impl DocumentDelta {
    /// The delta of two snapshots of the whole document: the blocks that differ when
    /// `by_paragraph` (every block of the document before and after is a paragraph) and neither
    /// has link fields, the whole document otherwise.
    pub(crate) fn between(
        before: DocumentSnapshot,
        after: DocumentSnapshot,
        by_paragraph: bool,
    ) -> Self {
        if by_paragraph && before.link_fields.is_empty() && after.link_fields.is_empty() {
            Self::trimmed(
                0,
                before.paragraphs,
                after.paragraphs,
                (Vec::new(), before.next_link_field_id),
                (Vec::new(), after.next_link_field_id),
            )
        } else {
            Self::whole(Rc::new(before), Rc::new(after))
        }
    }

    fn whole(before: Rc<DocumentSnapshot>, after: Rc<DocumentSnapshot>) -> Self {
        Self {
            start: 0,
            before,
            after,
            whole: true,
        }
    }

    /// The delta of the blocks from `start`, without the blocks at either end that are the same
    /// before and after.
    fn trimmed(
        start: usize,
        mut before: Vec<ParagraphSnapshot>,
        mut after: Vec<ParagraphSnapshot>,
        before_fields: Fields,
        after_fields: Fields,
    ) -> Self {
        let (prefix, suffix) = common_paragraphs(&|_| true, &before, &after);
        before.truncate(before.len() - suffix);
        before.drain(..prefix);
        after.truncate(after.len() - suffix);
        after.drain(..prefix);
        Self {
            start: start + prefix,
            before: Rc::new(piece(before, before_fields)),
            after: Rc::new(piece(after, after_fields)),
            whole: false,
        }
    }

    /// Whether the edit left the document as it was.
    pub(crate) fn is_unchanged(&self) -> bool {
        self.before == self.after
    }

    /// Whether `other` is this delta (the same pieces).
    pub(crate) fn is(&self, other: &Self) -> bool {
        self.start == other.start
            && self.whole == other.whole
            && Rc::ptr_eq(&self.before, &other.before)
            && Rc::ptr_eq(&self.after, &other.after)
    }

    /// Takes the document before the edit from `previous`, the delta of the unit before, when
    /// it is the same as the document after that unit: neighbouring whole units then hold one
    /// copy of the document between them, as the snapshot units before S8 did.
    pub(crate) fn share_with(&mut self, previous: &Self) {
        if self.whole && previous.whole && previous.after == self.before {
            self.before = previous.after.clone();
        }
    }

    /// The delta of this edit followed by `next` (a typing group, #243 review M6 and S1): from
    /// the document before this edit to the document after `next`. The blocks between the two
    /// that neither changed are read from `document`, the document after `next`. `None` when the
    /// two do not fit together.
    pub(crate) fn then(&self, next: &Self, document: &RichTextDocument) -> Option<Self> {
        let (ls, la) = (self.start, self.after.paragraphs.len());
        let (ns, nb) = (next.start, next.before.paragraphs.len());
        match (self.whole, next.whole) {
            (true, true) => Some(Self::whole(self.before.clone(), next.after.clone())),
            (true, false) => {
                // The document after `next`: the one after this edit with the blocks of `next`.
                let mut after = (*self.after).clone();
                if ns + nb > after.paragraphs.len() {
                    return None;
                }
                after
                    .paragraphs
                    .splice(ns..ns + nb, next.after.paragraphs.iter().cloned())
                    .for_each(drop);
                after.link_fields = next.after.link_fields.clone();
                after.next_link_field_id = next.after.next_link_field_id;
                Some(Self::whole(self.before.clone(), Rc::new(after)))
            }
            (false, true) => {
                // The document before this edit: the one before `next` with the blocks of this
                // edit put back.
                let mut before = (*next.before).clone();
                if ls + la > before.paragraphs.len() {
                    return None;
                }
                before
                    .paragraphs
                    .splice(ls..ls + la, self.before.paragraphs.iter().cloned())
                    .for_each(drop);
                before.link_fields = self.before.link_fields.clone();
                before.next_link_field_id = self.before.next_link_field_id;
                Some(Self::whole(Rc::new(before), next.after.clone()))
            }
            (false, false) => {
                // The blocks from the first to the last either edit changed, as they are between
                // the two edits.
                let first = ls.min(ns);
                let last = (ls + la).max(ns + nb);
                let blocks = document.blocks().snapshot();
                let mut middle = Vec::with_capacity(last - first);
                for index in first..last {
                    let snapshot = if (ls..ls + la).contains(&index) {
                        self.after.paragraphs[index - ls].clone()
                    } else if (ns..ns + nb).contains(&index) {
                        next.before.paragraphs[index - ns].clone()
                    } else {
                        // A block that neither edit changed, where it is now.
                        let current = if index < ns {
                            index
                        } else {
                            index - nb + next.after.paragraphs.len()
                        };
                        block_snapshot(blocks.get(current)?)
                    };
                    middle.push(snapshot);
                }
                let mut before = middle.clone();
                before
                    .splice(
                        ls - first..ls - first + la,
                        self.before.paragraphs.iter().cloned(),
                    )
                    .for_each(drop);
                let mut after = middle;
                after
                    .splice(
                        ns - first..ns - first + nb,
                        next.after.paragraphs.iter().cloned(),
                    )
                    .for_each(drop);
                Some(Self::trimmed(
                    first,
                    before,
                    after,
                    fields(&self.before),
                    fields(&next.after),
                ))
            }
        }
    }

    /// Test only: the index of the first block of the delta, the number of blocks it holds
    /// before the edit and after it, and whether it holds the whole document.
    #[cfg(test)]
    pub(crate) fn blocks(&self) -> (usize, usize, usize, bool) {
        (
            self.start,
            self.before.paragraphs.len(),
            self.after.paragraphs.len(),
            self.whole,
        )
    }
}

fn piece(paragraphs: Vec<ParagraphSnapshot>, fields: Fields) -> DocumentSnapshot {
    DocumentSnapshot {
        paragraphs,
        link_fields: fields.0,
        next_link_field_id: fields.1,
    }
}

fn fields(snapshot: &DocumentSnapshot) -> Fields {
    (snapshot.link_fields.clone(), snapshot.next_link_field_id)
}

/// The blocks a replacement of the text `range` spans: from the block of its start to the block
/// of its end, each located as `DocumentSnapshot::locate` locates an offset (an offset at the end
/// of a block is in that block).
fn block_span(blocks: &[Ref<Block>], range: Range<usize>) -> Range<usize> {
    let mut first = None;
    let mut offset = 0;
    for (index, block) in blocks.iter().enumerate() {
        let end = offset + block.text_length();
        let last = index + 1 == blocks.len();
        if first.is_none() && (range.start <= end || last) {
            first = Some(index);
        }
        if range.end <= end || last {
            return first.unwrap_or(index)..index + 1;
        }
        offset = end + 1;
    }
    0..0
}

impl RichTextDocument {
    /// Whether the edits of the document are recorded as deltas of the blocks they change (S8):
    /// every block is a paragraph and there are no link fields, as in the documents F1c edits a
    /// paragraph at a time. The undo units of the edits of any other document hold the whole
    /// document.
    pub(crate) fn edits_by_paragraph(&self) -> bool {
        !self.rebuilds_everything()
            && self.0.link_fields.borrow().is_empty()
            && self
                .0
                .blocks
                .snapshot()
                .iter()
                .all(|block| block.is::<Paragraph>())
    }

    /// Opens the journal of the blocks an edit changes (S8). The snapshots of the blocks the text
    /// `range` spans are taken now, before the edit, where a snapshot of the document was taken
    /// before S8; every change of blocks the document makes is noted in the journal until it is
    /// finished or dropped.
    pub(crate) fn begin_journal(&self, range: Option<Range<usize>>) -> BlockJournal {
        let version = self.mutation_version();
        let blocks = range.map(|range| {
            let blocks = self.0.blocks.snapshot();
            let span = block_span(&blocks, range);
            ChangedBlocks {
                start: span.start,
                before: blocks[span.clone()].iter().map(block_snapshot).collect(),
                current: span.len(),
            }
        });
        // A write while the blocks were read leaves their snapshots behind.
        let lost = self.mutation_version() != version;
        let id = self.0.next_journal.get();
        self.0.next_journal.set(id.wrapping_add(1));
        let fields = (
            self.0.link_fields.borrow().clone(),
            self.0.next_link_field_id.get(),
        );
        self.0.journals.borrow_mut().push(JournalFrame {
            id,
            blocks,
            fields,
            lost,
        });
        BlockJournal {
            document: self.clone(),
            id,
        }
    }

    /// Closes the journal of an edit and returns the delta of the edit (S8): the blocks it
    /// changed, read now, and what they were. It is a whole delta when the document now has link
    /// fields, or when `committed` (the document a visible composition began from) stands for
    /// the document after the edit. `None` when the journal could not follow the edit.
    pub(crate) fn finish_journal(
        &self,
        journal: BlockJournal,
        committed: Option<&DocumentSnapshot>,
    ) -> Option<DocumentDelta> {
        let frame = journal.finish()?;
        if frame.lost {
            return None;
        }
        let JournalFrame {
            blocks: changed,
            fields: before_fields,
            ..
        } = frame;
        let after_fields = (
            self.0.link_fields.borrow().clone(),
            self.0.next_link_field_id.get(),
        );
        let blocks = self.0.blocks.snapshot();
        if changed
            .as_ref()
            .is_some_and(|changed| changed.start + changed.current > blocks.len())
        {
            return None;
        }
        if committed.is_some() || !after_fields.0.is_empty() {
            // The document before the edit: the one now with the blocks the edit changed put
            // back.
            let now = self.snapshot();
            let mut before = now.clone();
            if let Some(changed) = changed {
                before
                    .paragraphs
                    .splice(
                        changed.start..changed.start + changed.current,
                        changed.before,
                    )
                    .for_each(drop);
            }
            before.link_fields = before_fields.0;
            before.next_link_field_id = before_fields.1;
            let after = committed.cloned().unwrap_or(now);
            return Some(DocumentDelta::whole(Rc::new(before), Rc::new(after)));
        }
        let Some(changed) = changed else {
            return Some(DocumentDelta::trimmed(
                0,
                Vec::new(),
                Vec::new(),
                before_fields,
                after_fields,
            ));
        };
        let after = blocks[changed.start..changed.start + changed.current]
            .iter()
            .map(block_snapshot)
            .collect();
        Some(DocumentDelta::trimmed(
            changed.start,
            changed.before,
            after,
            before_fields,
            after_fields,
        ))
    }

    /// Installs one side of a delta in place of the other: the blocks before the edit for an
    /// undo, the blocks after it for a redo (S8). The blocks at either end that are the same on
    /// both sides stay in the tree with their elements, as an installation of F1c leaves them; a
    /// whole delta installs its snapshot with `restore`.
    pub(crate) fn apply_delta(
        &self,
        delta: &DocumentDelta,
        undo: bool,
    ) -> Result<(), DocumentError> {
        let (piece, count) = if undo {
            (&delta.before, delta.after.paragraphs.len())
        } else {
            (&delta.after, delta.before.paragraphs.len())
        };
        if delta.whole {
            self.restore(piece)
        } else {
            self.restore_blocks(delta.start, count, piece)
        }
    }

    /// Installs the blocks of `piece`, with its link fields and next field identity, in place of
    /// the `count` blocks from `start`, as `restore` installs a snapshot of the whole document.
    fn restore_blocks(
        &self,
        start: usize,
        count: usize,
        piece: &DocumentSnapshot,
    ) -> Result<(), DocumentError> {
        self.ensure_available()?;
        let version = self.mutation_version();
        piece.validate()?;
        let blocks = self.blocks().snapshot();
        if start + count > blocks.len() {
            return Err(DocumentError::InvalidFormat(
                "The undo unit does not fit the document.",
            ));
        }
        let old: Vec<ParagraphSnapshot> = blocks[start..start + count]
            .iter()
            .map(block_snapshot)
            .collect();
        let unchanged = old == piece.paragraphs
            && *self.0.link_fields.borrow() == piece.link_fields
            && self.0.next_link_field_id.get() == piece.next_link_field_id;
        // Snapshot capture and custom brush equality can call application code.
        self.ensure_install_read(version)?;
        if unchanged {
            return Ok(());
        }
        let (prefix, suffix) = if self.rebuilds_everything() {
            (0, 0)
        } else {
            common_paragraphs(
                &|index| blocks[start + index].is::<Paragraph>(),
                &old,
                &piece.paragraphs,
            )
        };
        let removed = start + prefix..start + count - suffix;
        let inserted = &piece.paragraphs[prefix..piece.paragraphs.len() - suffix];
        let text = if self.rebuilds_everything() {
            None
        } else {
            let lengths: Vec<usize> = blocks.iter().map(|block| block.text_length()).collect();
            self.replaced_text(version, &lengths, removed.clone(), inserted)
        };
        self.install_blocks(
            version,
            BlockReplacement {
                start: removed.start,
                removed: &blocks[removed],
                before: Some(&old[prefix..count - suffix]),
                inserted,
                text,
            },
            Some((piece.link_fields.clone(), piece.next_link_field_id)),
            DocumentChangeKind::Structure,
            None,
        )
    }

    /// Notes in the open journals that the `removed` blocks from `start` are replaced by
    /// `inserted` blocks; `before` are the snapshots of the removed blocks when the change has
    /// them.
    pub(super) fn journal_replacement(
        &self,
        start: usize,
        removed: usize,
        inserted: usize,
        before: Option<&[ParagraphSnapshot]>,
    ) {
        for frame in self.0.journals.borrow_mut().iter_mut() {
            frame.note(start, removed, inserted, before);
        }
    }

    /// Notes in the open journals a change of the tree they cannot follow.
    pub(super) fn journal_untracked_change(&self) {
        for frame in self.0.journals.borrow_mut().iter_mut() {
            frame.lost = true;
        }
    }

    /// Marks the changes of the tree until the returned guard is dropped as changes the
    /// document notes in the journals itself.
    pub(super) fn journaling(&self) -> Journaling<'_> {
        self.0.journaling.set(self.0.journaling.get() + 1);
        Journaling(&self.0.journaling)
    }
}
