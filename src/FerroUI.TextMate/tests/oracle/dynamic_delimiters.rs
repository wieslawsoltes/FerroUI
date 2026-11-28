use super::Registry;

pub(super) fn verify() -> Result<usize, String> {
    const END: &str = r##"{"scopeName":"source.test","patterns":[{
        "begin":"^<<(.*)$","end":"(?x)^\\1$","name":"string.heredoc"
    }]}"##;
    const WHILE: &str = r##"{"scopeName":"source.test","patterns":[{
        "begin":"^([\\t ]+):","while":"(?x)^\\1(?=\\S)","name":"meta.indent"
    }]}"##;
    let cases = [
        (
            "tab in end",
            END,
            [("<<A\t,B#", 1), ("A,B#", 1), ("A\t,B#", 0)],
        ),
        (
            "nonbreaking space in end",
            END,
            [("<<A\u{a0}B", 1), ("AB", 1), ("A\u{a0}B", 0)],
        ),
        (
            "tab and space in while",
            WHILE,
            [("\t :open", 1), ("\t next", 1), (" next", 0)],
        ),
    ];
    for (label, source, lines) in cases {
        let mut registry = Registry::new();
        registry
            .add_grammar_json(source)
            .map_err(|error| error.to_string())?;
        let grammar = registry
            .load_grammar("source.test")
            .map_err(|error| error.to_string())?;
        let mut previous = None;
        for (text, depth) in lines {
            let result = grammar
                .tokenize_line(text, previous.as_ref())
                .map_err(|error| error.to_string())?;
            let actual = result.rule_stack.depth();
            if result.stopped_early || actual != depth {
                return Err(format!(
                    "{label} on {text:?}: expected depth {depth}, got {actual}, stopped_early={}",
                    result.stopped_early,
                ));
            }
            previous = Some(result.rule_stack);
        }
    }
    Ok(cases.len())
}

#[cfg(test)]
#[test]
fn free_spacing_expressions_match_literal_dynamic_delimiters() {
    assert_eq!(verify().unwrap(), 3);
}
