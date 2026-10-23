//! The real compiled RichEditBox theme, including live template bindings,
//! persistent presenter ownership, logical scrolling and command-flyout routing.

use super::support::start_themed_application;
use ferroui_base::input::IScrollable;
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::media::{Brushes, FontWeight, IBrush};
use ferroui_base::metadata::from_markup_value;
use ferroui_base::styling::{ControlTheme, ThemeVariant};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{ObjectType, Ref, Visual};
use ferroui_controls::documents::CharacterFormat;
use ferroui_controls::primitives::FlyoutBaseImplExt;
use ferroui_controls::testing::MockWindowingPlatform;
use ferroui_controls::{
    Border, Control, MenuFlyout, MenuItem, RichEditBox, RichEditBoxTextPresenter, ScrollViewer,
    Window,
};
use ferroui_markup_xaml::{RuntimeXamlLoaderConfiguration, RuntimeXamlLoaderDocument};
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use std::rc::Rc;

fn named<T: ObjectType>(visual: &Visual, name: &str) -> Ref<T> {
    visual
        .get_visual_descendants()
        .filter(|item| item.name().as_deref() == Some(name))
        .find_map(|item| item.cast::<T>())
        .unwrap_or_else(|| panic!("the compiled template supplies {name}"))
}

fn editor_window(editor: &Ref<RichEditBox>) -> Ref<Window> {
    editor.set_width(280.0);
    editor.set_height(160.0);
    let window = Window::with_impl(MockWindowingPlatform::create_window_mock_with_size(
        360.0, 240.0,
    ));
    window.set_content(Some(Control::boxed(editor)));
    window.show();
    window.update_layout();
    window
}

fn menu_item(flyout: &MenuFlyout, index: usize) -> Ref<MenuItem> {
    flyout
        .items()
        .get_at(index)
        .as_ref()
        .and_then(Control::from_boxed)
        .and_then(|item| item.cast::<MenuItem>())
        .expect("a themed rich editing command")
}

fn variants() -> Vec<ThemeVariant> {
    vec![
        ThemeVariant::light(),
        ThemeVariant::dark(),
        crate::WinUITheme::high_contrast(),
    ]
}

#[test]
fn rich_edit_compiled_theme_populates_live_parts_with_one_shared_presenter() {
    let _app = start_themed_application();
    let _text = TextTestScope::new();
    let editor = RichEditBox::new();
    editor.set_header(Some(Rc::new("Heading".to_owned())));
    editor.set_description(Some(Rc::new("Explanation".to_owned())));
    editor.set_placeholder_text("Write here");
    let window = editor_window(&editor);
    assert!(editor.template().is_some());
    let presenter = editor.text_presenter();
    let host: Ref<ScrollViewer> = named(&editor, "ContentElement");
    assert_eq!(
        host.content().as_ref().and_then(Control::from_boxed),
        Some(presenter.clone().upcast())
    );
    assert_eq!(
        editor
            .get_visual_descendants()
            .filter(|v| v.cast::<RichEditBoxTextPresenter>().is_some())
            .count(),
        1
    );
    assert_eq!(presenter.document(), editor.document());
    assert_eq!(
        from_markup_value::<String>(&editor.header_presenter().unwrap().content()),
        Some("Heading".to_owned())
    );
    assert_eq!(
        from_markup_value::<String>(&editor.description_presenter().unwrap().content()),
        Some("Explanation".to_owned())
    );
    assert_eq!(
        editor.placeholder_presenter().unwrap().text().as_deref(),
        Some("Write here")
    );
    assert!(editor.placeholder_presenter().unwrap().is_visible());
    editor.set_preedit_text(Some("日本"), Some(2)).unwrap();
    window.update_layout();
    assert!(!editor.placeholder_presenter().unwrap().is_visible());
    editor.cancel_composition().unwrap();
    assert!(editor.placeholder_presenter().unwrap().is_visible());
    editor.set_text("content").unwrap();
    editor.set_header(Some(Rc::new("Updated".to_owned())));
    editor.set_description(None);
    window.update_layout();
    assert!(!editor.placeholder_presenter().unwrap().is_visible());
    assert_eq!(
        from_markup_value::<String>(&editor.header_presenter().unwrap().content()),
        Some("Updated".to_owned())
    );
    assert!(!editor.description_presenter().unwrap().is_visible());
    editor.set_header(None);
    assert!(!editor.header_presenter().unwrap().is_visible());
    window.close();
}

#[test]
fn rich_edit_theme_scrolls_the_complete_story_and_reveals_the_end_caret() {
    let _app = start_themed_application();
    let _text = TextTestScope::new();
    let editor = RichEditBox::new();
    editor.set_header(Some(Rc::new("Document".to_owned())));
    editor
        .set_text(
            &(0..50)
                .map(|index| format!("Line {index}"))
                .collect::<Vec<_>>()
                .join("\n"),
        )
        .unwrap();
    let window = editor_window(&editor);
    let presenter = editor.text_presenter();
    assert_eq!(presenter.line_metrics().len(), 50);
    assert!(presenter.extent().height > presenter.viewport().height);
    let end = editor.document().text_length();
    editor.select(end, end);
    window.update_layout();
    // Selection can invalidate text layout; the control then reveals the caret
    // at Loaded priority after that layout has completed.
    Dispatcher::ui_thread().run_jobs(None);
    assert!(
        presenter.offset().y > 0.0,
        "offset {:?}, viewport {:?}, extent {:?}",
        presenter.offset(), presenter.viewport(), presenter.extent()
    );
    let caret = presenter.cursor_rectangle();
    assert!(caret.y >= -1e-6, "caret {caret:?}");
    assert!(
        caret.bottom() <= presenter.viewport().height + 1e-6,
        "caret {caret:?}, viewport {:?}",
        presenter.viewport()
    );
    assert_eq!(presenter.document(), editor.document());
    window.close();
}

#[test]
fn rich_edit_theme_changes_preserve_history_composition_and_presenter_identity() {
    let _app = start_themed_application();
    let _text = TextTestScope::new();
    let editor = RichEditBox::new();
    editor.set_text("original").unwrap();
    let window = editor_window(&editor);
    editor.select(8, 8);
    editor.insert_text("!").unwrap();
    let model = editor.document();
    let service = editor.editor();
    let presenter = editor.text_presenter();
    let committed = model.snapshot();
    editor.set_preedit_text(Some("日本"), Some(2)).unwrap();
    let revision = model.revision();
    for variant in variants() {
        window.set_requested_theme_variant(Some(variant));
        window.update_layout();
        assert_eq!(
            model.revision(),
            revision,
            "theme presentation is not a document edit"
        );
        assert_eq!(editor.document(), model);
        assert!(Rc::ptr_eq(&editor.editor(), &service));
        assert_eq!(editor.text_presenter(), presenter);
        assert!(service.is_composing());
        assert!(editor.can_undo());
        assert_eq!(editor.text(), "original!日本");
    }
    editor.cancel_composition().unwrap();
    assert_eq!(model.snapshot(), committed);
    assert!(editor.undo().unwrap());
    assert_eq!(editor.text(), "original");
    window.close();
}

#[test]
fn rich_edit_retemplate_detaches_previous_host_without_replacing_document_or_history() {
    let _app = start_themed_application();
    let _text = TextTestScope::new();
    let editor = RichEditBox::new();
    editor.set_text("editable").unwrap();
    let window = editor_window(&editor);
    let old_host: Ref<ScrollViewer> = named(&editor, "ContentElement");
    let presenter = editor.text_presenter();
    let document = editor.document();
    editor.select(0, 4);
    editor
        .apply_format(&CharacterFormat {
            font_weight: Some(FontWeight::Bold),
            ..CharacterFormat::default()
        })
        .unwrap();
    let snapshot = document.snapshot();
    let value = FerroRuntimeXamlLoader::load_document(RuntimeXamlLoaderDocument::new(
        r#"<ControlTheme xmlns="https://github.com/ferroui" TargetType="RichEditBox">
             <Setter Property="Template"><ControlTemplate><Border Name="ContentElement" Padding="7" /></ControlTemplate></Setter>
           </ControlTheme>"#), Some(RuntimeXamlLoaderConfiguration::new())).unwrap();
    let replacement = from_markup_value::<Ref<ControlTheme>>(&Some(value)).unwrap();
    editor.set_theme(Some(replacement));
    window.update_layout();
    assert!(old_host.content().is_none());
    assert_eq!(editor.document(), document);
    assert_eq!(editor.text_presenter(), presenter);
    assert_eq!(document.snapshot(), snapshot);
    let new_host: Ref<Border> = named(&editor, "ContentElement");
    assert_eq!(new_host.child(), Some(presenter.upcast()));
    assert!(editor.header_presenter().is_none());
    assert!(editor.undo().unwrap());
    assert_eq!(editor.text(), "editable");
    window.close();
}

#[test]
fn rich_edit_opened_command_flyout_uses_owner_commands_and_live_enablement() {
    let _app = start_themed_application();
    let _text = TextTestScope::new();
    let editor = RichEditBox::new();
    editor.set_text("format me").unwrap();
    let window = editor_window(&editor);
    editor.select(0, 6);
    editor.focus();
    let flyout = editor
        .context_flyout()
        .unwrap()
        .cast::<MenuFlyout>()
        .unwrap();
    flyout.show_at(&editor);
    window.update_layout();
    for index in 0..9 {
        assert!(
            from_markup_value::<String>(&menu_item(&flyout, index).header())
                .is_some_and(|label| !label.is_empty()),
            "the theme resolves the label of command {index}"
        );
    }
    let bold = menu_item(&flyout, 6);
    let italic = menu_item(&flyout, 7);
    assert!(bold.is_enabled());
    assert!(!bold.is_checked());
    let bold_command = bold
        .command()
        .expect("method binding resolves the RichEditBox owner");
    bold_command.execute(None);
    assert!(editor.is_bold());
    assert!(bold.is_checked());
    assert!(
        menu_item(&flyout, 3).is_enabled(),
        "Undo reflects the same editor history"
    );
    assert!(italic.command().is_some());
    editor.set_is_read_only(true);
    assert!(!bold.is_enabled());
    assert!(!italic.is_enabled());
    assert!(
        menu_item(&flyout, 1).is_enabled(),
        "Copy remains available for a read-only selection"
    );
    flyout.hide();
    window.close();
}

#[test]
fn rich_edit_local_placeholder_brush_survives_pointer_focus_and_disabled_states() {
    let _app = start_themed_application();
    let _text = TextTestScope::new();
    let editor = RichEditBox::new();
    let local: Rc<dyn IBrush> = Brushes::magenta();
    editor.set_placeholder_foreground(Some(local.clone()));
    editor.set_placeholder_text("custom hint");
    let window = editor_window(&editor);
    editor.pseudo_classes().set(":pointerover", true);
    editor.focus();
    editor.set_is_enabled(false);
    window.update_layout();
    assert_eq!(editor.placeholder_foreground(), Some(local.clone()));
    assert_eq!(
        editor.placeholder_presenter().unwrap().foreground(),
        Some(local)
    );
    window.close();
}
