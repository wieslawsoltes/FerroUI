//! Port of `Views/ReportsView` (markup): the reports, each in a content control that the locator
//! gives a `ReportView`.

use crate::class_prelude::*;
use crate::located_view;

use crate::view_models::ReportsViewModel;
use crate::views::content_of;
use ferroui_view::prelude::*;
use std::rc::Rc;

located_view!(ReportsView, ReportsViewModel, reports);

fn reports(view_model: &Rc<ReportsViewModel>) -> Element {
    scroll_viewer()
        .child(
            stack_panel()
                .margin(16.0)
                .spacing(8.0)
                .children(view_model.reports().iter().map(|report| content_control().content(content_of(report))).collect::<Vec<_>>()),
        )
        .into()
}
