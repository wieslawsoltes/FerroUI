//! ReactiveUI.Binding.Tests, `View`: the location of the views of view models.
//!
//! The tests that upstream runs under `BindingBuilderTestExecutor` start with [`initialize`]. Each
//! test runs on a thread of its own, so the generated lookups and the service locator (per thread)
//! start empty and need no cleanup.

mod default_view_locator_generated_lookup_chain_tests;
mod default_view_locator_object_resolve_tests;
mod default_view_locator_tests;
mod default_view_locator_typed_resolve_tests;
mod view_attribute_tests;
mod view_locator_not_found_exception_tests;
mod view_locator_tests;
mod view_mapping_builder_tests;

use ferroui_reactiveui::{GeneratedViewDispatch, IViewFor, RxBindingBuilder};
use std::any::Any;
use std::rc::Rc;

/// Builds the application with the core services (`BindingBuilderTestHelper.Initialize` of
/// `BindingBuilderTestExecutor`).
pub(super) fn initialize() {
    RxBindingBuilder::reset_for_testing();
    let _ = RxBindingBuilder::create_reactive_ui_binding_builder().with_core_services().build_app();
}

/// A generated lookup from a closure.
pub(super) fn dispatch(lookup: impl Fn(&dyn Any, &str) -> Option<Rc<dyn IViewFor>> + 'static) -> GeneratedViewDispatch {
    Rc::new(lookup)
}

/// Whether `view` is a view of type `V` (`IsTypeOf<V>()`).
pub(super) fn is_view<V: 'static>(view: &Option<Rc<dyn IViewFor>>) -> bool {
    view.clone().is_some_and(|view| (view as Rc<dyn Any>).is::<V>())
}

/// Whether `view` displays `view_model` (`IsSameReferenceAs`).
pub(super) fn displays<VM: 'static>(view: &Option<Rc<dyn IViewFor>>, view_model: &Rc<VM>) -> bool {
    let expected: Rc<dyn Any> = view_model.clone();
    view.as_ref().and_then(|view| view.view_model()).is_some_and(|actual| Rc::ptr_eq(&actual, &expected))
}

/// Whether `view` is the same object as `expected` (`IsSameReferenceAs`).
pub(super) fn is_same_view<V: IViewFor>(view: &Option<Rc<dyn IViewFor>>, expected: &Rc<V>) -> bool {
    let expected: Rc<dyn IViewFor> = expected.clone();
    view.as_ref().is_some_and(|view| Rc::ptr_eq(view, &expected))
}
