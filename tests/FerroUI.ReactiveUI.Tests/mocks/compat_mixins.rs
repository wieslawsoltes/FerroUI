//! `CompatMixins` (`ReactiveUI.Tests/Utilities/CompatMixins.cs`): helpers of the tests over
//! sequences.

/// `Run` and `SkipLast` of a sequence (`CompatMixins`).
pub trait CompatMixins<T> {
    /// Calls `block` with each item (`Run(Action<T>)`).
    fn run(self, block: impl FnMut(T));

    /// The items but the last `count`, or none when there are fewer (`SkipLast(int)`).
    fn skip_last_items(self, count: usize) -> Vec<T>;
}

impl<T, I: IntoIterator<Item = T>> CompatMixins<T> for I {
    fn run(self, block: impl FnMut(T)) {
        self.into_iter().for_each(block);
    }

    fn skip_last_items(self, count: usize) -> Vec<T> {
        let items: Vec<T> = self.into_iter().collect();
        let keep = items.len().saturating_sub(count);
        items.into_iter().take(keep).collect()
    }
}
