//! Not from upstream: the page of WinUI's AnnotatedScrollBar (docs/porting/winui-controls.md,
//! section 10.13), the class of `Pages/AnnotatedScrollBarPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{
    AnnotatedScrollBar, AnnotatedScrollBarLabel, Control, ContentPage, ScrollView, StackPanel, TextBlock,
};
use std::rc::Rc;

/// The height of a line of the list.
const LINE_HEIGHT: f64 = 24.0;
/// The lines of a section.
const SECTION_LINES: usize = 8;

#[repr(C)]
pub struct AnnotatedScrollBarPage {
    base: ContentPage,
}

content_page_class!(AnnotatedScrollBarPage);
ferro_class_info!(AnnotatedScrollBarPage { new: AnnotatedScrollBarPage::new });
xaml_class!(AnnotatedScrollBarPage, "/Pages/AnnotatedScrollBarPage.xaml");

/// The section of the line at `offset`.
fn section_at(offset: f64) -> char {
    let section = (offset / (LINE_HEIGHT * SECTION_LINES as f64)).floor().clamp(0.0, 25.0) as u8;
    char::from(b'A' + section)
}

impl AnnotatedScrollBarPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.wire();
        this
    }

    /// The lines and labels of the 26 sections, the detail label, the scrolling text and the link of
    /// the bar to the presenter of the view.
    fn wire(&self) {
        let lines = self.get_control::<StackPanel>("Lines");
        let bar = self.get_control::<AnnotatedScrollBar>("Bar");
        for section in 0..26u8 {
            let letter = char::from(b'A' + section);
            for line in 1..=SECTION_LINES {
                let text = TextBlock::new();
                text.set_height(LINE_HEIGHT);
                text.set_text(Some(format!("{letter} {line}")));
                lines.children().add(text.upcast::<Control>());
            }
            let content: BoxedValue = Rc::new(letter.to_string());
            let offset = f64::from(section) * SECTION_LINES as f64 * LINE_HEIGHT;
            bar.labels().add(AnnotatedScrollBarLabel::new(Some(content), offset));
        }
        bar.detail_label_requested(|_, args| {
            let content: BoxedValue = Rc::new(format!("Section {}", section_at(args.scroll_offset())));
            args.set_content(Some(content));
        })
        .detach();
        let text = self.get_control::<TextBlock>("Scrolling").downgrade();
        bar.scrolling(move |_, args| {
            if let Some(text) = text.upgrade() {
                text.set_text(Some(format!(
                    "Scrolling: {:?} to {:.0}",
                    args.scrolling_event_kind(),
                    args.scroll_offset()
                )));
            }
        })
        .detach();
        let view = self.get_control::<ScrollView>("View");
        let weak_view = view.downgrade();
        let weak_bar = bar.downgrade();
        view.loaded(move |_, _| {
            if let (Some(view), Some(bar)) = (weak_view.upgrade(), weak_bar.upgrade()) {
                if let Some(presenter) = view.scroll_presenter() {
                    presenter.set_vertical_scroll_controller(Some(bar.scroll_controller()));
                }
            }
        });
    }
}
