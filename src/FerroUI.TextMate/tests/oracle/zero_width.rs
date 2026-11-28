use super::{Registry, Token};
use std::ops::Range;

type Line = (&'static str, usize, &'static [(Range<usize>, &'static str)]);

pub(super) fn verify() -> Result<usize, String> {
    const ROOT: &str = "source.test";
    // Pinned tokenizer output, with the synthetic newline clipped and
    // adjacent equal scope stacks coalesced.
    let cases: &[(&str, &str, &[Line])] = &[
        (
            "dynamic end",
            r##"{"scopeName":"source.test","patterns":[{
                "begin":"(?=^(\\$\\$)open)","end":"(?=\\1)",
                "name":"meta.block","contentName":"inside"
            }]}"##,
            &[
                ("$$open", 1, &[(0..6, "meta.block")]),
                ("$$close", 0, &[(0..7, "")]),
            ],
        ),
        (
            "match inside a rule",
            r##"{"scopeName":"source.test","patterns":[{
                "begin":"<","end":">","name":"meta.block","contentName":"inside",
                "patterns":[{"match":"(?=x)","name":"zero"}]
            }]}"##,
            &[
                (
                    "<abx",
                    0,
                    &[(0..1, "meta.block"), (1..3, "meta.block inside"), (3..4, "")],
                ),
                ("next", 0, &[(0..4, "")]),
            ],
        ),
        (
            "match inside a retokenized capture",
            r##"{"scopeName":"source.test","patterns":[{
                "match":"\\[(x)\\]","name":"meta.outer",
                "captures":{"1":{"name":"capture.inner","contentName":"inside",
                              "patterns":[{"match":"(?=x)","name":"zero"}]}}
            }]}"##,
            &[("[x]", 0, &[(0..3, "meta.outer")])],
        ),
        (
            "recursive begin",
            r##"{"scopeName":"source.test","patterns":[{
                "begin":"(?=^#)","while":"^#","name":"meta.block","contentName":"inside",
                "patterns":[{"include":"$self"}]
            }]}"##,
            &[
                ("#text", 1, &[(0..5, "meta.block inside")]),
                ("next", 0, &[(0..4, "")]),
            ],
        ),
    ];

    let mut count = 0;
    for &(label, source, lines) in cases {
        let mut registry = Registry::new();
        registry
            .add_grammar_json(source)
            .map_err(|error| error.to_string())?;
        let grammar = registry
            .load_grammar(ROOT)
            .map_err(|error| error.to_string())?;
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
            if result.stopped_early
                || result.rule_stack.depth() != depth
                || result.tokens != expected
            {
                return Err(format!(
                    "{label} on {text:?}: expected {expected:?} at depth {depth}, received {:?} at depth {}, stopped_early={}",
                    result.tokens,
                    result.rule_stack.depth(),
                    result.stopped_early,
                ));
            }
            previous = Some(result.rule_stack);
            count += 1;
        }
    }
    Ok(count)
}

#[cfg(test)]
#[test]
fn zero_width_rules_follow_the_reference_recovery() {
    assert_eq!(verify().unwrap(), 7);
}
