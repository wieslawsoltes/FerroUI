//! ferroui-splat-tests
//!
//! The unit tests of `ferroui-splat`, ported from Splat's test projects: one module per upstream
//! test file, under the folders of the projects (`splat` for Splat.Tests, `logging` for
//! Splat.Logging.Tests, `builder` for Splat.Builder.Tests, and `common` for the shared helpers of
//! Splat.Common.Test), each test under the name of the upstream test method in Rust case. Each
//! file ends with the upstream tests it does not port and the reason.
//!
//! The tests of the internals of the crate (the containers of the resolvers) are in the crate
//! itself (`src/FerroUI.Splat/tests`).
//!
//! The state of Splat (the locator, the global containers, the builder flags, the mode detector)
//! is per thread, and the test harness runs each test on a thread of its own, so a test starts from
//! a fresh state; the scopes of Splat.Common.Test are kept for the tests that set state up.

#[cfg(test)]
mod tests;
