//! CRUD: a list of people filtered by the prefix of their surname, two text
//! fields for a name and a surname, and buttons that create a person from
//! the fields, update the selected one with them and delete it.
//!
//! Written with the `rsx!` macro of `ferroui-view`; `cargo run -p seven-guis-crud-macros`.

use ferroui_base::layout::{Orientation, VerticalAlignment};
use ferroui_view::prelude::*;
use seven_guis_model::{filter_people, initial_people, Person};

/// The CRUD task.
#[component]
pub fn Crud() -> Element {
    let people = signal(initial_people());
    let next_id = signal(3_u32);
    let prefix = signal(String::new());
    let selected = signal(-1_i32);
    let name = signal(String::new());
    let surname = signal(String::new());

    let filtered = memo(move |_| people.with(|people| filter_people(people, &prefix.get())));
    let selected_person =
        memo(move |_| usize::try_from(selected.get()).ok().and_then(|index| filtered.with(|list| list.get(index).cloned())));
    // Selecting a person shows it in the fields.
    effect(move || {
        if let Some(person) = selected_person.get() {
            name.set(person.name);
            surname.set(person.surname);
        }
    });

    let create = move |_: &_| {
        let id = next_id.get_untracked();
        next_id.set(id + 1);
        people.update(|people| people.push(Person { id, name: name.get_untracked(), surname: surname.get_untracked() }));
    };
    let update = move |_: &_| {
        if let Some(person) = selected_person.get_untracked() {
            let index = selected.get_untracked();
            people.update(|people| {
                if let Some(entry) = people.iter_mut().find(|entry| entry.id == person.id) {
                    entry.name = name.get_untracked();
                    entry.surname = surname.get_untracked();
                }
            });
            selected.set(index);
        }
    };
    let delete = move |_: &_| {
        if let Some(person) = selected_person.get_untracked() {
            people.update(|people| people.retain(|entry| entry.id != person.id));
            selected.set(-1);
        }
    };
    let has_selection = move || selected_person.get().is_some();

    rsx! {
        Grid { row_definitions: "Auto,*,Auto", column_definitions: "*,*", margin: 16.0,
            StackPanel { orientation: Orientation::Horizontal, spacing: 8.0, margin: (0.0, 0.0, 0.0, 8.0),
                TextBlock { text: "Filter prefix:", vertical_alignment: VerticalAlignment::Center }
                TextBox { name: "Prefix", width: 120.0, text: prefix.two_way() }
            }
            ListBox { name: "People", Grid.row: 1, selected_index: selected.two_way(),
                { for_each(move || filtered.get(), |person| person.id, |person| rsx! { ListBoxItem { content: person.label() } }) }
            }
            Grid { Grid.row: 1, Grid.column: 1, margin: (8.0, 0.0, 0.0, 0.0),
                row_definitions: "Auto,Auto", column_definitions: "Auto,*",
                TextBlock { text: "Name:", vertical_alignment: VerticalAlignment::Center }
                TextBox { name: "Name", Grid.column: 1, text: name.two_way() }
                TextBlock { text: "Surname:", Grid.row: 1, vertical_alignment: VerticalAlignment::Center }
                TextBox { name: "Surname", Grid.row: 1, Grid.column: 1, text: surname.two_way() }
            }
            StackPanel { Grid.row: 2, orientation: Orientation::Horizontal, spacing: 8.0, margin: (0.0, 8.0, 0.0, 0.0),
                Button { name: "Create", on_click: create, "Create" }
                Button { name: "Update", is_enabled: has_selection, on_click: update, "Update" }
                Button { name: "Delete", is_enabled: has_selection, on_click: delete, "Delete" }
            }
        }
    }
}
