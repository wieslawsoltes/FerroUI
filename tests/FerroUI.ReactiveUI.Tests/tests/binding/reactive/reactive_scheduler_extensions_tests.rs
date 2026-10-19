//! `ReactiveSchedulerExtensionsTests` (`ReactiveUI.Binding.Tests/Reactive/ReactiveSchedulerExtensionsTests.cs`).
//!
//! Upstream checks that the scheduler overloads a source generator displaces refuse the call when
//! no generated code claimed it. The bindings of the port take property paths, so every call is
//! served and there is no refusal to test; the scheduler forms are tested by
//! `binding::mixins::scheduler_unsafe_binding_tests`.

// Not ported: BindOneWay_WithNoGeneratedOverload_NamesTheUnsafeTwin - expression trees (the stub that refuses a call without generated code has no counterpart: a property path always carries its getter)
// Not ported: BindOneWay_ConvertingWithNoGeneratedOverload_NamesTheUnsafeTwin - expression trees (the stub that refuses a call without generated code has no counterpart: a property path always carries its getter)
// Not ported: BindTwoWay_WithNoGeneratedOverload_NamesTheUnsafeTwin - expression trees (the stub that refuses a call without generated code has no counterpart: a property path always carries its getter)
// Not ported: BindTwoWay_ConvertingWithNoGeneratedOverload_NamesTheUnsafeTwin - expression trees (the stub that refuses a call without generated code has no counterpart: a property path always carries its getter)
// Not ported: BindOneWay_WithAConverterAndNoGeneratedOverload_NamesTheUnsafeTwin - expression trees (the stub that refuses a call without generated code has no counterpart: a property path always carries its getter)
// Not ported: BindTwoWay_WithConvertersAndNoGeneratedOverload_NamesTheUnsafeTwin - expression trees (the stub that refuses a call without generated code has no counterpart: a property path always carries its getter)
// Not ported: OneWayBind_WithASelectorAndNoGeneratedOverload_NamesTheUnsafeTwin - expression trees (the stub that refuses a call without generated code has no counterpart: a property path always carries its getter)
// Not ported: OneWayBind_WithAConverterAndNoGeneratedOverload_NamesTheUnsafeTwin - expression trees (the stub that refuses a call without generated code has no counterpart: a property path always carries its getter)
// Not ported: Bind_WithConversionsAndNoGeneratedOverload_NamesTheUnsafeTwin - expression trees (the stub that refuses a call without generated code has no counterpart: a property path always carries its getter)
// Not ported: Bind_WithConvertersAndNoGeneratedOverload_NamesTheUnsafeTwin - expression trees (the stub that refuses a call without generated code has no counterpart: a property path always carries its getter)
