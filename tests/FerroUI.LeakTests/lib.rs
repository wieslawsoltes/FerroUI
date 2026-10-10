//! The leak tests of the upstream project, a scenario each.
//!
//! The reference tests build a control tree, take it apart, ask the
//! collector to run and assert that a weak reference to an element is dead.
//! Here objects are reference counted, so the question becomes whether the
//! last strong reference went away: the scenario runs in a block, the block
//! ends (the test drops its own handles), the dispatcher runs its pending
//! jobs (what the reference does before it collects) and a weak handle to
//! the element no longer upgrades. A survivor is a strong reference that was
//! left behind, or a cycle of strong references: there is no collector that
//! would free a cycle.
//!
//! One test for every test of the reference, with the same name and the same
//! steps, in a file for every file of the reference:
//!
//! | reference class | file |
//! |---|---|
//! | the object tests | `ferro_object_tests.rs` |
//! | the binding expression tests | `binding_expression_tests.rs` |
//! | the control tests | `control_tests.rs` |
//! | the data context tests | `data_context_tests.rs` |
//! | the transition tests | `transition_tests.rs` |
//!
//! The reference asserts between the scenario and the collection that the
//! element is still alive. That holds for a collector that has not run and
//! says nothing here, where the element is freed the moment its last handle
//! is dropped: the tests leave it out, except where being alive is what the
//! test is about (a binding keeps its source).
//!
//! `leak.rs` is the assertion: a failure prints the element, its strong and
//! weak reference counts and, with the feature `trace-holders`, the call
//! stacks that allocated the blocks which point at it (`holder_trace.rs`).
//!
//! ```sh
//! cargo test -p ferroui-leak-tests
//! cargo test -p ferroui-leak-tests --features trace-holders -- --test-threads=1
//! ```

#[cfg(test)]
mod holder_trace;
#[cfg(test)]
mod leak;
#[cfg(test)]
mod services;
#[cfg(test)]
mod subject;

#[cfg(test)]
mod binding_expression_tests;
#[cfg(test)]
mod control_tests;
#[cfg(test)]
mod data_context_tests;
#[cfg(test)]
mod ferro_object_tests;
#[cfg(test)]
mod transition_tests;
