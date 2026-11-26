//! The RichEditBox page: the read-only preview follows the editor's document.

use super::support::start_catalog_application;
use crate::pages::RichEditBoxPage;
use ferroui_base::threading::Dispatcher;
use ferroui_controls::{Control, RichEditBox, RichTextBlock, Window};

#[test]
fn the_preview_shows_the_document_of_the_editor() {
    let _app = start_catalog_application();
    let page = RichEditBoxPage::new();
    let window = Window::new();
    window.set_width(1000.0);
    window.set_height(900.0);
    window.set_content(Some(Control::boxed(page.clone())));
    window.show();
    Dispatcher::ui_thread().run_jobs(None);
    let editor = page.get_control::<RichEditBox>("Editor");
    let preview = page.get_control::<RichTextBlock>("Preview");
    assert!(editor.text().starts_with("A shared document"));
    assert_eq!(preview.rich_document().text(), editor.rich_document().text());
    editor.set_text("Changed").unwrap();
    assert_eq!(preview.rich_document().text(), "Changed");
    assert!(preview.bounds().width > 100.0, "{:?}", preview.bounds());
    window.close();
}

#[test]
fn the_list_buttons_set_the_list_type_and_level_of_the_selection() {
    use ferroui_controls::documents::MarkerType;
    use ferroui_base::interactivity::RoutedEventArgs;
    use ferroui_controls::Button;
    let _app = start_catalog_application();
    let click = RoutedEventArgs::with_event(&Button::click_event());
    let page = RichEditBoxPage::new();
    let editor = page.get_control::<RichEditBox>("Editor");
    let types = || editor.rich_document().paragraphs().iter().map(|p| p.list_type()).collect::<Vec<_>>();
    // The markup's last two paragraphs are a bulleted list.
    assert_eq!(&types()[3..], [MarkerType::Bullet, MarkerType::Bullet]);
    editor.set_text("one\ntwo").unwrap();
    editor.select(0, 7);
    page.get_control::<Button>("RomanButton").raise_event(&click);
    assert_eq!(types(), [MarkerType::UppercaseRoman; 2]);
    page.get_control::<Button>("IndentButton").raise_event(&click);
    let levels = editor.rich_document().paragraphs().iter().map(|p| p.list_level_index()).collect::<Vec<_>>();
    assert_eq!(levels, [2, 2]);
    page.get_control::<Button>("NoListButton").raise_event(&click);
    assert_eq!(types(), [MarkerType::None; 2]);
}
