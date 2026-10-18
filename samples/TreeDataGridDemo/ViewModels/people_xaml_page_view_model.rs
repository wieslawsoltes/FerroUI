//! Port of `ViewModels/PeopleXamlPageViewModel.cs`.

use crate::models::Person;
use ferroui_base::collections::FerroList;
use ferroui_base::{ferro_markup_type, BoxedValue};
use ferroui_controls::ItemsSource;
use std::rc::Rc;

/// The people page: the items of a grid whose columns are declared in markup, a hierarchy of
/// people.
pub struct PeopleXamlPageViewModel {
    // `ObservableCollection<Person> People`: a notifying list of the people as untyped values, the
    // `ItemsSource` of the grid.
    people: Rc<FerroList<BoxedValue>>,
}

impl PartialEq for PeopleXamlPageViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// `new Person { Name = .., Title = .., Age = .., IsActive = .., IsExpanded = .., Children = { .. } }`.
fn person(name: &str, title: &str, age: i32, is_active: bool, is_expanded: bool, children: Vec<Rc<Person>>) -> Rc<Person> {
    let person = Person::new();
    person.set_name(Some(name.to_string()));
    person.set_title(Some(title.to_string()));
    person.set_age(age);
    person.set_is_active(is_active);
    person.set_is_expanded(is_expanded);
    for child in children {
        person.add_child(child);
    }
    person
}

impl PeopleXamlPageViewModel {
    pub fn new() -> Rc<Self> {
        let people = FerroList::new();
        for item in [
            person(
                "Eleanor Pope",
                "Engineering Manager",
                32,
                true,
                true,
                vec![person("Marcel Gutierrez", "Intern", 19, true, false, Vec::new())],
            ),
            person(
                "Jeremy Navarro",
                "Director",
                47,
                true,
                true,
                vec![person(
                    "Jane Navarro",
                    "Staff Engineer",
                    42,
                    true,
                    true,
                    vec![person("Lailah Velazquez", "Product Designer", 28, false, false, Vec::new())],
                )],
            ),
            person("Jazmine Schroeder", "Support Lead", 36, false, false, Vec::new()),
        ] {
            people.add(item as BoxedValue);
        }

        Rc::new(Self { people: Rc::new(people) })
    }

    /// The people, as the notifying list of untyped values the grid reads.
    pub fn people(&self) -> Rc<FerroList<BoxedValue>> {
        self.people.clone()
    }
}

ferro_markup_type!(class PeopleXamlPageViewModel {
    this: Rc<PeopleXamlPageViewModel>,
    handles: [PeopleXamlPageViewModel, Rc<PeopleXamlPageViewModel>, Option<Rc<PeopleXamlPageViewModel>>],
    constructors: [() => PeopleXamlPageViewModel::new],
    properties: [
        People: ItemsSource { get: |this: &Rc<PeopleXamlPageViewModel>| ItemsSource::from(this.people()) },
    ],
});
