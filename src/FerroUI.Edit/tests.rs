use crate::{TextDocument, TextEditor, register_types};
use ferroui_edit_core::editing::{Selection, SelectionSegment};

pub(crate) struct TestScope {
    _app: ferroui_controls::testing::UnitTestApplicationScope,
    _text: ferroui_base::media::text_formatting::testing::TextTestScope,
}
pub(crate) fn test_scope() -> TestScope {
    use ferroui_base::media::text_formatting::testing::TextTestScope;
    use ferroui_base::platform::IPlatformRenderInterface;
    use ferroui_base::{FerroLocator, LocatorExtensions};
    use ferroui_controls::testing::{TestServices, UnitTestApplication};
    let text = TextTestScope::new();
    let renderer = FerroLocator::current()
        .get_service::<dyn IPlatformRenderInterface>()
        .unwrap();
    let app =
        UnitTestApplication::start(TestServices::styled_window().with_render_interface(renderer));
    TestScope {
        _app: app,
        _text: text,
    }
}

#[test]
fn editors_have_distinct_default_documents_and_shared_documents_update_both_sessions() {
    let _scope = test_scope();
    let a = TextEditor::new();
    let b = TextEditor::new();
    a.set_text("one").unwrap();
    assert_eq!(b.text(), "");
    let document = TextDocument::from_text("a😀b");
    a.set_document(document.clone());
    b.set_document(document.clone());
    a.select(1, 2).unwrap();
    a.replace_selection_with_text("z").unwrap();
    assert_eq!(b.text(), "azb");
    a.undo().unwrap();
    assert_eq!(b.text(), "a😀b");
}

#[test]
fn multi_caret_ui_commands_share_the_document_undo_group() {
    let _scope = test_scope();
    let editor = TextEditor::new();
    editor.set_text("ab\ncd").unwrap();
    editor
        .set_selection(Selection::from_segments(
            [SelectionSegment::empty(1), SelectionSegment::empty(4)],
            0,
        ))
        .unwrap();
    editor.replace_selection_with_text("X").unwrap();
    assert_eq!(editor.text(), "aXb\ncXd");
    editor.undo().unwrap();
    assert_eq!(editor.text(), "ab\ncd");
}

#[test]
fn editor_types_and_styled_properties_are_registered_without_instances() {
    use ferroui_base::{FerroPropertyRegistry, TypeInfo};
    register_types();
    assert!(TypeInfo::find("FerroUI.Edit", "TextEditor").is_some());
    assert!(
        FerroPropertyRegistry::instance()
            .find_registered(TextEditor::TYPE, "ShowLineNumbers")
            .is_some()
    );
}

fn laid_out_editor(text: &str) -> ferroui_base::Ref<TextEditor> {
    use ferroui_base::{Rect, Size};
    let editor = TextEditor::new();
    editor.set_font_family(ferroui_base::media::FontFamily::new("Test Sans"));
    editor.set_text(text).unwrap();
    let area = editor.text_area();
    area.measure(Size::new(360.0, 180.0));
    area.arrange(Rect::new(0.0, 0.0, 360.0, 180.0));
    editor
}

fn render_editor(editor: &TextEditor) -> Vec<String> {
    use ferroui_base::media::{DrawingContext, PlatformDrawingContext};
    use ferroui_base::rendering::testing::{DrawingLog, MockDrawingContextImpl};
    let log = DrawingLog::new();
    let mut platform = MockDrawingContextImpl::new(log.clone());
    platform.log_transforms = false;
    let mut core = PlatformDrawingContext::borrowed(&mut platform);
    let mut context = DrawingContext::new(&mut core);
    editor.text_area().render(&mut context);
    context.dispose();
    log.entries()
}

#[test]
fn direct_text_property_reports_document_edits_and_document_replacement() {
    use std::{cell::RefCell, rc::Rc};
    let _scope = test_scope();
    let editor = TextEditor::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let recorded = changes.clone();
    editor.add_property_changed_handler(Rc::new(move |change| {
        if change.property() == TextEditor::text_property().as_property() {
            recorded.borrow_mut().push((
                change.get_old_value::<String>().unwrap(),
                change.get_new_value::<String>(),
            ));
        }
    }));
    editor.document().insert(0, "😀").unwrap();
    editor.set_document(TextDocument::from_text("next"));
    assert_eq!(
        *changes.borrow(),
        vec![(String::new(), "😀".into()), ("😀".into(), "next".into())]
    );
    assert_eq!(
        editor.get_value(TextEditor::document_property()).unwrap(),
        editor.document()
    );
}

#[test]
fn routed_keys_and_text_input_share_unicode_navigation_read_only_and_undo() {
    use ferroui_base::input::{InputElement, Key, KeyEventArgs, KeyModifiers, TextInputEventArgs};
    let _scope = test_scope();
    let editor = TextEditor::new();
    editor.set_text("a😀b").unwrap();
    editor.set_caret_offset(3).unwrap();
    let area = editor.text_area();
    let mut key = KeyEventArgs::new();
    key.key = Key::Left;
    key.set_routed_event(Some(InputElement::key_down_event()));
    area.raise_event(&key);
    assert!(key.handled());
    assert_eq!(editor.caret_offset(), 1);
    let mut text = TextInputEventArgs::new();
    text.text = Some("é".into());
    text.set_routed_event(Some(InputElement::text_input_event()));
    area.raise_event(&text);
    assert_eq!(editor.text(), "aé😀b");
    let mut undo = KeyEventArgs::new();
    undo.key = Key::Z;
    undo.key_modifiers = KeyModifiers::CONTROL;
    undo.set_routed_event(Some(InputElement::key_down_event()));
    area.raise_event(&undo);
    assert_eq!(editor.text(), "a😀b");
    editor.set_is_read_only(true);
    let mut blocked = TextInputEventArgs::new();
    blocked.text = Some("x".into());
    blocked.set_routed_event(Some(InputElement::text_input_event()));
    area.raise_event(&blocked);
    assert_eq!(editor.text(), "a😀b");
}

#[test]
fn viewport_shapes_visible_lines_and_hit_tests_utf16_tabs() {
    use ferroui_base::input::IScrollable;
    use ferroui_base::{Point, Vector};
    let _scope = test_scope();
    let editor = laid_out_editor(&format!("a\t😀b\n{}", "line\n".repeat(100_000)));
    assert!(!render_editor(&editor).is_empty());
    let area = editor.text_area();
    assert!(area.cached_visual_line_count() < 30);
    editor.set_caret_offset(4).unwrap();
    let caret = area.cursor_rectangle();
    assert_eq!(
        area.offset_at_point(Point::new(caret.x + 0.1, caret.y + caret.height / 2.0)),
        4
    );
    area.set_offset(Vector::new(0.0, 1_000_000.0));
    render_editor(&editor);
    assert!(area.cached_visual_line_count() < 30);
    let hit = area.offset_at_point(Point::new(6.0, 1.0));
    assert!(
        editor
            .document()
            .get_line_by_offset(hit)
            .unwrap()
            .line_number()
            > 10_000
    );
}

#[test]
fn folding_projects_inline_suffix_and_unfolds_when_caret_enters_hidden_text() {
    use ferroui_base::Point;
    use ferroui_base::input::IScrollable;
    let _scope = test_scope();
    let editor = laid_out_editor("a{\n hidden\n}z\nlast");
    let manager = editor.install_folding();
    let section = manager.create_folding(1, 12).unwrap();
    section.set_title(Some("...".into()));
    section.set_is_folded(true);
    render_editor(&editor);
    editor.set_caret_offset(13).unwrap();
    let rect = editor.text_area().cursor_rectangle();
    assert!(rect.y < editor.text_area().line_height());
    assert_eq!(
        editor
            .text_area()
            .offset_at_point(Point::new(rect.x + 0.1, rect.y + rect.height / 2.0)),
        13
    );
    editor.set_caret_offset(5).unwrap();
    assert!(!section.is_folded());
    assert!(editor.text_area().cursor_rectangle().y >= editor.text_area().line_height());
    assert!(editor.text_area().extent().height >= editor.text_area().viewport().height);
}

#[test]
fn runtime_xaml_builds_editor_properties_and_direct_text() {
    use ferroui_base::metadata::from_markup_value;
    let _scope = test_scope();
    register_types();
    ferroui_markup_xaml::register_types();
    let value=ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load(
        r#"<edit:TextEditor xmlns="https://github.com/ferroui" xmlns:edit="clr-namespace:FerroUI.Edit;assembly=FerroUI.Edit" ShowLineNumbers="True" WordWrap="True" FontSize="17" Text="hello 😀"/>"#,
        Some(&crate::ASSEMBLY),None,None,false).unwrap();
    let editor = from_markup_value::<ferroui_base::Ref<TextEditor>>(&Some(value)).unwrap();
    assert!(editor.show_line_numbers());
    assert!(editor.word_wrap());
    assert_eq!(editor.font_size(), 17.0);
    assert_eq!(editor.text(), "hello 😀");
}

#[test]
fn measured_extent_notifies_after_render_and_coalesces_pending_frames() {
    use ferroui_controls::primitives::ILogicalScrollable;
    use ferroui_base::input::IScrollable;
    use ferroui_base::threading::Dispatcher;
    use std::{cell::Cell, rc::Rc};
    let _scope = test_scope();
    let editor = laid_out_editor(&"long text ".repeat(200));
    Dispatcher::ui_thread().run_jobs(None);
    let notifications = Rc::new(Cell::new(0));
    let count = notifications.clone();
    let area = editor.text_area();
    let subscription = area.scroll_invalidated(Rc::new(move || count.set(count.get() + 1)));
    let previous_width = area.extent().width;
    render_editor(&editor);
    assert!(area.extent().width > previous_width);
    assert_eq!(notifications.get(), 0);
    render_editor(&editor);
    assert_eq!(notifications.get(), 0);
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(notifications.get(), 1);
    subscription.dispose();
}

#[test]
fn ime_preedit_projects_selection_without_mutating_document_and_tracks_utf16_cursor() {
    let _scope = test_scope();
    let editor = laid_out_editor("a😀suffix");
    editor.select(1, 2).unwrap();
    let document = editor.document();
    document.clear_undo_history().unwrap();
    let version = document.version();
    let area = editor.text_area();
    area.set_preedit(Some("漢😀字"), Some(1));
    render_editor(&editor);
    let first = area.cursor_rectangle();
    area.set_preedit(Some("漢😀字"), Some(3));
    render_editor(&editor);
    let second = area.cursor_rectangle();
    assert!(second.x > first.x);
    assert_eq!(document.text(), "a😀suffix");
    assert_eq!(document.version(), version);
    assert!(!document.can_undo());
    area.set_preedit(None, None);
    editor.replace_selection_with_text("漢😀字").unwrap();
    assert_eq!(document.text(), "a漢😀字suffix");
    document.undo().unwrap();
    assert_eq!(document.text(), "a😀suffix");
}

#[test]
fn wrapped_vertical_navigation_uses_visual_rows_and_preserves_horizontal_position() {
    use ferroui_base::input::{InputElement, Key, KeyEventArgs};
    use ferroui_base::{Rect, Size};
    let _scope = test_scope();
    let editor = laid_out_editor(&"abcdefghij ".repeat(20));
    editor.set_word_wrap(true);
    let area = editor.text_area();
    area.measure(Size::new(130.0, 400.0));
    area.arrange(Rect::new(0.0, 0.0, 130.0, 400.0));
    editor.set_caret_offset(3).unwrap();
    render_editor(&editor);
    let start = area.cursor_rectangle();
    let mut down = KeyEventArgs::new();
    down.key = Key::Down;
    down.set_routed_event(Some(InputElement::key_down_event()));
    area.raise_event(&down);
    let after_down = area.cursor_rectangle();
    assert!(down.handled());
    assert!(editor.caret_offset() > 3);
    assert!(after_down.y > start.y);
    assert!((after_down.x - start.x).abs() < 1.0);
    let mut up = KeyEventArgs::new();
    up.key = Key::Up;
    up.set_routed_event(Some(InputElement::key_down_event()));
    area.raise_event(&up);
    assert_eq!(editor.caret_offset(), 3);
}
