//! `BackgroundThreadBindingTests` (`ReactiveUI.Tests/Bindings/PropertyBindings/BackgroundThreadBindingTests.cs`).
//!
//! The test sets the view model's property from a task of the thread pool. The objects of the port
//! are single-threaded (`Rc`), so a property cannot be set from another thread; the propagation of
//! a change through a two-way binding is tested by `property_binding_integration_tests`.

// Not ported: BackgroundThreadViewModelChangePropagatesToView - real threads
