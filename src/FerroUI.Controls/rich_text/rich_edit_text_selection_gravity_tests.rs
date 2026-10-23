use super::*;
use crate::documents::{Run, TextElement};
use crate::test_support::test_scope;
use crate::RichEditBox;
use ferroui_base::input::text_input::{TextInputMethodEdit, TextSelection};
use ferroui_base::media::{
    FontStyle, FontWeight, IBrush, ISolidColorBrush, ITransform, TextAlignment,
};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{Rect, Ref, RelativePoint, Size};
use std::any::Any;
use std::cell::Cell;

#[path = "rich_edit_text_selection_gravity_native_cases.rs"]
mod native_cases;

#[path = "rich_edit_text_paragraph_typing_native_cases.rs"]
mod paragraph_native_cases;

#[derive(Clone, Copy, Debug)]
enum NativeTarget {
    Primary,
    Duplicate,
    External(usize, usize),
}
#[derive(Clone, Copy, Debug)]
enum NativeOperation {
    GetGravity,
    SetGravity(i32),
    SetRange(usize, usize),
    Duplicate,
    TypeText(Option<&'static str>),
    MoveLeft(i32, i32, bool),
    MoveRight(i32, i32, bool),
    MoveUp(i32, i32, bool),
    MoveDown(i32, i32, bool),
    EndKey(i32, bool),
    SetBold(i32),
    SetItalic(i32),
    SetAlignment(i32),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct NativeState {
    anchor: usize,
    active: usize,
    gravity: i32,
    flags: u32,
    selection_type: i32,
    bold: i32,
    italic: i32,
    alignment: i32,
    duplicate_gravity: Option<i32>,
    story: &'static str,
}
#[allow(clippy::too_many_arguments)]
const fn state(
    anchor: usize,
    active: usize,
    gravity: i32,
    flags: u32,
    selection_type: i32,
    bold: i32,
    italic: i32,
    alignment: i32,
    duplicate_gravity: Option<i32>,
    story: &'static str,
) -> NativeState {
    NativeState {
        anchor,
        active,
        gravity,
        flags,
        selection_type,
        bold,
        italic,
        alignment,
        duplicate_gravity,
        story,
    }
}
struct NativeCommand {
    step: usize,
    setup: bool,
    target: NativeTarget,
    operation: NativeOperation,
    hresult: u32,
    output: Option<i32>,
    before: NativeState,
    after: NativeState,
    action_format: (i32, i32, i32),
}
#[allow(clippy::too_many_arguments)]
const fn command(
    step: usize,
    setup: bool,
    target: NativeTarget,
    operation: NativeOperation,
    hresult: u32,
    output: Option<i32>,
    before: NativeState,
    after: NativeState,
    action_format: (i32, i32, i32),
) -> NativeCommand {
    NativeCommand {
        step,
        setup,
        target,
        operation,
        hresult,
        output,
        before,
        after,
        action_format,
    }
}
struct NativeFixture {
    name: &'static str,
    text: &'static str,
    supplied_length: usize,
    story_length: usize,
    commands: &'static [NativeCommand],
}

struct Replay {
    view: Ref<RichTextBlock>,
    document: RichEditTextDocument,
    duplicate: Option<RichEditTextRange>,
}
impl Replay {
    fn new(fixture: &NativeFixture) -> Self {
        assert_eq!(fixture.text.encode_utf16().count(), fixture.supplied_length);
        assert_eq!(fixture.story_length, fixture.supplied_length + 1);
        let view = RichTextBlock::new();
        view.set_document(RichTextDocument::from_text(
            &fixture.text.replace('\r', "\n"),
        ));
        let document = RichEditTextDocument::new(view.document());
        document.editor().set_text_view(Some(view.clone())).unwrap();
        Self {
            view,
            document,
            duplicate: None,
        }
    }

    fn independent_target(&self, target: NativeTarget) -> RichEditTextRange {
        match target {
            NativeTarget::Duplicate => self.duplicate.as_ref().unwrap().clone(),
            NativeTarget::External(start, end) => {
                let length = self.document.document().text_length();
                assert!(start <= length && end <= length + 1);
                // Native paragraph inspection includes its synthetic terminal
                // EOP. Only that extra endpoint projects to the visible end;
                // this does not model independently styled paragraph marks.
                self.document.get_range(start, end.min(length)).unwrap()
            }
            NativeTarget::Primary => panic!("the native selection is not an independent range"),
        }
    }

    fn apply(&mut self, fixture: &NativeFixture, command: &NativeCommand) -> bool {
        use NativeOperation::*;
        let selection = self.document.selection();
        if matches!(command.operation, TypeText(None)) {
            // Rust's &str boundary cannot carry a null BSTR. Preserve the two
            // exact native rejections as transport observations, not fake calls.
            assert_eq!(command.hresult, 0x80070057);
            assert_eq!(command.before, command.after);
            return false;
        }
        if matches!(command.operation, MoveUp(..) | MoveDown(..) | EndKey(..)) {
            self.view.measure(Size::new(300.0, 500.0));
            self.view.arrange(Rect::new(0.0, 0.0, 300.0, 500.0));
        }
        let result: Result<Option<i32>, EditError> = match command.operation {
            GetGravity => Ok(Some(match command.target {
                NativeTarget::Primary => selection.gravity() as i32,
                target => self.independent_target(target).gravity() as i32,
            })),
            SetGravity(value) => match command.target {
                NativeTarget::Primary => selection.set_gravity_value(value).map(|_| None),
                target => self
                    .independent_target(target)
                    .set_gravity_value(value)
                    .map(|_| None),
            },
            SetRange(anchor, active) => selection.set_range(anchor, active).map(|_| None),
            Duplicate => {
                assert!(self.duplicate.is_none());
                self.duplicate = Some(selection.range().unwrap());
                Ok(None)
            }
            TypeText(Some(text)) => selection.type_text(text).map(|_| None),
            TypeText(None) => unreachable!(),
            MoveLeft(unit, count, extend) => selection
                .move_left(native_unit(unit), count, extend)
                .map(Some),
            MoveRight(unit, count, extend) => selection
                .move_right(native_unit(unit), count, extend)
                .map(Some),
            MoveUp(unit, count, extend) => selection
                .move_up(native_unit(unit), count, extend)
                .map(Some),
            MoveDown(unit, count, extend) => selection
                .move_down(native_unit(unit), count, extend)
                .map(Some),
            EndKey(unit, extend) => selection
                .end_key(native_unit(unit), extend)
                .map(|value| Some(value as i32)),
            SetBold(value) => self
                .independent_target(command.target)
                .set_character_format(&CharacterFormat {
                    font_weight: Some(if value == -1 {
                        FontWeight::Bold
                    } else {
                        FontWeight::Normal
                    }),
                    ..Default::default()
                })
                .map(|_| None),
            SetItalic(value) => self
                .independent_target(command.target)
                .set_character_format(&CharacterFormat {
                    font_style: Some(if value == -1 {
                        FontStyle::Italic
                    } else {
                        FontStyle::Normal
                    }),
                    ..Default::default()
                })
                .map(|_| None),
            SetAlignment(value) => self
                .independent_target(command.target)
                .set_paragraph_format(&ParagraphFormat {
                    text_alignment: Some(if value == 2 {
                        TextAlignment::Right
                    } else {
                        TextAlignment::Left
                    }),
                    ..Default::default()
                })
                .map(|_| None),
        };
        if command.hresult == 0 {
            assert_eq!(
                result.unwrap(),
                command.output,
                "{} step {} output",
                fixture.name,
                command.step
            );
        } else {
            assert_eq!(command.hresult, 0x80070057);
            assert!(
                matches!(
                    result,
                    Err(EditError::Document(DocumentError::InvalidFormat(_)))
                ),
                "{} step {} rejection",
                fixture.name,
                command.step
            );
        }
        true
    }

    fn assert_state(&self, fixture: &NativeFixture, step: usize, expected: NativeState) {
        let selection = self.document.selection();
        assert_eq!(
            selection.gravity() as i32,
            expected.gravity,
            "{} step {step} gravity",
            fixture.name
        );
        assert_eq!(
            selection.selection(),
            EditorSelection::new(expected.anchor, expected.active),
            "{} step {step} direction",
            fixture.name
        );
        assert_eq!(
            selection.selection_type() as i32,
            expected.selection_type,
            "{} step {step} type",
            fixture.name
        );
        // START_ACTIVE and REPLACE describe supported selection identity. Native
        // automatic AT_END_OF_LINE updates remain a separate navigation boundary;
        // its literal bit is retained in NativeState, not rewritten here.
        for option in [SelectionOptions::START_ACTIVE, SelectionOptions::REPLACE] {
            assert_eq!(
                selection.options().contains(option),
                expected.flags & option.bits() != 0,
                "{} step {step} option {option:?}",
                fixture.name
            );
        }
        assert_eq!(
            self.duplicate.as_ref().map(|range| range.gravity() as i32),
            expected.duplicate_gravity,
            "{} step {step} independent mode",
            fixture.name
        );
        let visible = expected
            .story
            .strip_suffix('\r')
            .expect("native terminal EOP")
            .replace('\r', "\n");
        assert_eq!(
            self.document.text(),
            visible,
            "{} step {step} exact visible story",
            fixture.name
        );
    }
}
fn native_unit(value: i32) -> TextRangeUnit {
    match value {
        1 => TextRangeUnit::Character,
        5 => TextRangeUnit::Line,
        _ => panic!("unmapped native fixture unit {value}"),
    }
}
fn character_pair(format: &CharacterFormat) -> (i32, i32) {
    (
        if format.font_weight == Some(FontWeight::Bold) {
            -1
        } else {
            0
        },
        if format.font_style == Some(FontStyle::Italic) {
            -1
        } else {
            0
        },
    )
}
fn range_pair(range: &RichEditTextRange) -> (i32, i32) {
    match range.character_format().unwrap() {
        TextFormatValue::Uniform(format) => character_pair(&format),
        TextFormatValue::Mixed => (-9_999_999, -9_999_999),
    }
}
fn paragraph_alignment(selection: &RichEditTextSelection) -> i32 {
    native_paragraph_alignment(selection.paragraph_format().unwrap())
}
fn native_paragraph_alignment(value: TextFormatValue<ParagraphFormat>) -> i32 {
    match value {
        TextFormatValue::Uniform(format) => match format.text_alignment {
            Some(TextAlignment::Left) => 0,
            Some(TextAlignment::Center) => 1,
            Some(TextAlignment::Right) => 2,
            Some(TextAlignment::Justify) => 3,
            Some(other) => panic!("Unmapped native paragraph alignment {other:?}"),
            // The queried fixtures contain paragraphs; effective formatting
            // must resolve even an inherited alignment to its concrete value.
            None => panic!("Native paragraph replay requires resolved alignment"),
        },
        TextFormatValue::Mixed => -9_999_999,
    }
}
fn formatted_model() -> RichTextDocument {
    let model = RichTextDocument::from_text("abCD");
    model
        .apply_format(
            0..4,
            &CharacterFormat {
                font_weight: Some(FontWeight::Normal),
                font_style: Some(FontStyle::Normal),
                ..Default::default()
            },
        )
        .unwrap();
    model
        .apply_format(
            0..2,
            &CharacterFormat {
                font_weight: Some(FontWeight::Bold),
                ..Default::default()
            },
        )
        .unwrap();
    model
        .apply_format(
            2..4,
            &CharacterFormat {
                font_style: Some(FontStyle::Italic),
                ..Default::default()
            },
        )
        .unwrap();
    model
}
fn formatted() -> RichEditTextDocument {
    RichEditTextDocument::new(formatted_model())
}

#[test]
fn rich_selection_gravity_replays_248_native_commands_and_keeps_two_null_observations() {
    let _scope = test_scope();
    assert_eq!(native_cases::FIXTURES.len(), 18);
    let mut commands = 0;
    let mut calls = 0;
    for fixture in native_cases::FIXTURES {
        let mut replay = Replay::new(fixture);
        for (index, command) in fixture.commands.iter().enumerate() {
            assert_eq!(command.step, index);
            replay.assert_state(fixture, command.step, command.before);
            calls += usize::from(replay.apply(fixture, command));
            replay.assert_state(fixture, command.step, command.after);
            commands += 1;
        }
    }
    assert_eq!((commands, calls), (250, 248));
}

#[test]
fn rich_selection_gravity_replays_ten_native_collapsed_format_and_typing_fixtures() {
    let _scope = test_scope();
    let mut fixtures = 0;
    let mut insertions = 0;
    for fixture in native_cases::FIXTURES
        .iter()
        .filter(|case| case.name.ends_with("_ordinary") || case.name.ends_with("_paragraph"))
    {
        let mut replay = Replay::new(fixture);
        for command in fixture.commands {
            replay.apply(fixture, command);
            if command.setup {
                continue;
            }
            let selection = replay.document.selection();
            let expected = (command.after.bold, command.after.italic);
            assert_eq!(
                character_pair(&selection.character_format().unwrap()),
                expected,
                "{} step {} live format",
                fixture.name,
                command.step
            );
            assert_eq!(
                paragraph_alignment(&selection),
                command.after.alignment,
                "{} step {} paragraph format",
                fixture.name,
                command.step
            );
            if let NativeTarget::External(start, end) = command.target {
                assert!(matches!(command.operation, NativeOperation::GetGravity));
                let inserted = replay.document.get_range(start, end).unwrap();
                assert_eq!(
                    range_pair(&inserted),
                    (command.action_format.0, command.action_format.1),
                    "{} inserted character format",
                    fixture.name
                );
                insertions += 1;
            }
        }
        fixtures += 1;
    }
    assert_eq!((fixtures, insertions), (10, 10));
}

#[test]
fn rich_selection_gravity_native_nonempty_replacement_retains_trailing_alignment() {
    let _scope = test_scope();
    let mut replacements = 0;
    for fixture in native_cases::FIXTURES
        .iter()
        .filter(|case| case.name.ends_with("_navigation"))
    {
        let mut replay = Replay::new(fixture);
        for command in fixture.commands {
            replay.apply(fixture, command);
            if matches!(command.operation, NativeOperation::TypeText(Some("😀"))) {
                assert_eq!(
                    range_pair(&replay.document.get_range(1, 3).unwrap()),
                    (-1, 0)
                );
                assert_eq!(
                    character_pair(&replay.document.selection().character_format().unwrap()),
                    (command.after.bold, command.after.italic)
                );
                // The extended native paragraph boundary matrix separately
                // confirms this alignment rule across direction and input form.
                assert_eq!(command.after.alignment, 2);
                assert_eq!(paragraph_alignment(&replay.document.selection()), 2);
                replacements += 1;
            }
        }
    }
    assert_eq!(replacements, 5);
}

#[test]
fn rich_selection_paragraph_typing_replays_240_native_commands_and_alignment_queries() {
    let _scope = test_scope();
    assert_eq!(paragraph_native_cases::FIXTURES.len(), 16);
    let mut commands = 0;
    let mut paragraph_queries = 0;
    let mut inserted_format_queries = 0;
    for fixture in paragraph_native_cases::FIXTURES {
        let mut replay = Replay::new(fixture);
        let printable_insertion = matches!(
            fixture.commands[9].operation,
            NativeOperation::TypeText(Some(text))
                if !text.is_empty() && !text.contains(['\r', '\n'])
        );
        for command in fixture.commands {
            assert!(replay.apply(fixture, command));
            replay.assert_state(fixture, command.step, command.after);
            if command.step >= 9 {
                assert_eq!(
                    paragraph_alignment(&replay.document.selection()),
                    command.after.alignment,
                    "{} step {} live alignment",
                    fixture.name,
                    command.step,
                );
                if matches!(command.operation, NativeOperation::GetGravity) {
                    if let NativeTarget::External(..) = command.target {
                        let range = replay.independent_target(command.target);
                        let alignment =
                            native_paragraph_alignment(range.paragraph_format().unwrap());
                        assert_eq!(
                            alignment, command.action_format.2,
                            "{} step {} paragraph alignment",
                            fixture.name, command.step,
                        );
                        if command.step == 10 && printable_insertion {
                            assert_eq!(
                                range_pair(&range),
                                (command.action_format.0, command.action_format.1),
                                "{} inserted character format",
                                fixture.name,
                            );
                            inserted_format_queries += 1;
                        }
                        paragraph_queries += 1;
                    }
                }
            }
            commands += 1;
        }
    }
    assert_eq!(
        (commands, paragraph_queries, inserted_format_queries),
        (240, 64, 11)
    );
}

#[test]
fn rich_selection_gravity_is_shared_and_drives_real_control_typing() {
    let _scope = test_scope();
    let owner = RichEditBox::new();
    owner.set_document(formatted_model());
    owner.select(2, 2);
    let document = owner.text_document();
    let first = document.selection();
    let second = document.selection();
    first.set_gravity(RangeGravity::Forward);
    assert_eq!(second.gravity(), RangeGravity::Forward);
    assert_eq!(document.editor().selection_gravity(), RangeGravity::Forward);
    assert_eq!(character_pair(&second.character_format().unwrap()), (0, -1));
    assert!(owner.insert_text("X").unwrap());
    assert_eq!(owner.text(), "abXCD");
    assert_eq!(range_pair(&document.get_range(2, 3).unwrap()), (0, -1));
    assert_eq!(first.gravity(), RangeGravity::Forward);
    first.set_gravity(RangeGravity::Outward);
    assert_eq!(second.gravity(), RangeGravity::UIBehavior);
}

#[test]
fn rich_selection_gravity_does_not_promote_resolved_format_into_typing_patch() {
    let _scope = Dispatcher::unit_test_scope();
    let document = formatted();
    let selection = document.selection();
    selection.set_range(2, 2).unwrap();
    let inherited_size = selection.character_format().unwrap().font_size;
    let explicit = CharacterFormat {
        font_size: Some(31.0),
        ..Default::default()
    };
    selection.set_character_format(&explicit).unwrap();
    selection.set_gravity(RangeGravity::Forward);
    selection.type_text("X").unwrap();
    assert_eq!(document.editor().typing_format(), explicit);
    assert_eq!(range_pair(&document.get_range(2, 3).unwrap()), (0, -1));
    selection.set_range(1, 1).unwrap();
    assert_eq!(
        document.editor().typing_format(),
        CharacterFormat::default()
    );
    assert_eq!(
        character_pair(&selection.character_format().unwrap()),
        (-1, 0)
    );
    assert_eq!(
        selection.character_format().unwrap().font_size,
        inherited_size
    );
}

#[test]
fn rich_selection_gravity_default_typing_retains_original_run_and_inheritance() {
    let _scope = Dispatcher::unit_test_scope();
    let model = RichTextDocument::from_text("abcd");
    let original = model.element_at(1, TextGravity::Forward).unwrap();
    assert!(original.cast::<Run>().is_some());
    let before = CharacterFormat::from_element(&original);
    let document = RichEditTextDocument::new(model.clone());
    document.selection().set_range(2, 2).unwrap();
    document.selection().type_text("X").unwrap();
    assert_eq!(document.text(), "abXcd");
    assert!(model.element_at(1, TextGravity::Forward) == Some(original.clone()));
    assert_eq!(CharacterFormat::from_element(&original), before);
}

#[test]
fn rich_selection_gravity_checked_values_leave_readonly_preview_and_redo_untouched() {
    let _scope = Dispatcher::unit_test_scope();
    let document = formatted();
    let selection = document.selection();
    selection.set_range(2, 2).unwrap();
    selection.type_text("Q").unwrap();
    document.undo().unwrap();
    let history = (document.can_undo(), document.can_redo());
    selection.set_gravity(RangeGravity::Forward);
    assert_eq!((document.can_undo(), document.can_redo()), history);
    document.editor().set_preedit("preview", Some(2)).unwrap();
    document.editor().set_is_read_only(true);
    let snapshot = document.document().snapshot();
    let position = selection.selection();
    let history = (document.can_undo(), document.can_redo());
    for value in [i32::MIN, -1, 5, i32::MAX] {
        assert!(matches!(
            selection.set_gravity_value(value),
            Err(EditError::Document(DocumentError::InvalidFormat(_)))
        ));
        assert_eq!(selection.gravity(), RangeGravity::Forward);
    }
    selection.set_gravity(RangeGravity::Inward);
    assert_eq!(document.document().snapshot(), snapshot);
    assert_eq!(selection.selection(), position);
    assert_eq!((document.can_undo(), document.can_redo()), history);
    assert!(document.editor().is_composing());
    assert!(!selection.type_text("").unwrap());
    assert!(matches!(selection.type_text("X"), Err(EditError::ReadOnly)));
    assert_eq!(selection.gravity(), RangeGravity::Inward);
}

#[test]
fn rich_selection_gravity_history_retains_current_mode_and_failed_group_restores_it() {
    let _scope = Dispatcher::unit_test_scope();
    let document = formatted();
    let selection = document.selection();
    selection.set_range(2, 2).unwrap();
    selection.set_gravity(RangeGravity::Forward);
    selection.type_text("X").unwrap();
    selection.set_gravity(RangeGravity::Backward);
    document.undo().unwrap();
    assert_eq!(selection.gravity(), RangeGravity::Backward);
    document.redo().unwrap();
    assert_eq!(selection.gravity(), RangeGravity::Backward);
    let before = document.document().snapshot();
    let position = selection.selection();
    let result: Result<(), EditError> = document.editor().edit_group(|editor| {
        editor.set_selection_gravity(RangeGravity::Inward);
        editor.type_text("Y")?;
        editor.set_selection_gravity(RangeGravity::UIBehavior);
        Err(EditError::ReadOnly)
    });
    assert!(matches!(result, Err(EditError::ReadOnly)));
    assert_eq!(document.document().snapshot(), before);
    assert_eq!(selection.selection(), position);
    assert_eq!(selection.gravity(), RangeGravity::Backward);
    document
        .editor()
        .edit_group(|editor| {
            editor.set_selection_gravity(RangeGravity::Forward);
            editor.type_text("Z")?;
            editor.set_selection_gravity(RangeGravity::Inward);
            Ok(())
        })
        .unwrap();
    document.undo().unwrap();
    assert_eq!(document.document().snapshot(), before);
    assert_eq!(selection.gravity(), RangeGravity::Inward);
}

#[test]
fn rich_selection_gravity_rejected_text_and_endpoint_requests_preserve_state() {
    let _scope = Dispatcher::unit_test_scope();
    let document = RichEditTextDocument::new(RichTextDocument::from_text("a😀b"));
    let selection = document.selection();
    selection.set_range(3, 3).unwrap();
    selection.set_gravity(RangeGravity::Backward);
    let before = document.document().snapshot();
    document.editor().set_max_length(4);
    assert!(!selection.type_text("Q").unwrap());
    assert!(selection.set_range(2, 2).is_err());
    assert_eq!(selection.selection(), EditorSelection::caret(3));
    assert_eq!(selection.gravity(), RangeGravity::Backward);
    assert_eq!(document.document().snapshot(), before);
    assert!(!document.can_undo());
}

struct CallbackBrush {
    callback: RefCell<Option<Box<dyn FnOnce()>>>,
    reads: Cell<usize>,
    fire_at: Cell<Option<usize>>,
    equality_reads: Cell<bool>,
}
impl CallbackBrush {
    fn new() -> Rc<Self> {
        Rc::new(Self {
            callback: RefCell::new(None),
            reads: Cell::new(0),
            fire_at: Cell::new(None),
            equality_reads: Cell::new(false),
        })
    }
    fn arm(&self, callback: impl FnOnce() + 'static) {
        self.arm_at(1, callback);
    }
    fn arm_at(&self, read: usize, callback: impl FnOnce() + 'static) {
        self.reads.set(0);
        self.fire_at.set(Some(read));
        self.equality_reads.set(false);
        *self.callback.borrow_mut() = Some(Box::new(callback));
    }
    fn arm_equality(&self, callback: impl FnOnce() + 'static) {
        self.arm(callback);
        self.equality_reads.set(true);
    }
    fn read(&self) {
        let read = self.reads.get() + 1;
        self.reads.set(read);
        if self.fire_at.get() == Some(read) {
            let callback = self.callback.borrow_mut().take();
            if let Some(callback) = callback {
                callback();
            }
        }
    }
    fn disarm(&self) -> usize {
        self.fire_at.set(None);
        self.equality_reads.set(false);
        let callback = self.callback.borrow_mut().take();
        drop(callback);
        self.reads.get()
    }
}
impl IBrush for CallbackBrush {
    fn opacity(&self) -> f64 {
        1.0
    }
    fn transform(&self) -> Option<Rc<dyn ITransform>> {
        None
    }
    fn transform_origin(&self) -> RelativePoint {
        RelativePoint::CENTER
    }
    fn relative_transform(&self) -> Option<Rc<dyn ITransform>> {
        None
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_solid_color_brush(&self) -> Option<&dyn ISolidColorBrush> {
        if !self.equality_reads.get() {
            self.read();
        }
        None
    }
    fn equals(&self, other: &dyn IBrush) -> bool {
        if self.equality_reads.get() {
            self.read();
        }
        self.reference_id() == other.reference_id()
    }
}
fn source_brush(document: &RichEditTextDocument) -> Rc<CallbackBrush> {
    let brush = CallbackBrush::new();
    document
        .document()
        .element_at(1, TextGravity::Forward)
        .unwrap()
        .set_foreground(Some(brush.clone()));
    brush
}

#[test]
fn rich_selection_gravity_read_rejects_mode_changed_away_and_back() {
    let _scope = Dispatcher::unit_test_scope();
    let document = formatted();
    let selection = document.selection();
    selection.set_range(2, 2).unwrap();
    let brush = source_brush(&document);
    let callback_selection = selection.clone();
    brush.arm(move || {
        callback_selection.set_gravity(RangeGravity::Forward);
        callback_selection.set_gravity(RangeGravity::UIBehavior);
    });
    assert!(matches!(
        selection.character_format(),
        Err(EditError::ReentrantEdit)
    ));
    assert_eq!(selection.gravity(), RangeGravity::UIBehavior);
    assert_eq!(document.text(), "abCD");
}

#[test]
fn rich_selection_gravity_read_rejects_same_value_newer_intent() {
    let _scope = Dispatcher::unit_test_scope();
    let document = formatted();
    let selection = document.selection();
    selection.set_range(2, 2).unwrap();
    let brush = source_brush(&document);
    let callback_selection = selection.clone();
    brush.arm(move || callback_selection.set_gravity(RangeGravity::UIBehavior));
    assert!(matches!(
        selection.character_format_value(),
        Err(EditError::ReentrantEdit)
    ));
    assert_eq!(selection.gravity(), RangeGravity::UIBehavior);
}

#[test]
fn rich_selection_gravity_typing_snapshot_reentry_releases_editor_borrow() {
    let _scope = Dispatcher::unit_test_scope();
    let document = formatted();
    let selection = document.selection();
    selection.set_range(2, 2).unwrap();
    let brush = CallbackBrush::new();
    selection
        .set_character_format(&CharacterFormat {
            foreground: Some(Some(brush.clone())),
            ..Default::default()
        })
        .unwrap();
    let callback_selection = selection.clone();
    brush.arm(move || {
        callback_selection.set_range(1, 1).unwrap();
        callback_selection.set_range(2, 2).unwrap();
    });
    assert!(matches!(
        selection.character_format(),
        Err(EditError::ReentrantEdit)
    ));
    assert_eq!(selection.selection(), EditorSelection::caret(2));
    assert_eq!(
        document.editor().typing_format(),
        CharacterFormat::default()
    );
}

#[test]
fn rich_selection_gravity_nonempty_format_query_fences_live_mode_reentry() {
    let _scope = Dispatcher::unit_test_scope();
    let document = formatted();
    let selection = document.selection();
    selection.set_range(0, 2).unwrap();
    let brush = source_brush(&document);
    let callback_selection = selection.clone();
    brush.arm(move || {
        callback_selection.set_gravity(RangeGravity::Inward);
        callback_selection.set_gravity(RangeGravity::UIBehavior);
    });
    assert!(matches!(
        selection.character_format_value(),
        Err(EditError::ReentrantEdit)
    ));
    assert_eq!(selection.selection(), EditorSelection::new(0, 2));
    assert_eq!(range_pair(&document.get_range(0, 2).unwrap()), (-1, 0));
}

#[test]
fn rich_selection_gravity_insertion_rejects_stale_snapshot_before_text_mutation() {
    let _scope = Dispatcher::unit_test_scope();
    let document = formatted();
    let selection = document.selection();
    selection.set_range(2, 2).unwrap();
    let brush = source_brush(&document);
    document.clear_undo_redo_history();
    let before = document.document().snapshot();
    let callback_selection = selection.clone();
    brush.arm(move || {
        callback_selection.set_gravity(RangeGravity::Forward);
        callback_selection.set_gravity(RangeGravity::UIBehavior);
    });
    assert!(matches!(
        selection.type_text("X"),
        Err(EditError::ReentrantEdit)
    ));
    assert_eq!(document.document().snapshot(), before);
    assert_eq!(selection.selection(), EditorSelection::caret(2));
    assert_eq!(selection.gravity(), RangeGravity::UIBehavior);
    assert!(!document.can_undo());
}

#[test]
fn rich_selection_gravity_format_callback_model_write_survives_rejected_read() {
    let _scope = Dispatcher::unit_test_scope();
    let document = formatted();
    let selection = document.selection();
    selection.set_range(2, 2).unwrap();
    let brush = source_brush(&document);
    let model = document.document();
    brush.arm(move || {
        model
            .replace_text(0..model.text_length(), "external")
            .unwrap()
    });
    assert!(matches!(
        selection.character_format(),
        Err(EditError::ReentrantEdit)
    ));
    assert_eq!(document.text(), "external");
    assert!(!document.can_undo());
}

#[test]
fn rich_selection_gravity_default_snapshot_callback_can_replace_defaults() {
    let _scope = Dispatcher::unit_test_scope();
    let document = RichEditTextDocument::new(RichTextDocument::new());
    let brush = CallbackBrush::new();
    document
        .editor()
        .set_default_character_format(&CharacterFormat {
            foreground: Some(Some(brush.clone())),
            ..Default::default()
        })
        .unwrap();
    let editor = document.editor();
    let callback_editor = editor.clone();
    brush.arm(move || {
        callback_editor
            .set_default_character_format(&CharacterFormat {
                font_size: Some(28.0),
                ..Default::default()
            })
            .unwrap()
    });
    let _ = editor.default_character_format();
    assert_eq!(editor.default_character_format().font_size, Some(28.0));
}

#[test]
fn rich_selection_gravity_publication_preserves_newer_mode_without_restyling_typed_text() {
    let _scope = test_scope();
    let owner = RichEditBox::new();
    owner.set_document(formatted_model());
    owner.select(2, 2);
    let document = owner.text_document();
    let selection = document.selection();
    let callback_selection = selection.clone();
    let called = Rc::new(Cell::new(false));
    let observed = called.clone();
    owner.selection_changing(move |_, _| {
        if !called.replace(true) {
            callback_selection.set_gravity(RangeGravity::Forward);
        }
    });
    assert!(owner.insert_text("X").unwrap());
    assert!(observed.get());
    assert_eq!(selection.gravity(), RangeGravity::Forward);
    assert_eq!(range_pair(&document.get_range(2, 3).unwrap()), (-1, 0));
    assert_eq!(
        character_pair(&selection.character_format().unwrap()),
        (0, -1)
    );
    document.undo().unwrap();
    assert_eq!(selection.gravity(), RangeGravity::Forward);
}

#[test]
fn rich_selection_gravity_independent_range_keeps_own_affinity_and_typing_snapshot() {
    let _scope = Dispatcher::unit_test_scope();
    let document = formatted();
    let selection = document.selection();
    selection.set_range(2, 2).unwrap();
    selection.set_gravity(RangeGravity::Forward);
    let patch = CharacterFormat {
        font_size: Some(33.0),
        ..Default::default()
    };
    selection.set_character_format(&patch).unwrap();
    let range = document.get_range(2, 2).unwrap();
    range.set_gravity(RangeGravity::Backward);
    range.set_text("X").unwrap();
    assert_eq!(selection.gravity(), RangeGravity::Forward);
    assert_eq!(range.gravity(), RangeGravity::Backward);
    assert_eq!(range_pair(&range), (-1, 0));
    assert_eq!(document.editor().typing_format(), patch);
    assert_eq!(
        selection.range().unwrap().gravity(),
        RangeGravity::UIBehavior
    );
}

#[test]
fn rich_selection_gravity_legacy_preview_and_commit_consume_transient_affinity() {
    let _scope = Dispatcher::unit_test_scope();
    let document = formatted();
    let selection = document.selection();
    selection.set_range(2, 2).unwrap();
    selection.set_gravity(RangeGravity::Forward);
    document.editor().set_preedit("X", Some(1)).unwrap();
    assert_eq!(range_pair(&document.get_range(2, 3).unwrap()), (0, -1));
    assert_eq!(
        document.editor().typing_format(),
        CharacterFormat::default()
    );
    selection.set_gravity(RangeGravity::Backward);
    assert!(document.editor().is_composing());
    document.editor().commit_composition("Y").unwrap();
    assert_eq!(document.text(), "abYCD");
    assert_eq!(range_pair(&document.get_range(2, 3).unwrap()), (-1, 0));
    assert_eq!(selection.gravity(), RangeGravity::Backward);
    document.undo().unwrap();
    assert_eq!(document.text(), "abCD");
    assert_eq!(selection.gravity(), RangeGravity::Backward);
}

#[test]
fn rich_selection_gravity_exact_input_uses_live_caret_but_not_remote_range() {
    let _scope = Dispatcher::unit_test_scope();
    for (caret, expected) in [(2, (0, -1)), (0, (-1, 0))] {
        let document = formatted();
        document.selection().set_range(caret, caret).unwrap();
        document.selection().set_gravity(RangeGravity::Forward);
        document
            .editor()
            .apply_exact_input_edit(
                &TextInputMethodEdit::new(TextSelection::new(2, 2), "X", TextSelection::new(3, 3)),
                false,
            )
            .unwrap();
        assert_eq!(range_pair(&document.get_range(2, 3).unwrap()), expected);
        assert_eq!(
            document.editor().typing_format(),
            CharacterFormat::default()
        );
        assert_eq!(document.selection().gravity(), RangeGravity::Forward);
    }
}

#[test]
fn rich_selection_gravity_exact_composition_keeps_affinity_format_after_decoration_removal() {
    let _scope = Dispatcher::unit_test_scope();
    let document = formatted();
    let selection = document.selection();
    selection.set_range(2, 2).unwrap();
    selection.set_gravity(RangeGravity::Forward);
    document.editor().begin_input_method_composition().unwrap();
    document
        .editor()
        .apply_exact_input_edit(
            &TextInputMethodEdit::new(TextSelection::new(2, 2), "X", TextSelection::new(3, 3)),
            true,
        )
        .unwrap();
    assert_eq!(range_pair(&document.get_range(2, 3).unwrap()), (0, -1));
    assert!(document.editor().complete_exact_composition().unwrap());
    assert_eq!(range_pair(&document.get_range(2, 3).unwrap()), (0, -1));
    assert_eq!(
        document.editor().typing_format(),
        CharacterFormat::default()
    );
    document.undo().unwrap();
    assert_eq!(document.text(), "abCD");
    assert_eq!(selection.gravity(), RangeGravity::Forward);
}

#[test]
fn rich_selection_gravity_surrogate_lookup_and_terminal_defaults_keep_visible_boundaries() {
    let _scope = Dispatcher::unit_test_scope();
    let model = RichTextDocument::from_text("a😀Z");
    model
        .apply_format(
            0..3,
            &CharacterFormat {
                font_weight: Some(FontWeight::Bold),
                ..Default::default()
            },
        )
        .unwrap();
    let document = RichEditTextDocument::new(model);
    let selection = document.selection();
    selection.set_range(3, 3).unwrap();
    selection.set_gravity(RangeGravity::Backward);
    assert_eq!(
        character_pair(&selection.character_format().unwrap()),
        (-1, 0)
    );
    selection.type_text("X").unwrap();
    assert_eq!(document.text(), "a😀XZ");
    selection.set_range(5, 5).unwrap();
    selection.set_gravity(RangeGravity::Forward);
    let final_paragraph = document.document().paragraphs().last().unwrap().clone();
    assert_eq!(
        selection.character_format().unwrap(),
        CharacterFormat::effective(&final_paragraph.upcast::<TextElement>())
    );
    assert_eq!(selection.selection(), EditorSelection::caret(5));
    assert!(selection.set_range(6, 6).is_err());
    assert_eq!(selection.gravity(), RangeGravity::Forward);
}

#[test]
fn rich_selection_gravity_explicit_affinity_survives_overtype_extent_planning() {
    let _scope = Dispatcher::unit_test_scope();
    for (gravity, expected) in [
        (RangeGravity::Backward, (-1, 0)),
        (RangeGravity::Forward, (0, -1)),
    ] {
        let document = formatted();
        let selection = document.selection();
        selection.set_range(2, 2).unwrap();
        selection.set_options(SelectionOptions::OVERTYPE).unwrap();
        selection.set_gravity(gravity);
        selection.type_text("X").unwrap();
        assert_eq!(document.text(), "abXD");
        assert_eq!(range_pair(&document.get_range(2, 3).unwrap()), expected);
        assert_eq!(
            document.editor().typing_format(),
            CharacterFormat::default()
        );
        assert_eq!(selection.gravity(), gravity);
    }
}

fn composing_with_brush(exact: bool) -> (RichEditTextDocument, Rc<CallbackBrush>) {
    let model = formatted_model();
    let brush = CallbackBrush::new();
    model
        .apply_format(
            0..4,
            &CharacterFormat {
                foreground: Some(Some(brush.clone())),
                ..Default::default()
            },
        )
        .unwrap();
    let document = RichEditTextDocument::new(model);
    document.selection().set_range(2, 2).unwrap();
    document.selection().set_gravity(RangeGravity::Forward);
    if exact {
        document.editor().begin_input_method_composition().unwrap();
        document
            .editor()
            .apply_exact_input_edit(
                &TextInputMethodEdit::new(TextSelection::new(2, 2), "P", TextSelection::new(3, 3)),
                true,
            )
            .unwrap();
    } else {
        document.editor().set_preedit("P", Some(1)).unwrap();
    }
    (document, brush)
}
fn next_composition_step(document: &RichEditTextDocument, kind: usize) -> Result<(), EditError> {
    match kind {
        0 => document.editor().set_preedit("Q", Some(1)),
        1 => document.editor().commit_composition("Q").map(|_| ()),
        2 => document
            .editor()
            .apply_exact_input_edit(
                &TextInputMethodEdit::new(TextSelection::new(3, 3), "Q", TextSelection::new(4, 4)),
                true,
            )
            .map(|_| ()),
        _ => unreachable!(),
    }
}

#[test]
fn rich_selection_gravity_composition_read_reentry_keeps_preview_and_newer_mode() {
    let _scope = Dispatcher::unit_test_scope();
    // Measure this fixture's service reads, then inject at every actual read
    // boundary. No hard-coded clone count or private phase hook is assumed.
    // Rejections before publication retain the complete visible IME transaction;
    // accepted later metadata changes own the new mode for the next operation.
    for kind in 0..3 {
        let (probe, brush) = composing_with_brush(kind == 2);
        brush.arm_at(usize::MAX, || {});
        next_composition_step(&probe, kind).unwrap();
        let reads = brush.disarm();
        assert!(reads > 0);
        let mut rejected = 0;
        for read in 1..=reads {
            let (document, brush) = composing_with_brush(kind == 2);
            let before = document.document().snapshot();
            let position = document.selection().selection();
            let callback_selection = document.selection();
            let called = Rc::new(Cell::new(false));
            let observed = called.clone();
            brush.arm_at(read, move || {
                called.set(true);
                callback_selection.set_gravity(RangeGravity::Backward);
                callback_selection.set_gravity(RangeGravity::Inward);
            });
            let result = next_composition_step(&document, kind);
            brush.disarm();
            assert!(observed.get(), "kind {kind} service read {read}/{reads}");
            assert_eq!(document.selection().gravity(), RangeGravity::Inward);
            match result {
                Err(EditError::ReentrantEdit) => {
                    rejected += 1;
                    assert_eq!(
                        document.document().snapshot(),
                        before,
                        "kind {kind} rejected read {read}"
                    );
                    assert_eq!(document.selection().selection(), position);
                    assert!(document.editor().is_composing());
                    assert!(!document.can_undo());
                }
                Ok(()) => {
                    assert_eq!(document.text(), if kind == 2 { "abPQCD" } else { "abQCD" });
                    assert_eq!(document.editor().is_composing(), kind != 1);
                }
                Err(error) => panic!("kind {kind} read {read}: {error}"),
            }
        }
        assert!(
            rejected > 0,
            "kind {kind} must exercise guarded format reads"
        );
    }
}

#[test]
fn rich_selection_gravity_composition_preflight_model_writes_survive() {
    let _scope = Dispatcher::unit_test_scope();
    // Sweep all observed service reads, but inject an application edit only
    // while the initial preview's model version is still current. This covers
    // editor snapshot copies and the model's restoration preflight; mutation
    // during a reused child's installation setter is a separate boundary.
    for kind in 0..3 {
        let (probe, brush) = composing_with_brush(kind == 2);
        brush.arm_at(usize::MAX, || {});
        next_composition_step(&probe, kind).unwrap();
        let reads = brush.disarm();
        let mut accepted_writes = 0;
        for read in 1..=reads {
            let (document, brush) = composing_with_brush(kind == 2);
            let model = document.document();
            let version = model.mutation_version();
            let wrote = Rc::new(Cell::new(false));
            let observed = wrote.clone();
            brush.arm_at(read, move || {
                if model.mutation_version() == version {
                    match model.replace_text(0..model.text_length(), "external") {
                        Ok(()) => wrote.set(true),
                        Err(DocumentError::EditInProgress) => {}
                        Err(error) => panic!("preflight callback: {error}"),
                    }
                }
            });
            let result = next_composition_step(&document, kind);
            brush.disarm();
            if observed.get() {
                accepted_writes += 1;
                assert!(
                    matches!(result, Err(EditError::ReentrantEdit)),
                    "kind {kind} read {read}: {result:?}"
                );
                assert_eq!(document.text(), "external", "kind {kind} read {read}");
                assert!(!document.editor().is_composing());
                assert!(!document.can_undo());
                assert!(!document.can_redo());
            } else {
                result.unwrap();
            }
        }
        assert!(
            accepted_writes > 0,
            "kind {kind} must exercise source reads"
        );
    }
}

#[test]
fn rich_selection_gravity_composition_snapshot_reentry_can_clear_history() {
    let _scope = Dispatcher::unit_test_scope();
    for kind in 0..3 {
        let (document, brush) = composing_with_brush(kind == 2);
        let before = document.document().snapshot();
        let editor = document.editor();
        brush.arm(move || editor.clear_history());
        let result = next_composition_step(&document, kind);
        brush.disarm();
        assert!(matches!(result, Err(EditError::ReentrantEdit)));
        assert_eq!(document.document().snapshot(), before);
        assert!(!document.editor().is_composing());
        assert!(!document.can_undo());
        assert!(!document.can_redo());
    }
}

#[test]
fn rich_selection_gravity_model_restore_callbacks_cannot_resurrect_cleared_history() {
    fn ready(redo: bool) -> (RichEditTextDocument, Rc<CallbackBrush>) {
        let document = formatted();
        let brush = CallbackBrush::new();
        document
            .document()
            .apply_format(
                0..4,
                &CharacterFormat {
                    foreground: Some(Some(brush.clone())),
                    ..Default::default()
                },
            )
            .unwrap();
        let selection = document.selection();
        selection.set_range(2, 2).unwrap();
        selection.set_gravity(RangeGravity::Forward);
        selection.type_text("X").unwrap();
        if redo {
            document.undo().unwrap();
        }
        // The brush belongs only to document content. Reads observed below
        // come from model restoration, not the editor's three format copies.
        assert!(document.editor().typing_format().foreground.is_none());
        assert!(document
            .editor()
            .default_character_format()
            .foreground
            .is_none());
        assert!(document
            .editor()
            .default_paragraph_format()
            .character
            .foreground
            .is_none());
        (document, brush)
    }
    fn apply(document: &RichEditTextDocument, redo: bool) -> Result<bool, EditError> {
        if redo {
            document.redo()
        } else {
            document.undo()
        }
    }
    let _scope = Dispatcher::unit_test_scope();
    for redo in [false, true] {
        let (probe, brush) = ready(redo);
        brush.arm_at(usize::MAX, || {});
        assert!(apply(&probe, redo).unwrap());
        let reads = brush.disarm();
        assert!(reads > 0);
        for read in 1..=reads {
            let (document, brush) = ready(redo);
            let editor = document.editor();
            let called = Rc::new(Cell::new(false));
            let observed = called.clone();
            brush.arm_at(read, move || {
                called.set(true);
                editor.clear_history();
            });
            let result = apply(&document, redo);
            brush.disarm();
            assert!(observed.get(), "redo {redo} read {read}/{reads}");
            assert!(matches!(result, Err(EditError::ReentrantEdit) | Ok(true)));
            assert!(!document.can_undo(), "redo {redo} read {read}");
            assert!(!document.can_redo(), "redo {redo} read {read}");
            assert!(!document.editor().is_composing());
            assert_eq!(document.selection().gravity(), RangeGravity::Forward);
            assert!(matches!(document.text().as_str(), "abCD" | "abXCD"));
            document
                .document()
                .validate_range(document.selection().selection().range())
                .unwrap();
        }
    }
}

#[test]
fn rich_selection_gravity_exact_completion_fences_custom_brush_equality() {
    let _scope = Dispatcher::unit_test_scope();
    for action in 0..3 {
        let document = formatted();
        let brush = source_brush(&document);
        document.selection().set_range(2, 2).unwrap();
        document.selection().set_gravity(RangeGravity::Forward);
        document.editor().begin_input_method_composition().unwrap();
        document
            .editor()
            .apply_exact_input_edit(
                &TextInputMethodEdit::new(TextSelection::new(2, 3), "C", TextSelection::new(3, 3)),
                true,
            )
            .unwrap();
        // The clean image has unchanged content. Comparing it with the before
        // image calls the custom brush's equality implementation before any
        // cleanup of temporary composition decorations.
        let before = document.document().snapshot();
        let selection = document.selection();
        let editor = document.editor();
        let model = document.document();
        let called = Rc::new(Cell::new(false));
        let observed = called.clone();
        brush.arm_equality(move || {
            called.set(true);
            match action {
                0 => {
                    selection.set_gravity(RangeGravity::Backward);
                    selection.set_gravity(RangeGravity::Inward);
                }
                1 => model
                    .replace_text(0..model.text_length(), "external")
                    .unwrap(),
                2 => editor.clear_history(),
                _ => unreachable!(),
            }
        });
        let result = document.editor().complete_exact_composition();
        brush.disarm();
        assert!(observed.get(), "action {action} must read custom equality");
        assert!(matches!(result, Err(EditError::ReentrantEdit)));
        if action == 1 {
            assert_eq!(document.text(), "external");
        } else {
            assert_eq!(document.document().snapshot(), before);
        }
        assert_eq!(document.editor().is_composing(), action == 0);
        assert_eq!(
            document.selection().gravity(),
            if action == 0 {
                RangeGravity::Inward
            } else {
                RangeGravity::Forward
            }
        );
        assert!(!document.can_undo());
        assert!(!document.can_redo());
    }
}

#[test]
fn rich_selection_gravity_nonempty_queries_preserve_uniform_and_mixed_content() {
    let _scope = Dispatcher::unit_test_scope();
    let document = formatted();
    let before = document.document().snapshot();
    let selection = document.selection();
    for gravity in [
        RangeGravity::UIBehavior,
        RangeGravity::Backward,
        RangeGravity::Forward,
        RangeGravity::Inward,
        RangeGravity::Outward,
    ] {
        selection.set_gravity(gravity);
        selection.set_range(0, 2).unwrap();
        match selection.character_format_value().unwrap() {
            TextFormatValue::Uniform(format) => assert_eq!(character_pair(&format), (-1, 0)),
            TextFormatValue::Mixed => panic!("uniform text for {gravity:?}"),
        }
        selection.set_range(1, 3).unwrap();
        assert!(matches!(
            selection.character_format_value().unwrap(),
            TextFormatValue::Mixed
        ));
        assert_eq!(document.document().snapshot(), before);
    }
}
