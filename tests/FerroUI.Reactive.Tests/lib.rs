//! ferroui-reactive-tests
//!
//! The unit tests of `ferroui-reactive`, ported from Rx.NET's `Tests.System.Reactive`: one module
//! per upstream test file, under the same folders, each test under the name of the upstream test
//! method in Rust case. Each file ends with the upstream tests it does not port and the reason
//! (argument validation that the types of Rust make impossible, real threads, operators of later
//! phases).

#[cfg(test)]
mod tests;
