//! Port of `Models/Person.cs`.

use ferroui_base::data::model::{Event, INotifyDataErrorInfo, INotifyPropertyChanged};
use ferroui_base::{ferro_markup_type, BoxedValue};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

/// A person; the first and the last name are required, and report an error
/// when they are blank.
#[derive(Default)]
pub struct Person {
    first_name: RefCell<String>,
    last_name: RefCell<String>,
    is_banned: Cell<bool>,
    age: Cell<i32>,
    error_lookup: RefCell<HashMap<String, Vec<String>>>,
    errors_changed: Event<str>,
    property_changed: Event<str>,
}

impl PartialEq for Person {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// `string.IsNullOrWhiteSpace(value)`.
fn is_null_or_white_space(value: &str) -> bool {
    value.chars().all(char::is_whitespace)
}

impl Person {
    pub fn new() -> Rc<Person> {
        Rc::new(Self::default())
    }

    pub fn first_name(&self) -> String {
        self.first_name.borrow().clone()
    }

    pub fn set_first_name(&self, value: String) {
        let blank = is_null_or_white_space(&value);
        *self.first_name.borrow_mut() = value;
        if blank {
            self.set_error("FirstName", Some("First Name Required"));
        } else {
            self.set_error("FirstName", None);
        }

        self.on_property_changed("FirstName");
    }

    pub fn last_name(&self) -> String {
        self.last_name.borrow().clone()
    }

    pub fn set_last_name(&self, value: String) {
        let blank = is_null_or_white_space(&value);
        *self.last_name.borrow_mut() = value;
        if blank {
            self.set_error("LastName", Some("Last Name Required"));
        } else {
            self.set_error("LastName", None);
        }

        self.on_property_changed("LastName");
    }

    pub fn is_banned(&self) -> bool {
        self.is_banned.get()
    }

    /// The notification names the field (`_isBanned`), as upstream.
    pub fn set_is_banned(&self, value: bool) {
        self.is_banned.set(value);

        self.on_property_changed("_isBanned");
    }

    /// The age of the person.
    pub fn age(&self) -> i32 {
        self.age.get()
    }

    pub fn set_age(&self, value: i32) {
        self.age.set(value);
        self.on_property_changed("Age");
    }

    fn set_error(&self, property_name: &str, error: Option<&str>) {
        match error {
            None | Some("") => {
                let removed = self.error_lookup.borrow_mut().remove(property_name).is_some();
                if removed {
                    self.on_errors_changed(property_name);
                }
            }
            Some(error) => {
                {
                    let mut lookup = self.error_lookup.borrow_mut();
                    match lookup.get_mut(property_name) {
                        Some(error_list) => {
                            error_list.clear();
                            error_list.push(error.to_string());
                        }
                        None => {
                            lookup.insert(property_name.to_string(), vec![error.to_string()]);
                        }
                    }
                }

                self.on_errors_changed(property_name);
            }
        }
    }

    fn on_errors_changed(&self, property_name: &str) {
        self.errors_changed.raise(property_name);
    }

    fn on_property_changed(&self, property_name: &str) {
        self.property_changed.raise(property_name);
    }
}

impl INotifyDataErrorInfo for Person {
    fn has_errors(&self) -> bool {
        !self.error_lookup.borrow().is_empty()
    }

    fn get_errors(&self, property_name: Option<&str>) -> Vec<BoxedValue> {
        match property_name.and_then(|name| self.error_lookup.borrow().get(name).cloned()) {
            Some(error_list) => error_list.into_iter().map(|error| Rc::new(error) as BoxedValue).collect(),
            None => Vec::new(),
        }
    }

    fn errors_changed(&self) -> &Event<str> {
        &self.errors_changed
    }
}

impl INotifyPropertyChanged for Person {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

ferro_markup_type!(class Person {
    this: Rc<Person>,
    handles: [Person, Rc<Person>, Option<Rc<Person>>],
    constructors: [() => Person::new],
    properties: [
        FirstName: String {
            get: |this: &Rc<Person>| this.first_name(),
            set: |this: &Rc<Person>, value: String| this.set_first_name(value)
        },
        LastName: String {
            get: |this: &Rc<Person>| this.last_name(),
            set: |this: &Rc<Person>, value: String| this.set_last_name(value)
        },
        IsBanned: bool {
            get: |this: &Rc<Person>| this.is_banned(),
            set: |this: &Rc<Person>, value: bool| this.set_is_banned(value)
        },
        Age: i32 {
            get: |this: &Rc<Person>| this.age(),
            set: |this: &Rc<Person>, value: i32| this.set_age(value)
        },
        HasErrors: bool { get: |this: &Rc<Person>| this.has_errors() },
    ],
    notify_property_changed: Person,
});

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;

    fn errors(person: &Person, property: &str) -> Vec<String> {
        person.get_errors(Some(property)).iter().map(|e| e.downcast_ref::<String>().cloned().unwrap()).collect()
    }

    #[test]
    fn a_blank_name_is_an_error_and_a_name_clears_it() {
        let person = Person::new();
        let changes = Rc::new(RefCell::new(Vec::new()));
        let sink = changes.clone();
        person.errors_changed().add(Rc::new(move |name: &str| sink.borrow_mut().push(name.to_string())));

        assert!(!person.has_errors());
        person.set_first_name("  ".to_string());
        assert!(person.has_errors());
        assert_eq!(vec!["First Name Required".to_string()], errors(&person, "FirstName"));
        person.set_first_name(String::new());
        assert_eq!(vec!["First Name Required".to_string()], errors(&person, "FirstName"));

        person.set_first_name("John".to_string());
        assert!(!person.has_errors());
        assert!(errors(&person, "FirstName").is_empty());
        // Clearing an error that is not there does not notify.
        person.set_first_name("Jane".to_string());
        assert_eq!(vec!["FirstName".to_string(); 3], *changes.borrow());
        assert!(person.get_errors(None).is_empty());
    }

    #[test]
    fn the_setters_notify_with_the_names_upstream_uses() {
        let person = Person::new();
        let seen = Rc::new(RefCell::new(Vec::new()));
        let sink = seen.clone();
        person.property_changed().add(Rc::new(move |name: &str| sink.borrow_mut().push(name.to_string())));

        person.set_last_name("Doe".to_string());
        person.set_is_banned(true);
        person.set_age(30);
        assert_eq!(vec!["LastName".to_string(), "_isBanned".to_string(), "Age".to_string()], *seen.borrow());
        assert_eq!(30, person.age());
        assert!(person.is_banned());
    }
}
