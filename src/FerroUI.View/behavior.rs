//! Declarative attachments to the native interaction engine.
//!
//! Pass an array to the `behaviors` field of an `rsx!` control or to
//! [`El::behaviors`]. The declaration is mounted on each native control.

mod mount;
#[cfg(test)]
mod tests;

use crate::{El, ViewClass};
use std::rc::Rc;

pub use ferroui_xaml_interactivity::ComparisonConditionType as Comparison;

/// The trigger and ordered actions to attach to one control.
pub struct Behavior {
    trigger: Trigger,
    actions: Vec<Action>,
}

enum Trigger {
    Event {
        name: String,
        source: Option<String>,
    },
    Data {
        path: String,
        comparison: Comparison,
        value: String,
        revert_on_false: bool,
    },
}

/// A native action declaration.
pub struct Action(ActionKind);

enum ActionKind {
    Property {
        target: Option<String>,
        property: String,
        value: String,
    },
    Command {
        path: String,
        parameter: Option<String>,
    },
    Method {
        target: Option<String>,
        name: String,
    },
    Class {
        target: Option<String>,
        name: String,
        remove: bool,
    },
}

impl<T: ViewClass> El<T> {
    /// Attach native triggers and actions for the lifetime of each mounted control.
    pub fn behaviors(self, behaviors: impl IntoIterator<Item = Behavior>) -> Self {
        let behaviors: Rc<[_]> = behaviors.into_iter().collect::<Vec<_>>().into();
        if behaviors.is_empty() {
            return self;
        }
        self.with(move |control| mount::install(&control.clone().upcast(), behaviors.clone()))
    }
}

/// Run actions when the named event is raised. With no source name, listen on the owner.
pub fn event(
    name: &str,
    source: Option<&str>,
    actions: impl IntoIterator<Item = Action>,
) -> Behavior {
    Behavior {
        trigger: Trigger::Event {
            name: name.into(),
            source: source.map(str::to_owned),
        },
        actions: actions.into_iter().collect(),
    }
}

/// Compare a reflection binding path on the owner's data context.
pub fn data(
    path: &str,
    comparison: Comparison,
    value: &str,
    revert_on_false: bool,
    actions: impl IntoIterator<Item = Action>,
) -> Behavior {
    Behavior {
        trigger: Trigger::Data {
            path: path.into(),
            comparison,
            value: value.into(),
            revert_on_false,
        },
        actions: actions.into_iter().collect(),
    }
}

/// Set a named property using the native property value converter.
pub fn change_property(target: Option<&str>, property: &str, value: &str) -> Action {
    Action(ActionKind::Property {
        target: target.map(str::to_owned),
        property: property.into(),
        value: value.into(),
    })
}

/// Invoke a command obtained from the owner's data context by reflection binding.
pub fn invoke_command(path: &str, parameter: Option<&str>) -> Action {
    Action(ActionKind::Command {
        path: path.into(),
        parameter: parameter.map(str::to_owned),
    })
}

/// Call a method registered by the native behavior runtime.
pub fn call_method(target: Option<&str>, name: &str) -> Action {
    Action(ActionKind::Method {
        target: target.map(str::to_owned),
        name: name.into(),
    })
}

/// Add a style class to the trigger's sender or to a named control.
pub fn add_class(target: Option<&str>, name: &str) -> Action {
    Action(ActionKind::Class {
        target: target.map(str::to_owned),
        name: name.into(),
        remove: false,
    })
}

/// Remove a style class from the trigger's sender or from a named control.
pub fn remove_class(target: Option<&str>, name: &str) -> Action {
    Action(ActionKind::Class {
        target: target.map(str::to_owned),
        name: name.into(),
        remove: true,
    })
}
