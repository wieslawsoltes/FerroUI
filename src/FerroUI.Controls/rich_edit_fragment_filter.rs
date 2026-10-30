//! Rich clipboard input policy over the shared snapshot. The visible UTF-16
//! boundary map keeps one field identity coherent when case expansion or
//! removal of control characters changes its paragraph-owned projections.

use super::{RichEditBox, RichEditCharacterCasing};
use crate::documents::{DocumentError, DocumentSnapshot, InlineSnapshot, InlineSnapshotKind};

pub(super) fn filter(
    owner: &RichEditBox,
    source: &DocumentSnapshot,
) -> Result<DocumentSnapshot, DocumentError> {
    source.validate()?;
    let mut fragment = source.clone();
    let mut offsets = OffsetMap::new(!source.link_fields.is_empty(), source.text_length());
    for (index, paragraph) in fragment.paragraphs.iter_mut().enumerate() {
        if index != 0 {
            offsets.scalar('\n', usize::from(owner.accepts_return()));
        }
        filter_inlines(owner, &mut paragraph.inlines, &mut offsets)?;
    }
    if !owner.accepts_return() && fragment.paragraphs.len() > 1 {
        let mut paragraphs = fragment.paragraphs.into_iter();
        let mut first = paragraphs
            .next()
            .expect("The fragment has multiple paragraphs.");
        for paragraph in paragraphs {
            first.inlines.push(InlineSnapshot {
                format: paragraph.format.character,
                kind: InlineSnapshotKind::Span(paragraph.inlines),
            });
        }
        fragment.paragraphs = vec![first];
    }
    if let Some(offsets) = offsets.boundaries {
        fragment.remap_link_field_offsets(&offsets)?;
    }
    Ok(fragment)
}

struct OffsetMap {
    boundaries: Option<Vec<Option<usize>>>,
    next: usize,
}

impl OffsetMap {
    fn new(track: bool, length: usize) -> Self {
        let boundaries = track.then(|| {
            let mut result = Vec::with_capacity(length.saturating_add(1));
            result.push(Some(0));
            result
        });
        Self {
            boundaries,
            next: 0,
        }
    }

    fn scalar(&mut self, original: char, replacement_length: usize) {
        self.next += replacement_length;
        if let Some(boundaries) = &mut self.boundaries {
            // Interior surrogate code units are never valid field boundaries.
            for _ in 1..original.len_utf16() {
                boundaries.push(None);
            }
            boundaries.push(Some(self.next));
        }
    }

    fn unchanged_ascii(&mut self, length: usize, retained: bool) {
        for _ in 0..length {
            self.scalar('\n', usize::from(retained));
        }
    }
}

fn filter_inlines(
    owner: &RichEditBox,
    inlines: &mut Vec<InlineSnapshot>,
    offsets: &mut OffsetMap,
) -> Result<(), DocumentError> {
    let mut filtered = Vec::with_capacity(inlines.len());
    for mut inline in std::mem::take(inlines) {
        let line_break_length = if matches!(&inline.kind, InlineSnapshotKind::LineBreak) {
            inline.text_length()
        } else {
            0
        };
        match &mut inline.kind {
            InlineSnapshotKind::Run(text) => {
                let output = owner.filter_input(text);
                if offsets.boundaries.is_some() {
                    let mut converted = output.chars();
                    for character in text.chars() {
                        let retained = character != '\u{7f}'
                            && (character != '\r' && character != '\n' || owner.accepts_return())
                            && (character != '\t' || owner.accepts_tab());
                        let count = if !retained {
                            0
                        } else {
                            match owner.character_casing() {
                                RichEditCharacterCasing::Normal => 1,
                                RichEditCharacterCasing::Lower => character.to_lowercase().count(),
                                RichEditCharacterCasing::Upper => character.to_uppercase().count(),
                            }
                        };
                        // Consume the actual whole-run conversion. Contextual
                        // final sigma changes a scalar's value, not its count;
                        // dotted-I and sharp-s can expand into several scalars.
                        let mut replacement_length = 0;
                        for _ in 0..count {
                            replacement_length +=
                                converted.next().ok_or_else(mapping_error)?.len_utf16();
                        }
                        offsets.scalar(character, replacement_length);
                    }
                    if converted.next().is_some() {
                        return Err(mapping_error());
                    }
                }
                *text = output;
            }
            InlineSnapshotKind::LineBreak => {
                let retained = owner.accepts_return();
                // The structured line break is CRLF on Windows and LF on
                // other backends. Both code units are scalar boundaries.
                offsets.unchanged_ascii(line_break_length, retained);
                if !retained {
                    continue;
                }
            }
            InlineSnapshotKind::Embedded(Some(_)) => offsets.scalar('\u{fffc}', 1),
            InlineSnapshotKind::Embedded(None) => {}
            _ => {
                if let Some(children) = inline.children_mut() {
                    filter_inlines(owner, children, offsets)?;
                }
            }
        }
        filtered.push(inline);
    }
    *inlines = filtered;
    Ok(())
}

fn mapping_error() -> DocumentError {
    DocumentError::InvalidFormat("Input casing could not preserve field boundaries.")
}
