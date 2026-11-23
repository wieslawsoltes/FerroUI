use crate::{FoldingMargin, LineNumberMargin, TextDocument, TextEditor, register_types};
use ferroui_edit_core::editing::{Selection, SelectionSegment};

pub(crate) struct TestScope {
    _app: ferroui_controls::testing::UnitTestApplicationScope,
    _text: ferroui_base::media::text_formatting::testing::TextTestScope,
}
pub(crate) fn test_scope() -> TestScope {
    test_scope_with_services(ferroui_controls::testing::TestServices::styled_window())
}

pub(crate) fn test_scope_with_services(
    services: ferroui_controls::testing::TestServices,
) -> TestScope {
    use ferroui_base::media::text_formatting::testing::TextTestScope;
    use ferroui_base::platform::IPlatformRenderInterface;
    use ferroui_base::{FerroLocator, LocatorExtensions};
    use ferroui_controls::testing::UnitTestApplication;
    let text = TextTestScope::new();
    let renderer = FerroLocator::current()
        .get_service::<dyn IPlatformRenderInterface>()
        .unwrap();
    let app = UnitTestApplication::start(services.with_render_interface(renderer));
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
fn large_highlight_fonts_expand_rows_and_vertical_keys_use_actual_row_boundaries() {
    use ferroui_base::input::{InputElement, Key, KeyEventArgs};
    use ferroui_edit_core::highlighting::HighlightingDefinition;
    use std::rc::Rc;
    let _scope = test_scope();
    let editor = laid_out_editor("TALL\nsmall\nlast");
    let definition = HighlightingDefinition::load(r#"<SyntaxDefinition><RuleSet><Rule fontSize="56">TALL</Rule></RuleSet></SyntaxDefinition>"#).unwrap();
    editor.set_syntax_highlighting(Some(Rc::new(definition)));
    editor.set_caret_offset(0).unwrap();
    render_editor(&editor);
    assert!(editor.text_area().cursor_rectangle().height > editor.text_area().line_height() * 2.0);
    for (key, offset) in [(Key::Down, 5), (Key::Down, 11), (Key::Up, 5), (Key::Up, 0)] {
        let mut event = KeyEventArgs::new();
        event.key = key;
        event.set_routed_event(Some(InputElement::key_down_event()));
        editor.text_area().raise_event(&event);
        assert!(event.handled());
        assert_eq!(editor.caret_offset(), offset);
    }
}

#[test]
fn rectangular_paste_uses_shaped_width_and_preserves_one_undo_group() {
    use ferroui_edit_core::highlighting::HighlightingDefinition;
    use std::rc::Rc;
    let _scope = test_scope();
    let editor = laid_out_editor("iiii\nWWWW\nzzzz");
    let definition = HighlightingDefinition::load(r#"<SyntaxDefinition><RuleSet><Rule fontSize="56">WWWW</Rule></RuleSet></SyntaxDefinition>"#).unwrap();
    editor.set_syntax_highlighting(Some(Rc::new(definition)));
    editor.set_caret_offset(1).unwrap();
    editor.document().clear_undo_history().unwrap();
    assert!(editor.text_area().paste_rectangle("A\r\nB", false).unwrap());
    editor.selection_changed();
    assert_eq!(editor.text(), "iAiii\nBWWWW\nzzzz");
    assert!(editor.undo().unwrap());
    assert_eq!(editor.text(), "iiii\nWWWW\nzzzz");
    assert!(!editor.undo().unwrap());
}

#[test]
fn highlighting_resource_changes_invalidate_resolved_cached_layouts() {
    use ferroui_base::media::Colors;
    use ferroui_edit_core::highlighting::HighlightingDefinition;
    use std::rc::Rc;
    let _scope = test_scope();
    let editor = laid_out_editor("word");
    editor.resources().add_value("Accent", Colors::RED);
    let definition = HighlightingDefinition::load(r#"<SyntaxDefinition><RuleSet><Rule foreground="resource:Accent">word</Rule></RuleSet></SyntaxDefinition>"#).unwrap();
    editor.set_syntax_highlighting(Some(Rc::new(definition)));
    render_editor(&editor);
    assert_eq!(editor.text_area().cached_visual_line_count(), 1);
    editor
        .resources()
        .set("Accent", Some(Rc::new(Colors::BLUE)));
    assert_eq!(editor.text_area().cached_visual_line_count(), 0);
    render_editor(&editor);
    assert_eq!(editor.text_area().cached_visual_line_count(), 1);
    assert!(editor.last_error().is_none());
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
    for property in ["ShowLineNumbers", "FontStyle", "FontWeight", "FontStretch"] {
        assert!(FerroPropertyRegistry::instance().find_registered(TextEditor::TYPE, property).is_some());
    }
}

pub(crate) fn laid_out_editor(text: &str) -> ferroui_base::Ref<TextEditor> {
    use ferroui_base::{Rect, Size};
    let editor = TextEditor::new();
    editor.set_font_family(ferroui_base::media::FontFamily::new("Test Sans"));
    editor.set_text(text).unwrap();
    let area = editor.text_area();
    area.measure(Size::new(360.0, 180.0));
    area.arrange(Rect::new(0.0, 0.0, 360.0, 180.0));
    editor
}

pub(crate) fn render_editor(editor: &TextEditor) -> Vec<String> {
    use ferroui_base::media::{DrawingContext, PlatformDrawingContext};
    use ferroui_base::rendering::testing::{DrawingLog, MockDrawingContextImpl};
    let log = DrawingLog::new();
    let mut platform = MockDrawingContextImpl::new(log.clone());
    platform.log_transforms = false;
    let mut core = PlatformDrawingContext::borrowed(&mut platform);
    let mut context = DrawingContext::new(&mut core);
    let area = editor.text_area();
    area.render(&mut context);
    for child in area.visual_children().snapshot().iter() {
        if child.cast::<LineNumberMargin>().is_some() || child.cast::<FoldingMargin>().is_some() {
            child.render(&mut context);
        }
    }
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
fn editor_text_notifications_publish_complete_document_updates() {
    use std::{cell::RefCell, rc::Rc};

    let _scope = test_scope();
    let editor = TextEditor::new();
    let document = editor.document();
    let routed = Rc::new(RefCell::new(Vec::new()));
    let recorded = routed.clone();
    let weak = editor.downgrade();
    let _subscription = editor.add_disposable_handler(TextEditor::text_changed_event(), move |_, _| {
        let editor = weak.upgrade().unwrap();
        let text = editor.text();
        recorded.borrow_mut().push(text.clone());
        if text == "ab" {
            editor.document().insert(2, "!").unwrap();
        }
    });
    let bound = Rc::new(RefCell::new(Vec::new()));
    let pairs = bound.clone();
    editor.add_property_changed_handler(Rc::new(move |change| {
        if change.property() == TextEditor::text_property().as_property() {
            pairs.borrow_mut().push((
                change.get_old_value::<String>().unwrap(),
                change.get_new_value::<String>(),
            ));
        }
    }));

    let update = document.begin_update().unwrap();
    document.insert(0, "a").unwrap();
    document.insert(1, "b").unwrap();
    assert!(routed.borrow().is_empty());
    assert!(bound.borrow().is_empty());
    update.finish().unwrap();
    assert_eq!(*routed.borrow(), ["ab", "ab!"]);
    assert_eq!(*bound.borrow(), [(String::new(), "ab".into()), ("ab".into(), "ab!".into())]);
    assert!(document.undo().unwrap());
    assert_eq!(editor.text(), "");
    assert_eq!(routed.borrow().last().unwrap(), "");
}

#[test]
fn document_replacement_during_snippet_teardown_keeps_the_latest_document() {
    use ferroui_edit_core::snippets::Snippet;
    use std::{cell::RefCell, rc::Rc};

    let _scope = test_scope();
    let editor = TextEditor::new();
    let initial = editor.document();
    let snippet = editor.insert_snippet(&Snippet::parse("${1:value}$0").unwrap()).unwrap();
    let discarded = TextDocument::from_text("discarded");
    let latest = TextDocument::from_text("latest");
    let replacement = latest.clone();
    let weak = editor.downgrade();
    let _deactivation = snippet.session().deactivated(move |_, _| {
        weak.upgrade().unwrap().set_document(replacement.clone());
    });

    let routed = Rc::new(RefCell::new(Vec::new()));
    let recorded = routed.clone();
    let weak = editor.downgrade();
    let _subscription = editor.add_disposable_handler(TextEditor::text_changed_event(), move |_, _| {
        recorded.borrow_mut().push(weak.upgrade().unwrap().text());
    });
    editor.set_document(discarded.clone());
    assert_eq!(editor.document(), latest);
    assert_eq!(editor.get_value(TextEditor::document_property()).unwrap(), latest);
    assert!(snippet.session().is_closed());
    assert_eq!(*routed.borrow(), ["latest"]);

    initial.insert(0, "!").unwrap();
    discarded.insert(0, "!").unwrap();
    latest.insert(latest.text_length(), "!").unwrap();
    assert_eq!(*routed.borrow(), ["latest", "latest!"]);

    let weak = editor.downgrade();
    drop(editor);
    assert!(weak.upgrade().is_none());
    latest.insert(latest.text_length(), "!").unwrap();
    assert_eq!(*routed.borrow(), ["latest", "latest!"]);
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
    use crate::{EditorCommandKind, EditorNavigation};
    use ferroui_base::Point;
    use ferroui_base::input::{ICommand, IScrollable};
    use ferroui_edit_core::editing::CaretMovement;
    let _scope = test_scope();
    let editor = laid_out_editor("a{\n hidden\n}z\nlast");
    let manager = editor.install_folding();
    let section = manager.create_folding(1, 12).unwrap();
    section.set_title(Some("...".into()));
    section.set_is_folded(true);
    render_editor(&editor);
    editor.set_caret_offset(1).unwrap();
    editor.command(EditorCommandKind::Navigate(EditorNavigation::Caret {
        movement: CaretMovement::CharacterRight,
        extend_selection: false,
    })).execute(None);
    assert_eq!(editor.caret_offset(), 12);
    assert!(section.is_folded());
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
    use ferroui_base::media::{FontStretch, FontStyle, FontWeight};
    use ferroui_base::metadata::from_markup_value;
    let _scope = test_scope();
    register_types();
    ferroui_markup_xaml::register_types();
    let value=ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load(
        r#"<TextEditor xmlns="https://github.com/ferroui"
            ShowLineNumbers="True" LineNumbersMinWidthInDigits="4" LineNumbersMargin="3,0,6,0" WordWrap="True"
            FontSize="17" FontStyle="Italic" FontWeight="Bold" FontStretch="Condensed" Watermark="Enter code" Text="hello 😀"/>"#,
        Some(&crate::ASSEMBLY),None,None,false).unwrap();
    let editor = from_markup_value::<ferroui_base::Ref<TextEditor>>(&Some(value)).unwrap();
    assert!(editor.show_line_numbers());
    assert_eq!(editor.line_numbers_min_width_in_digits(), 4);
    assert_eq!(editor.line_numbers_margin(), ferroui_base::Thickness::new(3.0, 0.0, 6.0, 0.0));
    assert!(editor.word_wrap());
    assert_eq!(editor.font_size(), 17.0);
    assert_eq!(editor.font_style(), FontStyle::Italic);
    assert_eq!(editor.font_weight(), FontWeight::Bold);
    assert_eq!(editor.font_stretch(), FontStretch::Condensed);
    assert_eq!(editor.watermark().as_deref(), Some("Enter code"));
    assert_eq!(editor.text(), "hello 😀");
}

#[test]
fn runtime_xaml_selects_named_syntax_highlighting_for_the_current_document() {
    use ferroui_base::Ref;
    use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;

    let _scope = test_scope();
    register_types();
    ferroui_markup_xaml::register_types();
    let markup = r#"<TextEditor xmlns="https://github.com/ferroui" SyntaxHighlighting="C#" Text="int value"/>"#;
    let editor = FerroRuntimeXamlLoader::parse_as::<Ref<TextEditor>>(markup, None).unwrap();
    assert_eq!(editor.syntax_highlighting().unwrap().name, "C#");
    assert!(!editor
        .document_highlighter()
        .unwrap()
        .highlight_line(1)
        .unwrap()
        .sections
        .is_empty());

    let next = TextDocument::from_text("return 1;");
    editor.set_document(next.clone());
    let highlighter = editor.document_highlighter().unwrap();
    assert_eq!(highlighter.document(), &next);
    assert!(!highlighter.highlight_line(1).unwrap().sections.is_empty());

    for name in ["No Such Definition", "c#", ""] {
        let missing =
            FerroRuntimeXamlLoader::parse_as::<Ref<TextEditor>>(&markup.replace("C#", name), None)
                .unwrap();
        assert!(missing.syntax_highlighting().is_none(), "{name}");
        assert!(missing.document_highlighter().is_none(), "{name}");
    }
}

#[test]
fn runtime_xaml_syntax_highlighting_setters_use_host_aliases_and_native_precedence() {
    use ferroui_base::data::core::{ValueType, ValueTypes};
    use ferroui_base::{BoxedValue, FerroLocator, Ref};
    use ferroui_controls::{UserControl, Window};
    use ferroui_edit_core::highlighting::{
        HighlightingDefinition, HighlightingDefinitionResolver, HighlightingManager,
    };
    use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
    use std::rc::Rc;

    let _scope = test_scope();
    register_types();
    ferroui_markup_xaml::register_types();
    let mut manager = HighlightingManager::empty();
    let expected = Rc::new(HighlightingDefinition::load(r#"<SyntaxDefinition name="Custom"><RuleSet><Rule foreground="Red">word</Rule></RuleSet></SyntaxDefinition>"#).unwrap());
    manager
        .register_definition("Host Alias", expected.clone())
        .unwrap();
    FerroLocator::current_mutable()
        .bind::<dyn HighlightingDefinitionResolver>()
        .to_constant(Rc::new(manager));
    let markup = r#"<UserControl xmlns="https://github.com/ferroui"
        xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">
        <UserControl.Styles>
            <Style Selector="TextEditor"><Setter Property="SyntaxHighlighting" Value="Host Alias"/></Style>
        </UserControl.Styles>
        <StackPanel>
            <TextEditor x:Name="Editor" Text="word"/>
            <TextEditor x:Name="Cleared" SyntaxHighlighting="{x:Null}"/>
        </StackPanel>
    </UserControl>"#;
    let view = FerroRuntimeXamlLoader::parse_as::<Ref<UserControl>>(markup, None).unwrap();
    let editor = view.find_control::<TextEditor>("Editor").unwrap();
    let cleared = view.find_control::<TextEditor>("Cleared").unwrap();
    let window = Window::new();
    window.set_content(Some(Rc::new(view)));
    window.show();

    assert!(Rc::ptr_eq(
        &editor.syntax_highlighting().unwrap(),
        &expected
    ));
    let value: BoxedValue = Rc::new(editor.get_value(TextEditor::syntax_highlighting_property()));
    let canonical_name = ValueTypes::try_convert(Some(&value), ValueType::of::<String>())
        .unwrap()
        .unwrap();
    assert_eq!(canonical_name.downcast_ref::<String>().unwrap(), "Custom");
    assert!(cleared.document_highlighter().is_none());
    let line = editor
        .document_highlighter()
        .unwrap()
        .highlight_line(1)
        .unwrap();
    assert_eq!(line.sections[0].color.foreground.as_deref(), Some("Red"));
    editor.set_syntax_highlighting(None);
    assert!(editor.document_highlighter().is_none());
    editor.clear_value(TextEditor::syntax_highlighting_property());
    assert!(Rc::ptr_eq(
        &editor.syntax_highlighting().unwrap(),
        &expected
    ));
    assert!(editor.document_highlighter().is_some());
    window.close();
}

#[test]
fn runtime_xaml_applies_native_selection_styles_to_text_area() {
    use ferroui_base::metadata::from_markup_value;
    let _scope = test_scope();
    register_types();
    ferroui_markup_xaml::register_types();
    let value = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load(
        r#"<TextArea xmlns="https://github.com/ferroui"
                        SelectionBrush="Red" SelectionForeground="White" SelectionCornerRadius="5">
            <TextArea.SelectionBorder><Pen Brush="Blue" Thickness="2" /></TextArea.SelectionBorder>
        </TextArea>"#,
        Some(&crate::ASSEMBLY), None, None, false,
    ).unwrap();
    let area = from_markup_value::<ferroui_base::Ref<crate::TextArea>>(&Some(value)).unwrap();
    assert!(area.selection_brush().is_some());
    assert!(area.selection_foreground().is_some());
    assert_eq!(area.selection_border().unwrap().thickness(), 2.0);
    assert_eq!(area.selection_corner_radius(), 5.0);
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
fn ime_surrounding_text_and_selection_share_logical_line_coordinates() {
    use ferroui_base::input::InputElement;
    use ferroui_base::input::text_input::{TextInputMethodClientRequestedEventArgs, TextSelection};
    use std::{cell::RefCell, rc::Rc};

    let _scope = test_scope();
    let editor = laid_out_editor("outside\r\na😀bc\r\nnext\ntrailing");
    let area = editor.text_area();
    let request = TextInputMethodClientRequestedEventArgs::new();
    request.set_routed_event(Some(InputElement::text_input_method_client_requested_event()));
    area.raise_event(&request);
    let client = request.client().unwrap();

    let snapshots = Rc::new(RefCell::new(Vec::new()));
    let _subscription = client.surrounding_text_changed(Rc::new({
        let client = Rc::downgrade(&client);
        let snapshots = snapshots.clone();
        move || {
            let client = client.upgrade().unwrap();
            snapshots.borrow_mut().push((client.surrounding_text(), client.selection()));
        }
    }));

    editor.set_caret_offset(12).unwrap();
    assert_eq!(
        snapshots.borrow().last(),
        Some(&("a😀bc".into(), TextSelection::new(3, 3)))
    );
    client.set_selection(TextSelection::new(3, 1));
    assert_eq!(editor.selection().primary(), SelectionSegment::new(12, 10));

    let multiple = Selection::from_segments(
        [SelectionSegment::new(12, 10), SelectionSegment::empty(18)],
        0,
    );
    editor.set_selection(multiple.clone()).unwrap();
    client.set_selection(client.selection());
    assert_eq!(editor.selection(), multiple);

    editor.set_selection(Selection::simple(18, 12)).unwrap();
    assert_eq!(client.surrounding_text(), "a😀bc\r\nnext");
    assert_eq!(client.selection(), TextSelection::new(9, 3));
    client.set_selection(TextSelection::new(7, 99));
    assert_eq!(editor.selection().primary(), SelectionSegment::new(16, 20));
    assert_eq!(client.surrounding_text(), "next");
    assert_eq!(client.selection(), TextSelection::new(0, 4));
    assert_eq!(editor.text(), "outside\r\na😀bc\r\nnext\ntrailing");

    let revision = client.input_method_state_revision();
    let notifications = snapshots.borrow().len();
    editor.session().set_selection(Selection::simple(12, 12)).unwrap();
    assert_eq!(snapshots.borrow().len(), notifications);
    assert_ne!(client.input_method_state_revision(), revision);
}

#[test]
fn native_input_connection_composition_preserves_selection_and_undo() {
    use crate::CompletionWindow;
    use ferroui_base::Rect;
    use ferroui_base::input::{InputElement, Key, KeyEventArgs};
    use ferroui_base::input::text_input::{
        ITextInputMethodImpl, TextInputMethodClient, TextInputMethodError, TextInputOptions,
    };
    use ferroui_base::interactivity::RoutingStrategies;
    use ferroui_controls::testing::{MockWindowingPlatform, TestServices};
    use ferroui_controls::{Button, Control, StackPanel, Window};
    use ferroui_edit_core::completion::CompletionItem;
    use std::{cell::RefCell, rc::Rc};

    #[derive(Default)]
    struct InputMethod(RefCell<Option<Rc<dyn TextInputMethodClient>>>);

    impl ITextInputMethodImpl for InputMethod {
        fn set_client(&self, client: Option<Rc<dyn TextInputMethodClient>>) {
            self.0.replace(client);
        }
        fn set_cursor_rect(&self, _: Rect) {}
        fn set_options(&self, _: &TextInputOptions) {}
        fn reset(&self) {}
    }

    let input = Rc::new(InputMethod::default());
    let method = input.clone();
    let _scope = test_scope_with_services(TestServices::focusable_window().with_windowing_platform(
        MockWindowingPlatform::with_window_impl(move || {
            let window = MockWindowingPlatform::create_window_mock();
            window.setup_feature::<dyn ITextInputMethodImpl>(method.clone());
            window
        }),
    ));
    let editor = laid_out_editor("a😀bc");
    let window = Window::new();
    let controls = StackPanel::new();
    controls.children().add(editor.clone());
    controls.children().add(Button::new());
    window.set_content(Some(Control::boxed(controls)));
    window.show();
    assert!(editor.text_area().focus());
    let client = input.0.borrow().clone().unwrap();
    let connection = client.input_method_connection().expect("Editor composition connection");
    assert!(!connection.supports_exact_edits());
    let document = editor.document();
    document.clear_undo_history().unwrap();
    let selection = Selection::simple(3, 1);
    editor.set_selection(selection.clone()).unwrap();

    let cancelled = connection.begin_composition().unwrap();
    cancelled.update("候補", Some(1)).unwrap();
    assert_eq!(editor.text(), "a😀bc");
    assert_eq!(editor.selection(), selection);
    assert!(cancelled.cancel().unwrap());
    assert_eq!(editor.text(), "a😀bc");
    assert_eq!(editor.selection(), selection);
    assert!(!document.can_undo());

    let accepted = connection.begin_composition().unwrap();
    accepted.update("候補", Some(2)).unwrap();
    assert!(accepted.commit("確定", 0).unwrap());
    assert_eq!(editor.text(), "a確定bc");
    assert_eq!(document.undo_count(), 1);
    assert!(editor.undo().unwrap());
    assert_eq!(editor.text(), "a😀bc");
    assert_eq!(editor.selection(), selection);

    let superseded = connection.begin_composition().unwrap();
    superseded.update("候補", None).unwrap();
    editor.set_caret_offset(4).unwrap();
    assert_eq!(superseded.commit("遅延", 0), Err(TextInputMethodError::StateChanged));
    assert_eq!(editor.text(), "a😀bc");
    assert!(!document.can_undo());
    assert!(!editor.text_area().has_preedit());

    editor.set_text("pre").unwrap();
    editor.set_caret_offset(3).unwrap();
    document.clear_undo_history().unwrap();
    let completion = CompletionWindow::show_for(
        &editor,
        0,
        3,
        vec![CompletionItem::new("prefix"), CompletionItem::new("preview")],
    )
    .unwrap();
    let list = completion.completion_list();
    let selected = list.selected_index();
    let route_key = |key| {
        let mut event = KeyEventArgs::new();
        event.key = key;
        event.set_routed_event(Some(InputElement::key_down_event()));
        editor.text_area().raise_event(&event);
        event.handled()
    };
    let composition = connection.begin_composition().unwrap();
    composition.update("f", Some(1)).unwrap();
    for key in [Key::Down, Key::Tab, Key::Return, Key::Escape] {
        route_key(key);
        assert!(editor.text_area().is_focused(), "{key:?} moved focus during preedit");
        assert_eq!(list.selected_index(), selected);
        assert!(!completion.is_closed());
        assert_eq!(editor.text(), "pre");
        assert!(!document.can_undo());
    }
    let _intercept_popup_text = completion.add_disposable_handler_with(
        InputElement::text_input_event(),
        |_, event| event.set_handled(true),
        RoutingStrategies::BUBBLE,
        false,
    );
    assert!(composition.commit("f", 0).unwrap());
    assert_eq!(editor.text(), "pref");
    assert_eq!(list.selected_item().unwrap().text(), "prefix");
    assert!(!completion.is_closed());
    assert!(route_key(Key::Tab));
    assert_eq!(editor.text(), "prefix");
    assert!(completion.is_closed());
    assert!(editor.undo().unwrap());
    assert_eq!(editor.text(), "pref");
    assert!(editor.undo().unwrap());
    assert_eq!(editor.text(), "pre");
    window.close();
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
    let ordinary = area.get_or_construct_visual_line(1).unwrap();
    area.set_preedit(None, None);
    assert!(std::rc::Rc::ptr_eq(&ordinary, &area.get_visual_line(1).unwrap()));
    area.set_preedit(Some("漢😀字"), Some(1));
    render_editor(&editor);
    let composition = area.get_visual_line(1).unwrap();
    let first = area.cursor_rectangle();
    area.set_preedit(Some("漢😀字"), Some(3));
    render_editor(&editor);
    let second = area.cursor_rectangle();
    assert!(second.x > first.x);
    assert!(std::rc::Rc::ptr_eq(&composition, &area.get_visual_line(1).unwrap()));
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
        key.key_modifiers = crate::text_area::box_selection_modifier() | KeyModifiers::SHIFT;
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
    let next_row = area.rectangle_for_offset(14).unwrap();
    area.set_rectangular_selection(Point::new(40.0, 1.0), Point::new(55.0, next_row.y + 1.0))
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
fn command_and_keyboard_page_navigation_preserve_virtual_columns_and_shift_anchor() {
    use crate::{EditorCommandKind, EditorNavigation, TextLocation, TextViewPosition};
    use ferroui_base::input::{ICommand, IScrollable, InputElement, Key, KeyEventArgs};
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
    editor.command(EditorCommandKind::Navigate(EditorNavigation::Page {
        upward: false,
        extend_selection: true,
    })).execute(None);
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
fn platform_box_modifier_takes_priority_over_pointer_word_selection() {
    use ferroui_base::Point;
    use ferroui_base::input::{
        IPointer, MouseButton, Pointer, PointerPointProperties,
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
        crate::text_area::box_selection_modifier(),
        2,
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
        crate::text_area::box_selection_modifier(),
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
fn pointer_drag_preserves_words_and_lines_through_release_and_scroll() {
    use ferroui_base::input::{
        IPointer, IScrollable, InputElement, KeyModifiers, MouseButton, Pointer, PointerEventArgs,
        PointerPointProperties, PointerPressedEventArgs, PointerReleasedEventArgs, PointerType,
        PointerUpdateKind, RawInputModifiers,
    };
    use ferroui_base::threading::Dispatcher;
    use ferroui_base::{Point, Rect, Size, Vector};
    use ferroui_controls::{Control, Window};
    let _scope = test_scope();
    let text = "alpha beta gamma\n".repeat(30);
    let editor = laid_out_editor(&text);
    let area = editor.text_area();
    let window = Window::new();
    window.set_content(Some(Control::boxed(editor.clone())));
    window.show();
    area.measure(Size::new(320.0, 90.0));
    area.arrange(Rect::new(0.0, 0.0, 320.0, 90.0));
    area.ensure_visual_lines().unwrap();
    let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
    let point_at = |offset| {
        let rect = area.rectangle_for_offset(offset).unwrap();
        area.translate_point(Point::new(rect.x + 0.25, rect.y + rect.height / 2.0), &window)
            .unwrap()
    };
    let press = |point, times| {
        let event = PointerPressedEventArgs::new(
            area.clone(), pointer.clone(), &window, point, 1,
            PointerPointProperties::new(RawInputModifiers::LEFT_MOUSE_BUTTON, PointerUpdateKind::LeftButtonPressed),
            KeyModifiers::NONE, times,
        );
        area.raise_event(&event);
        assert!(event.handled());
    };
    let move_to = |point| {
        let event = PointerEventArgs::new(
            Some(InputElement::pointer_moved_event()), area.clone(), pointer.clone(),
            Some(&window), point, 2,
            PointerPointProperties::new(RawInputModifiers::LEFT_MOUSE_BUTTON, PointerUpdateKind::Other),
            KeyModifiers::NONE,
        );
        area.raise_event(&event);
        assert!(event.handled());
    };
    let release = |point| {
        let event = PointerReleasedEventArgs::new(
            area.clone(), pointer.clone(), &window, point, 3,
            PointerPointProperties::new(RawInputModifiers::NONE, PointerUpdateKind::LeftButtonReleased),
            KeyModifiers::NONE, MouseButton::Left,
        );
        area.raise_event(&event);
        assert!(event.handled());
        assert!(pointer.captured().is_none());
    };
    press(point_at(7), 2);
    assert_eq!(editor.selected_text().unwrap(), "beta");
    move_to(point_at(13));
    assert_eq!(editor.selected_text().unwrap(), "beta gamma");
    release(point_at(2));
    assert_eq!(editor.selected_text().unwrap(), "alpha beta");
    assert_eq!(editor.caret_offset(), 0);

    let second_row = point_at(18);
    press(point_at(1), 3);
    assert_eq!(editor.selected_text().unwrap(), "alpha beta gamma");
    move_to(second_row);
    assert_eq!(editor.selected_text().unwrap(), text[..33]);
    area.set_offset(Vector::new(0.0, area.line_height() * 2.0));
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(editor.selected_text().unwrap(), text[..67]);
    release(second_row);
    let selection = editor.selection();
    area.set_offset(Vector::default());
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(editor.selection(), selection);
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
fn box_command_and_keyboard_extension_retain_original_and_opposite_rectangle_anchors() {
    use crate::{EditorCommandKind, EditorNavigation};
    use ferroui_base::input::{ICommand, InputElement, Key, KeyEventArgs, KeyModifiers};
    use ferroui_edit_core::editing::CaretMovement;
    let _scope = test_scope();
    let editor = laid_out_editor("abcd\nabcd\nabcd");
    editor.set_selection(Selection::simple(1, 7)).unwrap();
    let area = editor.text_area();
    let box_key = |value| {
        let mut key = KeyEventArgs::new();
        key.key = value;
        key.key_modifiers = crate::text_area::box_selection_modifier() | KeyModifiers::SHIFT;
        key.set_routed_event(Some(InputElement::key_down_event()));
        area.raise_event(&key);
        assert!(key.handled());
    };
    editor.command(EditorCommandKind::Navigate(EditorNavigation::Rectangle(CaretMovement::LineDown)))
        .execute(None);
    let selection = editor.selection();
    let rows: Vec<_> = selection.segments().iter()
        .map(|row| (row.start_offset(), row.end_offset()))
        .collect();
    assert_eq!(rows, [(1, 2), (6, 7), (11, 12)]);
    assert_eq!(selection.segments()[0].anchor, 1);
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

#[test]
fn home_toggles_indentation_and_preserves_the_selection_anchor() {
    use ferroui_base::input::{InputElement, Key, KeyEventArgs, KeyModifiers};
    let _scope = test_scope();
    let editor = laid_out_editor("  \t  // text\n    \r\nאבג");
    let area = editor.text_area();
    let press = |modifiers| {
        let mut key = KeyEventArgs::new();
        key.key = Key::Home;
        key.key_modifiers = modifiers;
        key.set_routed_event(Some(InputElement::key_down_event()));
        area.raise_event(&key);
        assert!(key.handled());
    };
    editor.set_caret_offset(10).unwrap();
    press(KeyModifiers::SHIFT);
    assert_eq!(editor.selection().primary(), SelectionSegment::new(10, 5));
    press(KeyModifiers::SHIFT);
    assert_eq!(editor.selection().primary(), SelectionSegment::new(10, 0));
    press(KeyModifiers::NONE);
    assert_eq!(editor.caret_offset(), 5);
    assert!(editor.selection().is_empty());
    editor.set_caret_offset(15).unwrap();
    press(KeyModifiers::NONE);
    assert_eq!(editor.caret_offset(), 17);
    press(KeyModifiers::NONE);
    assert_eq!(editor.caret_offset(), 13);
}

#[test]
fn home_and_end_use_logical_edges_of_the_current_wrapped_row() {
    use ferroui_base::input::{InputElement, Key, KeyEventArgs};
    use ferroui_base::{Rect, Size};
    let _scope = test_scope();
    let editor = laid_out_editor(&format!("  {}", "abc def ghi ".repeat(9)));
    let area = editor.text_area();
    editor.set_word_wrap(true);
    area.measure(Size::new(95.0, 260.0));
    area.arrange(Rect::new(0.0, 0.0, 95.0, 260.0));
    let second_start = (1..editor.document().text_length())
        .find(|&offset| area.rectangle_for_offset(offset).is_some_and(|rect| rect.y > 0.0))
        .unwrap();
    let press = |value| {
        let mut key = KeyEventArgs::new();
        key.key = value;
        key.set_routed_event(Some(InputElement::key_down_event()));
        area.raise_event(&key);
        assert!(key.handled());
    };
    editor.set_caret_offset(second_start + 2).unwrap();
    press(Key::Home);
    assert_eq!(editor.caret_offset(), second_start);
    assert!(!editor.caret_position().is_at_end_of_line);
    press(Key::Home);
    assert_eq!(editor.caret_offset(), 0);
    press(Key::End);
    assert_eq!(editor.caret_offset(), second_start);
    assert!(editor.caret_position().is_at_end_of_line);
    assert_eq!(area.cursor_rectangle().y, 0.0);
    press(Key::Home);
    assert_eq!(editor.caret_offset(), 2);
}

#[test]
fn caret_observers_follow_wrap_affinity_and_virtual_cells() {
    use crate::TextViewPosition;
    use ferroui_base::{Rect, Size};
    use std::{cell::RefCell, rc::Rc};

    let _scope = test_scope();
    let editor = laid_out_editor(&"abc def ghi ".repeat(9));
    let area = editor.text_area();
    editor.set_word_wrap(true);
    area.measure(Size::new(95.0, 260.0));
    area.arrange(Rect::new(0.0, 0.0, 95.0, 260.0));
    let document = editor.document();
    let wrap = (1..document.text_length())
        .find(|&offset| area.rectangle_for_offset(offset).is_some_and(|rect| rect.y > 0.0))
        .unwrap();
    editor.set_caret_offset(wrap).unwrap();
    let leading = editor.caret_position();
    let trailing = TextViewPosition {
        is_at_end_of_line: true,
        ..leading
    };
    let changes = Rc::new(RefCell::new(Vec::new()));
    let subscription = editor.caret_position_changed({
        let editor = editor.downgrade();
        let changes = changes.clone();
        move || {
            let editor = editor.upgrade().unwrap();
            changes.borrow_mut().push((editor.caret_offset(), editor.caret_position()));
        }
    });

    editor.set_caret_position(trailing).unwrap();
    editor.set_caret_position(trailing).unwrap();
    editor.set_caret_position(leading).unwrap();
    editor.set_selection(Selection::simple(0, wrap)).unwrap();
    editor.set_is_read_only(true);
    assert_eq!(*changes.borrow(), [(wrap, trailing), (wrap, leading)]);

    let mut options = editor.options();
    options.enable_virtual_space = true;
    editor.set_options(options.clone());
    let end = document.text_length();
    editor.set_caret_offset(end).unwrap();
    let physical = editor.caret_position();
    let virtual_cell = TextViewPosition {
        visual_column: physical.visual_column.map(|column| column + 2),
        ..physical
    };
    editor.set_caret_position(virtual_cell).unwrap();
    assert_eq!(changes.borrow()[2..], [(end, physical), (end, virtual_cell)]);
    options.enable_virtual_space = false;
    editor.set_options(options);
    assert_eq!(changes.borrow()[4..], [(end, physical)]);

    drop(subscription);
    editor.set_caret_offset(0).unwrap();
    assert_eq!(changes.borrow().len(), 5);
}

#[test]
fn caret_observers_see_committed_document_updates_and_undo() {
    use crate::TextLocation;
    use std::{cell::RefCell, rc::Rc};

    let _scope = test_scope();
    let editor = TextEditor::new();
    editor.set_text("a😀bc").unwrap();
    editor.set_caret_offset(3).unwrap();
    let document = editor.document();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let _subscription = editor.caret_position_changed({
        let editor = editor.downgrade();
        let changes = changes.clone();
        move || {
            let editor = editor.upgrade().unwrap();
            assert!(!editor.document().is_in_update());
            changes.borrow_mut().push((editor.caret_offset(), editor.caret_position().location()));
        }
    });

    let outer = document.begin_update().unwrap();
    editor.set_caret_offset(4).unwrap();
    let inner = document.begin_update().unwrap();
    editor.set_caret_offset(3).unwrap();
    drop(inner);
    document.replace(0, 1, "\n").unwrap();
    document.insert(0, "x").unwrap();
    assert!(changes.borrow().is_empty());
    drop(outer);
    assert_eq!(*changes.borrow(), [(4, TextLocation::new(2, 3))]);

    document.replace(1, 1, "a").unwrap();
    assert_eq!(changes.borrow()[1..], [(4, TextLocation::new(1, 5))]);
    editor.undo().unwrap();
    assert_eq!(changes.borrow()[2..], [(4, TextLocation::new(2, 3))]);

    let old_update = document.begin_update().unwrap();
    editor.set_caret_offset(2).unwrap();
    editor.set_document(TextDocument::from_text("new"));
    assert_eq!(changes.borrow()[3..], [(0, TextLocation::new(1, 1))]);
    drop(old_update);
    assert_eq!(changes.borrow().len(), 4);
}

#[test]
fn box_caret_moves_through_wrapped_rows_while_edits_use_first_row_segments() {
    use ferroui_base::input::{InputElement, Key, KeyEventArgs, KeyModifiers};
    use ferroui_base::{Rect, Size};
    use std::cell::Cell;
    use std::rc::Rc;
    let _scope = test_scope();
    let text = format!("{}\nnext", "abcdefghij".repeat(7));
    let editor = laid_out_editor(&text);
    editor.set_word_wrap(true);
    let area = editor.text_area();
    area.measure(Size::new(92.0, 280.0));
    area.arrange(Rect::new(0.0, 0.0, 92.0, 280.0));
    editor.set_caret_offset(1).unwrap();
    let notified = Rc::new(Cell::new(0));
    let calls = notified.clone();
    editor.add_handler(TextEditor::selection_changed_event(), move |_, _| {
        calls.set(calls.get() + 1);
    });
    let press = |value, modifiers| {
        let mut key = KeyEventArgs::new();
        key.key = value;
        key.key_modifiers = modifiers;
        key.set_routed_event(Some(InputElement::key_down_event()));
        area.raise_event(&key);
        assert!(key.handled());
    };
    let box_modifiers = crate::text_area::box_selection_modifier() | KeyModifiers::SHIFT;
    press(Key::Down, box_modifiers);
    let first_target = editor.caret_offset();
    let projected = editor.selection();
    assert!(projected.is_rectangular());
    assert_eq!(projected.segments(), &[SelectionSegment::empty(1)]);
    assert!(first_target > 1 && first_target < 70);
    assert!(area.cursor_rectangle().y > 0.0);
    press(Key::Down, KeyModifiers::SHIFT);
    assert!(editor.caret_offset() > first_target && editor.caret_offset() < 70);
    assert_eq!(editor.selection(), projected);
    assert_eq!(notified.get(), 2);
    press(Key::Up, box_modifiers);
    assert_eq!(editor.caret_offset(), first_target);
    press(Key::Right, KeyModifiers::NONE);
    assert_eq!(editor.caret_offset(), first_target + 1);
    assert!(!editor.selection().is_rectangular());
    editor.set_caret_offset(1).unwrap();
    press(Key::Down, box_modifiers);
    editor.replace_selection_with_text("!").unwrap();
    assert_eq!(editor.text(), format!("a!{}", &text[1..]));
    assert_eq!(editor.caret_offset(), 2);
}

#[test]
fn page_and_document_navigation_extend_existing_rectangles() {
    use crate::{EditorCommandKind, EditorNavigation};
    use ferroui_base::input::{InputElement, Key, KeyEventArgs, KeyModifiers};
    use ferroui_edit_core::editing::CaretMovement;
    let _scope = test_scope();
    let text = "xxxx\n".repeat(70) + "xxxx";
    for use_keyboard in [true, false] {
        let editor = laid_out_editor(&text);
        let area = editor.text_area();
        let press = |value, modifiers| {
            let mut key = KeyEventArgs::new();
            key.key = value;
            key.key_modifiers = modifiers;
            key.set_routed_event(Some(InputElement::key_down_event()));
            area.raise_event(&key);
            assert!(key.handled());
        };
        let extend = |key, modifiers, navigation| {
            if use_keyboard {
                press(key, modifiers | KeyModifiers::SHIFT);
            } else {
                let command = editor.command(EditorCommandKind::Navigate(navigation)).as_command();
                assert!(command.can_execute(None));
                command.execute(None);
            }
        };
        editor.set_caret_offset(11).unwrap();
        press(Key::Right, crate::text_area::box_selection_modifier() | KeyModifiers::SHIFT);
        extend(Key::PageDown, KeyModifiers::NONE, EditorNavigation::Page {
            upward: false,
            extend_selection: true,
        });
        let selection = editor.selection();
        assert!(selection.is_rectangular());
        assert!(selection.segments().len() > 2);
        for (row, segment) in selection.segments().iter().enumerate() {
            let offset = 10 + 5 * row;
            assert_eq!(*segment, SelectionSegment::new(offset + 1, offset + 2));
        }
        extend(Key::PageUp, KeyModifiers::NONE, EditorNavigation::Page {
            upward: true,
            extend_selection: true,
        });
        assert_eq!(editor.selection().segments(), &[SelectionSegment::new(11, 12)]);
        extend(Key::End, KeyModifiers::CONTROL, EditorNavigation::Caret {
            movement: CaretMovement::DocumentEnd,
            extend_selection: true,
        });
        let selection = editor.selection();
        assert!(selection.is_rectangular());
        assert_eq!(selection.segments().len(), 69);
        assert_eq!(selection.primary(), SelectionSegment::new(351, 354));
        extend(Key::Home, KeyModifiers::CONTROL, EditorNavigation::Caret {
            movement: CaretMovement::DocumentStart,
            extend_selection: true,
        });
        let selection = editor.selection();
        assert!(selection.is_rectangular());
        assert_eq!(selection.primary_index(), 0);
        assert_eq!(
            selection.segments(),
            &[SelectionSegment::new(1, 0), SelectionSegment::new(6, 5), SelectionSegment::new(11, 10)]
        );
    }
}

#[test]
fn ime_composition_keeps_virtual_padding_visual_until_commit_or_cancel() {
    use crate::{TextLocation, TextViewPosition};
    use ferroui_base::input::{InputElement, TextInputEventArgs};
    let _scope = test_scope();
    for text in ["abc", "abc אבג"] {
        let editor = laid_out_editor(text);
        let length = editor.document().text_length();
        let position =
            TextViewPosition::with_visual_column(TextLocation::new(1, length + 1), length + 3);
        editor.set_caret_position(position).unwrap();
        editor.document().clear_undo_history().unwrap();
        let area = editor.text_area();
        let initial = area.cursor_rectangle();
        let version = editor.document().version();
        area.set_preedit(Some("漢😀"), Some(0));
        render_editor(&editor);
        assert!((area.cursor_rectangle().x - initial.x).abs() < 1.0);
        assert_eq!(editor.text(), text);
        assert_eq!(editor.document().version(), version);
        assert!(!editor.document().can_undo());
        area.set_preedit(None, None);
        render_editor(&editor);
        assert_eq!(editor.caret_position(), position);
        assert_eq!(area.cursor_rectangle(), initial);
        area.set_preedit(Some("漢😀"), Some(3));
        render_editor(&editor);
        assert!(area.cursor_rectangle().x > initial.x);
        let mut input = TextInputEventArgs::new();
        input.text = Some("漢😀".into());
        input.set_routed_event(Some(InputElement::text_input_event()));
        area.raise_event(&input);
        assert_eq!(editor.text(), format!("{text}   漢😀"));
        editor.undo().unwrap();
        assert_eq!(editor.text(), text);
    }
}
