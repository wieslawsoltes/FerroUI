use ferroboard_core::text::{
    NormalizationError, TextCulture, TextUtils, Utf16Text, WordDictionary,
};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    hint::black_box,
};

// Count only this test thread's Rust allocations, excluding input construction/assertions.
thread_local! { static ALLOCATIONS: Cell<Option<usize>> = const { Cell::new(None) }; }
struct CountingAllocator;
fn record() {
    let _ = ALLOCATIONS.try_with(|counter| {
        if let Some(value) = counter.get() {
            counter.set(Some(value + 1));
        }
    });
}
// SAFETY: every allocation/deallocation is forwarded unchanged to System; TLS only records counts.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record();
        // SAFETY: forward the allocator contract's original layout.
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record();
        // SAFETY: forward the allocator contract's original layout.
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        record();
        // SAFETY: forward the original live pointer/layout and requested new size.
        unsafe { System.realloc(pointer, layout, size) }
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: forward the matching original allocation.
        unsafe { System.dealloc(pointer, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn unicode_path(value: &Utf16Text) -> Result<Utf16Text, NormalizationError> {
    TextUtils::remove_diacritics(&TextCulture::to_lower_invariant(value))
}
fn check(units: Vec<u16>) {
    let input = Utf16Text::from_units(units.clone());
    assert_eq!(WordDictionary::normalize(&input), unicode_path(&input));
    assert_eq!(input.units(), units);
}
#[test]
fn ascii_shortcut_matches_unicode_path_for_every_unit_pair_and_mixed_text() {
    check(vec![]);
    for first in 0..128 {
        check(vec![first]);
        for second in 0..128 {
            check(vec![first, second]);
        }
    }
    check((0..4096).map(|n| (n % 128) as u16).collect());
    for text in [
        "café",
        "İstanbul",
        "한글",
        "A\u{301}B",
        "😀\u{1d185}",
        "ＡＢＣ",
    ] {
        check(text.encode_utf16().collect());
    }
    for units in [vec![0xd800], vec![0xdc00], vec![0xfffe], vec![65, 0xfffe]] {
        check(units);
    }
}
fn counted<T>(operation: impl FnOnce() -> T) -> (T, usize) {
    ALLOCATIONS.with(|counter| {
        assert!(counter.get().is_none());
        counter.set(Some(0));
    });
    let value = black_box(operation());
    let count = ALLOCATIONS.with(|counter| counter.replace(None).unwrap());
    (value, count)
}
#[test]
fn common_word_normalization_uses_fewer_rust_allocations_than_unicode_path() {
    for word in ["hello", "HELLO", "can't", "hello-world", "A\0B"] {
        let input = Utf16Text::from(word);
        black_box(unicode_path(&input).unwrap());
        black_box(WordDictionary::normalize(&input).unwrap());
        let (native, native_count) = counted(|| WordDictionary::normalize(&input).unwrap());
        let (reference, reference_count) = counted(|| unicode_path(&input).unwrap());
        assert_eq!(native, reference);
        assert!(
            native_count < reference_count,
            "{word:?}: native={native_count}, unicode={reference_count}"
        );
        println!("{word:?}: native={native_count}, unicode={reference_count}");
    }
}

#[test]
fn fuzzy_search_workspace_allocations_follow_depth_rather_than_visited_nodes() {
    let mut words = Vec::new();
    for a in b'a'..=b'h' {
        for b in b'a'..=b'h' {
            for c in b'a'..=b'h' {
                words.push((
                    Utf16Text::from_units(vec![a as u16, b as u16, c as u16, 97, 97, 97, 97, 97]),
                    10,
                ));
            }
        }
    }
    let dictionary = WordDictionary::from_words(words, "en".into()).unwrap();
    let query = Utf16Text::from("zzzz");
    let calls = Cell::new(0);
    let cost = |_, _| {
        calls.set(calls.get() + 1);
        0.0
    };
    // All 512 dictionary words are too long; no result or sorting allocation can mask row costs.
    let (results, allocations) = counted(|| {
        dictionary
            .find_similar(&query, 1.0, 10, Some(&cost))
            .unwrap()
    });
    assert!(results.is_empty());
    assert!(calls.get() > 1000, "workload must visit many trie nodes");
    println!(
        "fuzzy workspace: {allocations} allocations, {} substitution callbacks",
        calls.get()
    );
    assert!(
        allocations <= 32,
        "bounded-depth search allocated {allocations} times"
    );
}
