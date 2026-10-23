//! CRUD: a list of people filtered by the prefix of their surname, two text
//! fields for a name and a surname, and buttons that create a person from
//! the fields, update the selected one with them and delete it.
//!
//! Written with the builders of `ferroui-view`; `cargo run -p seven-guis-crud-builders`.

use ferroui_base::layout::{Orientation, VerticalAlignment};
use ferroui_view::prelude::*;
use seven_guis_model::{filter_people, initial_people, Person};

/// The CRUD task.
pub fn crud() -> Element {
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
    let field = |label: &str, value: Signal<String>, row: i32| {
        (
            text_block().text(label).grid_row(row).vertical_alignment(VerticalAlignment::Center),
            text_box().name(label.trim_end_matches(':')).text(value.two_way()).grid_row(row).grid_column(1),
        )
    };

    grid()
        .row_definitions("Auto,*,Auto")
        .column_definitions("*,*")
        .margin(16.0)
        .child(
            stack_panel()
                .orientation(Orientation::Horizontal)
                .spacing(8.0)
                .margin((0.0, 0.0, 0.0, 8.0))
                .child(text_block().text("Filter prefix:").vertical_alignment(VerticalAlignment::Center))
                .child(text_box().name("Prefix").width(120.0).text(prefix.two_way())),
        )
        .child(
            list_box()
                .name("People")
                .grid_row(1)
                .selected_index(selected.two_way())
                .child(for_each(move || filtered.get(), |person| person.id, |person| list_box_item().content(person.label()))),
        )
        .child(
            grid()
                .grid_row(1)
                .grid_column(1)
                .margin((8.0, 0.0, 0.0, 0.0))
                .row_definitions("Auto,Auto")
                .column_definitions("Auto,*")
                .children(field("Name:", name, 0))
                .children(field("Surname:", surname, 1)),
        )
        .child(
            stack_panel()
                .grid_row(2)
                .orientation(Orientation::Horizontal)
                .spacing(8.0)
                .margin((0.0, 8.0, 0.0, 0.0))
                .child(button().name("Create").content("Create").on_click(create))
                .child(button().name("Update").content("Update").is_enabled(move || selected_person.get().is_some()).on_click(update))
                .child(button().name("Delete").content("Delete").is_enabled(move || selected_person.get().is_some()).on_click(delete)),
        )
        .into()
}
