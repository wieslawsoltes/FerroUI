//! Immutable instruction-inclusive text projection. The visual document keeps
//! its existing UTF-16 offsets; only this separately typed projection includes
//! link instruction strings and the final paragraph mark.
use std::collections::HashSet;
use std::fmt;
use std::ops::Range;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct VisibleOffset(usize);

impl VisibleOffset {
    pub(crate) const fn new(value: usize) -> Self {
        Self(value)
    }
    pub(crate) const fn get(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FieldStoryOffset(usize);

impl FieldStoryOffset {
    pub const fn new(value: usize) -> Self {
        Self(value)
    }
    pub const fn get(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct LinkFieldId(u64);

impl LinkFieldId {
    pub(crate) const fn new(value: u64) -> Self {
        Self(value)
    }
    pub(crate) const fn get(self) -> u64 {
        self.0
    }
}

/// Canonical quoted argument, with the optional sentinel stored separately
/// from the navigation target. Targets are not URI-validated.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LinkArgument {
    quoted_target: Rc<str>,
    sentinel: bool,
}

impl LinkArgument {
    pub(crate) fn from_target(target: &str, sentinel: bool) -> Result<Self, FieldStoryError> {
        if target.is_empty() || target.contains(['\0', '"']) {
            return Err(FieldStoryError::InvalidArgument);
        }
        target
            .len()
            .checked_add(2)
            .ok_or(FieldStoryError::CapacityExceeded)?;
        Ok(Self {
            quoted_target: format!("\"{target}\"").into(),
            sentinel,
        })
    }

    pub(crate) fn target(&self) -> &str {
        &self.quoted_target[1..self.quoted_target.len() - 1]
    }

    pub(crate) fn has_sentinel(&self) -> bool {
        self.sentinel
    }

    pub(crate) fn canonical(&self) -> String {
        if self.sentinel {
            format!("\u{fddf}{}", self.quoted_target)
        } else {
            self.quoted_target.to_string()
        }
    }

    pub(super) fn instruction(&self) -> Result<Rc<[u16]>, FieldStoryError> {
        let length = self
            .quoted_target
            .encode_utf16()
            .count()
            .checked_add(10 + usize::from(self.sentinel))
            .ok_or(FieldStoryError::CapacityExceeded)?;
        let mut result = Vec::new();
        result
            .try_reserve_exact(length)
            .map_err(|_| FieldStoryError::CapacityExceeded)?;
        if self.sentinel {
            result.push(0xfddf);
        }
        result.extend("HYPERLINK ".encode_utf16());
        result.extend(self.quoted_target.encode_utf16());
        Ok(result.into())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LinkField {
    id: LinkFieldId,
    result: Range<VisibleOffset>,
    argument: LinkArgument,
}

impl LinkField {
    pub(crate) fn new(
        id: LinkFieldId,
        result: Range<VisibleOffset>,
        argument: LinkArgument,
    ) -> Self {
        Self {
            id,
            result,
            argument,
        }
    }
    pub(crate) fn id(&self) -> LinkFieldId {
        self.id
    }
    pub(crate) fn result(&self) -> Range<VisibleOffset> {
        self.result.clone()
    }
    pub(crate) fn argument(&self) -> &LinkArgument {
        &self.argument
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InstructionBoundary {
    Before,
    After,
}

/// A projected offset is not necessarily a visible caret position. In
/// particular, instruction locations never silently become a document caret.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FieldLocation {
    Visible(VisibleOffset),
    Instruction {
        field: LinkFieldId,
        utf16_offset: usize,
    },
    FinalParagraphMark {
        after: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum VisibleCoverage {
    None,
    Caret(VisibleOffset),
    Text(Range<VisibleOffset>),
}

/// A range belongs to one immutable projection. Old ranges remain usable with
/// their old story and explicitly fail when passed to a replacement story.
#[derive(Clone)]
pub(crate) struct FieldStoryRange {
    identity: Rc<()>,
    start: FieldStoryOffset,
    end: FieldStoryOffset,
}

impl FieldStoryRange {
    pub(crate) fn start(&self) -> FieldStoryOffset {
        self.start
    }
    pub(crate) fn end(&self) -> FieldStoryOffset {
        self.end
    }
    pub(crate) fn is_empty(&self) -> bool {
        self.start == self.end
    }
}

impl fmt::Debug for FieldStoryRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FieldStoryRange")
            .field("start", &self.start)
            .field("end", &self.end)
            .finish()
    }
}

impl PartialEq for FieldStoryRange {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.identity, &other.identity)
            && self.start == other.start
            && self.end == other.end
    }
}
impl Eq for FieldStoryRange {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum FieldStoryError {
    InvalidRange,
    OutOfBounds,
    InvalidUtf16Boundary,
    InvalidParagraphBreak,
    InvalidField(&'static str),
    UnknownField,
    InvalidArgument,
    ParagraphOnly,
    Unsupported(&'static str),
    StaleRange,
    CapacityExceeded,
}

impl fmt::Display for FieldStoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRange => f.write_str("The range start exceeds its end."),
            Self::OutOfBounds => f.write_str("The position exceeds this text projection."),
            Self::InvalidUtf16Boundary => {
                f.write_str("The text range splits a UTF-16 surrogate pair.")
            }
            Self::InvalidParagraphBreak => {
                f.write_str("Paragraph breaks must be ordered, distinct LF positions.")
            }
            Self::InvalidField(message) | Self::Unsupported(message) => f.write_str(message),
            Self::UnknownField => f.write_str("The field does not belong to this projection."),
            Self::InvalidArgument => f.write_str("The link argument is invalid for this range."),
            Self::ParagraphOnly => f.write_str("A paragraph mark alone cannot become a link."),
            Self::StaleRange => {
                f.write_str("The range belongs to an earlier or different text projection.")
            }
            Self::CapacityExceeded => {
                f.write_str("The text projection exceeds its representable capacity.")
            }
        }
    }
}
impl std::error::Error for FieldStoryError {}

#[derive(Clone, Debug)]
pub(super) struct ProjectedField {
    pub(super) field_index: usize,
    pub(super) instruction_start: usize,
    pub(super) result_start: usize,
    pub(super) end: usize,
    pub(super) instruction: Rc<[u16]>,
    hidden_after: usize,
}

#[derive(Clone, Copy, Debug)]
enum SegmentKind {
    Visible { source_start: usize },
    Instruction { field_index: usize },
    FinalParagraphMark,
}

#[derive(Clone, Debug)]
struct Segment {
    range: Range<usize>,
    kind: SegmentKind,
}

#[derive(Debug)]
struct FieldStoryData {
    identity: Rc<()>,
    visible: Rc<str>,
    visible_units: Rc<[u16]>,
    projected_visible_units: Rc<[u16]>,
    paragraph_breaks: Rc<[VisibleOffset]>,
    fields: Rc<[LinkField]>,
    projected_fields: Rc<[ProjectedField]>,
    segments: Rc<[Segment]>,
    next_id: u64,
    length: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct FieldStory(Rc<FieldStoryData>);

impl FieldStory {
    /// `visible` is already in the document's coordinate space. Only explicitly
    /// identified structural LF separators become CR in the raw projection;
    /// literal run text and inline line-break sequences retain their units.
    pub(crate) fn new(
        visible: &str,
        paragraph_breaks: Vec<VisibleOffset>,
    ) -> Result<Self, FieldStoryError> {
        Self::from_fields(visible, paragraph_breaks, Vec::new())
    }

    pub(crate) fn from_fields(
        visible: &str,
        paragraph_breaks: Vec<VisibleOffset>,
        fields: Vec<LinkField>,
    ) -> Result<Self, FieldStoryError> {
        let next_id = fields
            .iter()
            .map(|field| field.id.get())
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or(FieldStoryError::CapacityExceeded)?;
        Self::from_fields_with_next_id(visible, paragraph_breaks, fields, next_id)
    }

    pub(crate) fn from_fields_with_next_id(
        visible: &str,
        paragraph_breaks: Vec<VisibleOffset>,
        fields: Vec<LinkField>,
        next_id: u64,
    ) -> Result<Self, FieldStoryError> {
        if next_id == 0 || fields.iter().any(|field| field.id.get() >= next_id) {
            return Err(FieldStoryError::InvalidField(
                "Field identities must precede the allocation watermark.",
            ));
        }
        let units: Rc<[u16]> = visible.encode_utf16().collect::<Vec<_>>().into();
        let mut previous = None;
        for position in &paragraph_breaks {
            if previous.is_some_and(|previous| previous >= *position)
                || units.get(position.get()) != Some(&0x000a)
            {
                return Err(FieldStoryError::InvalidParagraphBreak);
            }
            previous = Some(*position);
        }
        let projected_units = if paragraph_breaks.is_empty() {
            units.clone()
        } else {
            let mut projected = units.to_vec();
            for position in &paragraph_breaks {
                projected[position.get()] = 0x000d;
            }
            projected.into()
        };
        Self::build(
            visible.into(),
            units,
            projected_units,
            paragraph_breaks.into(),
            fields,
            next_id,
        )
    }

    fn build(
        visible: Rc<str>,
        visible_units: Rc<[u16]>,
        projected_visible_units: Rc<[u16]>,
        paragraph_breaks: Rc<[VisibleOffset]>,
        mut fields: Vec<LinkField>,
        next_id: u64,
    ) -> Result<Self, FieldStoryError> {
        fields.sort_by_key(|field| field.result.start);
        let mut ids = HashSet::new();
        let mut previous_end = 0;
        let mut hidden = 0usize;
        let mut projected_fields = Vec::new();
        let mut segments = Vec::new();
        let mut cursor = 0usize;
        for (field_index, field) in fields.iter().enumerate() {
            let start = field.result.start.get();
            let end = field.result.end.get();
            if field.id.get() == 0 || !ids.insert(field.id) {
                return Err(FieldStoryError::InvalidField(
                    "Field identities must be nonzero and distinct.",
                ));
            }
            if start >= end || start < previous_end || end > visible_units.len() {
                return Err(FieldStoryError::InvalidField(
                    "Field results must be nonempty, disjoint visible ranges.",
                ));
            }
            if !scalar_boundary(&visible_units, start) || !scalar_boundary(&visible_units, end) {
                return Err(FieldStoryError::InvalidUtf16Boundary);
            }
            let instruction = field.argument.instruction()?;
            let instruction_start = start
                .checked_add(hidden)
                .ok_or(FieldStoryError::CapacityExceeded)?;
            if start > previous_end {
                segments.push(Segment {
                    range: cursor..instruction_start,
                    kind: SegmentKind::Visible {
                        source_start: previous_end,
                    },
                });
            }
            let result_start = instruction_start
                .checked_add(instruction.len())
                .ok_or(FieldStoryError::CapacityExceeded)?;
            hidden = hidden
                .checked_add(instruction.len())
                .ok_or(FieldStoryError::CapacityExceeded)?;
            let projected_end = end
                .checked_add(hidden)
                .ok_or(FieldStoryError::CapacityExceeded)?;
            segments.push(Segment {
                range: instruction_start..result_start,
                kind: SegmentKind::Instruction { field_index },
            });
            segments.push(Segment {
                range: result_start..projected_end,
                kind: SegmentKind::Visible {
                    source_start: start,
                },
            });
            projected_fields.push(ProjectedField {
                field_index,
                instruction_start,
                result_start,
                end: projected_end,
                instruction,
                hidden_after: hidden,
            });
            previous_end = end;
            cursor = projected_end;
        }
        let before_final = visible_units
            .len()
            .checked_add(hidden)
            .ok_or(FieldStoryError::CapacityExceeded)?;
        if previous_end < visible_units.len() {
            segments.push(Segment {
                range: cursor..before_final,
                kind: SegmentKind::Visible {
                    source_start: previous_end,
                },
            });
        }
        let length = before_final
            .checked_add(1)
            .ok_or(FieldStoryError::CapacityExceeded)?;
        segments.push(Segment {
            range: before_final..length,
            kind: SegmentKind::FinalParagraphMark,
        });
        Ok(Self(Rc::new(FieldStoryData {
            identity: Rc::new(()),
            visible,
            visible_units,
            projected_visible_units,
            paragraph_breaks,
            fields: fields.into(),
            projected_fields: projected_fields.into(),
            segments: segments.into(),
            next_id,
            length,
        })))
    }

    pub(crate) fn visible_text(&self) -> &str {
        &self.0.visible
    }
    pub(crate) fn visible_length(&self) -> usize {
        self.0.visible_units.len()
    }
    pub(crate) fn length(&self) -> usize {
        self.0.length
    }
    pub(crate) fn next_field_id_watermark(&self) -> u64 {
        self.0.next_id
    }
    pub(crate) fn fields(&self) -> &[LinkField] {
        &self.0.fields
    }
    pub(crate) fn paragraph_breaks(&self) -> &[VisibleOffset] {
        &self.0.paragraph_breaks
    }
    pub(crate) fn same_snapshot(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0.identity, &other.0.identity)
    }

    pub(crate) fn range(
        &self,
        start: FieldStoryOffset,
        end: FieldStoryOffset,
    ) -> Result<FieldStoryRange, FieldStoryError> {
        if start > end {
            return Err(FieldStoryError::InvalidRange);
        }
        if end.get() > self.length() {
            return Err(FieldStoryError::OutOfBounds);
        }
        Ok(FieldStoryRange {
            identity: self.0.identity.clone(),
            start,
            end,
        })
    }

    pub(crate) fn whole_range(&self) -> FieldStoryRange {
        FieldStoryRange {
            identity: self.0.identity.clone(),
            start: FieldStoryOffset(0),
            end: FieldStoryOffset(self.length()),
        }
    }

    pub(super) fn ensure_range(&self, range: &FieldStoryRange) -> Result<(), FieldStoryError> {
        if !Rc::ptr_eq(&self.0.identity, &range.identity) {
            return Err(FieldStoryError::StaleRange);
        }
        Ok(())
    }

    pub(crate) fn project_visible(
        &self,
        position: VisibleOffset,
        boundary: InstructionBoundary,
    ) -> Result<FieldStoryOffset, FieldStoryError> {
        if position.get() > self.visible_length() {
            return Err(FieldStoryError::OutOfBounds);
        }
        let count = self.0.fields.partition_point(|field| {
            field.result.start < position
                || (field.result.start == position && boundary == InstructionBoundary::After)
        });
        let hidden = count
            .checked_sub(1)
            .map_or(0, |index| self.0.projected_fields[index].hidden_after);
        Ok(FieldStoryOffset(position.get() + hidden))
    }

    pub(crate) fn location(
        &self,
        position: FieldStoryOffset,
    ) -> Result<FieldLocation, FieldStoryError> {
        let position = position.get();
        if position > self.length() {
            return Err(FieldStoryError::OutOfBounds);
        }
        if position >= self.length() - 1 {
            return Ok(FieldLocation::FinalParagraphMark {
                after: position == self.length(),
            });
        }
        let count = self
            .0
            .projected_fields
            .partition_point(|field| field.instruction_start <= position);
        let hidden = if let Some(index) = count.checked_sub(1) {
            let projected = &self.0.projected_fields[index];
            if position < projected.result_start {
                return Ok(FieldLocation::Instruction {
                    field: self.fields()[index].id,
                    utf16_offset: position - projected.instruction_start,
                });
            }
            projected.hidden_after
        } else {
            0
        };
        Ok(FieldLocation::Visible(VisibleOffset(position - hidden)))
    }

    /// Resolves a tagged location. Visible locations resolve after any
    /// instruction at that boundary, making `location` round-trip exactly.
    pub(crate) fn resolve_location(
        &self,
        location: FieldLocation,
    ) -> Result<FieldStoryOffset, FieldStoryError> {
        match location {
            FieldLocation::Visible(position) => {
                self.project_visible(position, InstructionBoundary::After)
            }
            FieldLocation::Instruction {
                field,
                utf16_offset,
            } => {
                let index = self
                    .fields()
                    .iter()
                    .position(|candidate| candidate.id == field)
                    .ok_or(FieldStoryError::UnknownField)?;
                let projected = &self.0.projected_fields[index];
                if utf16_offset >= projected.instruction.len() {
                    return Err(FieldStoryError::OutOfBounds);
                }
                Ok(FieldStoryOffset(projected.instruction_start + utf16_offset))
            }
            FieldLocation::FinalParagraphMark { after } => {
                Ok(FieldStoryOffset(self.length() - usize::from(!after)))
            }
        }
    }

    pub(crate) fn visible_coverage(
        &self,
        range: &FieldStoryRange,
    ) -> Result<VisibleCoverage, FieldStoryError> {
        self.ensure_range(range)?;
        if range.is_empty() {
            return Ok(match self.location(range.start)? {
                FieldLocation::Visible(position) => VisibleCoverage::Caret(position),
                FieldLocation::FinalParagraphMark { after: false } => {
                    VisibleCoverage::Caret(VisibleOffset(self.visible_length()))
                }
                _ => VisibleCoverage::None,
            });
        }
        let mut result: Option<Range<VisibleOffset>> = None;
        for segment in self.intersecting_segments(range) {
            if let SegmentKind::Visible { source_start } = segment.kind {
                let start =
                    source_start + range.start.get().max(segment.range.start) - segment.range.start;
                let end =
                    source_start + range.end.get().min(segment.range.end) - segment.range.start;
                if let Some(result) = &mut result {
                    result.end = VisibleOffset(end);
                } else {
                    result = Some(VisibleOffset(start)..VisibleOffset(end));
                }
            }
        }
        Ok(result.map_or(VisibleCoverage::None, VisibleCoverage::Text))
    }

    fn intersecting_segments<'a>(
        &'a self,
        range: &'a FieldStoryRange,
    ) -> impl Iterator<Item = &'a Segment> {
        let first = self
            .0
            .segments
            .partition_point(|segment| segment.range.end <= range.start.get());
        self.0.segments[first..]
            .iter()
            .take_while(move |segment| segment.range.start < range.end.get())
    }

    /// Raw code units allow callers to inspect a UTF-16 range without inventing
    /// replacement characters. `text` separately requires scalar boundaries.
    pub(crate) fn utf16(&self, range: &FieldStoryRange) -> Result<Vec<u16>, FieldStoryError> {
        self.ensure_range(range)?;
        let mut result = Vec::new();
        result
            .try_reserve_exact(range.end.get() - range.start.get())
            .map_err(|_| FieldStoryError::CapacityExceeded)?;
        for segment in self.intersecting_segments(range) {
            let start = range.start.get().max(segment.range.start) - segment.range.start;
            let end = range.end.get().min(segment.range.end) - segment.range.start;
            match segment.kind {
                SegmentKind::Visible { source_start } => result.extend_from_slice(
                    &self.0.projected_visible_units[source_start + start..source_start + end],
                ),
                SegmentKind::Instruction { field_index } => result.extend_from_slice(
                    &self.0.projected_fields[field_index].instruction[start..end],
                ),
                SegmentKind::FinalParagraphMark => result.extend_from_slice(&[0x000d][start..end]),
            }
        }
        Ok(result)
    }

    pub(crate) fn text(&self, range: &FieldStoryRange) -> Result<String, FieldStoryError> {
        String::from_utf16(&self.utf16(range)?).map_err(|_| FieldStoryError::InvalidUtf16Boundary)
    }

    pub(super) fn projected_fields(&self) -> &[ProjectedField] {
        &self.0.projected_fields
    }

    pub(super) fn visible_boundary(&self, position: VisibleOffset) -> bool {
        position.get() <= self.visible_length()
            && scalar_boundary(&self.0.visible_units, position.get())
    }

    pub(super) fn visible_slice(
        &self,
        range: Range<VisibleOffset>,
    ) -> Result<String, FieldStoryError> {
        if range.start > range.end {
            return Err(FieldStoryError::InvalidRange);
        }
        if range.end.get() > self.visible_length() {
            return Err(FieldStoryError::OutOfBounds);
        }
        String::from_utf16(&self.0.visible_units[range.start.get()..range.end.get()])
            .map_err(|_| FieldStoryError::InvalidUtf16Boundary)
    }

    pub(super) fn next_field_id(&self) -> Result<LinkFieldId, FieldStoryError> {
        self.0
            .next_id
            .checked_add(1)
            .ok_or(FieldStoryError::CapacityExceeded)?;
        Ok(LinkFieldId(self.0.next_id))
    }

    pub(super) fn replace_fields(
        &self,
        fields: Vec<LinkField>,
        allocated: bool,
    ) -> Result<Self, FieldStoryError> {
        let next_id = if allocated {
            self.0
                .next_id
                .checked_add(1)
                .ok_or(FieldStoryError::CapacityExceeded)?
        } else {
            self.0.next_id
        };
        Self::build(
            self.0.visible.clone(),
            self.0.visible_units.clone(),
            self.0.projected_visible_units.clone(),
            self.0.paragraph_breaks.clone(),
            fields,
            next_id,
        )
    }
}

fn scalar_boundary(units: &[u16], position: usize) -> bool {
    position == 0
        || position == units.len()
        || !((0xd800..=0xdbff).contains(&units[position - 1])
            && (0xdc00..=0xdfff).contains(&units[position]))
}
