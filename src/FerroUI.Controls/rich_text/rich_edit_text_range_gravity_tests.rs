use super::*;
use crate::documents::{ParagraphSnapshot, Run};
use ferroui_base::media::{
    FontStyle, FontWeight, IBrush, ISolidColorBrush, ITransform, TextAlignment,
};
use ferroui_base::threading::Dispatcher;
use ferroui_base::RelativePoint;
use std::any::Any;
use std::cell::Cell;

#[path = "rich_edit_text_range_gravity_native_cases.rs"]
mod native_cases;

type NativeFormat = (i32, i32, i32);
type NativeRange = (usize, usize, i32);
struct AffinityCase {
    name: &'static str,
    text: &'static str,
    split: usize,
    position: usize,
    requested: i32,
    gravity: i32,
    before: NativeFormat,
    after: NativeFormat,
    story: &'static str,
}
#[derive(Clone, Copy)]
enum RetentionOperation {
    SetRange(usize, usize),
    SetGravity(i32),
    GetGravity,
    Duplicate,
    SetStart(usize),
    SetEnd(usize),
    Collapse(bool),
    SetText(&'static str),
}
struct RetentionCommand {
    step: usize,
    duplicate_target: bool,
    operation: RetentionOperation,
    hresult: u32,
    primary: NativeRange,
    duplicate: Option<NativeRange>,
}
struct RetentionCase {
    name: &'static str,
    commands: &'static [RetentionCommand],
}
struct InvalidCase {
    name: &'static str,
    start: usize,
    end: usize,
    value: i32,
    hresult: u32,
    expected: NativeRange,
}

fn services(text: &str) -> RichEditTextDocument {
    RichEditTextDocument::new(RichTextDocument::from_text(text))
}

fn formatted(text: &str, split: usize) -> RichEditTextDocument {
    let model = RichTextDocument::from_text(text);
    let length = model.text_length();
    model
        .apply_format(
            0..length,
            &CharacterFormat {
                font_weight: Some(FontWeight::Normal),
                font_style: Some(FontStyle::Normal),
                ..Default::default()
            },
        )
        .unwrap();
    model
        .apply_paragraph_format(
            0..length,
            &ParagraphFormat {
                text_alignment: Some(TextAlignment::Left),
                ..Default::default()
            },
        )
        .unwrap();
    if length > 0 {
        model
            .apply_format(
                0..split,
                &CharacterFormat {
                    font_weight: Some(FontWeight::Bold),
                    ..Default::default()
                },
            )
            .unwrap();
        model
            .apply_format(
                split..length,
                &CharacterFormat {
                    font_style: Some(FontStyle::Italic),
                    ..Default::default()
                },
            )
            .unwrap();
    }
    if text.contains('\r') {
        model
            .apply_paragraph_format(
                split..length,
                &ParagraphFormat {
                    text_alignment: Some(TextAlignment::Right),
                    ..Default::default()
                },
            )
            .unwrap();
    }
    RichEditTextDocument::new(model)
}

fn native_range(range: &RichEditTextRange) -> NativeRange {
    (
        range.start_position(),
        range.end_position(),
        range.gravity() as i32,
    )
}

fn native_format(range: &RichEditTextRange) -> NativeFormat {
    let TextFormatValue::Uniform(character) = range.character_format().unwrap() else {
        panic!("Expected one character format.");
    };
    let TextFormatValue::Uniform(paragraph) = range.paragraph_format().unwrap() else {
        panic!("Expected one paragraph format.");
    };
    (
        if character.font_weight.unwrap_or(FontWeight::Normal) == FontWeight::Bold {
            -1
        } else {
            0
        },
        if character.font_style.unwrap_or(FontStyle::Normal) == FontStyle::Italic {
            -1
        } else {
            0
        },
        if paragraph.text_alignment == Some(TextAlignment::Right) {
            2
        } else {
            0
        },
    )
}

#[test]
fn rich_range_gravity_retains_native_enum_values_and_checked_conversion() {
    for (value, expected) in [
        (0, RangeGravity::UIBehavior),
        (1, RangeGravity::Backward),
        (2, RangeGravity::Forward),
        (3, RangeGravity::Inward),
        (4, RangeGravity::Outward),
    ] {
        assert_eq!(expected as i32, value);
        assert_eq!(RangeGravity::try_from(value).unwrap(), expected);
    }
    use ferroui_base::metadata::MarkupTyped;
    let metadata = RangeGravity::MARKUP;
    assert_eq!(metadata.name, "RangeGravity");
    assert_eq!(metadata.enum_members.len(), 5);
    for (member, (name, value)) in metadata.enum_members.iter().zip([
        ("UIBehavior", 0),
        ("Backward", 1),
        ("Forward", 2),
        ("Inward", 3),
        ("Outward", 4),
    ]) {
        assert_eq!(member.name, name);
        assert_eq!(member.value, i64::from(value));
        let boxed = (member.get)();
        assert_eq!(*boxed.downcast_ref::<RangeGravity>().unwrap() as i32, value);
        assert_eq!(
            (metadata.enum_to_value.unwrap())(boxed.as_ref()),
            Some(i64::from(value))
        );
    }
    assert!((metadata.enum_from_value.unwrap())(5).is_none());
    assert_eq!(RangeGravity::default(), RangeGravity::UIBehavior);
    assert!(RangeGravity::try_from(i32::MIN).is_err());
    assert!(RangeGravity::try_from(i32::MAX).is_err());
}

#[test]
fn rich_range_gravity_replays_45_native_format_and_insertion_fixtures() {
    let _scope = Dispatcher::unit_test_scope();
    assert_eq!(native_cases::AFFINITY.len(), 45);
    for case in native_cases::AFFINITY {
        let document = formatted(case.text, case.split);
        let before = document.document().snapshot();
        let range = document.get_range(case.position, case.position).unwrap();
        range.set_gravity_value(case.requested).unwrap();
        assert_eq!(range.gravity() as i32, case.gravity, "{} setter", case.name);
        assert_eq!(native_format(&range), case.before, "{} before", case.name);
        assert_eq!(
            document.document().snapshot(),
            before,
            "{} getter is observational",
            case.name
        );
        assert!(!document.can_undo());
        range.set_text("X").unwrap();
        assert_eq!(
            range.range(),
            case.position..case.position + 1,
            "{} range",
            case.name
        );
        assert_eq!(
            range.gravity() as i32,
            case.gravity,
            "{} retained",
            case.name
        );
        assert_eq!(native_format(&range), case.after, "{} inserted", case.name);
        let expected = case.story.strip_suffix('\r').unwrap().replace('\r', "\n");
        assert_eq!(
            document.text(),
            expected,
            "{} exact visible story",
            case.name
        );
        assert!(document.undo().unwrap());
        assert_eq!(
            document.document().snapshot(),
            before,
            "{} exact undo",
            case.name
        );
        assert_eq!(
            range.gravity() as i32,
            case.gravity,
            "{} mode is range metadata",
            case.name
        );
        assert!(!document.can_undo());
    }
}

#[test]
fn rich_range_gravity_replays_190_native_retention_commands() {
    let _scope = Dispatcher::unit_test_scope();
    assert_eq!(native_cases::RETENTION.len(), 10);
    let mut count = 0;
    for case in native_cases::RETENTION {
        let document = services("abcdef");
        let primary = document.get_range(0, 0).unwrap();
        let mut duplicate: Option<RichEditTextRange> = None;
        for command in case.commands {
            let target = if command.duplicate_target {
                duplicate.as_ref().unwrap()
            } else {
                &primary
            };
            // Rust Result<()> deliberately merges native S_OK/S_FALSE success.
            assert!(matches!(command.hresult, 0 | 1));
            match command.operation {
                RetentionOperation::SetRange(a, b) => target.set_range(a, b).unwrap(),
                RetentionOperation::SetGravity(value) => target.set_gravity_value(value).unwrap(),
                RetentionOperation::GetGravity => {
                    target.gravity();
                }
                RetentionOperation::Duplicate => {
                    duplicate = Some(primary.get_clone().unwrap());
                }
                RetentionOperation::SetStart(value) => target.set_start_position(value).unwrap(),
                RetentionOperation::SetEnd(value) => target.set_end_position(value).unwrap(),
                RetentionOperation::Collapse(start) => target.collapse(start).unwrap(),
                RetentionOperation::SetText(text) => target.set_text(text).unwrap(),
            }
            assert_eq!(
                native_range(&primary),
                command.primary,
                "{} step {} primary",
                case.name,
                command.step
            );
            assert_eq!(
                duplicate.as_ref().map(native_range),
                command.duplicate,
                "{} step {} duplicate",
                case.name,
                command.step
            );
            count += 1;
        }
        assert_eq!(document.text(), "abQcdef");
    }
    assert_eq!(count, 190);
}

#[test]
fn rich_range_gravity_replays_24_invalid_native_setters_without_effects() {
    let _scope = Dispatcher::unit_test_scope();
    assert_eq!(native_cases::INVALID.len(), 24);
    for case in native_cases::INVALID {
        let document = services("abcdef");
        let range = document.get_range(case.start, case.end).unwrap();
        range.set_gravity(RangeGravity::Forward);
        let snapshot = document.document().snapshot();
        let selection = document.selection().selection();
        assert_eq!(case.hresult, 0x80070057);
        assert!(
            matches!(
                range.set_gravity_value(case.value),
                Err(EditError::Document(DocumentError::InvalidFormat(_)))
            ),
            "{}",
            case.name
        );
        assert_eq!(native_range(&range), case.expected, "{}", case.name);
        assert_eq!(document.document().snapshot(), snapshot);
        assert_eq!(document.selection().selection(), selection);
        let duplicate = range.get_clone().unwrap();
        assert_eq!(duplicate.gravity(), RangeGravity::UIBehavior);
        range.set_range(2, 2).unwrap();
        assert_eq!(range.gravity(), RangeGravity::Forward);
        assert!(!document.can_undo());
    }
}

#[test]
fn rich_range_gravity_shared_clone_and_independent_duplicate_have_distinct_modes() {
    let _scope = Dispatcher::unit_test_scope();
    let document = formatted("abCD", 2);
    let range = document.get_range(2, 2).unwrap();
    range.set_gravity(RangeGravity::Forward);
    let shared = range.clone();
    let duplicate = range.get_clone().unwrap();
    assert_eq!(native_format(&shared), (0, -1, 0));
    assert_eq!(native_format(&duplicate), (-1, 0, 0));
    shared.set_gravity(RangeGravity::Backward);
    assert_eq!(range.gravity(), RangeGravity::Backward);
    duplicate.set_gravity(RangeGravity::Inward);
    assert_eq!(range.gravity(), RangeGravity::Backward);
    assert_eq!(native_format(&duplicate), (0, -1, 0));
    range.set_gravity(RangeGravity::Outward);
    assert_eq!(shared.gravity(), RangeGravity::UIBehavior);
}

#[test]
fn rich_range_gravity_mode_is_observational_on_readonly_composition_and_history() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("abc");
    document.set_text("edited").unwrap();
    document.undo().unwrap();
    document.selection().set_range(1, 1).unwrap();
    document.editor().set_preedit("preview", Some(0)).unwrap();
    let range = document.get_range(0, 0).unwrap();
    let before = document.document().snapshot();
    let selection = document.selection().selection();
    let notifications = Rc::new(Cell::new(0));
    let observed = notifications.clone();
    let _subscription = document
        .editor()
        .changed(move || observed.set(observed.get() + 1));
    document.editor().set_is_read_only(true);
    for gravity in [
        RangeGravity::Backward,
        RangeGravity::Forward,
        RangeGravity::Inward,
        RangeGravity::Outward,
    ] {
        range.set_gravity(gravity);
        range.character_format().unwrap();
    }
    assert_eq!(document.document().snapshot(), before);
    assert_eq!(document.selection().selection(), selection);
    assert!(document.editor().is_composing());
    assert!(document.can_redo());
    assert_eq!(notifications.get(), 0);
}

#[test]
fn rich_range_gravity_explicit_typing_patch_wins_without_changing_neighbors() {
    let _scope = Dispatcher::unit_test_scope();
    let document = formatted("abCD", 2);
    let range = document.get_range(2, 2).unwrap();
    range
        .set_character_format(&CharacterFormat {
            font_weight: Some(FontWeight::Bold),
            ..Default::default()
        })
        .unwrap();
    range.set_gravity(RangeGravity::Forward);
    assert_eq!(native_format(&range), (-1, -1, 0));
    range.set_text("X").unwrap();
    assert_eq!(native_format(&range), (-1, -1, 0));
    assert_eq!(
        native_format(&document.get_range(0, 2).unwrap()),
        (-1, 0, 0)
    );
    assert_eq!(
        native_format(&document.get_range(3, 5).unwrap()),
        (0, -1, 0)
    );
}

#[test]
fn rich_range_gravity_unlink_plain_insertion_uses_the_same_affinity() {
    let _scope = Dispatcher::unit_test_scope();
    let document = formatted("abCD", 2);
    let range = document.get_range(2, 2).unwrap();
    range.set_gravity(RangeGravity::Forward);
    range
        .set_text_with_options(TextSetOptions::UNLINK, "X")
        .unwrap();
    assert_eq!(native_format(&range), (0, -1, 0));
    assert_eq!(document.text(), "abXCD");
    assert_eq!(range.gravity(), RangeGravity::Forward);
}

#[test]
fn rich_range_gravity_rich_fragment_keeps_explicit_format_and_mode() {
    let _scope = Dispatcher::unit_test_scope();
    let document = formatted("abCD", 2);
    let range = document.get_range(2, 2).unwrap();
    range.set_gravity(RangeGravity::Forward);
    let mut fragment = DocumentSnapshot::from_text("rich");
    fragment.paragraphs[0].format.character.font_weight = Some(FontWeight::Bold);
    fragment.paragraphs[0].format.character.font_style = Some(FontStyle::Normal);
    range.set_snapshot(&fragment).unwrap();
    assert_eq!(native_format(&range), (-1, 0, 0));
    assert_eq!(range.gravity(), RangeGravity::Forward);
    let mut empty = DocumentSnapshot::default();
    empty.paragraphs.push(ParagraphSnapshot::default());
    empty.paragraphs[0].format.character.font_weight = Some(FontWeight::Bold);
    range.set_snapshot(&empty).unwrap();
    assert_eq!(range.gravity(), RangeGravity::Forward);
    assert_eq!(native_format(&range).0, -1);
}

#[test]
fn rich_range_gravity_terminal_and_endpoint_validation_use_visible_model_contract() {
    let _scope = Dispatcher::unit_test_scope();
    let paragraph = Paragraph::new();
    paragraph.set_font_weight(FontWeight::Light);
    let run = Run::with_text(Some("😀"));
    run.set_font_weight(FontWeight::Bold);
    paragraph.add_child(run);
    let model = RichTextDocument::new();
    model.blocks().add(paragraph);
    let document = RichEditTextDocument::new(model);
    let range = document.get_range(2, 2).unwrap();
    range.set_gravity(RangeGravity::Forward);
    let TextFormatValue::Uniform(format) = range.character_format().unwrap() else {
        panic!()
    };
    assert_eq!(format.font_weight, Some(FontWeight::Light));
    for (start, end) in [(1, 1), (3, 3), (usize::MAX, 0)] {
        assert!(range.set_range(start, end).is_err());
        assert_eq!(range.range(), 2..2);
        assert_eq!(range.gravity(), RangeGravity::Forward);
    }
    assert_eq!(range.story_length(), 2);
    assert_eq!(range.text().unwrap(), "");
}

#[test]
fn rich_range_gravity_set_text_publishes_range_before_newer_mode_or_endpoint_requests() {
    let _scope = Dispatcher::unit_test_scope();
    for move_range in [false, true] {
        let document = formatted("abCD", 2);
        let range = document.get_range(2, 2).unwrap();
        range.set_gravity(RangeGravity::Forward);
        let changed_range = range.clone();
        let called = Rc::new(Cell::new(false));
        let observed = called.clone();
        let _subscription = document.editor().changed(move || {
            if !observed.replace(true) {
                assert_eq!(changed_range.range(), 2..3);
                changed_range.set_gravity(RangeGravity::Backward);
                if move_range {
                    changed_range.set_range(0, 0).unwrap();
                }
            }
        });
        range.set_text("X").unwrap();
        assert!(called.get());
        assert_eq!(document.text(), "abXCD");
        assert_eq!(range.gravity(), RangeGravity::Backward);
        assert_eq!(range.range(), if move_range { 0..0 } else { 2..3 });
        assert!(document.undo().unwrap());
        assert_eq!(range.gravity(), RangeGravity::Backward);
    }
}

#[test]
fn rich_range_gravity_external_publication_write_survives_interrupted_set_text() {
    let _scope = Dispatcher::unit_test_scope();
    let document = formatted("abCD", 2);
    let range = document.get_range(2, 2).unwrap();
    range.set_gravity(RangeGravity::Forward);
    let model = document.document();
    let changed_range = range.clone();
    let called = Rc::new(Cell::new(false));
    let observed = called.clone();
    let _subscription = document.editor().changed(move || {
        if !observed.replace(true) {
            model
                .replace_text(0..model.text_length(), "external")
                .unwrap();
            changed_range.set_range(1, 1).unwrap();
            changed_range.set_gravity(RangeGravity::Backward);
        }
    });
    assert!(matches!(range.set_text("X"), Err(EditError::ReentrantEdit)));
    assert_eq!(document.text(), "external");
    assert_eq!(range.range(), 1..1);
    assert_eq!(range.gravity(), RangeGravity::Backward);
    assert!(!document.can_undo());
}

struct CallbackBrush {
    callback: RefCell<Option<Box<dyn FnOnce()>>>,
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
        let callback = self.callback.borrow_mut().take();
        if let Some(callback) = callback {
            callback();
        }
        None
    }
}

#[test]
fn rich_range_gravity_format_query_rejects_callback_owned_state_without_borrow_panics() {
    let _scope = Dispatcher::unit_test_scope();
    for change in 0..3 {
        let document = formatted("abCD", 2);
        let range = document.get_range(2, 2).unwrap();
        let brush = Rc::new(CallbackBrush {
            callback: RefCell::new(None),
        });
        document
            .document()
            .element_at(1, TextGravity::Forward)
            .unwrap()
            .set_foreground(Some(brush.clone()));
        let changed_range = range.clone();
        let model = document.document();
        *brush.callback.borrow_mut() = Some(Box::new(move || match change {
            0 => changed_range.set_gravity(RangeGravity::Forward),
            1 => changed_range.set_range(3, 3).unwrap(),
            _ => model
                .replace_text(0..model.text_length(), "external")
                .unwrap(),
        }));
        assert!(matches!(
            range.character_format(),
            Err(EditError::ReentrantEdit)
        ));
        match change {
            0 => assert_eq!(range.gravity(), RangeGravity::Forward),
            1 => assert_eq!(range.range(), 3..3),
            _ => assert_eq!(document.text(), "external"),
        }
        assert!(!document.can_undo());
    }
}

#[test]
fn rich_range_gravity_typing_snapshot_callback_cannot_install_stale_affinity() {
    let _scope = Dispatcher::unit_test_scope();
    let document = formatted("abCD", 2);
    let range = document.get_range(2, 2).unwrap();
    let brush = Rc::new(CallbackBrush {
        callback: RefCell::new(None),
    });
    range
        .set_character_format(&CharacterFormat {
            foreground: Some(Some(brush.clone())),
            ..Default::default()
        })
        .unwrap();
    let changed_range = range.clone();
    *brush.callback.borrow_mut() = Some(Box::new(move || {
        changed_range.set_gravity(RangeGravity::Forward)
    }));
    assert!(matches!(range.set_text("X"), Err(EditError::ReentrantEdit)));
    assert_eq!(range.gravity(), RangeGravity::Forward);
    assert_eq!(range.range(), 2..2);
    assert_eq!(document.text(), "abCD");
    assert!(!document.can_undo());
}

#[test]
fn rich_range_gravity_same_rebased_caret_restores_exact_independent_typing_patch() {
    let _scope = Dispatcher::unit_test_scope();
    let document = formatted("abCD", 2);
    document.selection().set_range(2, 2).unwrap();
    document
        .editor()
        .apply_format(&CharacterFormat {
            font_size: Some(30.0),
            ..Default::default()
        })
        .unwrap();
    let typing = document.editor().typing_format();
    document.clear_undo_redo_history();
    let before = document.document().snapshot();
    let range = document.get_range(2, 2).unwrap();
    range.set_gravity(RangeGravity::Forward);
    range.set_text("X").unwrap();
    assert_eq!(document.selection().selection(), EditorSelection::caret(3));
    assert_eq!(document.editor().typing_format(), typing);
    let TextFormatValue::Uniform(inserted) = range.character_format().unwrap() else {
        panic!()
    };
    assert_eq!(inserted.font_style, Some(FontStyle::Italic));
    assert_ne!(inserted.font_size, Some(30.0));
    assert!(document.undo().unwrap());
    assert_eq!(document.document().snapshot(), before);
    assert_eq!(document.editor().typing_format(), typing);
    assert_eq!(document.selection().selection(), EditorSelection::caret(2));
    assert!(!document.can_undo());
}

#[test]
fn rich_range_gravity_failed_group_rolls_back_text_but_keeps_range_metadata() {
    let _scope = Dispatcher::unit_test_scope();
    let document = formatted("abCD", 2);
    let range = document.get_range(2, 2).unwrap();
    let before = document.document().snapshot();
    let result: Result<(), EditError> = document.edit_group(|_| {
        range.set_gravity(RangeGravity::Forward);
        range.set_text("X")?;
        range.set_gravity(RangeGravity::Backward);
        Err(DocumentError::InvalidFormat("cancel outer group").into())
    });
    assert!(result.is_err());
    assert_eq!(document.document().snapshot(), before);
    assert_eq!(range.gravity(), RangeGravity::Backward);
    assert!(!document.can_undo());
}

#[test]
fn rich_range_gravity_nonempty_queries_keep_uniform_and_mixed_content_formats() {
    let _scope = Dispatcher::unit_test_scope();
    let document = formatted("abCD", 2);
    let uniform = document.get_range(0, 2).unwrap();
    let mixed = document.get_range(0, 4).unwrap();
    let before = document.document().snapshot();
    for gravity in [
        RangeGravity::UIBehavior,
        RangeGravity::Backward,
        RangeGravity::Forward,
        RangeGravity::Inward,
        RangeGravity::Outward,
    ] {
        uniform.set_gravity(gravity);
        mixed.set_gravity(gravity);
        assert_eq!(native_format(&uniform), (-1, 0, 0));
        assert!(matches!(
            mixed.character_format().unwrap(),
            TextFormatValue::Mixed
        ));
    }
    assert_eq!(document.document().snapshot(), before);
    assert_eq!(uniform.range(), 0..2);
    assert_eq!(mixed.range(), 0..4);
    assert!(!document.can_undo());
}

#[test]
fn rich_range_gravity_empty_document_uses_own_defaults_without_inheriting_selection_patch() {
    let _scope = Dispatcher::unit_test_scope();
    let document = RichEditTextDocument::new(RichTextDocument::new());
    document
        .editor()
        .apply_format(&CharacterFormat {
            font_weight: Some(FontWeight::Bold),
            ..Default::default()
        })
        .unwrap();
    let typing = document.editor().typing_format();
    document.clear_undo_redo_history();
    let range = document.get_range(0, 0).unwrap();
    range.set_gravity(RangeGravity::Forward);
    assert_eq!(native_format(&range), (0, 0, 0));
    range.set_text("X").unwrap();
    assert_eq!(native_format(&range), (0, 0, 0));
    assert_eq!(document.editor().typing_format(), typing);
    assert!(document.undo().unwrap());
    assert_eq!(document.text(), "");
    assert_eq!(document.editor().typing_format(), typing);
}

#[test]
fn rich_range_gravity_rich_snapshot_publishes_endpoints_and_defaults_before_callbacks() {
    let _scope = Dispatcher::unit_test_scope();
    for text in ["X", ""] {
        let document = formatted("abCD", 2);
        let before = document.document().snapshot();
        let range = document.get_range(2, 3).unwrap();
        range.set_gravity(RangeGravity::Forward);
        let mut fragment = DocumentSnapshot::from_text(text);
        fragment.paragraphs[0].format.character.font_weight = Some(FontWeight::Bold);
        let changed_range = range.clone();
        let called = Rc::new(Cell::new(false));
        let observed = called.clone();
        let _subscription = document.editor().changed(move || {
            if !observed.replace(true) {
                assert_eq!(changed_range.range(), 2..2 + text.len());
                if text.is_empty() {
                    assert_eq!(native_format(&changed_range).0, -1);
                }
                changed_range.set_range(0, 0).unwrap();
                changed_range.set_gravity(RangeGravity::Backward);
            }
        });
        range.set_snapshot(&fragment).unwrap();
        assert!(called.get());
        assert_eq!(range.range(), 0..0);
        assert_eq!(range.gravity(), RangeGravity::Backward);
        assert_eq!(
            document.text(),
            if text.is_empty() { "abD" } else { "abXD" }
        );
        assert!(document.undo().unwrap());
        assert_eq!(document.document().snapshot(), before);
        assert_eq!(range.gravity(), RangeGravity::Backward);
    }
}
