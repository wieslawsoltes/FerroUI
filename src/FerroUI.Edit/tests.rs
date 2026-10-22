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
    use ferroui_base::input::IScrollable;
    use ferroui_base::threading::Dispatcher;
    use ferroui_controls::primitives::ILogicalScrollable;
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

#[test]
fn virtual_caret_round_trips_through_formatter_and_typing_pads_only_on_commit() {
    use crate::{TextLocation, TextViewPosition};
    use ferroui_base::Point;
    let _scope = test_scope();
    let editor = laid_out_editor("a\nlong");
    let mut options = editor.options();
    options.enable_virtual_space = true;
    editor.set_options(options);
    let position = TextViewPosition::with_visual_column(TextLocation::new(1, 2), 5);
    editor.set_caret_position(position).unwrap();
    assert_eq!(editor.session().caret_virtual_space(), 4);
    assert_eq!(editor.text(), "a\nlong");
    render_editor(&editor);
    let rect = editor.text_area().cursor_rectangle();
    let hit = editor
        .text_area()
        .position_at_point(Point::new(rect.x, rect.y + rect.height / 2.0), true)
        .unwrap();
    assert_eq!(hit.location(), position.location());
    assert_eq!(hit.visual_column, position.visual_column);
    assert_eq!(editor.caret_position(), position);
    editor.replace_selection_with_text("X").unwrap();
    assert_eq!(editor.text(), "a    X\nlong");
    editor.undo().unwrap();
    assert_eq!(editor.text(), "a\nlong");
}

#[test]
fn box_selection_keyboard_keeps_short_lines_virtual_and_replaces_each_row_once() {
    use ferroui_base::input::{InputElement, Key, KeyEventArgs, KeyModifiers};
    let _scope = test_scope();
    let editor = laid_out_editor("ab\nx\nabcd");
    editor.set_caret_offset(1).unwrap();
    let area = editor.text_area();
    let box_key = |value| {
        let mut key = KeyEventArgs::new();
        key.key = value;
        key.key_modifiers = KeyModifiers::ALT | KeyModifiers::SHIFT;
        key.set_routed_event(Some(InputElement::key_down_event()));
        area.raise_event(&key);
        assert!(key.handled());
    };
    box_key(Key::Down);
    box_key(Key::Right);
    let selection = editor.selection();
    assert!(selection.is_rectangular());
    assert_eq!(selection.segments().len(), 2);
    assert_eq!(selection.primary().caret_virtual_space, 1);
    assert!(!selection.is_empty());
    editor.replace_selection_with_text("Q").unwrap();
    assert_eq!(editor.text(), "aQ\nxQ\nabcd");
    editor.undo().unwrap();
    assert_eq!(editor.text(), "ab\nx\nabcd");
}

#[test]
fn virtual_vertical_navigation_preserves_screen_column_across_short_lines() {
    use ferroui_base::input::{InputElement, Key, KeyEventArgs};
    let _scope = test_scope();
    let editor = laid_out_editor("abcd\nx\nabcd");
    let mut options = editor.options();
    options.enable_virtual_space = true;
    editor.set_options(options);
    editor.set_caret_offset(3).unwrap();
    let area = editor.text_area();
    let first_x = area.cursor_rectangle().x;
    for _ in 0..2 {
        let mut key = KeyEventArgs::new();
        key.key = Key::Down;
        key.set_routed_event(Some(InputElement::key_down_event()));
        area.raise_event(&key);
        assert!(key.handled());
        assert!((area.cursor_rectangle().x - first_x).abs() < 1.0);
    }
    assert_eq!(editor.caret_offset(), 10);
    assert_eq!(editor.session().caret_virtual_space(), 0);
}

#[test]
fn rectangle_geometry_skips_collapsed_lines_and_uses_visible_suffix() {
    use ferroui_base::Point;
    let _scope = test_scope();
    let editor = laid_out_editor("a{\n hidden\n}z\nlast");
    let section = editor.install_folding().create_folding(1, 12).unwrap();
    section.set_is_folded(true);
    render_editor(&editor);
    let area = editor.text_area();
    area.set_rectangular_selection(
        Point::new(40.0, 1.0),
        Point::new(55.0, area.line_height() + 1.0),
    )
    .unwrap();
    assert!(editor.selection().is_rectangular());
    assert_eq!(editor.selection().segments().len(), 2);
    assert!(section.is_folded());
}

#[test]
fn soft_wrap_position_preserves_trailing_row_affinity() {
    use ferroui_base::{Point, Rect, Size};
    use std::{cell::RefCell, rc::Rc};
    let _scope = test_scope();
    let editor = laid_out_editor(&"abcdefghijk".repeat(12));
    editor.set_word_wrap(true);
    let area = editor.text_area();
    area.measure(Size::new(130.0, 400.0));
    area.arrange(Rect::new(0.0, 0.0, 130.0, 400.0));
    render_editor(&editor);
    let mut position = area
        .position_at_point(Point::new(10_000.0, 2.0), false)
        .unwrap();
    assert!(position.is_at_end_of_line);
    let observed = Rc::new(RefCell::new(Vec::new()));
    let values = observed.clone();
    let weak = editor.downgrade();
    let subscription =
        editor.add_disposable_handler(TextEditor::selection_changed_event(), move |_, _| {
            if let Some(editor) = weak.upgrade() {
                values.borrow_mut().push((
                    editor.caret_position(),
                    editor.text_area().cursor_rectangle(),
                ));
            }
        });
    editor.set_caret_position(position).unwrap();
    let trailing = area.cursor_rectangle();
    assert_eq!(*observed.borrow(), vec![(position, trailing)]);
    subscription.dispose();
    position.is_at_end_of_line = false;
    editor.set_caret_position(position).unwrap();
    let leading = area.cursor_rectangle();
    assert!(leading.y > trailing.y);
    assert!(leading.x < trailing.x);
    assert_eq!(editor.caret_position(), position);
}

#[test]
fn popup_offset_geometry_omits_hidden_and_offscreen_positions_without_moving_caret() {
    use ferroui_base::Vector;
    use ferroui_base::input::IScrollable;
    let _scope = test_scope();
    let editor = laid_out_editor(&format!("a{{\n hidden\n}}z\n{}", "last\n".repeat(100)));
    let area = editor.text_area();
    let section = editor.install_folding().create_folding(1, 12).unwrap();
    section.set_is_folded(true);
    render_editor(&editor);
    editor.set_caret_offset(0).unwrap();
    let selection = editor.selection();
    let first = area.rectangle_for_offset(0).unwrap();
    let suffix = area.rectangle_for_offset(12).unwrap();
    assert_eq!(first.y, suffix.y);
    assert!(suffix.x > first.x);
    assert!(area.rectangle_for_offset(5).is_none());
    assert!(
        area.rectangle_for_offset(editor.document().text_length() + 1)
            .is_none()
    );
    assert!(
        area.rectangle_for_offset(editor.document().text_length())
            .is_none()
    );
    assert_eq!(editor.selection(), selection);
    assert!(section.is_folded());
    area.set_offset(Vector::new(0.0, area.line_height() * 20.0));
    render_editor(&editor);
    assert!(area.rectangle_for_offset(0).is_none());
    assert_eq!(editor.selection(), selection);
}

#[test]
fn page_navigation_preserves_virtual_columns_and_shift_anchor() {
    use crate::{TextLocation, TextViewPosition};
    use ferroui_base::input::{IScrollable, InputElement, Key, KeyEventArgs, KeyModifiers};
    let _scope = test_scope();
    let editor = laid_out_editor(&"x\n".repeat(100));
    let mut options = editor.options();
    options.enable_virtual_space = true;
    editor.set_options(options);
    editor
        .set_caret_position(TextViewPosition::with_visual_column(
            TextLocation::new(1, 2),
            5,
        ))
        .unwrap();
    let area = editor.text_area();
    let start_x = area.cursor_rectangle().x + area.offset().x;
    let mut down = KeyEventArgs::new();
    down.key = Key::PageDown;
    down.key_modifiers = KeyModifiers::SHIFT;
    down.set_routed_event(Some(InputElement::key_down_event()));
    area.raise_event(&down);
    assert!(down.handled());
    assert!(editor.caret_offset() > 1);
    assert_eq!(editor.selection().primary().anchor_tuple(), (1, 4));
    assert_eq!(editor.session().caret_virtual_space(), 4);
    assert!((area.cursor_rectangle().x + area.offset().x - start_x).abs() < 1.0);
    let mut up = KeyEventArgs::new();
    up.key = Key::PageUp;
    up.set_routed_event(Some(InputElement::key_down_event()));
    area.raise_event(&up);
    assert!(up.handled());
    assert!(editor.selection().is_empty());
    assert_eq!(editor.session().caret_virtual_space(), 4);
    assert!((area.cursor_rectangle().x + area.offset().x - start_x).abs() < 1.0);
}

#[test]
fn alt_click_starts_box_selection_without_retaining_the_previous_caret() {
    use ferroui_base::Point;
    use ferroui_base::input::{
        IPointer, KeyModifiers, MouseButton, Pointer, PointerPointProperties,
        PointerPressedEventArgs, PointerReleasedEventArgs, PointerType, PointerUpdateKind,
        RawInputModifiers,
    };
    use ferroui_controls::{Control, Window};
    let _scope = test_scope();
    let editor = laid_out_editor("abcd");
    let window = Window::new();
    window.set_content(Some(Control::boxed(editor.clone())));
    window.show();
    let area = editor.text_area();
    let target = area.rectangle_for_offset(2).unwrap();
    let point = area
        .translate_point(
            Point::new(target.x, target.y + target.height / 2.0),
            &window,
        )
        .unwrap();
    editor.set_caret_offset(0).unwrap();
    let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
    let press = PointerPressedEventArgs::new(
        area.clone(),
        pointer.clone(),
        &window,
        point,
        1,
        PointerPointProperties::new(
            RawInputModifiers::LEFT_MOUSE_BUTTON,
            PointerUpdateKind::LeftButtonPressed,
        ),
        KeyModifiers::ALT,
        1,
    );
    area.raise_event(&press);
    assert!(press.handled());
    area.raise_event(&PointerReleasedEventArgs::new(
        area.clone(),
        pointer.clone(),
        &window,
        point,
        2,
        PointerPointProperties::new(
            RawInputModifiers::NONE,
            PointerUpdateKind::LeftButtonReleased,
        ),
        KeyModifiers::ALT,
        MouseButton::Left,
    ));
    assert!(pointer.captured().is_none());
    assert_eq!(editor.selection().segments().len(), 1);
    assert_eq!(editor.caret_offset(), 2);
    editor.replace_selection_with_text("Q").unwrap();
    assert_eq!(editor.text(), "abQcd");
    window.close();
}

#[test]
fn virtual_cells_follow_the_physical_right_edge_of_a_trailing_rtl_run() {
    use crate::{TextLocation, TextViewPosition};
    use ferroui_base::Point;
    let _scope = test_scope();
    let editor = laid_out_editor("abc אבג");
    let area = editor.text_area();
    let length = editor.document().text_length();
    let rightmost = (0..=length)
        .filter_map(|offset| area.rectangle_for_offset(offset))
        .map(|rect| rect.x)
        .fold(0.0, f64::max);
    let position =
        TextViewPosition::with_visual_column(TextLocation::new(1, length + 1), length + 3);
    editor.set_caret_position(position).unwrap();
    let rect = area.cursor_rectangle();
    assert!(rect.x > rightmost);
    let hit = area
        .position_at_point(Point::new(rect.x, rect.y + rect.height / 2.0), true)
        .unwrap();
    assert_eq!(hit, position);
    assert_eq!(editor.session().caret_virtual_space(), 3);
    editor.replace_selection_with_text("Q").unwrap();
    assert_eq!(editor.text(), "abc אבג   Q");
}

#[test]
fn box_keyboard_extension_retains_original_and_opposite_rectangle_anchors() {
    use ferroui_base::input::{InputElement, Key, KeyEventArgs, KeyModifiers};
    let _scope = test_scope();
    let editor = laid_out_editor("abcd\nabcd\nabcd");
    editor.set_selection(Selection::simple(1, 7)).unwrap();
    let area = editor.text_area();
    let box_key = |value| {
        let mut key = KeyEventArgs::new();
        key.key = value;
        key.key_modifiers = KeyModifiers::ALT | KeyModifiers::SHIFT;
        key.set_routed_event(Some(InputElement::key_down_event()));
        area.raise_event(&key);
        assert!(key.handled());
    };
    box_key(Key::Down);
    assert_eq!(editor.selection().segments().len(), 3);
    assert_eq!(editor.selection().segments()[0].anchor, 1);
    // A programmatically supplied rectangle must replace any cached box state.
    editor
        .set_selection(Selection::from_rectangle_segments(
            [
                SelectionSegment::new(1, 2),
                SelectionSegment::new(6, 7),
                SelectionSegment::new(11, 12),
            ],
            0,
        ))
        .unwrap();
    box_key(Key::Right);
    let selection = editor.selection();
    assert_eq!(selection.segments().len(), 3);
    assert_eq!(selection.primary().caret, 3);
    assert_eq!(selection.segments()[2].anchor, 11);
}

#[test]
fn shift_home_in_wrapped_text_retains_a_virtual_anchor() {
    use crate::{TextLocation, TextViewPosition};
    use ferroui_base::input::{InputElement, Key, KeyEventArgs, KeyModifiers};
    let _scope = test_scope();
    let editor = laid_out_editor("abc");
    editor.set_word_wrap(true);
    editor
        .set_caret_position(TextViewPosition::with_visual_column(
            TextLocation::new(1, 4),
            6,
        ))
        .unwrap();
    let mut key = KeyEventArgs::new();
    key.key = Key::Home;
    key.key_modifiers = KeyModifiers::SHIFT;
    key.set_routed_event(Some(InputElement::key_down_event()));
    editor.text_area().raise_event(&key);
    assert!(key.handled());
    assert_eq!(editor.selection().primary().anchor_tuple(), (3, 3));
    assert_eq!(editor.selection().primary().caret_tuple(), (0, 0));
}
