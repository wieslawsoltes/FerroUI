use super::{Registry, Token};
use std::ops::Range;

pub(super) fn verify() -> Result<usize, String> {
    const ROOT: &str = "source.while-test";
    let mut registry = Registry::new();
    registry
        .add_grammar_json(
            r#"{
        "scopeName": "source.while-test",
        "patterns": [{
            "begin": "^OUT", "while": "(OUT)", "name": "meta.outer", "contentName": "text.outer",
            "whileCaptures": {"1": {"name": "punctuation.outer"}},
            "patterns": [{
                "begin": "IN", "while": "(IN)", "name": "meta.inner", "contentName": "text.inner",
                "whileCaptures": {"1": {"name": "punctuation.inner"}},
                "patterns": [{"match": "\\G!", "name": "keyword.anchor"}]
            }]
        }]
    }"#,
        )
        .map_err(|error| error.to_string())?;
    let grammar = registry
        .load_grammar(ROOT)
        .map_err(|error| error.to_string())?;

    const OUTER: &str = "meta.outer text.outer";
    const INNER_NAME: &str = "meta.outer text.outer meta.inner";
    const INNER: &str = "meta.outer text.outer meta.inner text.inner";
    const OUTER_MARK: &str = "meta.outer text.outer punctuation.outer";
    const INNER_MARK: &str = "meta.outer text.outer meta.inner text.inner punctuation.inner";
    const ANCHOR: &str = "meta.outer text.outer meta.inner text.inner keyword.anchor";

    // Observed in the pinned reference, with its synthetic newline clipped.
    let lines: &[(&str, usize, &[(Range<usize>, &str)])] = &[
        (
            "OUTINfirst",
            2,
            &[(0..3, "meta.outer"), (3..5, INNER_NAME), (5..10, INNER)],
        ),
        (
            "😀 OUTgapIN!tail",
            2,
            &[
                (0..3, OUTER),
                (3..6, OUTER_MARK),
                (6..9, INNER),
                (9..11, INNER_MARK),
                (11..12, ANCHOR),
                (12..16, INNER),
            ],
        ),
        (
            "pre OUT only",
            1,
            &[(0..4, OUTER), (4..7, OUTER_MARK), (7..12, OUTER)],
        ),
        ("ROOT", 0, &[(0..4, "")]),
        ("OUTIN", 2, &[(0..3, "meta.outer"), (3..5, INNER_NAME)]),
        (
            "gap OUT middling IN !",
            2,
            &[
                (0..4, OUTER),
                (4..7, OUTER_MARK),
                (7..17, INNER),
                (17..19, INNER_MARK),
                (19..21, INNER),
            ],
        ),
        ("IN but no outer", 0, &[(0..15, "")]),
    ];
    let mut previous = None;
    for &(text, depth, spans) in lines {
        let expected: Vec<_> = spans
            .iter()
            .map(|(range, scopes)| Token {
                start_index: range.start,
                end_index: range.end,
                scopes: std::iter::once(ROOT)
                    .chain(scopes.split_whitespace())
                    .map(str::to_owned)
                    .collect(),
            })
            .collect();
        let result = grammar
            .tokenize_line(text, previous.as_ref())
            .map_err(|error| error.to_string())?;
        if result.stopped_early || result.rule_stack.depth() != depth || result.tokens != expected {
            return Err(format!(
                "{text:?}: expected {expected:?} at depth {depth}, received {:?} at depth {}, stopped_early={}",
                result.tokens,
                result.rule_stack.depth(),
                result.stopped_early,
            ));
        }
        previous = Some(result.rule_stack);
    }
    Ok(lines.len())
}

#[cfg(test)]
#[test]
fn nested_while_conditions_keep_contents_through_matches_ahead_of_the_cursor() {
    assert_eq!(verify().unwrap(), 7);
}
