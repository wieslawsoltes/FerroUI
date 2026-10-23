//! The XSHD compiler always uses .NET's ExplicitCapture option. The backend
//! has no equivalent option, so normalize group syntax before compilation.
//! This is a lexical adaptation, not a replacement regex implementation.

use std::collections::{HashMap, HashSet};
use std::ops::Range;

#[derive(Clone, Copy)]
struct Options {
    explicit_capture: bool,
    ignore_whitespace: bool,
}

struct Scope {
    options: Options,
    extra_closes: usize,
}

enum Target {
    Number(usize),
    Name(String),
}

struct Reference {
    range: Range<usize>,
    target: Target,
    condition: bool,
}

/// Preserve the original syntax tree pattern; only the executable expression
/// is normalized. Backreferences are remapped because .NET numbers enabled
/// unnamed captures before named captures, while the backend uses source order.
pub(super) fn explicit_capture(pattern: &str, ignore_whitespace: bool) -> Result<String, String> {
    let mut output = String::with_capacity(pattern.len());
    let mut options = Options {
        explicit_capture: true,
        ignore_whitespace,
    };
    let mut scopes: Vec<Scope> = Vec::new();
    let mut captures: Vec<Option<String>> = Vec::new();
    let mut capture_names = HashSet::new();
    let mut references: Vec<Reference> = Vec::new();
    let mut index = 0;
    while index < pattern.len() {
        let rest = &pattern[index..];
        match rest.as_bytes()[0] {
            b'\\' => {
                let mut end = index + 1;
                if let Some(next) = pattern[end..].chars().next() {
                    end += next.len_utf8();
                    if ('1'..='9').contains(&next) {
                        while pattern.as_bytes().get(end).is_some_and(u8::is_ascii_digit) {
                            end += 1;
                        }
                        reference(
                            &mut output,
                            &mut references,
                            &pattern[index + 1..end],
                            false,
                        )?;
                        index = end;
                        continue;
                    }
                    if next == 'k' {
                        if let Some((name, after)) = delimited_name(pattern, end) {
                            reference(&mut output, &mut references, name, false)?;
                            index = after;
                            continue;
                        }
                    }
                }
                output.push_str(&pattern[index..end]);
                index = end;
            }
            b'[' => {
                let end = class_end(pattern, index)?;
                output.push_str(&pattern[index..end]);
                index = end;
            }
            b'#' if options.ignore_whitespace => {
                let end = rest.find('\n').map_or(pattern.len(), |end| index + end);
                output.push_str(&pattern[index..end]);
                index = end;
            }
            b'(' if rest.starts_with("(?#") => {
                let end = rest.find(')').ok_or("Unclosed regex comment")? + index + 1;
                output.push_str(&pattern[index..end]);
                index = end;
            }
            b'(' if rest.starts_with("(?(") => {
                // Capture-existence conditions have a delimiter parenthesis,
                // which must not turn into an unnamed capturing group.
                output.push_str("(?:(?(");
                scopes.push(Scope {
                    options,
                    extra_closes: 1,
                });
                let start = index + 3;
                if let Some(length) = pattern[start..].find(')') {
                    let target = &pattern[start..start + length];
                    if !target.is_empty()
                        && target.chars().all(|ch| ch.is_alphanumeric() || ch == '_')
                    {
                        reference(&mut output, &mut references, target, true)?;
                        output.push(')');
                        index = start + length + 1;
                        continue;
                    }
                }
                // In .NET an assertion condition omits the extra parentheses
                // required by the backend around the conditional expression.
                if pattern[start..].starts_with("?=")
                    || pattern[start..].starts_with("?!")
                    || pattern[start..].starts_with("?<=")
                    || pattern[start..].starts_with("?<!")
                {
                    let prefix_len = if pattern[start..].starts_with("?<") {
                        3
                    } else {
                        2
                    };
                    output.push('(');
                    output.push_str(&pattern[start..start + prefix_len]);
                    output.push_str("(?:");
                    scopes.push(Scope {
                        options,
                        extra_closes: 2,
                    });
                    index = start + prefix_len;
                } else {
                    // An expression condition is a zero-width assertion in
                    // .NET, even when its syntax has no explicit `?=` prefix.
                    output.push_str("(?=(?:");
                    scopes.push(Scope {
                        options,
                        extra_closes: 2,
                    });
                    index = start;
                }
            }
            b'(' => {
                if let Some((end, scoped, new_options, flags)) =
                    inline_options(pattern, index, options)
                {
                    if scoped {
                        output.push_str("(?");
                        output.push_str(&flags);
                        output.push(':');
                        scopes.push(Scope {
                            options,
                            extra_closes: 0,
                        });
                    } else if flags.is_empty() {
                        // An option directive is an empty expression. Keeping
                        // it avoids attaching a following quantifier to the
                        // preceding token when the sole option was `n`.
                        output.push_str("(?:)");
                    } else {
                        output.push_str("(?");
                        output.push_str(&flags);
                        output.push(')');
                    }
                    options = new_options;
                    index = end;
                    continue;
                }
                let name_start = if rest.starts_with("(?<")
                    && !rest.starts_with("(?<=")
                    && !rest.starts_with("(?<!")
                {
                    Some(index + 2)
                } else if rest.starts_with("(?'") {
                    Some(index + 2)
                } else if rest.starts_with("(?P<") {
                    Some(index + 3)
                } else {
                    None
                };
                if let Some((name, end)) =
                    name_start.and_then(|start| delimited_name(pattern, start))
                {
                    if name.contains('-') || name.chars().all(|ch| ch.is_ascii_digit()) {
                        return Err("Balancing groups and explicitly numbered capture names are not supported".into());
                    }
                    if !capture_names.insert(name) {
                        return Err(format!("Duplicate capture name '{name}' is not supported"));
                    }
                    captures.push(Some(name.to_owned()));
                    // The backend restores inline flags for scoped option
                    // groups; the wrapper also gives named groups that scope.
                    output.push_str("(?:");
                    output.push_str(&pattern[index..end]);
                    scopes.push(Scope {
                        options,
                        extra_closes: 1,
                    });
                    index = end;
                } else if !rest.starts_with("(?") && !rest.starts_with("(*") {
                    if options.explicit_capture {
                        output.push_str("(?:");
                        scopes.push(Scope {
                            options,
                            extra_closes: 0,
                        });
                    } else {
                        captures.push(None);
                        output.push_str("(?:(");
                        scopes.push(Scope {
                            options,
                            extra_closes: 1,
                        });
                    }
                    index += 1;
                } else {
                    // Lookarounds and atomic groups keep their own syntax and
                    // gain a scope for any nested unscoped option directives.
                    output.push_str("(?:(");
                    scopes.push(Scope {
                        options,
                        extra_closes: 1,
                    });
                    index += 1;
                }
            }
            b')' => {
                output.push(')');
                if let Some(scope) = scopes.pop() {
                    options = scope.options;
                    for _ in 0..scope.extra_closes {
                        output.push(')');
                    }
                }
                index += 1;
            }
            _ => {
                let ch = rest.chars().next().unwrap();
                output.push(ch);
                index += ch.len_utf8();
            }
        }
    }
    if !scopes.is_empty() {
        return Err("Unclosed regex group".into());
    }
    let mut numbers = HashMap::new();
    let mut names = HashMap::new();
    let mut next_number = 1;
    for named in [false, true] {
        for (index, name) in captures.iter().enumerate() {
            if name.is_some() == named {
                numbers.insert(next_number, index + 1);
                next_number += 1;
                if let Some(name) = name {
                    names.insert(name.as_str(), index + 1);
                }
            }
        }
    }
    let mut result = String::with_capacity(output.len());
    let mut start = 0;
    for reference in references {
        result.push_str(&output[start..reference.range.start]);
        start = reference.range.end;
        if let Target::Name(name) = &reference.target {
            if reference.condition && !names.contains_key(name.as_str()) {
                // .NET treats an unknown name in a condition as an expression
                // test, while an unknown numeric capture remains an error.
                result.push_str(&format!("(?={name})"));
                continue;
            }
        }
        let number = match reference.target {
            Target::Number(number) => numbers.get(&number).copied()
                .ok_or_else(|| format!("Reference to undefined capture {number}; legacy octal escapes are not supported"))?,
            Target::Name(ref name) => names.get(name.as_str()).copied()
                .ok_or_else(|| format!("Reference to undefined capture '{name}'"))?,
        };
        if reference.condition {
            result.push_str(&number.to_string());
        } else {
            result.push_str(&format!("\\k<{number}>"));
        }
    }
    result.push_str(&output[start..]);
    Ok(result)
}

fn reference(
    output: &mut String,
    references: &mut Vec<Reference>,
    target: &str,
    condition: bool,
) -> Result<(), String> {
    let start = output.len();
    output.push_str(target);
    references.push(Reference {
        range: start..output.len(),
        target: if target.chars().all(|ch| ch.is_ascii_digit()) {
            Target::Number(target.parse().map_err(|_| "Invalid capture number")?)
        } else {
            Target::Name(target.to_owned())
        },
        condition,
    });
    Ok(())
}

fn delimited_name(pattern: &str, start: usize) -> Option<(&str, usize)> {
    let delimiter = match pattern.as_bytes().get(start)? {
        b'<' => '>',
        b'\'' => '\'',
        _ => return None,
    };
    let name_start = start + 1;
    let end = pattern[name_start..].find(delimiter)? + name_start;
    Some((&pattern[name_start..end], end + 1))
}

fn class_end(pattern: &str, start: usize) -> Result<usize, String> {
    let bytes = pattern.as_bytes();
    let mut index = start;
    let mut nesting = 0;
    while let Some(&byte) = bytes.get(index) {
        index += 1;
        match byte {
            b'\\' => index += pattern[index..].chars().next().map_or(0, char::len_utf8),
            b'[' => {
                nesting += 1;
                if bytes.get(index) == Some(&b'^') {
                    index += 1;
                }
                if bytes.get(index) == Some(&b']') {
                    index += 1;
                }
            }
            b']' => {
                nesting -= 1;
                if nesting == 0 {
                    return Ok(index);
                }
            }
            _ => (),
        }
    }
    Err("Unclosed regex character class".into())
}

fn inline_options(
    pattern: &str,
    start: usize,
    mut options: Options,
) -> Option<(usize, bool, Options, String)> {
    if !pattern[start..].starts_with("(?") {
        return None;
    }
    let mut positive = String::new();
    let mut negative = String::new();
    let mut enabled = true;
    let mut seen_flag = false;
    let mut index = start + 2;
    loop {
        let byte = *pattern.as_bytes().get(index)?;
        match byte {
            b'n' => {
                options.explicit_capture = enabled;
                seen_flag = true;
            }
            b'i' | b'm' | b's' | b'x' | b'R' | b'U' | b'u' => {
                if byte == b'x' {
                    options.ignore_whitespace = enabled;
                }
                if enabled {
                    positive.push(byte as char);
                } else {
                    negative.push(byte as char);
                }
                seen_flag = true;
            }
            b'-' if enabled => enabled = false,
            b':' | b')' if seen_flag || (byte == b':' && enabled) => {
                if !negative.is_empty() {
                    positive.push('-');
                    positive.push_str(&negative);
                }
                return Some((index + 1, byte == b':', options, positive));
            }
            _ => return None,
        }
        index += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fancy_regex::{Regex, RegexBuilder};

    fn regex(pattern: &str, whitespace: bool) -> Regex {
        let normalized = explicit_capture(pattern, whitespace).unwrap();
        RegexBuilder::new(&normalized)
            .ignore_whitespace(whitespace)
            .build()
            .unwrap()
    }

    #[test]
    fn plain_groups_are_not_captures_and_named_captures_remain() {
        let expression = regex(r"(a)(?<word>b)\1\k<word>", false);
        let found = expression.captures("abbb").unwrap().unwrap();
        assert_eq!(found.len(), 2);
        assert_eq!(found.name("word").unwrap().as_str(), "b");
        assert!(explicit_capture(r"(a)\1", false).is_err());
    }

    #[test]
    fn explicit_capture_options_scope_and_unnamed_numbering_precede_names() {
        let expression = regex(r"(?<named>a)(?-n:(b)(?n)(c))(d)\1\2", false);
        assert!(expression.is_match("abcdba").unwrap());
        assert!(!expression.is_match("abcdab").unwrap());
        assert_eq!(expression.captures_len(), 3);
        let expression = regex(r"(?-n)(a)(?n)(b)(?-n:(c))(d)\1\2", false);
        assert!(expression.is_match("abcdac").unwrap());
        assert_eq!(expression.captures_len(), 3);
    }

    #[test]
    fn named_and_assertion_conditions_keep_their_delimiter_structure() {
        let expression = regex(r"^(?<letter>a)?(?(letter)b|c)$", false);
        assert!(expression.is_match("ab").unwrap());
        assert!(expression.is_match("c").unwrap());
        assert!(!expression.is_match("b").unwrap());
        let expression = regex(r"^(?(?=a)(a)|(b))$", false);
        assert!(expression.is_match("a").unwrap());
        assert!(expression.is_match("b").unwrap());
        assert_eq!(expression.captures_len(), 1);
        for pattern in [r"^(?(ab)ab|c)$", r"^(?(a[b])ab|c)$"] {
            let expression = regex(pattern, false);
            assert!(expression.is_match("ab").unwrap());
            assert!(expression.is_match("c").unwrap());
            assert!(!expression.is_match("b").unwrap());
        }
    }

    #[test]
    fn escapes_classes_and_comments_do_not_create_groups_or_change_flags() {
        let expression = regex(
            "(?x) \\( (a) [()] [\\[()] (?# (?n ) # (?-n) (\n (?<word>b) \\k<word>",
            false,
        );
        assert_eq!(expression.captures_len(), 2);
        assert!(expression.is_match("(a([bb").unwrap());
        let expression = regex(
            r"(?x:(a)# ignored (?-n) (
        )(?-x:(b)#)(c)",
            false,
        );
        assert_eq!(expression.captures_len(), 1);
        assert!(expression.is_match("ab#c").unwrap());
    }

    #[test]
    fn named_and_assertion_groups_restore_nested_inline_option_scopes() {
        let expression = regex(r"(?<word>(?i)a)b", false);
        assert!(expression.is_match("Ab").unwrap());
        assert!(!expression.is_match("AB").unwrap());
        let expression = regex(r"(?=(?i)a)ab", false);
        assert!(expression.is_match("ab").unwrap());
        assert!(!expression.is_match("Ab").unwrap());
    }

    #[test]
    fn unsupported_capture_histories_are_rejected_instead_of_rebound() {
        for pattern in [r"(?<x>a)(?<x>b)", r"(?<1>a)", r"(?<x-y>a)", r"(a)\10"] {
            assert!(explicit_capture(pattern, false).is_err(), "{pattern}");
        }
    }
}
