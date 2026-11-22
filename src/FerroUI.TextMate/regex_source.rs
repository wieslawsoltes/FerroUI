use crate::{Error, Result};
use std::{borrow::Cow, cell::RefCell, collections::HashMap, fmt::Write as _, rc::Rc};

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

#[cfg(all(
    feature = "native-onig",
    not(feature = "portable-regex"),
    not(target_arch = "wasm32")
))]
pub(crate) type Captures = Rc<[Option<(usize, usize)>]>;
#[cfg(not(all(
    feature = "native-onig",
    not(feature = "portable-regex"),
    not(target_arch = "wasm32")
)))]
pub(crate) type Captures = Vec<Option<(usize, usize)>>;
const CACHE_CAPACITY: usize = 8192;

pub(crate) struct RegexInput<'a> {
    text: &'a str,
    #[cfg(all(
        feature = "native-onig",
        not(feature = "portable-regex"),
        not(target_arch = "wasm32")
    ))]
    native: RefCell<native::LineSearch>,
    #[cfg(all(target_arch = "wasm32", not(feature = "portable-regex")))]
    browser_line: std::cell::OnceCell<crate::browser::BrowserLine>,
}

impl<'a> RegexInput<'a> {
    pub fn new(text: &'a str) -> Self {
        Self {
            text,
            #[cfg(all(
                feature = "native-onig",
                not(feature = "portable-regex"),
                not(target_arch = "wasm32")
            ))]
            native: RefCell::default(),
            #[cfg(all(target_arch = "wasm32", not(feature = "portable-regex")))]
            browser_line: std::cell::OnceCell::new(),
        }
    }

    pub fn text(&self) -> &'a str {
        self.text
    }

    #[cfg(all(target_arch = "wasm32", not(feature = "portable-regex")))]
    fn browser_line(&self) -> Result<&crate::browser::BrowserLine> {
        if let Some(line) = self.browser_line.get() {
            return Ok(line);
        }
        let line = crate::browser::BrowserLine::new(self.text)?;
        Ok(self.browser_line.get_or_init(|| line))
    }
}

#[derive(Default)]
pub(crate) struct RegexCache {
    values: RefCell<HashMap<String, Rc<CompiledRegex>>>,
    #[cfg(all(
        not(feature = "portable-regex"),
        any(target_arch = "wasm32", feature = "native-onig")
    ))]
    groups: RefCell<GroupCache>,
}

#[cfg(all(
    not(feature = "portable-regex"),
    any(target_arch = "wasm32", feature = "native-onig")
))]
type RuleListId = (usize, usize, bool);

#[cfg(all(target_arch = "wasm32", not(feature = "portable-regex")))]
type CompiledGroup = crate::browser::BrowserRegex;

#[cfg(all(
    feature = "native-onig",
    not(feature = "portable-regex"),
    not(target_arch = "wasm32")
))]
type CompiledGroup = native::RuleGroup;

#[cfg(all(
    not(feature = "portable-regex"),
    any(target_arch = "wasm32", feature = "native-onig")
))]
#[derive(Default)]
struct GroupCache {
    values: HashMap<(RuleListId, bool, bool), Option<Rc<CompiledGroup>>>,
    patterns: usize,
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

    /// `None` requests the ordinary scan, including its per-pattern error reporting.
    #[cfg(all(
        not(feature = "portable-regex"),
        any(target_arch = "wasm32", feature = "native-onig")
    ))]
    pub fn search_group<'a, I: ExactSizeIterator<Item = &'a str>>(
        &self,
        list: RuleListId,
        source: impl FnOnce() -> I,
        input: &RegexInput<'_>,
        from: usize,
        first_line: bool,
        allow_g: bool,
    ) -> Option<Result<Option<(usize, Captures)>>> {
        #[cfg(target_arch = "wasm32")]
        if !crate::browser::browser_oniguruma_ready() {
            return None;
        }
        let key = (list, first_line, allow_g);
        let cached = self.groups.borrow().values.get(&key).cloned();
        let scanner = if let Some(cached) = cached {
            cached?
        } else {
            let source = source();
            let count = source.len();
            if count > CACHE_CAPACITY {
                return None;
            }
            let patterns: Vec<_> = source.collect();
            // \K can move capture zero after the scanner's chosen start.
            let compiled = if patterns.iter().any(|pattern| pattern.contains("\\K")) {
                None
            } else {
                self.compile_group(&patterns, first_line, allow_g).map(Rc::new)
            };
            let cached_patterns = if compiled.is_some() { count } else { 0 };
            let mut groups = self.groups.borrow_mut();
            if groups.patterns + cached_patterns > CACHE_CAPACITY
                || groups.values.len() >= CACHE_CAPACITY
            {
                *groups = GroupCache::default();
            }
            groups.patterns += cached_patterns;
            groups.values.insert(key, compiled.clone());
            compiled?
        };
        #[cfg(target_arch = "wasm32")]
        {
            scanner.search_group(input.browser_line().ok()?, from).ok().map(Ok)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            Some(scanner.search(
                &mut input.native.borrow_mut(),
                input.text(),
                from,
                (first_line, allow_g),
            ))
        }
    }

    #[cfg(all(
        not(feature = "portable-regex"),
        any(target_arch = "wasm32", feature = "native-onig")
    ))]
    fn compile_group(
        &self,
        source: &[&str],
        first_line: bool,
        allow_g: bool,
    ) -> Option<CompiledGroup> {
        #[cfg(target_arch = "wasm32")]
        {
            let patterns: Vec<_> = source
                .iter()
                .map(|pattern| resolve_anchors(pattern, first_line, allow_g))
                .collect();
            CompiledGroup::group(&patterns)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            source
                .iter()
                .map(|&pattern| {
                    let regex = self.compile(resolve_anchors(pattern, first_line, allow_g)).ok()?;
                    Some(native::RulePattern { source: pattern.into(), regex })
                })
                .collect::<Option<Vec<_>>>()
                .map(native::RuleGroup)
        }
    }

    pub fn search(
        &self,
        pattern: &str,
        input: &RegexInput<'_>,
        from: usize,
        first_line: bool,
        allow_g: bool,
    ) -> Result<Option<Captures>> {
        #[cfg(all(target_arch = "wasm32", not(feature = "portable-regex")))]
        if !crate::browser::browser_oniguruma_ready() {
            return Err(Error::BackendNotInitialized);
        }
        #[cfg(all(
            feature = "native-onig",
            not(feature = "portable-regex"),
            not(target_arch = "wasm32")
        ))]
        let original = pattern;
        #[cfg(all(
            feature = "native-onig",
            not(feature = "portable-regex"),
            not(target_arch = "wasm32")
        ))]
        if let Some(found) = input
            .native
            .borrow()
            .cached(pattern, (first_line, allow_g), from)
        {
            return Ok(found);
        }
        let regex = self.compile(resolve_anchors(pattern, first_line, allow_g))?;
        #[cfg(all(
            feature = "native-onig",
            not(feature = "portable-regex"),
            not(target_arch = "wasm32")
        ))]
        {
            input.native.borrow_mut().search(
                &regex,
                original,
                input.text(),
                (first_line, allow_g),
                from,
            )
        }
        #[cfg(all(target_arch = "wasm32", not(feature = "portable-regex")))]
        {
            regex.search(input.browser_line()?, from)
        }
        #[cfg(any(
            feature = "portable-regex",
            all(not(feature = "native-onig"), not(target_arch = "wasm32"))
        ))]
        {
            let captures = regex
                .captures_from_pos(input.text(), from)
                .map_err(|e| Error::RegexLimit(e.to_string()))?;
            Ok(captures.map(|captures| {
                (0..captures.len())
                    // `\K` in a lookahead can report a start after the end.
                    .map(|i| captures.get(i).map(|m| (m.start().min(m.end()), m.end())))
                    .collect()
            }))
        }
    }

    fn compile(&self, pattern: Cow<'_, str>) -> Result<Rc<CompiledRegex>> {
        let existing = self.values.borrow().get(pattern.as_ref()).cloned();
        if let Some(regex) = existing {
            return Ok(regex);
        }
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
            // The bundled HTML entity expression has roughly 900 groups in
            // shallow alternatives. Fancy Regex limits nesting separately.
            // Bound source size and total groups before its VM compilation.
            if pattern.len() > 16 * 1024 || pattern.matches('(').count() > 1024 {
                return Err(Error::Regex {
                    pattern: pattern.into_owned(),
                    message: "pattern exceeds the portable backend compilation limit; use native Oniguruma for this grammar".into(),
                });
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
        if values.len() >= CACHE_CAPACITY {
            values.clear();
        }
        values.insert(pattern.into_owned(), compiled.clone());
        Ok(compiled)
    }
}

#[cfg(all(
    feature = "native-onig",
    not(feature = "portable-regex"),
    not(target_arch = "wasm32")
))]
mod native {
    use super::{Captures, Error, Result, CACHE_CAPACITY};
    use std::{collections::HashMap, ops::RangeInclusive, rc::Rc};

    pub(super) struct RulePattern {
        pub(super) source: Box<str>,
        pub(super) regex: Rc<onig::Regex>,
    }

    pub(super) struct RuleGroup(pub(super) Vec<RulePattern>);

    impl RuleGroup {
        pub(super) fn search(
            &self,
            input: &mut LineSearch,
            line: &str,
            from: usize,
            anchors: (bool, bool),
        ) -> Result<Option<(usize, Captures)>> {
            let mut winner = None;
            let mut first_start = usize::MAX;
            for (index, pattern) in self.0.iter().enumerate() {
                let captures = match input.cached(&pattern.source, anchors, from) {
                    Some(cached) => cached,
                    None => input.search(&pattern.regex, &pattern.source, line, anchors, from)?,
                };
                let Some(captures) = captures else { continue };
                let Some((start, end)) = captures[0] else { continue };
                if start < first_start && end <= line.len() {
                    first_start = start;
                    winner = Some((index, captures));
                    if start == from {
                        break;
                    }
                }
            }
            Ok(winner)
        }
    }

    #[derive(Default)]
    pub(super) struct LineSearch {
        region: onig::Region,
        results: HashMap<String, CachedSearch>,
        slots: usize,
    }

    struct CachedSearch {
        starts: RangeInclusive<usize>,
        first_line: Option<bool>,
        allow_g: Option<bool>,
        captures: Option<Captures>,
    }

    impl CachedSearch {
        fn slots(&self) -> usize {
            1 + self.captures.as_ref().map_or(0, |captures| captures.len())
        }
    }

    impl LineSearch {
        pub(super) fn cached(
            &self,
            pattern: &str,
            anchors: (bool, bool),
            from: usize,
        ) -> Option<Option<Captures>> {
            let result = self.results.get(pattern)?;
            (result.first_line.is_none_or(|value| value == anchors.0)
                && result.allow_g.is_none_or(|value| value == anchors.1)
                && result.starts.contains(&from))
                .then(|| result.captures.clone())
        }

        pub(super) fn search(
            &mut self,
            regex: &onig::Regex,
            pattern: &str,
            line: &str,
            anchors: (bool, bool),
            from: usize,
        ) -> Result<Option<Captures>> {
            self.region.clear();
            let mut params = onig::MatchParam::default();
            params.set_retry_limit_in_match(1_000_000);
            params.set_match_stack_limit(1_000_000);
            // The limit above is per start position: a search over a long line needs its own.
            // SAFETY: `params` owns the live parameter block for the duration of the call.
            unsafe {
                onig_sys::onig_set_retry_limit_in_search_of_match_param(params.as_raw(), 10_000_000);
            }
            let start = regex
                .search_with_param(
                    line,
                    from,
                    line.len(),
                    onig::SearchOptions::SEARCH_OPTION_NONE,
                    Some(&mut self.region),
                    params,
                )
                .map_err(|error| Error::RegexLimit(error.to_string()))?;
            let captures =
                start.map(|_| (0..self.region.len()).map(|i| self.region.pos(i)).collect());
            // A callout such as (*SKIP) can affect which starts are tried.
            if !pattern.contains("(*") && !pattern.contains("(?{") {
                // Only the corresponding escape can make each flag matter.
                let first_line = pattern.contains(r"\A").then_some(anchors.0);
                let allow_g = pattern.contains(r"\G").then_some(anchors.1);
                let through = if allow_g == Some(true) {
                    from
                } else {
                    // Oniguruma returns the actual start, before any \K reset.
                    start.unwrap_or(line.len())
                };
                self.remember(
                    pattern,
                    CachedSearch {
                        starts: from..=through,
                        first_line,
                        allow_g,
                        captures: captures.clone(),
                    },
                );
            }
            Ok(captures)
        }

        fn remember(&mut self, pattern: &str, result: CachedSearch) {
            let previous = self.results.get_mut(pattern);
            let previous_slots = previous.as_deref().map_or(0, CachedSearch::slots);
            let slots = self.slots - previous_slots + result.slots();
            if slots > CACHE_CAPACITY {
                return;
            }
            self.slots = slots;
            if let Some(previous) = previous {
                *previous = result;
            } else {
                self.results.insert(pattern.to_owned(), result);
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::super::{RegexCache, RegexInput};
        use std::rc::Rc;

        #[test]
        fn cached_searches_survive_irrelevant_anchor_flag_changes() {
            let cache = RegexCache::default();
            let input = RegexInput::new("b😀");
            for (pattern, first_line, allow_g) in [
                ("(b)", false, false),
                (r"\A(b)", true, false),
                (r"\G(b)", false, true),
            ] {
                let original = cache.search(pattern, &input, 0, true, true).unwrap().unwrap();
                let reused = cache.search(pattern, &input, 0, first_line, allow_g).unwrap().unwrap();
                assert!(Rc::ptr_eq(&original, &reused), "{pattern}");
            }
            for pattern in [r"\A(b)", r"\G(b)"] {
                assert!(cache.search(pattern, &input, 0, false, false).unwrap().is_none());
            }
            assert!(cache.search("missing", &input, 0, true, true).unwrap().is_none());
            assert!(matches!(
                input.native.borrow().cached("missing", (false, false), 1),
                Some(None)
            ));
        }

        #[test]
        fn rule_group_stops_at_the_first_match_at_the_cursor() {
            let cache = RegexCache::default();
            let input = RegexInput::new("a😀zq");
            let rules = || ["q", "(?<word>😀)", "(😀)"].into_iter();
            let (index, captures) = cache
                .search_group((0, 0, false), rules, &input, 1, true, false)
                .unwrap()
                .unwrap()
                .unwrap();
            assert_eq!(index, 1);
            assert_eq!(&*captures, &[Some((1, 5)), Some((1, 5))]);
            assert!(input.native.borrow().cached("(😀)", (true, false), 1).is_none());
            let (index, next) = cache
                .search_group(
                    (0, 0, false),
                    || -> std::iter::Empty<&str> { panic!("compiled group should be reused") },
                    &input,
                    1,
                    true,
                    false,
                )
                .unwrap()
                .unwrap()
                .unwrap();
            assert_eq!(index, 1);
            assert!(Rc::ptr_eq(&captures, &next));
        }

        #[test]
        fn reuse_ends_at_the_native_search_start_before_keep_out() {
            let cache = RegexCache::default();
            let input = RegexInput::new("😀:ab--ab");
            let first = cache
                .search(r"a\K(b)", &input, 0, true, false)
                .unwrap()
                .unwrap();
            let carried = cache
                .search(r"a\K(b)", &input, 5, true, false)
                .unwrap()
                .unwrap();
            assert!(Rc::ptr_eq(&first, &carried));
            assert_eq!(&*carried, &[Some((6, 7)), Some((6, 7))]);
            let next = cache
                .search(r"a\K(b)", &input, 6, true, false)
                .unwrap()
                .unwrap();
            assert_eq!(&*next, &[Some((10, 11)), Some((10, 11))]);
            let missing = cache.search("z", &input, 4, true, false).unwrap();
            assert!(missing.is_none());
            assert!(matches!(
                input.native.borrow().cached("z", (true, false), 10),
                Some(None)
            ));
            assert!(input
                .native
                .borrow()
                .cached("z", (true, false), 0)
                .is_none());
        }

        #[test]
        fn starting_positions_callouts_captures_and_anchors_match_fresh_searches() {
            let cache = RegexCache::default();
            for (source, text) in [
                (r"\G.", "😀ab"),
                (r"\A(a)|(😀)?b", "a😀b bb"),
                (r"(?<=😀)([a-z]+)\1", "😀abab x"),
                (r"(?~x)z", "abc z😀z"),
                (r"a.*?c(*SKIP)(*FAIL)|b", "abc b"),
                (r"(?=b)", "😀ab b"),
            ] {
                let input = RegexInput::new(text);
                for (first_line, allow_g) in [(true, false), (false, false), (true, true)] {
                    for from in text
                        .char_indices()
                        .map(|(byte, _)| byte)
                        .chain([text.len()])
                    {
                        let found = cache.search(source, &input, from, first_line, allow_g);
                        let fresh =
                            cache.search(source, &RegexInput::new(text), from, first_line, allow_g);
                        assert!(fresh.is_ok(), "{source}: {fresh:?}");
                        assert_eq!(found, fresh, "{source} at {from}");
                    }
                }
            }
            let input = RegexInput::new("abc b");
            let skip = r"a.*?c(*SKIP)(*FAIL)|b";
            assert_eq!(
                cache
                    .search(skip, &input, 0, false, false)
                    .unwrap()
                    .unwrap()[0],
                Some((4, 5))
            );
            assert_eq!(
                cache
                    .search(skip, &input, 1, false, false)
                    .unwrap()
                    .unwrap()[0],
                Some((1, 2))
            );
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
                    // Free-spacing expressions can discard raw whitespace,
                    // including non-ASCII whitespace on the portable backend.
                    if value != ' ' && value.is_whitespace() {
                        write!(output, r"\x{{{:X}}}", u32::from(value))
                            .expect("writing to a string cannot fail");
                        continue;
                    }
                    if matches!(
                        value,
                        '\\' | '.'
                            | ','
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

pub(crate) fn resolve_scope_name<'a>(
    name: &'a str,
    line: &str,
    captures: &Captures,
) -> Cow<'a, str> {
    enum CaptureCase {
        Original,
        Lower,
        Upper,
    }

    if !name.contains('$') {
        return Cow::Borrowed(name);
    }

    let mut output = String::with_capacity(name.len());
    let mut remaining = name;
    while let Some((literal, candidate)) = remaining.split_once('$') {
        output.push_str(literal);
        remaining = candidate;
        let digits_and_suffix = candidate.strip_prefix('{').unwrap_or(candidate);
        let digits_len = digits_and_suffix
            .bytes()
            .take_while(u8::is_ascii_digit)
            .count();
        let (digits, suffix) = digits_and_suffix.split_at(digits_len);
        if digits.is_empty() {
            output.push('$');
            continue;
        }

        let (case, suffix) = if candidate.starts_with('{') {
            if let Some(rest) = suffix.strip_prefix(":/downcase}") {
                (CaptureCase::Lower, rest)
            } else if let Some(rest) = suffix.strip_prefix(":/upcase}") {
                (CaptureCase::Upper, rest)
            } else {
                output.push('$');
                continue;
            }
        } else {
            (CaptureCase::Original, suffix)
        };

        match digits
            .parse::<usize>()
            .ok()
            .and_then(|index| captures.get(index))
        {
            Some(Some((start, end))) => {
                let value = line[*start..*end].trim_start_matches('.');
                match case {
                    CaptureCase::Original => output.push_str(value),
                    CaptureCase::Lower => output.push_str(&value.to_lowercase()),
                    CaptureCase::Upper => output.push_str(&value.to_uppercase()),
                }
            }
            // A group that exists but did not participate contributes empty text.
            Some(None) => {}
            None => {
                output.push('$');
                output.push_str(&candidate[..candidate.len() - suffix.len()]);
            }
        }
        remaining = suffix;
    }
    output.push_str(remaining);
    Cow::Owned(output)
}
