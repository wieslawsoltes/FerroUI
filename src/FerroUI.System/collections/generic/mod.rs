//! `System.Collections.Generic`: the collections whose behaviour code that ports .NET relies on,
//! beyond what the standard library has.

mod slot_order_map;

pub use slot_order_map::SlotOrderMap;

/// The former name of [`SlotOrderMap`], the port of .NET's `Dictionary<TKey, TValue>`.
///
/// A public signature takes `std::collections::HashMap`, or `indexmap::IndexMap` where the order
/// of addition is observed (C5 of #423, docs/idiomatic-rust-signatures.md, section 3.7).
#[deprecated(since = "0.5.0", note = "use `SlotOrderMap`, or `HashMap` or `IndexMap`")]
pub type Dictionary<K, V> = SlotOrderMap<K, V>;
