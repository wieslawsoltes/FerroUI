use ferroui_base::{
    BoxedValue,
    data::core::Value,
    data::model::Model,
};
use ferroui_controls::ItemsSource;
use ferroui_controls_treedatagrid::TreeDataGridSourceRef;
use ferroui_view::prelude::*;
use std::rc::Rc;

#[derive(Clone)]
struct Account {
    name: String,
    owner: String,
    active: bool,
    children: Vec<Rc<Account>>,
}

ferroui_base::ferro_model!(Account, |model| model
    .read_only::<Value<String>>("Name", |row| row.name.clone())
    .read_only::<Value<String>>("Owner", |row| row.owner.clone())
    .read_only::<Value<bool>>("Active", |row| row.active)
    .read_only::<Value<ItemsSource>>("Children", |row| ItemsSource::from_items(
        row.children.iter().map(|child| Some(child.clone() as BoxedValue))
    )));

fn account(name: &str, owner: &str, active: bool, children: Vec<Rc<Account>>) -> Rc<Account> {
    Model::new_model(Account { name: name.into(), owner: owner.into(), active, children })
}

fn accounts() -> ItemsSource {
    let west = account("West division", "Emilia Kowalska", true, vec![]);
    let north = account("Northwind", "Avery Chen", true, vec![west]);
    let alpine = account("Alpine Labs", "Maya Novak", false, vec![]);
    ItemsSource::from_items([north, alpine].map(|row| Some(row as BoxedValue)))
}

fn assigned_model() -> Option<TreeDataGridSourceRef> {
    None
}

pub fn workbench() -> Element {
    rsx! {
        Border { background: "#EEF3F9", padding: 24.0,
            StackPanel { spacing: 12.0,
                TextBlock { text: "Customer accounts", font_size: 26.0, foreground: "#192F50" }
                TextBlock { text: "Portfolio hierarchy", font_size: 15.0, foreground: "#406083" }
                Border { corner_radius: 8.0, background: "#FFFFFF", border_brush: "#D9E3EF", border_thickness: 1.0,
                    TreeDataGrid { name: "Accounts", height: 218.0, items_source: accounts(),
                        column_definitions: ::ferroui_view::tree_data_grid_columns([
                            ::ferroui_view::tree_data_grid_row_header_column().build(),
                            // Keep the account labels with the hierarchy.
                            ::ferroui_view::tree_data_grid_expander_column(
                                ::ferroui_view::tree_data_grid_text_column()
                                    .header("Account 🧭").binding_path("Name")
                                    .width(::ferroui_controls::GridLength::from_pixels(160.0)).build()
                            ).children_path("Children").build(),
                            ::ferroui_view::tree_data_grid_text_column().header("Owner").binding_path("Owner")
                                .width(::ferroui_controls::GridLength::STAR).build(),
                            ::ferroui_view::tree_data_grid_check_box_column().header("Active").binding_path("Active").build(),
                            ::ferroui_view::tree_data_grid_template_column().header("Status")
                                .cell_template(::ferroui_view::template(|account: Account| {
                                    // The app evaluates this expression with its own row.
                                    rsx! { Border { background: "#E6F2EC", corner_radius: 6.0, padding: 8.0,
                                        TextBlock { text: if account.active { "Current" } else { "Review" } }
                                    } }
                                })).build(),
                        ])
                    }
                }
                TextBlock { text: "Review queue", font_size: 15.0, foreground: "#406083" }
                TreeDataGrid { name: "Queue", height: 80.0 }
                TreeDataGrid { name: "ApplicationModel", height: 0.0, model: assigned_model() }
            }
        }
    }
}
