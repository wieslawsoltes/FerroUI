use crate::{Error, Result};
use std::{borrow::Cow, cell::RefCell, collections::HashMap, rc::Rc};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegexBackend {
    Oniguruma,
    BrowserOniguruma,
    PortableOnigurumaMode,
}

#[cfg(all(
    feature = "native-onig",
    not(feature = "portable-regex"),
    not(target_arch = "wasm32")
))]
type CompiledRegex = onig::Regex;
#[cfg(all(target_arch = "wasm32", not(feature = "portable-regex")))]
type CompiledRegex = crate::browser::BrowserRegex;
#[cfg(any(
    feature = "portable-regex",
    all(not(feature = "native-onig"), not(target_arch = "wasm32"))
))]
type CompiledRegex = fancy_regex::Regex;

pub(crate) type Captures = Vec<Option<(usize, usize)>>;

#[derive(Default)]
pub(crate) struct RegexCache {
    values: RefCell<HashMap<String, Rc<CompiledRegex>>>,
}

impl RegexCache {
    pub fn backend() -> RegexBackend {
        if cfg!(all(
            feature = "native-onig",
            not(feature = "portable-regex"),
            not(target_arch = "wasm32")
        )) {
            RegexBackend::Oniguruma
        } else if cfg!(all(target_arch = "wasm32", not(feature = "portable-regex"))) {
            RegexBackend::BrowserOniguruma
        } else {
            RegexBackend::PortableOnigurumaMode
        }
    }

    pub fn search(
        &self,
        pattern: &str,
        line: &str,
        from: usize,
        first_line: bool,
        allow_g: bool,
    ) -> Result<Option<Captures>> {
        #[cfg(all(target_arch = "wasm32", not(feature = "portable-regex")))]
        if !crate::browser::browser_oniguruma_ready() {
            return Err(Error::BackendNotInitialized);
        }
        let pattern = resolve_anchors(pattern, first_line, allow_g);
        let existing = self.values.borrow().get(pattern.as_ref()).cloned();
        let regex = if let Some(regex) = existing {
            regex
        } else {
            #[cfg(all(
                feature = "native-onig",
                not(feature = "portable-regex"),
                not(target_arch = "wasm32")
            ))]
            let compiled = onig::Regex::with_options(
                &pattern,
                onig::RegexOptions::REGEX_OPTION_CAPTURE_GROUP,
                onig::Syntax::default(),
            );
            #[cfg(all(target_arch = "wasm32", not(feature = "portable-regex")))]
            let compiled = crate::browser::BrowserRegex::new(&pattern);
            #[cfg(any(
                feature = "portable-regex",
                all(not(feature = "native-onig"), not(target_arch = "wasm32"))
            ))]
            let compiled = {
                // The portable engine does not expose a VM compilation-memory quota.
                // Reject excessive patterns before compilation rather than risking a
                // browser allocation failure. Native Oniguruma has no such restriction.
                if pattern.len() > 8192 || pattern.matches('(').count() > 128 {
                    return Err(Error::Regex { pattern: pattern.into_owned(), message: "pattern exceeds the portable backend compilation limit; use native Oniguruma for this grammar".into() });
                }
                fancy_regex::RegexBuilder::new(&pattern)
                    .oniguruma_mode(true)
                    .multi_line(true)
                    .backtrack_limit(100_000)
                    .delegate_size_limit(2 * 1024 * 1024)
                    .delegate_dfa_size_limit(2 * 1024 * 1024)
                    .build()
            };
            let compiled = Rc::new(compiled.map_err(|e| Error::Regex {
                pattern: pattern.to_string(),
                message: e.to_string(),
            })?);
            // Dynamic heredoc terminators may be distinct for every line. Bound the cache.
            let mut values = self.values.borrow_mut();
            if values.len() >= 8192 {
                values.clear();
            }
            values.insert(pattern.into_owned(), compiled.clone());
            compiled
        };
        #[cfg(all(
            feature = "native-onig",
            not(feature = "portable-regex"),
            not(target_arch = "wasm32")
        ))]
        {
            let mut region = onig::Region::new();
            let mut params = onig::MatchParam::default();
            params.set_retry_limit_in_match(1_000_000);
            params.set_match_stack_limit(1_000_000);
            let matched = regex
                .search_with_param(
                    line,
                    from,
                    line.len(),
                    onig::SearchOptions::SEARCH_OPTION_NONE,
                    Some(&mut region),
                    params,
                )
                .map_err(|e| Error::RegexLimit(e.to_string()))?;
            Ok(matched.map(|_| (0..region.len()).map(|i| region.pos(i)).collect()))
        }
        #[cfg(all(target_arch = "wasm32", not(feature = "portable-regex")))]
        {
            regex.search(line, from)
        }
        #[cfg(any(
            feature = "portable-regex",
            all(not(feature = "native-onig"), not(target_arch = "wasm32"))
        ))]
        {
            let captures = regex
                .captures_from_pos(line, from)
                .map_err(|e| Error::RegexLimit(e.to_string()))?;
            Ok(captures.map(|captures| {
                (0..captures.len())
                    .map(|i| captures.get(i).map(|m| (m.start(), m.end())))
                    .collect()
            }))
        }
    }
}

fn resolve_anchors(pattern: &str, first_line: bool, allow_g: bool) -> Cow<'_, str> {
    if !pattern.contains("\\A") && !pattern.contains("\\G") && !pattern.contains("\\z") {
        return Cow::Borrowed(pattern);
    }
    let mut out = String::with_capacity(pattern.len());
    let mut chars = pattern.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            // Preserve the lexical shape inside regex comments and character
            // classes. The upstream engine uses an escaped noncharacter here.
            Some('A') if !first_line => out.push_str("\\\u{ffff}"),
            Some('G') if !allow_g => out.push_str("\\\u{ffff}"),
            Some('z') => out.push_str("$(?!\\n)(?<!\\n)"),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    Cow::Owned(out)
}

pub(crate) fn resolve_back_references(pattern: &str, line: &str, captures: &Captures) -> String {
    let mut output = String::new();
    let mut chars = pattern.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' && chars.peek().is_some_and(char::is_ascii_digit) {
            let mut index = 0usize;
            while let Some(digit) = chars.peek().and_then(|c| c.to_digit(10)) {
                chars.next();
                index = index.saturating_mul(10).saturating_add(digit as usize);
            }
            if let Some(Some((start, end))) = captures.get(index) {
                for value in line[*start..*end].chars() {
                    if matches!(
                        value,
                        '\\' | '.'
                            | '*'
                            | '+'
                            | '?'
                            | '('
                            | ')'
                            | '['
                            | ']'
                            | '{'
                            | '}'
                            | '^'
                            | '$'
                            | '|'
                            | '-'
                            | '#'
                            | ' '
                    ) {
                        output.push('\\');
                    }
                    output.push(value);
                }
            }
        } else {
            output.push(c);
            if c == '\\' {
                if let Some(next) = chars.next() {
                    output.push(next);
                }
            }
        }
    }
    output
}

pub(crate) fn resolve_scope_name(name: &str, line: &str, captures: &Captures) -> String {
    let mut output = String::new();
    let mut chars = name.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '$' {
            output.push(c);
            continue;
        }
        let braces = chars.peek() == Some(&'{');
        if braces {
            chars.next();
        }
        let mut digits = String::new();
        while chars.peek().is_some_and(char::is_ascii_digit) {
            digits.push(chars.next().expect("peeked digit"));
        }
        if digits.is_empty() {
            output.push('$');
            if braces {
                output.push('{');
            }
            continue;
        }
        let index = digits.parse::<usize>().unwrap_or(usize::MAX);
        let mut transform = String::new();
        if braces {
            while let Some(c) = chars.next() {
                if c == '}' {
                    break;
                }
                transform.push(c);
            }
        }
        if let Some(Some((start, end))) = captures.get(index) {
            let value = line[*start..*end].trim_start_matches('.');
            if transform == ":/downcase" {
                output.push_str(&value.to_lowercase());
            } else if transform == ":/upcase" {
                output.push_str(&value.to_uppercase());
            } else {
                output.push_str(value);
            }
        }
    }
    output
}
