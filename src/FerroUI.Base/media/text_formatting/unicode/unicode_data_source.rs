/// Identifies the Unicode Character Database revision that the committed
/// trie data and conformance test suites are aligned with.
/// Updated together with the generated data (see `scripts/convert-unicode-tries.py`).
pub(crate) struct UnicodeDataSource;

#[allow(dead_code)] // informational; the conformance tests read local files instead of the URL
impl UnicodeDataSource {
    pub(crate) const VERSION: &'static str = "17.0.0";
    pub(crate) const UCD: &'static str = "https://www.unicode.org/Public/17.0.0/ucd/";
}
