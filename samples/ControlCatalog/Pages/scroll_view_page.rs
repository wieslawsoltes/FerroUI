//! Not from upstream: the page of WinUI's ScrollView (docs/porting/winui-controls.md, section
//! 10.12), the class of `Pages/ScrollViewPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::{
    Button, Control, ContentPage, ScrollView, ScrollingAnimationMode, ScrollingZoomOptions, StackPanel, TextBlock,
};

#[repr(C)]
pub struct ScrollViewPage {
    base: ContentPage,
}

content_page_class!(ScrollViewPage);
ferro_class_info!(ScrollViewPage { new: ScrollViewPage::new });
xaml_class!(ScrollViewPage, "/Pages/ScrollViewPage.xaml");

impl ScrollViewPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.wire();
        this
    }

    /// The buttons of the first view, the text of its view, and the lines of the second view.
    fn wire(&self) {
        let lines = self.get_control::<StackPanel>("Lines");
        for line in 1..=30 {
            let text = TextBlock::new();
            text.set_text(Some(&format!("Line {line}")));
            lines.children().add(text.upcast::<Control>());
        }
        let both = self.get_control::<ScrollView>("Both");
        let weak = both.downgrade();
        self.get_control::<Button>("ScrollToEnd").click(move |_, _| {
            if let Some(view) = weak.upgrade() {
                view.scroll_to(view.scrollable_width(), view.scrollable_height());
            }
        });
        let weak = both.downgrade();
        self.get_control::<Button>("ZoomIn").click(move |_, _| {
            if let Some(view) = weak.upgrade() {
                view.zoom_by(0.5, None);
            }
        });
        let weak = both.downgrade();
        self.get_control::<Button>("Reset").click(move |_, _| {
            if let Some(view) = weak.upgrade() {
                view.zoom_to_with(1.0, None, ScrollingZoomOptions::new(ScrollingAnimationMode::Disabled));
                view.scroll_to_with(0.0, 0.0, Default::default());
            }
        });
        let text = self.get_control::<TextBlock>("View").downgrade();
        both.view_changed(move |view| {
            if let Some(text) = text.upgrade() {
                text.set_text(Some(&format!(
                    "HorizontalOffset {:.0}, VerticalOffset {:.0}, ZoomFactor {:.2}",
                    view.horizontal_offset(),
                    view.vertical_offset(),
                    view.zoom_factor()
                )));
            }
        })
        .detach();
    }
}
