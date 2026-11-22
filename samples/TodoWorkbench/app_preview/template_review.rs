use super::template_rows::queue_row;
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::{ComboBox, Control, ItemsSource, ListBox, StackPanel, TableView, TextBlock};
use ferroui_view::prelude::*;
use std::rc::Rc;

pub(super) fn view() -> Ref<Control> {
    rsx! {
        StackPanel { width: 520.0, height: 390.0, margin: 20.0, spacing: 12.0,
            TextBlock { text: "Customer review", font_size: 27.0, foreground: "#203657" }
            ListBox { name: "Queue", height: 105.0, items_source: customers(), item_template: queue_row() }
            ComboBox { name: "Priority", selected_index: 0, items_source: customers(),
                item_template: /* Queue 🧭 */ ::ferroui_view::template(|customer: String| rsx! { TextBlock { text: customer } }),
                selection_box_item_template: ::ferroui_view::template(|customer: String| rsx! {
                    TextBlock { text: customer, foreground: "#305DB6" }
                }),
            }
            TableView { name: "Reviewers", height: 115.0, items_source: customers(),
                columns: ferroui_view::table_columns([ferroui_view::table_column().header("Account")
                    .cell_template(::ferroui_view::template(|customer: String| rsx! {
                        TextBlock { text: customer, foreground: "#204565" }
                    }))]),
            }
        }
    }.mount()
}

fn customers() -> ItemsSource {
    ItemsSource::from_items(["Northwind Studio", "Fabrikam Design"].map(|name| {
        Some(Rc::new(name.to_owned()) as BoxedValue)
    }))
}
