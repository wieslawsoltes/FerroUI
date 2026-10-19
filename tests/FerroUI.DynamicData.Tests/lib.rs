//! ferroui-dynamic-data-tests
//!
//! The unit tests of `ferroui-dynamic-data`, ported from DynamicData's `DynamicData.Tests`: one
//! module per upstream test file, under the same folders (`tests/cache`, `tests/list`,
//! `tests/binding`, `tests/aggregation`, `tests/kernel`), each test under the name of the upstream
//! test method in Rust case. Each file names the upstream tests it does not port and the reason.
//!
//! - [`domain`]: the objects of the tests (`DynamicData.Tests.Domain`): people, animals and their
//!   owners, markets and prices, and the seeded generator of random data that replaces Bogus.
//! - [`utilities`]: the aids of the tests (`DynamicData.Tests.Utilities`): test sources, recording
//!   observers, validators of change sets, a fake scheduler and the assertions.

pub mod domain;
pub mod utilities;

#[cfg(test)]
mod tests;
