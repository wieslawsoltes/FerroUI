//! Port of `Views/DashboardView` (markup): the summary and the metrics of the dashboard, each metric
//! in a content control that the locator gives a `MetricView`.

use crate::class_prelude::*;
use crate::located_view;

use crate::view_models::DashboardViewModel;
use crate::views::content_of;
use ferroui_base::media::FontWeight;
use ferroui_view::prelude::*;
use std::rc::Rc;

located_view!(DashboardView, DashboardViewModel, dashboard);

fn dashboard(view_model: &Rc<DashboardViewModel>) -> Element {
    scroll_viewer()
        .child(
            stack_panel()
                .spacing(12.0)
                .margin(16.0)
                .child(text_block().text(view_model.summary()).font_size(18.0).font_weight(FontWeight::SemiBold))
                .child(wrap_panel().children(
                    view_model.metrics().iter().map(|metric| content_control().width(220.0).height(140.0).margin(4.0).content(content_of(metric))).collect::<Vec<_>>(),
                )),
        )
        .into()
}
