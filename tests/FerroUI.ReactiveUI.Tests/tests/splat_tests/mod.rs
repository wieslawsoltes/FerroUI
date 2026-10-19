//! `ReactiveUI.Splat.Tests`: the builder of ReactiveUI over the resolvers of other containers.
//!
//! `AssemblyHooks` overrides the mode detector so that the tests run as unit tests; nothing these
//! tests run reads the mode, so the port does not need it.

mod splat_adapter_tests;
