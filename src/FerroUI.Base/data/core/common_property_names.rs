/// The name of the indexer of a type, as used in property change
/// notifications.
pub const INDEXER_NAME: &str = "Item";

/// Whether a property change notification names the indexer: `Item`, `Item[]` (what an observable
/// collection raises) or `Item[key]` (what a dictionary raises).
pub(crate) fn is_indexer_change(property_name: &str) -> bool {
    match property_name.strip_prefix(INDEXER_NAME) {
        Some("") => true,
        Some(rest) => rest.len() >= 2 && rest.starts_with('[') && rest.ends_with(']'),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::is_indexer_change;

    #[test]
    fn indexer_change_names() {
        for name in ["Item", "Item[]", "Item[key]", "Item[0]"] {
            assert!(is_indexer_change(name), "{name}");
        }
        for name in ["", "Items", "Item[", "Item]", "item[]", "Item[]x"] {
            assert!(!is_indexer_change(name), "{name}");
        }
    }
}
