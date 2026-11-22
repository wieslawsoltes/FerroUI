use ferroui_base::Ref;
use ferroui_controls::{Control, GridLength, ItemsSource, StackPanel, TextBlock, TextBox};
use ferroui_controls_treedatagrid::TreeDataGrid;
use ferroui_view::prelude::*;

pub(super) fn view() -> Ref<Control> {
    rsx! {
        StackPanel { width: 600.0, height: 290.0, margin: 22.0, spacing: 13.0,
            TextBlock { text: "Account hierarchy", font_size: 27.0, foreground: "#203657" }
            TextBlock { text: "Review company names and onboarding status", foreground: "#496280" }
            TreeDataGrid { name: "Accounts", height: 178.0, items_source: customers(),
                column_definitions: ::ferroui_view::tree_data_grid_columns([
                    ::ferroui_view::tree_data_grid_expander_column(
                        ::ferroui_view::tree_data_grid_template_column().header("Company")
                            .width(GridLength::from_pixels(285.0))
                            .cell_template(::ferroui_view::template(|name: String| rsx! {
                                TextBlock { text: name, foreground: "#203657" }
                            }))
                            .cell_editing_template(::ferroui_view::template(|name: String| rsx! {
                                TextBox { text: name, placeholder_text: "Company name" }
                            })).build()
                    ).build(),
                    ::ferroui_view::tree_data_grid_template_column().header("Review")
                        .width(GridLength::from_pixels(190.0))
                        .cell_template(::ferroui_view::template(|_: String| rsx! {
                            TextBlock { text: "Ready for review", foreground: "#306859" }
                        }))
                        .cell_editing_template(::ferroui_view::template(|_: String| rsx! {
                            TextBox { placeholder_text: "Review status" }
                        })).build(),
                ]),
            }
        }
    }
    .mount()
}

pub(super) fn customers() -> ItemsSource {
    ItemsSource::from_strs(["Northwind Studio", "Fabrikam Design", "Cedar Works"])
}
