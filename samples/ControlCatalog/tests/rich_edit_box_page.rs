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
