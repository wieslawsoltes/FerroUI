//! Interactive search must not clone every keyword vector or materialize every name-word list.
use ferroboard_core::{emoji::EmojiCatalog, text::Utf16Text};
use std::hint::black_box;
#[path = "support/rust_allocation_counter.rs"]
mod allocations;
use allocations::counted;

fn catalog(entries: usize, extra_keywords: usize) -> std::sync::Arc<EmojiCatalog> {
    let mut input = String::new();
    for index in 0..entries {
        input.push_str(&format!("symbol{index};entry;needle"));
        for keyword in 0..extra_keywords {
            input.push_str(&format!("|unused{keyword}"));
        }
        input.push('\n');
    }
    EmojiCatalog::parse(&input.as_str().into(), "custom".into()).unwrap()
}

#[test]
fn search_does_not_copy_irrelevant_keyword_storage_or_allocate_name_word_vectors() {
    const ENTRIES: usize = 512;
    let small = catalog(ENTRIES, 0);
    let large = catalog(ENTRIES, 64);
    let query = Utf16Text::from("needle");
    for _ in 0..3 {
        black_box(small.search(&query, 4));
        black_box(large.search(&query, 4));
    }
    let (a, a_count) = counted(|| small.search(&query, 4));
    let (b, b_count) = counted(|| large.search(&query, 4));
    let values = |items: &[std::sync::Arc<ferroboard_core::emoji::EmojiInfo>]| {
        items
            .iter()
            .map(|item| item.value.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(values(&a), values(&b));
    assert_eq!(
        values(&a),
        (0..4)
            .map(|i| Utf16Text::from(format!("symbol{i}").as_str()))
            .collect::<Vec<_>>()
    );
    println!("small={a_count:?}, large={b_count:?}, entries={ENTRIES}");
    // One source invariant-case workspace per name remains; fixed query/snapshot/ranking
    // allocations get explicit slack. Per-entry keyword/word-vector copies would add >=512.
    assert!(
        a_count.calls <= ENTRIES + 64,
        "redundant per-entry allocations: {a_count:?}"
    );
    // Matching the first keyword should not copy any unused keyword storage.
    assert_eq!(
        a_count, b_count,
        "search allocations depend on unused keywords"
    );
}
