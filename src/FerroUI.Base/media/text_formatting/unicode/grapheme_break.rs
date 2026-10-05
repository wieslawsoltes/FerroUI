/// Placeholder kept from upstream: the grapheme break data lives in the
/// segmentation trie, this type only exposes an empty table.
pub(crate) struct GraphemeBreak;

#[allow(dead_code)] // upstream member without users
impl GraphemeBreak {
    pub(crate) const fn data() -> &'static [u8] {
        &[]
    }
}
