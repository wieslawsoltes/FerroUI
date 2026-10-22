use super::{EditError, EditKind, EditorSelection, RichTextEditor, RtfError, RtfReader, RtfWriter};
use crate::documents::{
    CharacterFormat, DocumentError, DocumentSnapshot, InlineSnapshot, InlineSnapshotKind,
    Paragraph, ParagraphFormat, RichTextDocument, TextGravity, TextPointer,
};
use crate::RichTextBlock;
use ferroui_base::media::text_formatting::unicode::{GraphemeEnumerator, WordBreakEnumerator};
use ferroui_base::{Point, Rect};
use std::cell::RefCell;
use std::fmt;
use std::ops::Range;
use std::rc::Rc;

#[path = "rich_edit_text_stream.rs"]
mod stream;
pub use stream::{TextStreamEncoding, TextStreamOptions};

#[path = "rich_edit_text_case.rs"]
mod case;
pub use case::LetterCase;

#[path = "rich_edit_text_range_operations.rs"]
mod range_operations;

#[path = "rich_edit_text_clipboard.rs"]
mod clipboard;

#[path = "rich_edit_text_selection_navigation.rs"]
mod selection_navigation;

#[path = "rich_edit_text_selection_movement.rs"]
mod selection_movement;

#[path = "rich_edit_text_selection_options.rs"]
mod selection_options;
pub use selection_options::{SelectionOptions, SelectionType};

#[path = "rich_edit_text_range_gravity.rs"]
mod range_gravity;
pub use range_gravity::RangeGravity;

#[path = "rich_edit_text_units.rs"]
mod units;
use units::TextUnits;

/// Implemented TOM units, with their native numeric values. Display lines use the
/// editor's associated view and otherwise follow the native paragraph fallback.
/// Range Character and selection MoveBy traverse Unicode scalars without
/// splitting surrogate pairs. Selection MoveLeft/MoveRight follow grapheme
/// boundaries, matching their arrow-command contract.
/// Cluster uses the associated view's shaping output and requires a complete,
/// untrimmed layout. It never substitutes Unicode grapheme boundaries.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum TextRangeUnit {
    Character = 0,
    Word = 1,
    Sentence = 2,
    Paragraph = 3,
    Line = 4,
    Story = 5,
    HardParagraph = 12,
    Cluster = 13,
}
impl TextRangeUnit {
    /// Compatibility name for the one story represented by a FerroUI document.
    #[allow(non_upper_case_globals)]
    pub const Document: Self = Self::Story;
}
ferroui_base::ferro_markup_enum!(TextRangeUnit {
    Character, Word, Sentence, Paragraph, Line, Story, HardParagraph, Cluster,
}, { namespace: "FerroUI.Controls" });

/// Framework Unicode boundaries used by search and automation. These deliberately
/// remain separate from TOM's whitespace-inclusive navigation units.
#[derive(Clone, Copy)]
pub(crate) enum UnicodeTextUnit {
    Character,
    Word,
    Paragraph,
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct TextFindOptions: u32 {
        const MATCH_CASE = 1;
        const WHOLE_WORD = 2;
        const BACKWARDS = 4;
    }

    /// Public text-object-model search flags. Direction belongs to the signed
    /// scan length; the convenience TextFindOptions also offers Backwards.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct FindOptions: u32 {
        const NONE = 0;
        const WORD = 2;
        const CASE = 4;
    }
}

bitflags::bitflags! {
    /// Native text retrieval flags. Unsupported content projections return an error.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct TextGetOptions: u32 {
        const NONE = 0;
        const ADJUST_CRLF = 1;
        const USE_CRLF = 2;
        const USE_OBJECT_TEXT = 4;
        const ALLOW_FINAL_EOP = 8;
        const NO_HIDDEN = 32;
        const INCLUDE_NUMBERING = 64;
        const FORMAT_RTF = 8192;
        const USE_LF = 16777216;
    }

    /// Native text insertion flags. Plain insertion and the supported RTF grammar
    /// share the same document transaction and length-limit policy.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct TextSetOptions: u32 {
        const NONE = 0;
        const UNICODE_BIDI = 1;
        const UNLINK = 8;
        const UNHIDE = 16;
        const CHECK_TEXT_LIMIT = 32;
        const FORMAT_RTF = 8192;
        const APPLY_RTF_DOCUMENT_DEFAULTS = 16384;
    }
}

ferroui_base::ferro_markup_enum!(flags FindOptions {
    None = FindOptions::NONE,
    Word = FindOptions::WORD,
    Case = FindOptions::CASE,
}, { namespace: "FerroUI.Controls" });

ferroui_base::ferro_markup_enum!(flags TextGetOptions {
    None = TextGetOptions::NONE,
    AdjustCrlf = TextGetOptions::ADJUST_CRLF,
    UseCrlf = TextGetOptions::USE_CRLF,
    UseObjectText = TextGetOptions::USE_OBJECT_TEXT,
    AllowFinalEop = TextGetOptions::ALLOW_FINAL_EOP,
    NoHidden = TextGetOptions::NO_HIDDEN,
    IncludeNumbering = TextGetOptions::INCLUDE_NUMBERING,
    FormatRtf = TextGetOptions::FORMAT_RTF,
    UseLf = TextGetOptions::USE_LF,
}, { namespace: "FerroUI.Controls" });

ferroui_base::ferro_markup_enum!(flags TextSetOptions {
    None = TextSetOptions::NONE,
    UnicodeBidi = TextSetOptions::UNICODE_BIDI,
    Unlink = TextSetOptions::UNLINK,
    Unhide = TextSetOptions::UNHIDE,
    CheckTextLimit = TextSetOptions::CHECK_TEXT_LIMIT,
    FormatRtf = TextSetOptions::FORMAT_RTF,
    ApplyRtfDocumentDefaults = TextSetOptions::APPLY_RTF_DOCUMENT_DEFAULTS,
}, { namespace: "FerroUI.Controls" });

/// Failure to interpret a text operation. Unsupported flags are never silently
/// ignored, and parse/option validation finishes before an edit can cancel IME.
#[derive(Debug)]
pub enum TextOperationError {
    InvalidOptions(&'static str),
    UnsupportedOptions { operation: &'static str, bits: u32 },
    Edit(EditError),
    Rtf(RtfError),
    Io(std::io::Error),
    InvalidEncoding(&'static str),
    StreamLimitExceeded { limit: usize },
}

impl fmt::Display for TextOperationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidOptions(message) => f.write_str(message),
            Self::UnsupportedOptions { operation, bits } => {
                write!(f, "{operation} does not implement option bits 0x{bits:X}.")
            }
            Self::Edit(error) => error.fmt(f),
            Self::Rtf(error) => error.fmt(f),
            Self::Io(error) => error.fmt(f),
            Self::InvalidEncoding(message) => f.write_str(message),
            Self::StreamLimitExceeded { limit } => {
                write!(f, "The text stream exceeds its {limit}-byte limit.")
            }
        }
    }
}
impl std::error::Error for TextOperationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Edit(error) => Some(error),
            Self::Rtf(error) => Some(error),
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}
impl From<EditError> for TextOperationError {
    fn from(error: EditError) -> Self {
        Self::Edit(error)
    }
}
impl From<DocumentError> for TextOperationError {
    fn from(error: DocumentError) -> Self {
        Self::Edit(error.into())
    }
}
impl From<RtfError> for TextOperationError {
    fn from(error: RtfError) -> Self {
        Self::Rtf(error)
    }
}
impl From<std::io::Error> for TextOperationError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

/// A formatting query distinguishes uniform values from mixed selections.
#[derive(Clone, Debug, PartialEq)]
pub enum TextFormatValue<T> {
    Uniform(T),
    Mixed,
}

/// Editing services over a shared FerroUI document. Range objects and selection commands
/// use the same transaction history as the visual editor, and can also be used headlessly.
#[derive(Clone)]
pub struct RichEditTextDocument {
    editor: Rc<RichTextEditor>,
}

impl RichEditTextDocument {
    pub fn new(document: RichTextDocument) -> Self {
        Self::from_editor(Rc::new(RichTextEditor::new(document)))
    }
    pub fn from_editor(editor: Rc<RichTextEditor>) -> Self {
        Self { editor }
    }
    pub fn document(&self) -> RichTextDocument {
        self.editor.document()
    }
    pub fn editor(&self) -> Rc<RichTextEditor> {
        self.editor.clone()
    }
    pub fn text(&self) -> String {
        self.document().text()
    }
    pub fn text_length(&self) -> usize {
        self.document().text_length()
    }
    pub fn selection(&self) -> RichEditTextSelection {
        RichEditTextSelection {
            editor: self.editor.clone(),
        }
    }
    pub fn get_range(&self, start: usize, end: usize) -> Result<RichEditTextRange, EditError> {
        RichEditTextRange::new(self.editor.clone(), start.min(end)..start.max(end))
    }
    pub fn document_range(&self) -> Result<RichEditTextRange, EditError> {
        self.get_range(0, self.text_length())
    }
    /// Hit tests the host's measured, shaped layout in local control coordinates.
    pub fn get_range_from_point(
        &self,
        view: &RichTextBlock,
        point: Point,
    ) -> Result<RichEditTextRange, EditError> {
        ensure_view_document(&self.document(), view)?;
        let position = view.hit_test_text_position(point);
        self.get_range(position, position)
    }
    pub fn set_text(&self, text: &str) -> Result<(), EditError> {
        let snapshot = self.plain_document(text);
        self.replace_document(&snapshot, true)
    }

    pub fn get_default_character_format(&self) -> CharacterFormat {
        self.editor.default_character_format()
    }

    pub fn set_default_character_format(&self, format: &CharacterFormat) -> Result<(), EditError> {
        self.editor.set_default_character_format(format)
    }

    pub fn get_default_paragraph_format(&self) -> ParagraphFormat {
        self.editor.default_paragraph_format()
    }

    pub fn set_default_paragraph_format(&self, format: &ParagraphFormat) -> Result<(), EditError> {
        self.editor.set_default_paragraph_format(format)
    }

    /// Default tab spacing in TOM points; shared by all facades of this editor.
    pub fn default_tab_stop(&self) -> f64 {
        self.editor
            .default_paragraph_format()
            .default_tab_stop
            .unwrap_or(48.0)
            * (72.0 / 96.0)
    }

    /// Applies document-wide implicit tab spacing as one undoable format edit.
    /// Explicit paragraph stops and unrelated formatting are retained.
    pub fn set_default_tab_stop(&self, points: f64) -> Result<(), EditError> {
        let patch = ParagraphFormat {
            default_tab_stop: Some(points * (96.0 / 72.0)),
            ..Default::default()
        };
        patch.validate()?;
        self.editor.edit_group(|editor| {
            editor.set_default_paragraph_format(&patch)?;
            editor
                .document()
                .apply_paragraph_format(0..editor.document().text_length(), &patch)?;
            Ok(())
        })
    }

    /// Retrieves native paragraph separators while retaining the model's UTF-16
    /// coordinate system. The complete-story projection synthesizes its terminal
    /// paragraph mark for None or AllowFinalEop, as the source TOM fixture requires.
    pub fn get_text(&self, options: TextGetOptions) -> Result<String, TextOperationError> {
        validate_get_options(options)?;
        if options.contains(TextGetOptions::FORMAT_RTF) {
            return self.document_range()?.get_text(options);
        }
        let mut text = self.text();
        if options.is_empty() || options.contains(TextGetOptions::ALLOW_FINAL_EOP) {
            text.push('\n');
        }
        Ok(convert_line_endings(&text, options))
    }

    /// Replaces the complete story and its paragraph metadata. Plain content starts
    /// with document defaults; RTF retains all formatting the codec can represent.
    pub fn set_text_with_options(
        &self,
        options: TextSetOptions,
        text: &str,
    ) -> Result<(), TextOperationError> {
        validate_set_options(options)?;
        let snapshot = if options.contains(TextSetOptions::FORMAT_RTF) {
            RtfReader::read(&rtf_text_bytes(text)?)?
        } else {
            self.plain_document(text)
        };
        self.replace_document(
            &snapshot,
            options.contains(TextSetOptions::CHECK_TEXT_LIMIT),
        )?;
        Ok(())
    }

    fn plain_document(&self, text: &str) -> DocumentSnapshot {
        let mut snapshot = DocumentSnapshot::from_text(text);
        let defaults = self.editor.insertion_defaults();
        for paragraph in &mut snapshot.paragraphs {
            paragraph.format = defaults.overlay(&paragraph.format);
        }
        snapshot
    }

    pub(crate) fn replace_document(
        &self,
        snapshot: &DocumentSnapshot,
        check_limit: bool,
    ) -> Result<(), EditError> {
        snapshot.validate()?;
        self.editor.prepare_edit()?;
        let snapshot = limited_snapshot(
            snapshot,
            if check_limit {
                self.editor.max_length()
            } else {
                0
            },
        );
        let previous_limit = self.editor.max_length();
        let result = self.editor.edit_group(|editor| {
            if !check_limit && previous_limit > 0 {
                editor.set_max_length(previous_limit.max(snapshot.text_length()));
            }
            let default_tab_stop = snapshot
                .paragraphs
                .iter()
                .find_map(|paragraph| paragraph.format.default_tab_stop)
                .unwrap_or(48.0);
            if editor
                .default_paragraph_format()
                .default_tab_stop
                .unwrap_or(48.0)
                != default_tab_stop
            {
                editor.set_default_paragraph_format(&ParagraphFormat {
                    default_tab_stop: Some(default_tab_stop),
                    ..Default::default()
                })?;
            }
            editor.document().restore(&snapshot)?;
            editor.set_selection(EditorSelection::caret(snapshot.text_length()))?;
            Ok(())
        });
        if result.is_err() {
            self.editor.set_max_length(previous_limit);
        }
        result
    }

    pub fn undo_limit(&self) -> usize {
        self.editor.undo_limit()
    }

    pub fn set_undo_limit(&self, limit: usize) {
        self.editor.set_undo_limit(limit);
    }

    pub fn clear_undo_redo_history(&self) {
        self.editor.clear_history();
    }
    pub fn can_undo(&self) -> bool {
        self.editor.can_undo()
    }
    pub fn can_redo(&self) -> bool {
        self.editor.can_redo()
    }
    pub fn undo(&self) -> Result<bool, EditError> {
        self.editor.undo()
    }
    pub fn redo(&self) -> Result<bool, EditError> {
        self.editor.redo()
    }
    pub fn edit_group<T>(
        &self,
        edit: impl FnOnce(&Self) -> Result<T, EditError>,
    ) -> Result<T, EditError> {
        self.editor.edit_group(|_| edit(self))
    }
}

struct RangeData {
    start: TextPointer,
    end: TextPointer,
    typing_format: Rc<CharacterFormat>,
    gravity: RangeGravity,
    // Explicit range/format ownership changes are distinct from the document's
    // mutation identity, including an asynchronous target moved away and back.
    generation: u64,
}

/// A live, independent range. Its endpoints use the document's TextPointer tracking.
/// RangeGravity independently selects formatting at a collapsed boundary.
/// Cloning shares the range object; [`get_clone`](Self::get_clone) creates independent endpoints.
#[derive(Clone)]
pub struct RichEditTextRange {
    editor: Rc<RichTextEditor>,
    data: Rc<RefCell<RangeData>>,
}

impl RichEditTextRange {
    fn new(editor: Rc<RichTextEditor>, range: Range<usize>) -> Result<Self, EditError> {
        let document = editor.document();
        document.validate_range(range.clone())?;
        let start_gravity = if range.is_empty() {
            TextGravity::Forward
        } else {
            TextGravity::Backward
        };
        Ok(Self {
            data: Rc::new(RefCell::new(RangeData {
                start: document.create_pointer(range.start, start_gravity)?,
                end: document.create_pointer(range.end, TextGravity::Forward)?,
                typing_format: Rc::new(CharacterFormat::default()),
                gravity: RangeGravity::UIBehavior,
                generation: 0,
            })),
            editor,
        })
    }
    pub fn document(&self) -> RichTextDocument {
        self.editor.document()
    }
    pub fn start_position(&self) -> usize {
        self.range().start
    }
    pub fn end_position(&self) -> usize {
        self.range().end
    }
    pub fn range(&self) -> Range<usize> {
        let data = self.data.borrow();
        data.start.offset().min(data.end.offset())..data.start.offset().max(data.end.offset())
    }
    pub fn length(&self) -> usize {
        self.range().len()
    }
    /// Length of the single story in FerroUI's UTF-16 model coordinates. As with
    /// the range endpoints, this excludes the synthetic terminal mark emitted by
    /// complete-document text retrieval.
    pub fn story_length(&self) -> usize {
        self.document().text_length()
    }
    pub fn is_degenerate(&self) -> bool {
        self.range().is_empty()
    }
    pub fn text(&self) -> Result<String, EditError> {
        Ok(self.document().get_text(self.range())?)
    }

    pub fn get_text(&self, options: TextGetOptions) -> Result<String, TextOperationError> {
        validate_get_options(options)?;
        let mut range = self.range();
        if options.contains(TextGetOptions::ADJUST_CRLF) {
            range.start = adjusted_text_start(&self.document().text(), range.start);
        }
        if options.contains(TextGetOptions::FORMAT_RTF) {
            return Ok(RtfWriter::write_with_default_tab_stop(
                &self.snapshot_with_effective_paragraphs(range)?,
                self.editor
                    .default_paragraph_format()
                    .default_tab_stop
                    .unwrap_or(48.0),
            )?);
        }
        // Live model ranges do not include the native synthetic final paragraph
        // mark. AllowFinalEop therefore adds no text to an ordinary content range.
        Ok(convert_line_endings(
            &self.document().get_text(range)?,
            options,
        ))
    }
    pub fn snapshot(&self) -> Result<DocumentSnapshot, EditError> {
        Ok(self.document().slice(self.range())?)
    }

    fn snapshot_with_effective_paragraphs(
        &self,
        range: Range<usize>,
    ) -> Result<DocumentSnapshot, EditError> {
        let document = self.document();
        let full = document.snapshot();
        let (start, _) = full.locate(range.start);
        let mut snapshot = document.slice(range)?;
        let paragraphs = document.paragraphs();
        for (index, paragraph) in snapshot.paragraphs.iter_mut().enumerate() {
            if let Some(original) = paragraphs.get(start + index) {
                paragraph.format = ParagraphFormat::effective(original).overlay(&paragraph.format);
                if !original.is_set(Paragraph::default_tab_stop_property().as_property()) {
                    paragraph.format.default_tab_stop = Some(
                        self.editor
                            .default_paragraph_format()
                            .default_tab_stop
                            .unwrap_or(48.0),
                    );
                }
            }
        }
        Ok(snapshot)
    }
    /// Returns one rectangle for each visual fragment, including bidirectional and wrapped text.
    pub fn get_rectangles(&self, view: &RichTextBlock) -> Result<Vec<Rect>, EditError> {
        ensure_view_document(&self.document(), view)?;
        Ok(view.get_rects_for_range(self.range()))
    }
    pub fn get_clone(&self) -> Result<Self, EditError> {
        let result = Self::new(self.editor.clone(), self.range())?;
        result.data.borrow_mut().typing_format = self.data.borrow().typing_format.clone();
        Ok(result)
    }

    fn set_range_typing_format(&self, format: CharacterFormat) -> Result<(), EditError> {
        let checkpoint = self.range_read_checkpoint();
        let previous = self.data.borrow().typing_format.clone();
        let changed = *previous != format;
        self.ensure_range_read(checkpoint)?;
        if changed {
            let mut data = self.data.borrow_mut();
            data.typing_format = Rc::new(format);
            data.generation = data.generation.wrapping_add(1);
        }
        Ok(())
    }

    pub fn formatted_text(&self) -> Result<Self, EditError> {
        self.get_clone()
    }

    pub fn set_formatted_text(&self, value: &Self) -> Result<(), EditError> {
        let snapshot = value.snapshot_with_effective_paragraphs(value.range())?;
        self.set_snapshot(&snapshot)
    }

    pub fn set_start_position(&self, position: usize) -> Result<(), EditError> {
        self.set_range(position, self.end_position().max(position))
    }

    pub fn set_end_position(&self, position: usize) -> Result<(), EditError> {
        self.set_range(self.start_position().min(position), position)
    }
    pub fn set_range(&self, start: usize, end: usize) -> Result<(), EditError> {
        let range = start.min(end)..start.max(end);
        self.document().validate_range(range.clone())?;
        if range == self.range() {
            return Ok(());
        }
        let replacement = RangeData {
            start: self.document().create_pointer(
                range.start,
                if range.is_empty() {
                    TextGravity::Forward
                } else {
                    TextGravity::Backward
                },
            )?,
            end: self
                .document()
                .create_pointer(range.end, TextGravity::Forward)?,
            typing_format: Rc::new(CharacterFormat::default()),
            gravity: self.data.borrow().gravity,
            generation: self.data.borrow().generation.wrapping_add(1),
        };
        let old = self.data.replace(replacement);
        drop(old);
        Ok(())
    }
    pub fn collapse(&self, to_start: bool) -> Result<(), EditError> {
        let position = if to_start {
            self.start_position()
        } else {
            self.end_position()
        };
        self.set_range(position, position)
    }
    pub fn select(&self) -> Result<(), EditError> {
        let range = self.range();
        self.editor
            .set_selection(EditorSelection::new(range.start, range.end))
    }
    pub fn in_range(&self, other: &Self) -> bool {
        self.document() == other.document()
            && self.start_position() >= other.start_position()
            && self.end_position() <= other.end_position()
    }
    pub fn is_equal(&self, other: &Self) -> bool {
        self.document() == other.document() && self.range() == other.range()
    }

    pub fn in_story(&self, other: &Self) -> bool {
        self.document() == other.document()
    }

    pub fn match_selection(&self) -> Result<(), EditError> {
        let selection = self.editor.selection().range();
        self.set_range(selection.start, selection.end)
    }

    /// Decodes the scalar at a signed UTF-16 offset from the range's end. Either
    /// half of a surrogate pair returns its scalar; outside the story returns zero.
    pub fn get_character_utf32(&self, offset: i32) -> u32 {
        let target = self.end_position() as i128 + i128::from(offset);
        if target < 0 || target >= self.document().text_length() as i128 {
            return 0;
        }
        let target = target as usize;
        let mut position = 0;
        for character in self.document().text().chars() {
            position += character.len_utf16();
            if target < position {
                return character as u32;
            }
        }
        0
    }

    /// Replaces the range while preserving and rebasing the editor's independent selection.
    /// The change, formatting, and selection are one undoable transaction.
    pub fn set_text(&self, text: &str) -> Result<(), EditError> {
        self.editor.prepare_edit()?;
        let checkpoint = self.range_read_checkpoint();
        let original = self.range();
        let before_length = self.document().text_length();
        let selection = self.editor.selection();
        let editor_checkpoint = self.editor.navigation_checkpoint();
        let typing_format = self.editor.typing_format();
        let original_format = self.range_typing_format();
        let insertion_format = original
            .is_empty()
            .then(|| self.caret_character_format())
            .transpose()?;
        self.ensure_range_read(checkpoint)?;
        if self.editor.navigation_checkpoint() != editor_checkpoint {
            return Err(EditError::ReentrantEdit);
        }
        let mut owned_generation = checkpoint.0;
        let external_generation = self.editor.external_change_generation();
        let result = self.editor.edit_group(|editor| {
            self.ensure_range_read(checkpoint)?;
            editor.set_selection(EditorSelection::new(original.start, original.end))?;
            if let Some(format) = &insertion_format {
                editor.replace_range_typing_format(format)?;
                // A collapsed typing patch changes no document content. Any
                // model write during its services belongs to the callback.
                self.ensure_range_read(checkpoint)?;
            }
            self.ensure_range_generation(checkpoint.0)?;
            editor.insert_text(text, EditKind::Programmatic)?;
            self.ensure_range_generation(checkpoint.0)?;
            let inserted = self
                .document()
                .text_length()
                .saturating_sub(before_length - original.len());
            let updated = rebase_selection(selection, original.clone(), inserted);
            editor.set_selection(updated)?;
            if updated.is_empty() {
                editor.replace_range_typing_format(&typing_format)?;
            }
            self.ensure_range_generation(checkpoint.0)?;
            // Install before editor publication. A newer callback-owned range
            // or affinity must survive the enclosing command's completion.
            self.set_range(original.start, original.start + inserted)?;
            owned_generation = self.data.borrow().generation;
            Ok(())
        });
        if result.is_err()
            && self.editor.external_change_generation() == external_generation
            && self.data.borrow().generation == owned_generation
        {
            self.set_range(original.start, original.end)?;
            self.set_range_typing_format(original_format)?;
        }
        result
    }

    pub fn set_text_with_options(
        &self,
        options: TextSetOptions,
        text: &str,
    ) -> Result<(), TextOperationError> {
        validate_set_options(options)?;
        let fragment = if options.contains(TextSetOptions::FORMAT_RTF) {
            Some(RtfReader::read(&rtf_text_bytes(text)?)?)
        } else if options.contains(TextSetOptions::UNLINK) {
            let mut fragment = DocumentSnapshot::from_text(text);
            let checkpoint = self.range_read_checkpoint();
            let format = if self.is_degenerate() {
                self.caret_character_format()?
            } else {
                self.document()
                    .character_format_at(self.start_position())?
                    .overlay(&self.range_typing_format())
            };
            self.ensure_range_read(checkpoint)?;
            for paragraph in &mut fragment.paragraphs {
                paragraph.format.character = format.clone();
            }
            Some(fragment)
        } else {
            None
        };
        self.replace_with_options(options, text, fragment.as_ref())
    }

    fn replace_with_options(
        &self,
        options: TextSetOptions,
        text: &str,
        fragment: Option<&DocumentSnapshot>,
    ) -> Result<(), TextOperationError> {
        if let Some(fragment) = fragment {
            fragment.validate()?;
        }
        // Native range insertion ignores RTF document defaults unless explicitly
        // requested. The unsupported ApplyRtfDocumentDefaults option is rejected
        // before this point; explicit paragraph stops are retained.
        let normalized_rtf = if options.contains(TextSetOptions::FORMAT_RTF) {
            fragment.map(|fragment| {
                let mut fragment = fragment.clone();
                let interval = self
                    .editor
                    .default_paragraph_format()
                    .default_tab_stop
                    .unwrap_or(48.0);
                for paragraph in &mut fragment.paragraphs {
                    paragraph.format.default_tab_stop = Some(interval);
                }
                fragment
            })
        } else {
            None
        };
        let fragment = normalized_rtf.as_ref().or(fragment);
        let inserted_length = fragment.map_or_else(
            || DocumentSnapshot::from_text(text).text_length(),
            DocumentSnapshot::text_length,
        );
        let previous_limit = self.editor.max_length();
        let result = self.editor.edit_group(|editor| {
            if !options.contains(TextSetOptions::CHECK_TEXT_LIMIT) && previous_limit > 0 {
                let length = self
                    .document()
                    .text_length()
                    .saturating_sub(self.length())
                    .saturating_add(inserted_length);
                editor.set_max_length(previous_limit.max(length));
            }
            if let Some(fragment) = fragment {
                self.set_snapshot(fragment)
            } else {
                self.set_text(text)
            }
        });
        if result.is_err() {
            self.editor.set_max_length(previous_limit);
        }
        result?;
        Ok(())
    }

    pub fn set_snapshot(&self, snapshot: &DocumentSnapshot) -> Result<(), EditError> {
        snapshot.validate()?;
        self.editor.prepare_edit()?;
        if snapshot.paragraphs.is_empty() {
            return self.set_text("");
        }
        let checkpoint = self.range_read_checkpoint();
        let original = self.range();
        let before_length = self.document().text_length();
        let selection = self.editor.selection();
        let editor_checkpoint = self.editor.navigation_checkpoint();
        let typing_format = self.editor.typing_format();
        let original_format = self.range_typing_format();
        let empty_paragraph = (snapshot.text_length() == 0)
            .then(|| snapshot.paragraphs.first().cloned())
            .flatten();
        let empty_format = empty_paragraph
            .as_ref()
            .map(|paragraph| paragraph.format.character.clone());
        self.ensure_range_read(checkpoint)?;
        if self.editor.navigation_checkpoint() != editor_checkpoint {
            return Err(EditError::ReentrantEdit);
        }
        let mut owned_generation = checkpoint.0;
        let external_generation = self.editor.external_change_generation();
        let result = self.editor.edit_group(|editor| {
            self.ensure_range_read(checkpoint)?;
            editor.set_selection(EditorSelection::new(original.start, original.end))?;
            if let Some(paragraph) = empty_paragraph {
                // An empty rich paragraph still carries formatting. Delete the
                // selected content and retain that paragraph if the resulting
                // target is empty, without restyling surrounding surviving text.
                editor.insert_text("", EditKind::Programmatic)?;
                self.ensure_range_generation(checkpoint.0)?;
                let mutation = self.document().mutation_version();
                let mut document = self.document().snapshot();
                self.ensure_range_read((checkpoint.0, mutation))?;
                let (index, _) = document.locate(original.start);
                if document.paragraphs.is_empty() {
                    document.paragraphs.push(paragraph);
                    editor.document().restore(&document)?;
                } else if document.paragraphs[index].text_length() == 0 {
                    document.paragraphs[index] = paragraph;
                    editor.document().restore(&document)?;
                }
            } else {
                editor.insert_fragment(snapshot, EditKind::Paste)?;
            }
            self.ensure_range_generation(checkpoint.0)?;
            let inserted = self
                .document()
                .text_length()
                .saturating_sub(before_length - original.len());
            let updated = rebase_selection(selection, original.clone(), inserted);
            editor.set_selection(updated)?;
            if updated.is_empty() {
                editor.replace_range_typing_format(&typing_format)?;
            }
            self.ensure_range_generation(checkpoint.0)?;
            self.set_range(original.start, original.start + inserted)?;
            owned_generation = self.data.borrow().generation;
            if let Some(format) = empty_format {
                // Degenerate range defaults belong to this independent range,
                // and must be installed before publication can change ownership.
                self.set_range_typing_format(format)?;
                owned_generation = self.data.borrow().generation;
            }
            Ok(())
        });
        if result.is_err()
            && self.editor.external_change_generation() == external_generation
            && self.data.borrow().generation == owned_generation
        {
            self.set_range(original.start, original.end)?;
            self.set_range_typing_format(original_format)?;
        }
        result
    }

    fn apply_to_selection(
        &self,
        action: impl FnOnce(&RichTextEditor) -> Result<(), EditError>,
    ) -> Result<(), EditError> {
        self.editor.prepare_edit()?;
        let range = self.range();
        let selection = self.editor.selection();
        let typing_format = self.editor.typing_format();
        self.editor.edit_group(|editor| {
            editor.set_selection(EditorSelection::new(range.start, range.end))?;
            action(editor)?;
            editor.set_selection(selection)?;
            if selection.is_empty() {
                editor.replace_range_typing_format(&typing_format)?;
            }
            Ok(())
        })
    }
    pub fn set_character_format(&self, format: &CharacterFormat) -> Result<(), EditError> {
        format.validate()?;
        if self.is_degenerate() {
            self.editor.prepare_edit()?;
            let checkpoint = self.range_read_checkpoint();
            let current = self.data.borrow().typing_format.clone();
            let updated = current.overlay(format).clone();
            self.ensure_range_read(checkpoint)?;
            return self.set_range_typing_format(updated);
        }
        self.apply_to_selection(|editor| editor.apply_format(format))
    }
    pub fn set_paragraph_format(&self, format: &ParagraphFormat) -> Result<(), EditError> {
        format.validate()?;
        self.apply_to_selection(|editor| editor.apply_paragraph_format(format))
    }

    /// Queries effective formats at run boundaries, so a long uniform run is inspected once.
    pub fn character_format(&self) -> Result<TextFormatValue<CharacterFormat>, EditError> {
        let checkpoint = self.range_read_checkpoint();
        let range = self.range();
        if range.is_empty() {
            return self.caret_character_format().map(TextFormatValue::Uniform);
        }
        let document = self.document();
        let format_at = |offset| {
            document
                .element_at(offset, TextGravity::Forward)
                .map(|element| CharacterFormat::effective(&element))
                .unwrap_or_else(|| self.editor.insertion_defaults().character)
        };
        let first = format_at(range.start);
        self.ensure_range_read(checkpoint)?;
        let mut boundaries = Vec::new();
        let mut offset = 0;
        let snapshot = document.snapshot();
        self.ensure_range_read(checkpoint)?;
        for (index, paragraph) in snapshot.paragraphs.iter().enumerate() {
            if index > 0 {
                boundaries.push(offset);
                offset += 1;
            }
            inline_boundaries(&paragraph.inlines, &mut offset, &mut boundaries);
        }
        for offset in boundaries
            .into_iter()
            .filter(|offset| *offset > range.start && *offset < range.end)
        {
            let mixed = format_at(offset) != first;
            self.ensure_range_read(checkpoint)?;
            if mixed {
                return Ok(TextFormatValue::Mixed);
            }
        }
        Ok(TextFormatValue::Uniform(first))
    }
    pub fn paragraph_format(&self) -> Result<TextFormatValue<ParagraphFormat>, EditError> {
        let range = self.range();
        let mut offset = 0;
        let mut selected = None;
        for paragraph in self.document().paragraphs() {
            let end = offset + paragraph.text_length();
            if range.start <= end
                && (range.end > offset || range.is_empty() && range.start == offset)
            {
                let mut format = ParagraphFormat::effective(&paragraph);
                // The shared snapshot type also stores inherited character
                // defaults; those are outside TOM's paragraph-format query.
                format.character = CharacterFormat::default();
                if let Some(first) = &selected {
                    if first != &format {
                        return Ok(TextFormatValue::Mixed);
                    }
                } else {
                    selected = Some(format);
                }
            }
            offset = end + 1;
        }
        let mut format = selected.unwrap_or_else(|| self.editor.insertion_defaults());
        format.character = CharacterFormat::default();
        Ok(TextFormatValue::Uniform(format))
    }

    /// Expands to the smallest whole TOM units containing the range. Whole-word
    /// search retains independent Unicode boundaries and excludes trailing blanks.
    pub fn expand(&self, unit: TextRangeUnit) -> Result<usize, EditError> {
        let units = TextUnits::resolve(&self.editor, unit)?;
        let range = self.range();
        let expanded = units.expanded(range.clone());
        self.set_range(expanded.start, expanded.end)?;
        Ok(expanded.len().saturating_sub(range.len()))
    }

    /// Moves to the beginning of the first overlapping unit. Extension changes
    /// only the start; otherwise the range collapses. The signed UTF-16 displacement
    /// is always nonpositive, including zero when already on a unit boundary.
    pub fn start_of(&self, unit: TextRangeUnit, extend: bool) -> Result<isize, EditError> {
        let units = TextUnits::resolve(&self.editor, unit)?;
        let range = self.range();
        let start = units.start_of(range.start);
        let moved = units::signed_displacement(start, range.start)?;
        self.set_range(start, if extend { range.end } else { start })?;
        Ok(moved)
    }

    /// Moves to the end of the last overlapping unit. A collapse contributes one
    /// to the returned UTF-16 displacement, matching the native EndOf contract.
    /// The synthetic final-CR case is outside FerroUI's range coordinates.
    pub fn end_of(&self, unit: TextRangeUnit, extend: bool) -> Result<isize, EditError> {
        let units = TextUnits::resolve(&self.editor, unit)?;
        let range = self.range();
        let end = units.end_of(range.end);
        let moved = units::signed_displacement(end, range.end)?
            .checked_add(isize::from(!extend && !range.is_empty()))
            .ok_or(DocumentError::InvalidFormat(
                "The text movement exceeds the signed offset range.",
            ))?;
        self.set_range(if extend { range.start } else { end }, end)?;
        Ok(moved)
    }

    /// Gets the one-based story index at StartPosition. At the end of a story it
    /// reports the total unit count; a unit absent from empty content has index zero.
    pub fn get_index(&self, unit: TextRangeUnit) -> Result<usize, EditError> {
        Ok(TextUnits::resolve(&self.editor, unit)?.index(self.start_position()))
    }

    /// Relocates to a whole unit (extend) or its starting insertion point. Positive
    /// indices count from one; negative indices count back from the last unit.
    /// Zero and out-of-range indices fail without changing the live range.
    pub fn set_index(
        &self,
        unit: TextRangeUnit,
        index: i32,
        extend: bool,
    ) -> Result<(), EditError> {
        let units = TextUnits::resolve(&self.editor, unit)?;
        let range = units.indexed(index)?;
        self.set_range(range.start, if extend { range.end } else { range.start })
    }

    /// A non-degenerate range first collapses toward the movement direction and
    /// reaches the nearest unit boundary. That collapse counts as the first unit.
    pub fn move_by(&self, unit: TextRangeUnit, count: i32) -> Result<i32, EditError> {
        if count == 0 {
            return Ok(0);
        }
        let units = TextUnits::resolve(&self.editor, unit)?;
        let (position, moved) = units.move_range(self.range(), count);
        self.set_range(position, position)?;
        Ok(moved)
    }
    pub fn move_start(&self, unit: TextRangeUnit, count: i32) -> Result<i32, EditError> {
        if count == 0 {
            return Ok(0);
        }
        let units = TextUnits::resolve(&self.editor, unit)?;
        let (position, moved) = units.move_position(self.start_position(), count);
        self.set_range(position, self.end_position().max(position))?;
        Ok(moved)
    }
    pub fn move_end(&self, unit: TextRangeUnit, count: i32) -> Result<i32, EditError> {
        if count == 0 {
            return Ok(0);
        }
        let units = TextUnits::resolve(&self.editor, unit)?;
        let (position, moved) = units.move_position(self.end_position(), count);
        self.set_range(self.start_position().min(position), position)?;
        Ok(moved)
    }

    /// Searches this range. Case-insensitive matching uses Unicode lowercase mappings;
    /// UTF-16 offsets remain exact even when a character maps to several lowercase scalars.
    pub fn find(&self, text: &str, options: TextFindOptions) -> Result<Option<Self>, EditError> {
        if options.bits() & !TextFindOptions::all().bits() != 0 {
            return Err(DocumentError::InvalidFormat("Unknown text search options").into());
        }
        if text.is_empty() {
            return Ok(None);
        }
        let source = self.text()?;
        let case_sensitive = options.contains(TextFindOptions::MATCH_CASE);
        let (haystack, boundaries) = searchable(&source, case_sensitive);
        let (needle, _) = searchable(text, case_sensitive);
        let words = if options.contains(TextFindOptions::WHOLE_WORD) {
            unit_ranges(&self.document().text(), UnicodeTextUnit::Word)
        } else {
            Vec::new()
        };
        let mut found = None;
        for byte_start in SubstringMatches::new(&haystack, &needle) {
            let byte_end = byte_start + needle.len();
            let start = boundaries
                .binary_search_by_key(&byte_start, |(byte, _)| *byte)
                .ok()
                .map(|index| boundaries[index].1);
            let end = boundaries
                .binary_search_by_key(&byte_end, |(byte, _)| *byte)
                .ok()
                .map(|index| boundaries[index].1);
            let (Some(start), Some(end)) = (start, end) else {
                continue;
            };
            let start = self.start_position() + start;
            let end = self.start_position() + end;
            if options.contains(TextFindOptions::WHOLE_WORD)
                && (words
                    .binary_search_by_key(&start, |word| word.start)
                    .is_err()
                    || words.binary_search_by_key(&end, |word| word.end).is_err())
            {
                continue;
            }
            found = Some(start..end);
            if !options.contains(TextFindOptions::BACKWARDS) {
                break;
            }
        }
        found
            .map(|range| Self::new(self.editor.clone(), range))
            .transpose()
    }
    pub fn find_text(&self, text: &str, options: TextFindOptions) -> Result<bool, EditError> {
        let Some(found) = self.find(text, options)? else {
            return Ok(false);
        };
        self.set_range(found.start_position(), found.end_position())?;
        Ok(true)
    }

    /// Native bounded search: positive lengths start at StartPosition, negative
    /// lengths search backward from EndPosition. Zero searches the existing range,
    /// or the remaining story for a caret. A repeat skips the range already found.
    pub fn find_text_with_length(
        &self,
        text: &str,
        scan_length: i32,
        options: FindOptions,
    ) -> Result<usize, EditError> {
        if options.bits() & !FindOptions::all().bits() != 0 {
            return Err(DocumentError::InvalidFormat("Unknown text search options").into());
        }
        if text.is_empty() {
            return Ok(0);
        }
        let original = self.range();
        let source = self.document().text();
        let length = source.encode_utf16().count();
        let mut search = if scan_length < 0 {
            original
                .end
                .saturating_sub(scan_length.unsigned_abs() as usize)..original.end
        } else if scan_length > 0 {
            original.start
                ..original
                    .start
                    .saturating_add(scan_length as usize)
                    .min(length)
        } else if original.is_empty() {
            original.start..length
        } else {
            original.clone()
        };
        search.start = scalar_ceiling(&source, search.start);
        search.end = scalar_floor(&source, search.end);
        if search.start > search.end {
            return Ok(0);
        }
        let mut convenience = TextFindOptions::empty();
        convenience.set(
            TextFindOptions::MATCH_CASE,
            options.contains(FindOptions::CASE),
        );
        convenience.set(
            TextFindOptions::WHOLE_WORD,
            options.contains(FindOptions::WORD),
        );
        convenience.set(TextFindOptions::BACKWARDS, scan_length < 0);
        let candidate = Self::new(self.editor.clone(), search.clone())?;
        let mut found = candidate.find(text, convenience)?;
        if scan_length != 0
            && found
                .as_ref()
                .is_some_and(|range| range.range() == original)
        {
            if scan_length < 0 {
                search.end = scalar_floor(&source, original.end.saturating_sub(1));
            } else {
                search.start =
                    scalar_ceiling(&source, original.start.saturating_add(1).min(length));
            }
            found = if search.start <= search.end {
                Self::new(self.editor.clone(), search)?.find(text, convenience)?
            } else {
                None
            };
        }
        let Some(found) = found else {
            return Ok(0);
        };
        self.set_range(found.start_position(), found.end_position())?;
        Ok(found.length())
    }
}

/// Linear-time substring matching includes overlaps, including when an earlier match
/// is rejected by document word boundaries or a Unicode lowercase expansion boundary.
struct SubstringMatches<'a> {
    text: &'a [u8],
    pattern: &'a [u8],
    prefix: Vec<usize>,
    position: usize,
    matched: usize,
}
impl<'a> SubstringMatches<'a> {
    fn new(text: &'a str, pattern: &'a str) -> Self {
        let pattern = pattern.as_bytes();
        let mut prefix = vec![0; pattern.len()];
        let mut matched = 0;
        for position in 1..pattern.len() {
            while matched > 0 && pattern[position] != pattern[matched] {
                matched = prefix[matched - 1];
            }
            if pattern[position] == pattern[matched] {
                matched += 1;
            }
            prefix[position] = matched;
        }
        Self {
            text: text.as_bytes(),
            pattern,
            prefix,
            position: 0,
            matched: 0,
        }
    }
}
impl Iterator for SubstringMatches<'_> {
    type Item = usize;
    fn next(&mut self) -> Option<Self::Item> {
        if self.pattern.is_empty() {
            return None;
        }
        while let Some(byte) = self.text.get(self.position) {
            while self.matched > 0 && *byte != self.pattern[self.matched] {
                self.matched = self.prefix[self.matched - 1];
            }
            if *byte == self.pattern[self.matched] {
                self.matched += 1;
            }
            self.position += 1;
            if self.matched == self.pattern.len() {
                self.matched = self.prefix[self.matched - 1];
                return Some(self.position - self.pattern.len());
            }
        }
        None
    }
}

/// Live directed editor selection. Extending preserves the anchor; moving without
/// extension collapses to the requested text-unit boundary.
#[derive(Clone)]
pub struct RichEditTextSelection {
    editor: Rc<RichTextEditor>,
}
impl RichEditTextSelection {
    pub fn selection(&self) -> EditorSelection {
        self.editor.selection()
    }
    pub fn set_range(&self, anchor: usize, active: usize) -> Result<(), EditError> {
        let requested = EditorSelection::new(anchor, active);
        let generation = self.editor.navigation_generation();
        let result = self.editor.set_selection(requested);
        if result.is_ok()
            && self.selection() == requested
            && self.editor.navigation_generation() == generation
        {
            // Explicit source offsets carry no retained column or line-end
            // affinity, even when equal to the current offsets. Preserve any
            // newer decision made by a reentrant navigation callback.
            self.editor.clear_navigation_affinity();
        }
        result
    }
    pub fn range(&self) -> Result<RichEditTextRange, EditError> {
        RichEditTextRange::new(self.editor.clone(), self.selection().range())
    }
    pub fn text(&self) -> Result<String, EditError> {
        self.editor.selected_text()
    }
    pub fn select_all(&self) -> Result<(), EditError> {
        self.set_range(0, self.editor.document().text_length())
    }
    /// Enters typed scalars using the shared selection modes. CR and LF each
    /// type a paragraph break; an empty call leaves selection and preview intact.
    pub fn type_text(&self, text: &str) -> Result<bool, EditError> {
        self.editor.type_text(text)
    }
    pub fn delete(&self) -> Result<bool, EditError> {
        self.editor.delete_selection()
    }
    pub fn character_format(&self) -> Result<CharacterFormat, EditError> {
        self.editor.current_character_format()
    }
    /// Unlike the insertion-format convenience getter, this query reports mixed
    /// formatting across a nonempty selection instead of choosing its first run.
    pub fn character_format_value(&self) -> Result<TextFormatValue<CharacterFormat>, EditError> {
        if self.selection().is_empty() {
            Ok(TextFormatValue::Uniform(
                self.editor.current_character_format()?,
            ))
        } else {
            self.range()?.character_format()
        }
    }
    pub fn paragraph_format(&self) -> Result<TextFormatValue<ParagraphFormat>, EditError> {
        self.range()?.paragraph_format()
    }
    pub fn set_character_format(&self, format: &CharacterFormat) -> Result<(), EditError> {
        self.editor.apply_format(format)
    }
    pub fn set_paragraph_format(&self, format: &ParagraphFormat) -> Result<(), EditError> {
        self.editor.apply_paragraph_format(format)
    }
    pub fn move_by(&self, unit: TextRangeUnit, count: i32, extend: bool) -> Result<i32, EditError> {
        self.move_logically(unit, count, extend)
    }
}

fn ensure_view_document(
    document: &RichTextDocument,
    view: &RichTextBlock,
) -> Result<(), EditError> {
    if *document == view.document() {
        Ok(())
    } else {
        Err(
            DocumentError::InvalidFormat("The text view must display this range's document.")
                .into(),
        )
    }
}

fn rebase_selection(
    selection: EditorSelection,
    range: Range<usize>,
    inserted: usize,
) -> EditorSelection {
    let map = |position: usize| {
        if position < range.start {
            position
        } else if position > range.end {
            position - range.len() + inserted
        } else {
            range.start + inserted
        }
    };
    EditorSelection::new(map(selection.anchor), map(selection.active))
}
pub(crate) fn unit_ranges(text: &str, unit: UnicodeTextUnit) -> Vec<Range<usize>> {
    let utf16: Vec<u16> = text.encode_utf16().collect();
    match unit {
        UnicodeTextUnit::Character => GraphemeEnumerator::new(&utf16)
            .map(|segment| segment.offset()..segment.offset() + segment.length())
            .collect(),
        UnicodeTextUnit::Word => WordBreakEnumerator::new(&utf16)
            .map(|segment| segment.offset()..segment.offset() + segment.length())
            .collect(),
        UnicodeTextUnit::Paragraph => {
            let mut ranges = Vec::new();
            let mut start = 0;
            for (index, unit) in utf16.iter().enumerate() {
                if *unit == b'\n' as u16 {
                    ranges.push(start..index + 1);
                    start = index + 1;
                }
            }
            ranges.push(start..utf16.len());
            ranges
        }
    }
}
fn searchable(text: &str, case_sensitive: bool) -> (String, Vec<(usize, usize)>) {
    let mut output = String::new();
    let mut boundaries = vec![(0, 0)];
    let mut offset = 0;
    for ch in text.chars() {
        if case_sensitive {
            output.push(ch);
        } else {
            output.extend(ch.to_lowercase());
        }
        offset += ch.len_utf16();
        boundaries.push((output.len(), offset));
    }
    (output, boundaries)
}
fn inline_boundaries(inlines: &[InlineSnapshot], offset: &mut usize, output: &mut Vec<usize>) {
    for inline in inlines {
        output.push(*offset);
        match &inline.kind {
            InlineSnapshotKind::Span(inlines)
            | InlineSnapshotKind::Bold(inlines)
            | InlineSnapshotKind::Italic(inlines)
            | InlineSnapshotKind::Underline(inlines)
            | InlineSnapshotKind::Hyperlink { inlines, .. } => {
                inline_boundaries(inlines, offset, output)
            }
            _ => *offset += inline.text_length(),
        }
    }
}

fn validate_get_options(options: TextGetOptions) -> Result<(), TextOperationError> {
    if options.bits() & !TextGetOptions::all().bits() != 0 {
        return Err(TextOperationError::InvalidOptions(
            "Unknown text retrieval options.",
        ));
    }
    if options.contains(TextGetOptions::USE_CRLF | TextGetOptions::USE_LF) {
        return Err(TextOperationError::InvalidOptions(
            "UseCrlf and UseLf are mutually exclusive.",
        ));
    }
    let unsupported = options
        & (TextGetOptions::USE_OBJECT_TEXT
            | TextGetOptions::INCLUDE_NUMBERING
            | TextGetOptions::NO_HIDDEN);
    if !unsupported.is_empty() {
        return Err(TextOperationError::UnsupportedOptions {
            operation: "GetText",
            bits: unsupported.bits(),
        });
    }
    if options.contains(TextGetOptions::FORMAT_RTF) && options != TextGetOptions::FORMAT_RTF {
        return Err(TextOperationError::InvalidOptions(
            "RTF retrieval cannot be combined with plain-text conversion options.",
        ));
    }
    Ok(())
}

fn validate_set_options(options: TextSetOptions) -> Result<(), TextOperationError> {
    if options.bits() & !TextSetOptions::all().bits() != 0 {
        return Err(TextOperationError::InvalidOptions(
            "Unknown text insertion options.",
        ));
    }
    if options.contains(TextSetOptions::APPLY_RTF_DOCUMENT_DEFAULTS) {
        if !options.contains(TextSetOptions::FORMAT_RTF) {
            return Err(TextOperationError::InvalidOptions(
                "ApplyRtfDocumentDefaults requires FormatRtf.",
            ));
        }
        return Err(TextOperationError::UnsupportedOptions {
            operation: "SetText",
            bits: TextSetOptions::APPLY_RTF_DOCUMENT_DEFAULTS.bits(),
        });
    }
    if options.contains(TextSetOptions::FORMAT_RTF | TextSetOptions::UNLINK) {
        return Err(TextOperationError::InvalidOptions(
            "Unlink with explicit RTF hyperlinks requires a separate link projection.",
        ));
    }
    let unsupported = options & (TextSetOptions::UNICODE_BIDI | TextSetOptions::UNHIDE);
    if !unsupported.is_empty() {
        return Err(TextOperationError::UnsupportedOptions {
            operation: "SetText",
            bits: unsupported.bits(),
        });
    }
    Ok(())
}

fn convert_line_endings(text: &str, options: TextGetOptions) -> String {
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    if options.contains(TextGetOptions::USE_LF) {
        normalized
    } else if options.contains(TextGetOptions::USE_CRLF) {
        normalized.replace('\n', "\r\n")
    } else {
        normalized.replace('\n', "\r")
    }
}

fn adjusted_text_start(text: &str, requested: usize) -> usize {
    let mut offset = 0;
    let mut previous = None;
    let mut construct_start = 0;
    for character in text.chars() {
        let variation = matches!(character as u32, 0xFE00..=0xFE0F | 0xE0100..=0xE01EF);
        if !variation {
            construct_start = offset;
        }
        let end = offset + character.len_utf16();
        if requested < end {
            return if character == '\n' && previous == Some('\r') {
                offset.saturating_sub(1)
            } else if variation {
                construct_start
            } else {
                offset
            };
        }
        previous = Some(character);
        offset = end;
    }
    offset
}

fn scalar_floor(text: &str, offset: usize) -> usize {
    super::utf16_prefix(text, offset).encode_utf16().count()
}

fn scalar_ceiling(text: &str, offset: usize) -> usize {
    let floor = scalar_floor(text, offset);
    if floor < offset && floor < text.encode_utf16().count() {
        floor + 2
    } else {
        floor
    }
}

fn limited_snapshot(snapshot: &DocumentSnapshot, limit: usize) -> DocumentSnapshot {
    if limit == 0 || snapshot.text_length() <= limit {
        return snapshot.clone();
    }
    let end = scalar_floor(&snapshot.text(), limit);
    snapshot.slice_unchecked(0..end)
}

fn rtf_text_bytes(text: &str) -> Result<Vec<u8>, RtfError> {
    // Native GetText widens RTF bytes to UTF-16 code units. Reverse that widening
    // rather than feeding a UTF-8 encoding of byte-valued characters to the codec.
    text.chars()
        .map(|character| {
            u8::try_from(character as u32).map_err(|_| {
                RtfError::UnsupportedFeature("non-byte RTF literals; use RTF Unicode escapes")
            })
        })
        .collect()
}

#[cfg(test)]
#[path = "rich_edit_text_document_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "rich_edit_text_selection_options_tests.rs"]
mod selection_options_tests;
