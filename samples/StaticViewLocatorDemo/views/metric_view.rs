//! Port of `Views/MetricView` (markup): a card of a metric.

use crate::class_prelude::*;
use crate::located_view;

use crate::view_models::MetricViewModel;
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_view::prelude::*;
use std::rc::Rc;

located_view!(MetricView, MetricViewModel, metric);

fn metric(view_model: &Rc<MetricViewModel>) -> Element {
    border()
        .padding(12.0)
        .corner_radius(6.0)
        .border_thickness(1.0)
        .classes("card")
        .child(
            stack_panel()
                .spacing(4.0)
                .child(text_block().text(view_model.name()).font_weight(FontWeight::SemiBold))
                .child(text_block().text(format!("{} {}", view_model.value(), view_model.unit())).font_size(24.0))
                .child(text_block().text(view_model.description()).text_wrapping(TextWrapping::Wrap).opacity(0.7)),
        )
        .into()
}
