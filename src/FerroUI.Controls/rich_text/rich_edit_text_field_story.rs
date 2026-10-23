//! Live projected Link services over the shared visible document. The projection
//! includes measured instruction strings and a synthetic terminal paragraph
//! mark; existing editor positions continue to count visible UTF-16 only.
use super::rich_text_field_story::{
    FieldLocation, FieldStory, FieldStoryError, FieldStoryOffset, FieldStoryRange,
    InstructionBoundary, LinkArgument, LinkField, LinkFieldId, VisibleCoverage, VisibleOffset,
};
use super::{EditError, RichEditTextDocument, RichTextEditor};
use crate::documents::{
    DocumentError, DocumentLinkField, DocumentSnapshot, TextGravity, TextPointer,
};
use std::cell::RefCell;
use std::fmt;
use std::ops::Range;
use std::rc::Rc;

/// A projected operation reports unsupported instruction cases before changing
/// the live document, selection, composition or history.
#[derive(Debug)]
pub enum TextFieldError {
    Edit(EditError),
    InvalidRange,
    OutOfBounds,
    InvalidUtf16Boundary,
    InvalidArgument,
    ParagraphOnly,
    InvalidProjection(&'static str),
    Unsupported(&'static str),
    StaleRange,
    CapacityExceeded,
}
impl fmt::Display for TextFieldError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Edit(error) => error.fmt(formatter),
            Self::InvalidRange => formatter.write_str("The projected range start exceeds its end."),
            Self::OutOfBounds => {
                formatter.write_str("The position exceeds the projected field story.")
            }
            Self::InvalidUtf16Boundary => {
                formatter.write_str("The text range splits a UTF-16 surrogate pair.")
            }
            Self::InvalidArgument => {
                formatter.write_str("The Link argument is invalid for this range.")
            }
            Self::ParagraphOnly => {
                formatter.write_str("A paragraph mark alone cannot become a link.")
            }
            Self::InvalidProjection(message) | Self::Unsupported(message) => {
                formatter.write_str(message)
            }
            Self::StaleRange => formatter.write_str(
                "The projected instruction or raw UTF-16 endpoint is no longer available.",
            ),
            Self::CapacityExceeded => {
                formatter.write_str("The field story exceeds its representable capacity.")
            }
        }
    }
}
impl std::error::Error for TextFieldError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Edit(error) => Some(error),
            _ => None,
        }
    }
}
impl From<EditError> for TextFieldError {
    fn from(error: EditError) -> Self {
        Self::Edit(error)
    }
}
impl From<DocumentError> for TextFieldError {
    fn from(error: DocumentError) -> Self {
        Self::Edit(error.into())
    }
}
impl From<FieldStoryError> for TextFieldError {
    fn from(error: FieldStoryError) -> Self {
        match error {
            FieldStoryError::InvalidRange => Self::InvalidRange,
            FieldStoryError::OutOfBounds => Self::OutOfBounds,
            FieldStoryError::InvalidUtf16Boundary => Self::InvalidUtf16Boundary,
            FieldStoryError::InvalidParagraphBreak => {
                Self::InvalidProjection("A structural paragraph separator is invalid.")
            }
            FieldStoryError::InvalidField(message) => Self::InvalidProjection(message),
            FieldStoryError::UnknownField | FieldStoryError::StaleRange => Self::StaleRange,
            FieldStoryError::InvalidArgument => Self::InvalidArgument,
            FieldStoryError::ParagraphOnly => Self::ParagraphOnly,
            FieldStoryError::Unsupported(message) => Self::Unsupported(message),
            FieldStoryError::CapacityExceeded => Self::CapacityExceeded,
        }
    }
}

/// Explicit conversion at a field-result start. Before includes the instruction;
/// After starts at the friendly text. Neither changes the visible caret.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FieldInstructionBoundary {
    #[default]
    Before,
    After,
}
impl From<FieldInstructionBoundary> for InstructionBoundary {
    fn from(value: FieldInstructionBoundary) -> Self {
        match value {
            FieldInstructionBoundary::Before => Self::Before,
            FieldInstructionBoundary::After => Self::After,
        }
    }
}

/// A view over the editor's current shared document, using separately tagged
/// instruction-inclusive UTF-16 coordinates. Visible range APIs are unchanged.
#[derive(Clone)]
pub struct RichEditTextFieldStory {
    editor: Rc<RichTextEditor>,
}

impl RichEditTextDocument {
    pub fn field_story(&self) -> RichEditTextFieldStory {
        RichEditTextFieldStory {
            editor: self.editor(),
        }
    }
}

struct Projection {
    snapshot: DocumentSnapshot,
    story: FieldStory,
    version: u64,
    editor_checkpoint: u64,
    authoritative_ids: Vec<u64>,
}

impl RichEditTextFieldStory {
    pub fn text(&self) -> Result<String, TextFieldError> {
        let projection = self.read()?;
        Ok(projection.story.text(&projection.story.whole_range())?)
    }
    pub fn text_length(&self) -> Result<usize, TextFieldError> {
        Ok(self.read()?.story.length())
    }

    pub fn project_visible(
        &self,
        offset: usize,
        boundary: FieldInstructionBoundary,
    ) -> Result<FieldStoryOffset, TextFieldError> {
        Ok(self
            .read()?
            .story
            .project_visible(VisibleOffset::new(offset), boundary.into())?)
    }

    /// Reads raw code units, including a requested half of a surrogate pair.
    pub fn utf16(&self, range: Range<FieldStoryOffset>) -> Result<Vec<u16>, TextFieldError> {
        let projection = self.read()?;
        let range = projection.story.range(range.start, range.end)?;
        Ok(projection.story.utf16(&range)?)
    }

    pub fn get_range(
        &self,
        start: FieldStoryOffset,
        end: FieldStoryOffset,
    ) -> Result<RichEditTextFieldRange, TextFieldError> {
        let projection = self.read()?;
        let range = projection.story.range(start, end)?;
        let endpoints = self.endpoints(&projection, &projection.story, &range)?;
        self.ensure_read(&projection)?;
        Ok(RichEditTextFieldRange {
            story: self.clone(),
            data: Rc::new(RefCell::new(RangeData {
                endpoints,
                generation: 0,
            })),
        })
    }

    /// Both boundaries precede an instruction beginning at that visible offset.
    /// Call project_visible with After when only the friendly result is wanted.
    pub fn get_range_from_visible(
        &self,
        range: Range<usize>,
    ) -> Result<RichEditTextFieldRange, TextFieldError> {
        let projection = self.read()?;
        let start = projection
            .story
            .project_visible(VisibleOffset::new(range.start), InstructionBoundary::Before)?;
        let end = projection
            .story
            .project_visible(VisibleOffset::new(range.end), InstructionBoundary::Before)?;
        let range = projection.story.range(start, end)?;
        let endpoints = self.endpoints(&projection, &projection.story, &range)?;
        self.ensure_read(&projection)?;
        Ok(RichEditTextFieldRange {
            story: self.clone(),
            data: Rc::new(RefCell::new(RangeData {
                endpoints,
                generation: 0,
            })),
        })
    }

    fn read(&self) -> Result<Projection, TextFieldError> {
        let document = self.editor.document();
        let version = document.mutation_version();
        let editor_checkpoint = self.editor.navigation_checkpoint();
        let snapshot = document.snapshot();
        self.ensure_version(version, editor_checkpoint)?;
        let authoritative_ids = snapshot.link_fields.iter().map(|field| field.id).collect();
        let promoted = snapshot.with_promoted_links(document.link_allocation_floor());
        self.ensure_version(version, editor_checkpoint)?;
        let snapshot = promoted?;
        // TargetName remains losslessly represented by the shared document and
        // RTF codec. Its raw instruction spelling has no measured oracle yet.
        if snapshot
            .link_fields
            .iter()
            .any(|field| field.target_name.is_some())
        {
            return Err(TextFieldError::Unsupported(
                "Projected instructions with TargetName have not been characterized.",
            ));
        }
        let fields = snapshot
            .link_fields
            .iter()
            .map(|field| {
                Ok(LinkField::new(
                    LinkFieldId::new(field.id),
                    VisibleOffset::new(field.range.start)..VisibleOffset::new(field.range.end),
                    LinkArgument::from_target(&field.target, field.has_sentinel)?,
                ))
            })
            .collect::<Result<Vec<_>, FieldStoryError>>()?;
        let story = FieldStory::from_fields_with_next_id(
            &snapshot.text(),
            snapshot
                .structural_paragraph_breaks()
                .into_iter()
                .map(VisibleOffset::new)
                .collect(),
            fields,
            snapshot.next_link_field_id,
        )?;
        self.ensure_version(version, editor_checkpoint)?;
        Ok(Projection {
            snapshot,
            story,
            version,
            editor_checkpoint,
            authoritative_ids,
        })
    }

    fn ensure_version(&self, version: u64, checkpoint: u64) -> Result<(), EditError> {
        if self.editor.document().mutation_version() != version {
            self.editor.observe_external_document_change();
            return Err(EditError::ReentrantEdit);
        }
        if self.editor.navigation_checkpoint() != checkpoint {
            return Err(EditError::ReentrantEdit);
        }
        Ok(())
    }
    fn ensure_read(&self, projection: &Projection) -> Result<(), EditError> {
        self.ensure_version(projection.version, projection.editor_checkpoint)
    }

    fn endpoints(
        &self,
        projection: &Projection,
        story: &FieldStory,
        range: &FieldStoryRange,
    ) -> Result<[Endpoint; 2], TextFieldError> {
        Ok([
            self.endpoint(projection, story, range.start(), TextGravity::Backward)?,
            self.endpoint(projection, story, range.end(), TextGravity::Forward)?,
        ])
    }
    fn endpoint(
        &self,
        projection: &Projection,
        story: &FieldStory,
        offset: FieldStoryOffset,
        gravity: TextGravity,
    ) -> Result<Endpoint, TextFieldError> {
        let visible = |offset: VisibleOffset,
                       boundary: InstructionBoundary|
         -> Result<Endpoint, TextFieldError> {
            if story.visible_boundary(offset) {
                Ok(Endpoint::Visible {
                    pointer: TextPointer::new(&self.editor.document(), offset.get(), gravity)?,
                    boundary,
                })
            } else {
                Ok(Endpoint::RawVisible {
                    offset,
                    version: projection.version,
                })
            }
        };
        match story.location(offset)? {
            FieldLocation::Visible(offset) => visible(offset, InstructionBoundary::After),
            FieldLocation::Instruction {
                field,
                utf16_offset: 0,
            } => {
                let start = story
                    .fields()
                    .iter()
                    .find(|entry| entry.id() == field)
                    .ok_or(TextFieldError::StaleRange)?
                    .result()
                    .start;
                visible(start, InstructionBoundary::Before)
            }
            FieldLocation::Instruction {
                field,
                utf16_offset,
            } => Ok(Endpoint::Instruction {
                field,
                utf16_offset,
                provisional_version: (!projection.authoritative_ids.contains(&field.get()))
                    .then_some(projection.version),
            }),
            FieldLocation::FinalParagraphMark { after } => {
                Ok(Endpoint::FinalParagraphMark { after })
            }
        }
    }
}

#[derive(Clone)]
enum Endpoint {
    Visible {
        pointer: TextPointer,
        boundary: InstructionBoundary,
    },
    /// Raw surrogate-interior positions are snapshot-bound; scalar positions
    /// use the shared pointer mechanism and its established insertion affinity.
    RawVisible {
        offset: VisibleOffset,
        version: u64,
    },
    Instruction {
        field: LinkFieldId,
        utf16_offset: usize,
        provisional_version: Option<u64>,
    },
    FinalParagraphMark {
        after: bool,
    },
}
impl Endpoint {
    fn resolve(&self, projection: &Projection) -> Result<FieldStoryOffset, TextFieldError> {
        match self {
            Self::Visible { pointer, boundary } => Ok(projection
                .story
                .project_visible(VisibleOffset::new(pointer.offset()), *boundary)?),
            Self::RawVisible { offset, version } => {
                if *version != projection.version {
                    return Err(TextFieldError::StaleRange);
                }
                Ok(projection
                    .story
                    .project_visible(*offset, InstructionBoundary::After)?)
            }
            Self::Instruction {
                field,
                utf16_offset,
                provisional_version,
            } => {
                if provisional_version.is_some_and(|version| version != projection.version) {
                    return Err(TextFieldError::StaleRange);
                }
                projection
                    .story
                    .resolve_location(FieldLocation::Instruction {
                        field: *field,
                        utf16_offset: *utf16_offset,
                    })
                    .map_err(|error| match error {
                        FieldStoryError::UnknownField | FieldStoryError::OutOfBounds => {
                            TextFieldError::StaleRange
                        }
                        error => error.into(),
                    })
            }
            Self::FinalParagraphMark { after } => Ok(projection
                .story
                .resolve_location(FieldLocation::FinalParagraphMark { after: *after })?),
        }
    }
}
struct RangeData {
    endpoints: [Endpoint; 2],
    generation: u64,
}

/// An independently movable range in the projected field story. Clones share
/// its position; duplicate creates a separate handle. Visible anchors track
/// ordinary model edits using FerroUI pointers. Instruction-interior anchors
/// become stale when their field disappears; native paired-range tracking has
/// not been established by the measured Link transition fixtures.
#[derive(Clone)]
pub struct RichEditTextFieldRange {
    story: RichEditTextFieldStory,
    data: Rc<RefCell<RangeData>>,
}
impl RichEditTextFieldRange {
    pub fn duplicate(&self) -> Self {
        let endpoints = self.data.borrow().endpoints.clone();
        Self {
            story: self.story.clone(),
            data: Rc::new(RefCell::new(RangeData {
                endpoints,
                generation: 0,
            })),
        }
    }
    pub fn range(&self) -> Result<Range<FieldStoryOffset>, TextFieldError> {
        let projection = self.story.read()?;
        let range = self.resolved(&projection)?;
        Ok(range.start()..range.end())
    }
    pub fn set_range(
        &self,
        start: FieldStoryOffset,
        end: FieldStoryOffset,
    ) -> Result<(), TextFieldError> {
        let generation = self.generation();
        let projection = self.story.read()?;
        let range = projection.story.range(start, end)?;
        let endpoints = self
            .story
            .endpoints(&projection, &projection.story, &range)?;
        self.story.ensure_read(&projection)?;
        self.ensure_generation(generation)?;
        self.replace_endpoints(endpoints);
        Ok(())
    }
    pub fn utf16(&self) -> Result<Vec<u16>, TextFieldError> {
        let projection = self.story.read()?;
        Ok(projection.story.utf16(&self.resolved(&projection)?)?)
    }
    pub fn text(&self) -> Result<String, TextFieldError> {
        let projection = self.story.read()?;
        Ok(projection.story.text(&self.resolved(&projection)?)?)
    }
    /// Returns only represented visible content; an instruction-only request
    /// has no visible range and is never redirected to the editor's caret.
    pub fn visible_range(&self) -> Result<Option<Range<usize>>, TextFieldError> {
        let projection = self.story.read()?;
        let range = self.resolved(&projection)?;
        Ok(match projection.story.visible_coverage(&range)? {
            VisibleCoverage::None => None,
            VisibleCoverage::Caret(offset) => Some(offset.get()..offset.get()),
            VisibleCoverage::Text(range) => Some(range.start.get()..range.end.get()),
        })
    }

    /// Returns the canonical quoted argument (including an optional sentinel).
    /// A successful nonempty getter moves only this acted range to the complete
    /// field. The editor's selection and history remain unchanged.
    pub fn get_link(&self) -> Result<Option<String>, TextFieldError> {
        let generation = self.generation();
        let projection = self.story.read()?;
        self.ensure_generation(generation)?;
        let lookup = projection.story.get_link(&self.resolved(&projection)?)?;
        let endpoints = self
            .story
            .endpoints(&projection, &projection.story, &lookup.range)?;
        self.story.ensure_read(&projection)?;
        self.ensure_generation(generation)?;
        self.replace_endpoints(endpoints);
        Ok(lookup.argument.map(|argument| argument.canonical()))
    }

    /// Applies a measured Link transition as one shared editor transaction.
    /// Argument validation and no-op detection precede composition cancellation.
    /// Valid edits preserve the independent directed selection/typing patch;
    /// their acted endpoints are installed before application notifications.
    pub fn set_link(&self, argument: Option<&str>) -> Result<bool, TextFieldError> {
        let editor = &self.story.editor;
        if editor.is_read_only() {
            return Err(EditError::ReadOnly.into());
        }
        if editor.is_in_operation() {
            return Err(EditError::ReentrantEdit.into());
        }
        let generation = self.generation();
        // An earlier direct model edit can retire history and advance the
        // editor checkpoint. Settle it before capturing this command's read;
        // a current preview is retained for validation and no-op detection.
        editor.synchronize();
        self.ensure_generation(generation)?;
        let initial = self.story.read()?;
        self.ensure_generation(generation)?;
        let initial_change = initial
            .story
            .set_link(&self.resolved(&initial)?, argument)?;
        if initial_change.change.is_none() {
            return Ok(false);
        }
        // Validate the semantic projection before settling preedit too: a
        // valid native argument can still exceed the shared nesting limit.
        let initial_candidate = candidate_for(&initial, &initial_change)?;
        self.story.ensure_read(&initial)?;
        self.ensure_generation(generation)?;
        let composed = editor.prepare_edit()?;
        self.ensure_generation(generation)?;
        // Cancelling preedit can change both visible text and the field table.
        // Rebuild and resolve live anchors against the committed image.
        let (projection, mutation, candidate) = if composed {
            let projection = self.story.read()?;
            self.ensure_generation(generation)?;
            let mutation = projection
                .story
                .set_link(&self.resolved(&projection)?, argument)?;
            if mutation.change.is_none() {
                return Ok(false);
            }
            let candidate = candidate_for(&projection, &mutation)?;
            (projection, mutation, candidate)
        } else {
            (initial, initial_change, initial_candidate)
        };
        let endpoints = self
            .story
            .endpoints(&projection, &mutation.story, &mutation.range)?;
        self.story.ensure_read(&projection)?;
        self.ensure_generation(generation)?;
        let before_endpoints = self.data.borrow().endpoints.clone();
        let external_generation = editor.external_change_generation();
        let mut owned_generation = generation;
        let result = editor.edit_group(|editor| {
            self.story.ensure_read(&projection)?;
            self.ensure_generation(generation)?;
            editor.install_field_snapshot(&candidate, projection.version)?;
            self.ensure_generation(generation)?;
            owned_generation = self.replace_endpoints(endpoints);
            Ok(true)
        });
        if result.is_err()
            && editor.external_change_generation() == external_generation
            && self.generation() == owned_generation
        {
            self.replace_endpoints(before_endpoints);
        }
        result.map_err(Into::into)
    }

    fn resolved(&self, projection: &Projection) -> Result<FieldStoryRange, TextFieldError> {
        let endpoints = self.data.borrow().endpoints.clone();
        let start = endpoints[0].resolve(projection)?;
        let end = endpoints[1].resolve(projection)?;
        Ok(projection.story.range(start.min(end), start.max(end))?)
    }
    fn generation(&self) -> u64 {
        self.data.borrow().generation
    }
    fn ensure_generation(&self, generation: u64) -> Result<(), EditError> {
        if self.generation() == generation {
            Ok(())
        } else {
            Err(EditError::ReentrantEdit)
        }
    }
    fn replace_endpoints(&self, endpoints: [Endpoint; 2]) -> u64 {
        let mut data = self.data.borrow_mut();
        data.endpoints = endpoints;
        data.generation = data.generation.wrapping_add(1);
        data.generation
    }
}

fn candidate_for(
    projection: &Projection,
    mutation: &super::rich_text_link::LinkMutation,
) -> Result<DocumentSnapshot, TextFieldError> {
    let change = mutation
        .change
        .as_ref()
        .ok_or(TextFieldError::InvalidArgument)?;
    if let Some(after) = &change.after {
        let visible = after.result().start.get()..after.result().end.get();
        if !projection
            .snapshot
            .field_result_preserves_inline_boundaries(visible)
        {
            return Err(TextFieldError::Unsupported("A Link result cannot divide the platform representation of a structured LineBreak."));
        }
    }
    let mut candidate = projection.snapshot.clone();
    let after = change.after.as_ref().map(|field| DocumentLinkField {
        id: field.id().get(),
        range: field.result().start.get()..field.result().end.get(),
        target: field.argument().target().to_owned(),
        target_name: None,
        has_sentinel: field.argument().has_sentinel(),
    });
    candidate.set_link_field(
        change.id.get(),
        after,
        mutation.story.next_field_id_watermark(),
    )?;
    Ok(candidate)
}

#[cfg(test)]
#[path = "rich_edit_text_field_story_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "rich_edit_text_field_installation_tests.rs"]
mod installation_tests;
