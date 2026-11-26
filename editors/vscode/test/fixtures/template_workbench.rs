use ferroui_base::BoxedValue;
use ferroui_controls::ItemsSource;
use ferroui_view::prelude::*;
use std::rc::Rc;

#[derive(Clone)]
struct Customer {
    name: String,
}

fn customers() -> ItemsSource {
    ItemsSource::from_items(["Northwind", "Alpine"].into_iter().map(|name| {
        Some(Rc::new(Customer { name: name.into() }) as BoxedValue)
    }))
}

pub fn workbench() -> Element {
    rsx! {
        StackPanel { spacing: 12.0, margin: 16.0,
            TextBlock { text: "Customer accounts", font_size: 24.0 }
            ListBox { name: "Directory", height: 124.0, items_source: customers(),
                item_template: ::ferroui_view::template(|customer: Customer| {
                    // Keep the authored row and its data access together.
                    rsx! {
                        Border { background: "#EFF4FA", padding: 24.0,
                            Border { width: 460.0, background: "#FFFFFF", corner_radius: 10.0, padding: 20.0,
                                border_brush: "#D6E1ED", border_thickness: 1.0,
                                horizontal_alignment: ferroui_base::layout::HorizontalAlignment::Center,
                                vertical_alignment: ferroui_base::layout::VerticalAlignment::Center,
                                StackPanel { spacing: 8.0,
                                    TextBlock { text: "Customer directory", font_size: 13.0, foreground: "#406083" }
                                    TextBlock { name: "CustomerTitle", text: "Customer profile 🧭",
                                        font_size: 24.0, foreground: "#192F50" }
                                    TextBlock { text: "Account owner and contact details", foreground: "#406083" }
                                    TextBlock { text: customer.name, foreground: "#192F50" }
                                }
                            }
                        }
                    }
                })
            }
            TableView { name: "Accounts", height: 114.0, items_source: customers(),
                columns: ::ferroui_view::table_columns([
                    ::ferroui_view::table_column().header("Account").width(ferroui_controls::GridLength::from_pixels(180.0)),
                    ::ferroui_view::table_column().header("Status")
                        .cell_template(::ferroui_view::template::<Customer, _>(|_item| {
                            ::ferroui_view::rsx! {
                                TextBlock { name: "AccountStatus", text: "Active account" }
                            }
                        })),
                ])
            }
            ComboBox { name: "Queue", items_source: customers() }
            ListBox { name: "Computed", height: 80.0, item_template: computed_template(), items_source: customers() }
        }
    }
}

fn computed_template() -> Rc<dyn ferroui_controls::templates::IDataTemplate> {
    template(|customer: Customer| text_block().text(customer.name))
}
