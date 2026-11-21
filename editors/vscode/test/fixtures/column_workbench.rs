use ferroui_view::prelude::*;

fn accounts() -> Element {
    rsx! {
        StackPanel {
            spacing: 8.0,
            TextBlock { text: "Customer accounts", font_size: 24.0 }
            TableView {
                name: "Accounts", height: 160.0,
                columns: ferroui_view::table_columns([
                    // Main account name 🧭
                    ferroui_view::table_column().header("Account").binding_path("Customer.Name")
                        .width(ferroui_controls::GridLength::from_pixels(156.0)),
                    ferroui_view::table_column().header("Region").binding_path("Region"),
                    ferroui_view::table_column().header("Status").cell_template(status_cell())
                ]),
            }
            TableView { name: "Computed", height: 90.0, columns: computed_columns() }
        }
    }
}

fn footer() -> Element {
    rsx! { TextBlock { text: "Quarterly report" } }
}

fn status_cell() -> std::rc::Rc<dyn ferroui_controls::templates::IDataTemplate> {
    ferroui_view::template::<String, _>(|text| text_block().text(text))
}

fn computed_columns() -> ferroui_base::collections::FerroList<ferroui_base::Ref<ferroui_controls::TableViewColumn>> {
    ferroui_view::table_columns([ferroui_view::table_column().header("Computed")])
}
