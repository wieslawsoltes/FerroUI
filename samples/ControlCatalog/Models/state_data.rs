//! Port of `Models/StateData.cs`.

use ferroui_base::ferro_markup_type;
use std::fmt;
use std::rc::Rc;

/// A state: its name, its abbreviation and its capital.
pub struct StateData {
    name: String,
    abbreviation: String,
    capital: String,
}

impl PartialEq for StateData {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl StateData {
    pub fn new(name: &str, abbreviation: &str, capital: &str) -> Rc<StateData> {
        Rc::new(Self { name: name.to_string(), abbreviation: abbreviation.to_string(), capital: capital.to_string() })
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn abbreviation(&self) -> String {
        self.abbreviation.clone()
    }

    pub fn capital(&self) -> String {
        self.capital.clone()
    }
}

/// `ToString()`: the name.
impl fmt::Display for StateData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

ferro_markup_type!(class StateData {
    this: Rc<StateData>,
    handles: [StateData, Rc<StateData>, Option<Rc<StateData>>],
    constructors: [
        (String, String, String) => |name: String, abbreviation: String, capital: String| {
            StateData::new(&name, &abbreviation, &capital)
        },
    ],
    properties: [
        Name: String { get: |this: &Rc<StateData>| this.name() },
        Abbreviation: String { get: |this: &Rc<StateData>| this.abbreviation() },
        Capital: String { get: |this: &Rc<StateData>| this.capital() },
    ],
    methods: [fn ToString() -> String => |this: &Rc<StateData>| this.to_string()],
});

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;

    #[test]
    fn the_text_of_a_state_is_its_name() {
        let state = StateData::new("Alabama", "AL", "Montgomery");
        assert_eq!("Alabama", state.to_string());
        assert_eq!("AL", state.abbreviation());
        assert_eq!("Montgomery", state.capital());
    }
}
