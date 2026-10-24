//! Not from the reference: the view of a page not ported yet.

use super::PageInfo;
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::FontWeight;
use ferroui_base::{Ref, Thickness};
use ferroui_controls::{Control, StackPanel, TextBlock};

/// The text the placeholder of a page shows below the name of the page.
pub const NOT_PORTED_YET: &str = "not ported yet";

/// A view that shows the name of the page and that it is not ported yet.
pub fn create(page: &PageInfo) -> Ref<Control> {
    let panel = StackPanel::new();
    panel.set_margin(Thickness::uniform(24.0));
    panel.set_spacing(8.0);
    panel.set_horizontal_alignment(HorizontalAlignment::Center);
    panel.set_vertical_alignment(VerticalAlignment::Center);

    let name = TextBlock::new();
    name.set_text(Some(page.view_name()));
    name.set_font_size(24.0);
    name.set_font_weight(FontWeight::SemiBold);
    name.set_horizontal_alignment(HorizontalAlignment::Center);

    let state = TextBlock::new();
    state.set_text(Some(NOT_PORTED_YET));
    state.set_horizontal_alignment(HorizontalAlignment::Center);

    panel.children().add(name);
    panel.children().add(state);
    panel.upcast()
}
