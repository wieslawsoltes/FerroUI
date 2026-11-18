use ferroui_view::prelude::*;

// The Rust outside the selected invocation must stay exact.
fn grid_view() -> Element {
    rsx! {
        Grid { name: "CustomerGrid", row_definitions: "Auto,*", column_definitions: "*,*",
            TextBlock { name: "Cell", text: "Customer details", Grid.row: 1, Grid.column: 1 }
        }
    }
}
