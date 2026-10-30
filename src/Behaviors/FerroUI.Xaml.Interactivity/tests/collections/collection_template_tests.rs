//! The collection template tests of the reference (`Collections/ActionCollectionTemplateTests.cs`
//! and `Collections/BehaviorCollectionTemplateTests.cs` of
//! `tests/Xaml.Behaviors.Interactivity.UnitTests` of Xaml.Behaviors), as an integration test
//! target of this crate.
//!
//! Their windows set an event trigger with a change property action of the interactions crate,
//! which depends on this crate. In the unit tests of the library the interactions crate would be
//! built on a second copy of this crate, whose classes and registered behavior types are distinct
//! from those of the library under test (a behavior of the second copy is not a behavior of the
//! first). An integration test links the library once, so the documents, the interactions crate and
//! the tests share one copy of each type.

mod action_collection_template_tests;
mod behavior_collection_template_tests;

use ferroui_base::{FerroObject, ObjectType, Ref};
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::Control;

/// Starts the application of a test (the headless test attribute of the reference), as
/// `tests/support.rs` of the library does, with the types of the interactions crate registered for
/// the documents.
pub(crate) fn start_app() -> UnitTestApplicationScope {
    // The queue of the `Loaded` events belongs to the thread; start from an empty one.
    Control::reset_loaded_queue_for_unit_tests();
    ferroui_xaml_interactivity::register_types();
    ferroui_xaml_interactions::register_types();
    UnitTestApplication::start(TestServices::styled_window())
}

/// The object as the root class.
pub(crate) fn object<T: ObjectType>(value: &Ref<T>) -> Ref<FerroObject> {
    value.clone().upcast()
}
