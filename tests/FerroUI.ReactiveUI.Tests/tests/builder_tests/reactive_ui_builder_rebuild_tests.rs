//! `ReactiveUIBuilderRebuildTests` (`ReactiveUI.Builder.Tests/ReactiveUIBuilderRebuildTests.cs`).

use super::executors::reset_only;
use ferroui_reactiveui::*;
use ferroui_splat::ModernDependencyResolver;
use std::rc::Rc;

#[test]
fn second_build_leaves_global_state_from_first_build() {
    // `ResetOnlyExecutor`: the test configures and builds the builders itself.
    reset_only(|| {
        let original_bus = MessageBus::current();

        let first_builder = RxAppBuilder::create_reactive_ui_builder();
        let _ = first_builder.with_core_services();
        let _ = first_builder.build_app();

        let first_service = BindingConverters::current();
        let first_bus = MessageBus::current();

        let resolver = Rc::new(ModernDependencyResolver::new());
        let second_builder = ReactiveUIBuilder::new(resolver.clone(), Some(resolver));
        let _ = second_builder.with_core_services();
        let _ = second_builder.with_message_bus_instance(MessageBus::new()).build_app();

        assert!(Rc::ptr_eq(&BindingConverters::current(), &first_service));
        assert!(std::ptr::addr_eq(Rc::as_ptr(&MessageBus::current()), Rc::as_ptr(&first_bus)));

        MessageBus::set_current(original_bus);
    });
}
