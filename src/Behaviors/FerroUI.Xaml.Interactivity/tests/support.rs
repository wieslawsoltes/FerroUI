//! The test application (`TestAppBuilder.cs`, `App.xaml`, `App.xaml.cs` of the reference).
//!
//! The reference runs its tests on the headless platform with a window whose theme is the Fluent
//! theme; here they run in the unit test application of the framework, with the services of a
//! styled window (whose test theme gives windows a template that presents their content).

use crate::BehaviorCollection;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{BoxedValue, FerroObject, ObjectType, Ref};
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{Control, Window};
use std::rc::Rc;

/// Starts the application of a test (the headless test attribute of the reference).
pub(crate) fn start_app() -> UnitTestApplicationScope {
    // The queue of the `Loaded` events belongs to the thread; start from an empty one.
    Control::reset_loaded_queue_for_unit_tests();
    crate::register_types();
    UnitTestApplication::start(TestServices::styled_window())
}

/// `Dispatcher.UIThread.RunJobs()`.
pub(crate) fn run_jobs() {
    Dispatcher::ui_thread().run_jobs(None);
}

/// `new Window { Content = content }`.
pub(crate) fn window_with(content: Option<Ref<Control>>) -> Ref<Window> {
    let window = Window::new();
    if let Some(content) = content {
        window.set_content(Some(Control::boxed(content)));
    }
    window
}

/// The object as the root class (the `FerroObject` items of the collections).
pub(crate) fn object<T: ObjectType>(value: &Ref<T>) -> Ref<FerroObject> {
    value.clone().upcast()
}

/// The object as an untyped value (an `object` argument of the reference).
pub(crate) fn boxed<T: ObjectType>(value: &Ref<T>) -> BoxedValue {
    Rc::new(object(value))
}

/// A text as an untyped value.
pub(crate) fn text(value: &str) -> BoxedValue {
    Rc::new(value.to_string())
}

/// `new BehaviorCollection { items.. }`.
pub(crate) fn behavior_collection(items: &[Ref<FerroObject>]) -> BehaviorCollection {
    let collection = BehaviorCollection::new();
    for item in items {
        collection.add(item.clone());
    }
    collection
}

/// Whether running `f` panics (`Assert.Throws<..>` of the reference: the invalid operations and
/// argument errors of the reference are panics here).
pub(crate) fn panics(f: impl FnOnce()) -> bool {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).is_err()
}
