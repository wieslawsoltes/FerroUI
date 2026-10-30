//! Not from upstream: the page of WinUI's RichEditBox (docs/porting/winui-rich-text.md), the
//! class of `Pages/RichEditBoxPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::{ContentPage, RichEditBox, RichTextBlock};

#[repr(C)]
pub struct RichEditBoxPage {
    base: ContentPage,
}

content_page_class!(RichEditBoxPage);
ferro_class_info!(RichEditBoxPage { new: RichEditBoxPage::new });
xaml_class!(RichEditBoxPage, "/Pages/RichEditBoxPage.xaml");

impl RichEditBoxPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.follow_the_editor();
        this
    }

    /// The preview shows a copy of the editor's document: a document belongs to one
    /// view, so the page restores each change of the editor's document into the
    /// preview's own.
    fn follow_the_editor(&self) {
        let editor = self.get_control::<RichEditBox>("Editor");
        let preview = self.get_control::<RichTextBlock>("Preview");
        let document = editor.rich_document();
        let _ = preview.rich_document().restore(&document.snapshot());
        let weak_preview = preview.downgrade();
        let source = document.downgrade();
        // The handler holds the preview and the document weakly; it ends with the editor.
        let _subscription = document.changed(move |_| {
            if let (Some(preview), Some(source)) = (weak_preview.upgrade(), source.upgrade()) {
                let _ = preview.rich_document().restore(&source.snapshot());
            }
        });
    }
}
