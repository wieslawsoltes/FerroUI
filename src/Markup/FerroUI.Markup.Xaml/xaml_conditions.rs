//! Custom conditions of conditional XAML namespaces (`xmlns:new="uri?local:Flag(Name)"`).
//!
//! An application registers each condition by name before the first document that uses it
//! loads. The result of a (name, argument) pair is computed once per thread and kept, so a
//! document reads the same however many times it loads. An unregistered condition is false.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use ferroui_base::logging::{LogArea, LogEventLevel, Logger};

/// A condition a document can name in a conditional namespace.
pub trait IXamlCondition {
    /// Whether the nodes in the namespace are read, for the text between the parentheses.
    fn evaluate(&self, argument: &str) -> bool;
}

impl<F: Fn(&str) -> bool> IXamlCondition for F {
    fn evaluate(&self, argument: &str) -> bool {
        self(argument)
    }
}

#[derive(Default)]
struct Conditions {
    registered: HashMap<String, Rc<dyn IXamlCondition>>,
    results: HashMap<(String, String), bool>,
    generation: u64,
}

ferroui_base::ferro_thread_local! {
    static CONDITIONS: RefCell<Conditions> = RefCell::new(Conditions::default());
}

/// Registers `condition` under `name`. Registering a name again replaces the condition and
/// forgets the results of the previous one.
pub fn register_xaml_condition(name: &str, condition: impl IXamlCondition + 'static) {
    register_boxed(name, Rc::new(condition));
}

fn register_boxed(name: &str, condition: Rc<dyn IXamlCondition>) {
    CONDITIONS.with(|conditions| {
        let mut conditions = conditions.borrow_mut();
        conditions.results.retain(|(known, _), _| known != name);
        conditions.generation += 1;
        conditions.registered.insert(name.to_string(), condition);
    });
}

/// Changes whenever a condition is registered: what a parsed document stays valid for.
#[doc(hidden)]
pub fn xaml_conditions_generation() -> u64 {
    CONDITIONS.with(|conditions| conditions.borrow().generation)
}

/// The answer of the condition `name` for `argument`.
pub fn evaluate_xaml_condition(name: &str, argument: &str) -> bool {
    let key = (name.to_string(), argument.to_string());
    let condition = CONDITIONS.with(|conditions| {
        let conditions = conditions.borrow();
        match conditions.results.get(&key) {
            Some(result) => Err(*result),
            None => Ok(conditions.registered.get(name).cloned()),
        }
    });
    let condition = match condition {
        Err(result) => return result,
        Ok(condition) => condition,
    };
    let Some(condition) = condition else {
        if let Some(log) = Logger::try_get(LogEventLevel::Warning, LogArea::CONTROL) {
            log.log_with_values(None, "the XAML condition {Name} is not registered: its nodes are left out", &[&name]);
        }
        return false;
    };
    // Outside the borrow: a condition may read or register others.
    let result = condition.evaluate(argument);
    CONDITIONS.with(|conditions| conditions.borrow_mut().results.insert(key, result));
    result
}
