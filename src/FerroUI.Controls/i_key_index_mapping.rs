// This source file is adapted from the WinUI project.
// (https://github.com/microsoft/microsoft-ui-xaml, `controls/dev/Repeater/ItemsRepeater.idl`)

/// Provides methods that let an items source give each item a unique key, which stays the
/// same while the item is in the source, whatever its index (WinUI `IKeyIndexMapping`).
///
/// An [`IItemsList`](crate::IItemsList) states that it implements the contract through
/// [`key_index_mapping`](crate::IItemsList::key_index_mapping);
/// [`ItemsSourceView::has_key_index_mapping`](crate::ItemsSourceView::has_key_index_mapping)
/// reports it, and a control that realizes items can keep the element of an item across a
/// reset of the source by its key.
// Deviation (DEVIATIONS.md, Items sources): upstream has no such contract (its `ItemsSourceView`
// keeps `HasKeyIndexMapping` and `KeyFromIndex` as internal stubs); this is WinUI's.
pub trait IKeyIndexMapping {
    /// Retrieves the unique key of the item at the specified index.
    fn key_from_index(&self, index: i32) -> String;

    /// Retrieves the index of the item that has the specified unique key, or `-1` when no item
    /// has it.
    fn index_from_key(&self, key: &str) -> i32;
}
