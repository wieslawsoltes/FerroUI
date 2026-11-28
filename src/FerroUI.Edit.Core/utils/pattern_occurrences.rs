/// Scans a nonempty pattern with KMP and yields every matching start,
/// including overlapping matches. Constructing the iterator costs O(m).
pub(crate) fn pattern_occurrences<'a, T: Eq + 'a>(
    source: impl Iterator<Item = T> + 'a,
    pattern: &'a [T],
) -> impl Iterator<Item = usize> + 'a {
    assert!(!pattern.is_empty());
    let mut failure = vec![0; pattern.len()];
    let mut matched = 0;
    for index in 1..pattern.len() {
        while matched > 0 && pattern[index] != pattern[matched] {
            matched = failure[matched - 1];
        }
        if pattern[index] == pattern[matched] {
            matched += 1;
        }
        failure[index] = matched;
    }

    matched = 0;
    source.enumerate().filter_map(move |(index, unit)| {
        while matched > 0 && unit != pattern[matched] {
            matched = failure[matched - 1];
        }
        if unit == pattern[matched] {
            matched += 1;
        }
        if matched == pattern.len() {
            let start = index + 1 - matched;
            matched = failure[matched - 1];
            Some(start)
        } else {
            None
        }
    })
}
