//! Port of `Views/ReportView` (markup): a report of the reports page.

use crate::class_prelude::*;
use crate::located_view;

use crate::view_models::{ReportViewModel, ViewModelBase};
use ferroui_base::media::FontWeight;
use ferroui_view::prelude::*;
use std::rc::Rc;

located_view!(ReportView, ReportViewModel, report);

fn report(view_model: &Rc<ReportViewModel>) -> Element {
    border()
        .padding(12.0)
        .margin(4.0)
        .border_thickness(1.0)
        .corner_radius(6.0)
        .child(
            stack_panel()
                .spacing(4.0)
                .child(text_block().text(view_model.title()).font_weight(FontWeight::SemiBold))
                .child(text_block().text(view_model.summary()))
                .child(text_block().text(format!("Last generated: {}", view_model.last_generated())).opacity(0.7))
                .child(text_block().text(format!("Status: {}", view_model.status()))),
        )
        .into()
}
