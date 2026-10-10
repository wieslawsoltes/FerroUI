//! The "freed" assertion of the leak tests.
//!
//! A [`Tracked`] is a weak handle to something a scenario created, taken
//! while the scenario still holds it (the weak reference the tests of the
//! reference return from their scenario). After the scenario dropped its
//! handles, [`Tracked::assert_freed`] asserts that the weak handle no longer
//! upgrades. A failure says what is known about who holds the survivor: its
//! strong and weak reference counts and, in a build with the feature
//! `trace-holders`, the call stacks that allocated the blocks which point at
//! it (`holder_trace.rs`).

use crate::holder_trace;
use ferroui_base::collections::FerroList;
use ferroui_base::utilities::WeakEventSender;
use ferroui_base::{ObjectType, Ref};
use std::any::Any;
use std::fmt::Write;
use std::rc::Rc;

/// What a tracked object that is still alive says about itself.
struct Survivor {
    /// The object as text.
    description: String,
    /// The strong references to it, besides the one the probe holds; `None`
    /// where the handle type does not say.
    strong: Option<usize>,
    /// The weak references to it, besides the one of the [`Tracked`].
    weak: Option<usize>,
    /// The address of the object.
    address: usize,
}

/// A weak handle to an object of a scenario, with a name for the messages.
pub struct Tracked {
    name: String,
    probe: Box<dyn Fn() -> Option<Survivor>>,
}

impl Tracked {
    /// Tracks an object of the class hierarchy.
    pub fn object<T: ObjectType>(name: &str, object: &Ref<T>) -> Tracked {
        let weak = object.downgrade();
        Tracked {
            name: name.to_string(),
            probe: Box::new(move || {
                let object = weak.upgrade()?;
                let (strong, weak, address) = counts(&object);
                Some(Survivor {
                    description: format!("{object:?}"),
                    strong: Some(strong - 1),
                    weak: Some(weak.saturating_sub(1)),
                    address,
                })
            }),
        }
    }

    /// Tracks a shared object outside the class hierarchy (a view model, a
    /// transition instance, a subject).
    pub fn shared<T: ?Sized + 'static>(name: &str, value: &Rc<T>) -> Tracked {
        let weak = Rc::downgrade(value);
        Tracked {
            name: name.to_string(),
            probe: Box::new(move || {
                let value = weak.upgrade()?;
                Some(Survivor {
                    description: std::any::type_name::<T>().to_string(),
                    strong: Some(Rc::strong_count(&value) - 1),
                    weak: Some(Rc::weak_count(&value).saturating_sub(1)),
                    address: Rc::as_ptr(&value) as *const () as usize,
                })
            }),
        }
    }

    /// Tracks a list (the handle of a list is the list: every clone of the
    /// handle keeps it alive).
    pub fn list<T: 'static>(name: &str, list: &FerroList<T>) -> Tracked {
        let weak = list.downgrade_sender();
        Tracked {
            name: name.to_string(),
            probe: Box::new(move || {
                let list = FerroList::<T>::upgrade_sender(&weak)?;
                Some(Survivor {
                    description: std::any::type_name::<FerroList<T>>().to_string(),
                    strong: None,
                    weak: None,
                    address: list.sender_address(),
                })
            }),
        }
    }

    /// Whether the object is alive.
    pub fn is_alive(&self) -> bool {
        (self.probe)().is_some()
    }

    /// Asserts that the object is alive.
    #[track_caller]
    pub fn assert_alive(&self) {
        assert!(self.is_alive(), "{} was freed, and something that is alive needs it", self.name);
    }

    /// Asserts that the object was freed; a failure describes who holds it.
    #[track_caller]
    pub fn assert_freed(&self) {
        assert_all_freed(std::slice::from_ref(self));
    }

    fn describe_survivor(&self, out: &mut String) -> bool {
        let Some(survivor) = (self.probe)() else { return false };
        let count = |count: Option<usize>| count.map_or("unknown".to_string(), |count| count.to_string());
        let _ = writeln!(
            out,
            "{} is still alive: {} at {:#x}; strong references: {}, weak references besides the one of the test: {}",
            self.name,
            survivor.description,
            survivor.address,
            count(survivor.strong),
            count(survivor.weak),
        );
        out.push_str(&holder_trace::report(survivor.address));
        true
    }
}

/// Asserts that every object was freed; a failure describes every survivor
/// and who holds it.
#[track_caller]
pub fn assert_all_freed(tracked: &[Tracked]) {
    let mut message = String::new();
    let mut survivors = 0;
    for tracked in tracked {
        if tracked.describe_survivor(&mut message) {
            survivors += 1;
        }
    }
    assert!(survivors == 0, "{survivors} of {} tracked objects survived the scenario\n{message}", tracked.len());
}

/// The strong count, the weak count and the address of an object.
fn counts<T: ObjectType>(object: &Ref<T>) -> (usize, usize, usize) {
    object.reference_counts()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::threading::Dispatcher;
    use ferroui_controls::Border;

    #[test]
    fn an_object_is_alive_while_a_handle_is_and_freed_after() {
        let _scope = Dispatcher::unit_test_scope();
        let object = Border::new();
        let tracked = Tracked::object("object", &object);
        tracked.assert_alive();
        drop(object);
        tracked.assert_freed();
    }

    #[test]
    fn a_survivor_is_described_with_its_reference_counts() {
        let _scope = Dispatcher::unit_test_scope();
        let object = Border::new();
        let second = object.clone();
        let tracked = Tracked::object("object", &object);
        let mut message = String::new();
        assert!(tracked.describe_survivor(&mut message));
        assert!(message.contains("object is still alive"), "{message}");
        assert!(message.contains("strong references: 2"), "{message}");
        drop(second);
        drop(object);
        assert!(!tracked.describe_survivor(&mut String::new()));
    }

    #[test]
    fn a_shared_object_and_a_list_are_tracked_by_their_handles() {
        let shared = Rc::new(5);
        let list: FerroList<i32> = FerroList::new();
        let tracked = [Tracked::shared("shared", &shared), Tracked::list("list", &list)];
        assert!(tracked.iter().all(Tracked::is_alive));
        drop(shared);
        drop(list);
        assert_all_freed(&tracked);
    }
}
