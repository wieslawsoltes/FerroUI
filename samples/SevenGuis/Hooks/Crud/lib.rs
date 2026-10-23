//! CRUD: a list of people filtered by the prefix of their surname, two text
//! fields for a name and a surname, and buttons that create a person from
//! the fields, update the selected one with them and delete it.
//!
//! Written as a component that runs again, with its state in hooks; `cargo run -p seven-guis-crud-hooks`.

use ferroui_base::layout::{Orientation, VerticalAlignment};
use ferroui_view::prelude::*;
use seven_guis_model::{filter_people, initial_people, Person};

/// The CRUD task. The filtered list and the selected person are computed on
/// each run; the list is a loop of `rsx!` whose items have keys, so the
/// reconciler keeps the item of a person that stays.
#[component(rerender)]
pub fn Crud() -> Element {
    let people = use_signal(initial_people);
    let next_id = use_signal(|| 3_u32);
    let prefix = use_signal(String::new);
    let selected = use_signal(|| -1_i32);
    let name = use_signal(String::new);
    let surname = use_signal(String::new);

    let filtered = people.with(|people| filter_people(people, &prefix.get()));
    let selected_person = usize::try_from(selected.get()).ok().and_then(|index| filtered.get(index).cloned());
    // Selecting a person shows it in the fields.
    use_effect_with(selected_person.clone(), move |person| {
        if let Some(person) = person {
            name.set(person.name.clone());
            surname.set(person.surname.clone());
        }
    });

    let create = move |_: &_| {
        let id = next_id.get();
        next_id.set(id + 1);
        people.update(|people| people.push(Person { id, name: name.get(), surname: surname.get() }));
    };
    let update = {
        let person = selected_person.clone();
        move |_: &_| {
            if let Some(person) = &person {
                people.update(|people| {
                    if let Some(entry) = people.iter_mut().find(|entry| entry.id == person.id) {
                        entry.name = name.get();
                        entry.surname = surname.get();
                    }
                });
            }
        }
    };
    let delete = {
        let person = selected_person.clone();
        move |_: &_| {
            if let Some(person) = &person {
                people.update(|people| people.retain(|entry| entry.id != person.id));
                selected.set(-1);
            }
        }
    };
    let has_selection = selected_person.is_some();

    rsx! {
        Grid { row_definitions: "Auto,*,Auto", column_definitions: "*,*", margin: 16.0,
            StackPanel { orientation: Orientation::Horizontal, spacing: 8.0, margin: (0.0, 0.0, 0.0, 8.0),
                TextBlock { text: "Filter prefix:", vertical_alignment: VerticalAlignment::Center }
                TextBox { name: "Prefix", width: 120.0, text: prefix.two_way() }
            }
            ListBox { name: "People", Grid.row: 1, selected_index: selected.two_way(),
                for person in &filtered {
                    ListBoxItem { key: person.id, content: person.label() }
                }
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
