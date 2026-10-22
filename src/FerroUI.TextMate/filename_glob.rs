//! Bounded, filesystem-independent matching for language filename patterns.
//!
//! Patterns use `/`, `*`, `?`, whole-component `**`, brace alternatives, and
//! character classes such as `[a-z]` and `[!0-9]`. Brackets quote special
//! characters: `[[]` matches `[` and `[]]` matches `]`. Backslashes in paths are
//! separators; patterns must use `/`. Matching operates on Unicode scalar
//! values. Case-insensitive matching folds ASCII only, independently of the
//! host operating system.
//!
//! The parser and epsilon-NFA evaluator use explicit work lists. Alternatives
//! are never expanded into combinations, and matching never backtracks. Work
//! is bounded by the input length times the compiled pattern size; memory is
//! bounded by the pattern size. Neither parsing nor matching uses recursion.

use std::{error::Error, fmt};

const MAX_PATTERN_BYTES: usize = 8192;
const MAX_GROUP_DEPTH: usize = 128;
const MAX_STATES: usize = MAX_PATTERN_BYTES * 4 + 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilenameGlobErrorKind {
    PatternTooLong,
    PatternTooComplex,
    TooManyGroups,
    UnclosedGroup,
    UnexpectedGroupEnd,
    UnclosedCharacterClass,
    UnexpectedCharacterClassEnd,
    EmptyCharacterClass,
    ReversedCharacterRange,
    PathSeparatorInClass,
    BackslashInPattern,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilenameGlobError {
    /// UTF-8 byte offset in the pattern.
    pub offset: usize,
    pub kind: FilenameGlobErrorKind,
}
impl fmt::Display for FilenameGlobError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self.kind {
            FilenameGlobErrorKind::PatternTooLong => "pattern exceeds 8192 bytes",
            FilenameGlobErrorKind::PatternTooComplex => "pattern exceeds the compiled state limit",
            FilenameGlobErrorKind::TooManyGroups => "pattern exceeds 128 nested groups",
            FilenameGlobErrorKind::UnclosedGroup => "unclosed brace group",
            FilenameGlobErrorKind::UnexpectedGroupEnd => "unmatched closing brace",
            FilenameGlobErrorKind::UnclosedCharacterClass => "unclosed character class",
            FilenameGlobErrorKind::UnexpectedCharacterClassEnd => "unmatched closing bracket",
            FilenameGlobErrorKind::EmptyCharacterClass => "empty character class",
            FilenameGlobErrorKind::ReversedCharacterRange => "reversed character range",
            FilenameGlobErrorKind::PathSeparatorInClass => {
                "path separators are not character class members"
            }
            FilenameGlobErrorKind::BackslashInPattern => {
                "use '/' for separators and brackets to quote special characters"
            }
        };
        write!(
            formatter,
            "invalid filename glob at byte {}: {message}",
            self.offset
        )
    }
}
impl Error for FilenameGlobError {}

#[derive(Clone, Debug)]
enum Predicate {
    Literal(char),
    SegmentCharacter,
    AnyCharacter,
    Class {
        negated: bool,
        ranges: Vec<(char, char)>,
    },
}
impl Predicate {
    fn matches(&self, character: char, ignore_ascii_case: bool) -> bool {
        match self {
            Self::Literal(expected) => {
                *expected == character
                    || ignore_ascii_case && expected.eq_ignore_ascii_case(&character)
            }
            Self::SegmentCharacter => character != '/',
            Self::AnyCharacter => true,
            Self::Class { negated, ranges } => {
                if character == '/' {
                    return false;
                }
                let contains = |value: char| {
                    ranges
                        .iter()
                        .any(|(lower, upper)| *lower <= value && value <= *upper)
                };
                let included = contains(character)
                    || ignore_ascii_case
                        && (contains(character.to_ascii_lowercase())
                            || contains(character.to_ascii_uppercase()));
                included != *negated
            }
        }
    }
}

#[derive(Clone, Debug, Default)]
struct State {
    epsilon: Vec<usize>,
    edges: Vec<(Predicate, usize)>,
}

#[derive(Clone, Copy)]
struct Fragment {
    start: usize,
    end: usize,
}

enum Node {
    Character(Predicate),
    Stars(usize),
    GlobStar,
    Concat(Vec<usize>),
    Alternatives(Vec<usize>),
    Path(Vec<usize>),
}

#[derive(Default)]
struct ParseFrame {
    offset: usize,
    segment: Vec<usize>,
    segments: Vec<usize>,
    alternatives: Vec<usize>,
}
impl ParseFrame {
    fn end_segment(&mut self, nodes: &mut Vec<Node>) {
        let segment = std::mem::take(&mut self.segment);
        if segment.len() == 1 && matches!(nodes[segment[0]], Node::Stars(2)) {
            nodes[segment[0]] = Node::GlobStar;
            self.segments.push(segment[0]);
        } else {
            self.segments.push(nodes.len());
            nodes.push(Node::Concat(segment));
        }
    }
    fn end_alternative(&mut self, nodes: &mut Vec<Node>) {
        self.end_segment(nodes);
        self.alternatives.push(nodes.len());
        nodes.push(Node::Path(std::mem::take(&mut self.segments)));
    }
}

/// A compiled language filename pattern with predictable memory and work bounds.
#[derive(Clone, Debug)]
pub struct FilenameGlob {
    pattern: String,
    states: Vec<State>,
    start: usize,
    accept: usize,
    has_path_separator: bool,
}
impl FilenameGlob {
    pub fn new(pattern: &str) -> Result<Self, FilenameGlobError> {
        if pattern.len() > MAX_PATTERN_BYTES {
            return Err(invalid(
                MAX_PATTERN_BYTES,
                FilenameGlobErrorKind::PatternTooLong,
            ));
        }
        let nodes = parse(pattern)?;
        let mut compiler = Compiler { states: Vec::new() };
        let mut fragments = Vec::<Option<Fragment>>::with_capacity(nodes.len());
        for node in &nodes {
            let fragment = match node {
                Node::Character(predicate) => Some(compiler.character(predicate.clone())),
                Node::Stars(_) => Some(compiler.star(Predicate::SegmentCharacter)),
                Node::GlobStar => None,
                Node::Concat(parts) => Some(
                    compiler.concat(parts.iter().map(|id| fragments[*id].expect("segment part"))),
                ),
                Node::Alternatives(parts) => Some(
                    compiler
                        .alternatives(parts.iter().map(|id| fragments[*id].expect("alternative"))),
                ),
                Node::Path(segments) => Some(compiler.path(segments, &nodes, &fragments)),
            };
            if compiler.states.len() > MAX_STATES {
                return Err(invalid(
                    pattern.len(),
                    FilenameGlobErrorKind::PatternTooComplex,
                ));
            }
            fragments.push(fragment);
        }
        let root = fragments
            .last()
            .and_then(|fragment| *fragment)
            .expect("root path");
        Ok(Self {
            pattern: pattern.to_owned(),
            states: compiler.states,
            start: root.start,
            accept: root.end,
            has_path_separator: pattern.contains('/'),
        })
    }

    pub fn pattern(&self) -> &str {
        &self.pattern
    }

    /// Whether language detection should inspect the full path instead of its basename.
    pub fn has_path_separator(&self) -> bool {
        self.has_path_separator
    }

    /// Matches an entire path, treating both slash styles as separators.
    pub fn is_match(&self, path: &str) -> bool {
        self.matches(path, false)
    }

    /// Uses the catalog's platform-independent ASCII case-insensitive policy.
    pub fn is_match_ignore_ascii_case(&self, path: &str) -> bool {
        self.matches(path, true)
    }

    fn matches(&self, path: &str, ignore_ascii_case: bool) -> bool {
        let mut active = Vec::new();
        let mut next = Vec::new();
        let mut visited = vec![false; self.states.len()];
        let mut work = Vec::new();
        self.closure(self.start, &mut active, &mut visited, &mut work);
        for mut character in path.chars() {
            if character == '\\' {
                character = '/';
            }
            next.clear();
            visited.fill(false);
            for &state in &active {
                for (predicate, destination) in &self.states[state].edges {
                    if predicate.matches(character, ignore_ascii_case) {
                        self.closure(*destination, &mut next, &mut visited, &mut work);
                    }
                }
            }
            if next.is_empty() {
                return false;
            }
            std::mem::swap(&mut active, &mut next);
        }
        active.contains(&self.accept)
    }

    fn closure(
        &self,
        start: usize,
        active: &mut Vec<usize>,
        visited: &mut [bool],
        work: &mut Vec<usize>,
    ) {
        if visited[start] {
            return;
        }
        visited[start] = true;
        work.push(start);
        while let Some(state) = work.pop() {
            active.push(state);
            for &destination in &self.states[state].epsilon {
                if !visited[destination] {
                    visited[destination] = true;
                    work.push(destination);
                }
            }
        }
    }
}

fn invalid(offset: usize, kind: FilenameGlobErrorKind) -> FilenameGlobError {
    FilenameGlobError { offset, kind }
}

fn parse(pattern: &str) -> Result<Vec<Node>, FilenameGlobError> {
    let mut nodes = Vec::new();
    let mut frames = vec![ParseFrame::default()];
    let mut characters = pattern.char_indices().peekable();
    while let Some((offset, character)) = characters.next() {
        match character {
            '{' => {
                if frames.len() > MAX_GROUP_DEPTH {
                    return Err(invalid(offset, FilenameGlobErrorKind::TooManyGroups));
                }
                frames.push(ParseFrame {
                    offset,
                    ..ParseFrame::default()
                });
            }
            '}' => {
                if frames.len() == 1 {
                    return Err(invalid(offset, FilenameGlobErrorKind::UnexpectedGroupEnd));
                }
                let mut frame = frames.pop().expect("nested frame");
                frame.end_alternative(&mut nodes);
                frames
                    .last_mut()
                    .expect("outer frame")
                    .segment
                    .push(nodes.len());
                nodes.push(Node::Alternatives(frame.alternatives));
            }
            ',' if frames.len() > 1 => {
                frames
                    .last_mut()
                    .expect("current frame")
                    .end_alternative(&mut nodes);
            }
            '/' => frames
                .last_mut()
                .expect("current frame")
                .end_segment(&mut nodes),
            '\\' => return Err(invalid(offset, FilenameGlobErrorKind::BackslashInPattern)),
            ']' => {
                return Err(invalid(
                    offset,
                    FilenameGlobErrorKind::UnexpectedCharacterClassEnd,
                ))
            }
            character => {
                let node = match character {
                    '*' => {
                        let mut count = 1;
                        while characters.peek().is_some_and(|(_, next)| *next == '*') {
                            characters.next();
                            count += 1;
                        }
                        Node::Stars(count)
                    }
                    '?' => Node::Character(Predicate::SegmentCharacter),
                    '[' => Node::Character(character_class(offset, &mut characters)?),
                    _ => Node::Character(Predicate::Literal(character)),
                };
                frames
                    .last_mut()
                    .expect("current frame")
                    .segment
                    .push(nodes.len());
                nodes.push(node);
            }
        }
    }
    if frames.len() > 1 {
        return Err(invalid(
            frames.last().expect("unclosed frame").offset,
            FilenameGlobErrorKind::UnclosedGroup,
        ));
    }
    let mut root = frames.pop().expect("root frame");
    root.end_alternative(&mut nodes);
    Ok(nodes)
}

fn character_class(
    offset: usize,
    characters: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
) -> Result<Predicate, FilenameGlobError> {
    let negated = characters
        .peek()
        .is_some_and(|(_, next)| *next == '!' || *next == '^');
    if negated {
        characters.next();
    }
    let mut members = Vec::new();
    let mut closed = false;
    for (position, character) in characters.by_ref() {
        if character == ']' && !members.is_empty() {
            closed = true;
            break;
        }
        if character == '/' || character == '\\' {
            return Err(invalid(
                position,
                FilenameGlobErrorKind::PathSeparatorInClass,
            ));
        }
        members.push((position, character));
    }
    if !closed {
        return Err(invalid(
            offset,
            FilenameGlobErrorKind::UnclosedCharacterClass,
        ));
    }
    if members.is_empty() {
        return Err(invalid(offset, FilenameGlobErrorKind::EmptyCharacterClass));
    }
    let mut ranges = Vec::new();
    let mut index = 0;
    while index < members.len() {
        let lower = members[index].1;
        if index + 2 < members.len() && members[index + 1].1 == '-' {
            let upper = members[index + 2].1;
            if lower > upper {
                return Err(invalid(
                    members[index + 1].0,
                    FilenameGlobErrorKind::ReversedCharacterRange,
                ));
            }
            ranges.push((lower, upper));
            index += 3;
        } else {
            ranges.push((lower, lower));
            index += 1;
        }
    }
    Ok(Predicate::Class { negated, ranges })
}

struct Compiler {
    states: Vec<State>,
}
impl Compiler {
    fn state(&mut self) -> usize {
        let id = self.states.len();
        self.states.push(State::default());
        id
    }
    fn empty(&mut self) -> Fragment {
        let state = self.state();
        Fragment {
            start: state,
            end: state,
        }
    }
    fn character(&mut self, predicate: Predicate) -> Fragment {
        let start = self.state();
        let end = self.state();
        self.states[start].edges.push((predicate, end));
        Fragment { start, end }
    }
    fn star(&mut self, predicate: Predicate) -> Fragment {
        let start = self.state();
        let end = self.state();
        self.states[start].epsilon.push(end);
        self.states[start].edges.push((predicate, start));
        Fragment { start, end }
    }
    fn concat(&mut self, parts: impl IntoIterator<Item = Fragment>) -> Fragment {
        let mut parts = parts.into_iter();
        let Some(mut result) = parts.next() else {
            return self.empty();
        };
        for next in parts {
            self.states[result.end].epsilon.push(next.start);
            result.end = next.end;
        }
        result
    }
    fn alternatives(&mut self, parts: impl IntoIterator<Item = Fragment>) -> Fragment {
        let start = self.state();
        let end = self.state();
        for part in parts {
            self.states[start].epsilon.push(part.start);
            self.states[part.end].epsilon.push(end);
        }
        Fragment { start, end }
    }
    /// Zero or more complete directory segments, including their separators.
    fn directories(&mut self) -> Fragment {
        let boundary = self.state();
        let inside = self.state();
        let end = self.state();
        self.states[boundary].epsilon.push(end);
        self.states[boundary]
            .edges
            .push((Predicate::Literal('/'), boundary));
        self.states[boundary]
            .edges
            .push((Predicate::SegmentCharacter, inside));
        self.states[inside]
            .edges
            .push((Predicate::SegmentCharacter, inside));
        self.states[inside]
            .edges
            .push((Predicate::Literal('/'), boundary));
        Fragment {
            start: boundary,
            end,
        }
    }
    /// A final `/**` also matches the parent itself, never a partial basename.
    fn descendants(&mut self) -> Fragment {
        let separator = self.character(Predicate::Literal('/'));
        let tail = self.star(Predicate::AnyCharacter);
        let result = self.concat([separator, tail]);
        self.states[result.start].epsilon.push(result.end);
        result
    }
    fn path(
        &mut self,
        segments: &[usize],
        nodes: &[Node],
        fragments: &[Option<Fragment>],
    ) -> Fragment {
        let mut distinct = Vec::new();
        for &segment in segments {
            if matches!(nodes[segment], Node::GlobStar)
                && distinct
                    .last()
                    .is_some_and(|previous| matches!(nodes[*previous], Node::GlobStar))
            {
                continue;
            }
            distinct.push(segment);
        }
        if distinct.len() == 1 && matches!(nodes[distinct[0]], Node::GlobStar) {
            return self.star(Predicate::AnyCharacter);
        }
        let mut result = Vec::new();
        for (index, segment) in distinct.iter().copied().enumerate() {
            if matches!(nodes[segment], Node::GlobStar) {
                if index + 1 == distinct.len() {
                    result.push(self.descendants());
                } else {
                    if index != 0 {
                        result.push(self.character(Predicate::Literal('/')));
                    }
                    result.push(self.directories());
                }
            } else {
                if index != 0 && !matches!(nodes[distinct[index - 1]], Node::GlobStar) {
                    result.push(self.character(Predicate::Literal('/')));
                }
                result.push(fragments[segment].expect("path segment"));
            }
        }
        self.concat(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_filename_pattern_families_match() {
        let cases = [
            ("Dockerfile.*", "Dockerfile.release", true),
            ("Containerfile.*", "Containerfile.debug", true),
            ("Jenkinsfile.*", "Jenkinsfile.ci", true),
            ("**/.config/git/config", ".config/git/config", true),
            (
                "**/.config/git/config",
                "/home/user/.config/git/config",
                true,
            ),
            ("**/.git/config", "C:\\work\\project\\.git\\config", true),
            ("**/.git/config", ".git/config", true),
            ("compose.*.yml", "compose.production.yml", true),
            ("compose.*.yaml", "compose.development.yaml", true),
            ("*docker*compose*.yml", "my-docker-compose.test.yml", true),
            ("*docker*compose*.yaml", "docker-compose.yaml", true),
            ("tsconfig.*.json", "tsconfig.app.json", true),
            ("jsconfig.*.json", "jsconfig.editor.json", true),
            ("tsconfig-*.json", "tsconfig-node.json", true),
            ("jsconfig-*.json", "jsconfig-web.json", true),
            ("Dockerfile.*", "nested/Dockerfile.release", false),
            ("**/.git/config", "project/not.git/config", false),
            ("compose.*.yaml", "compose.yaml", false),
        ];
        for (pattern, path, expected) in cases {
            assert_eq!(
                FilenameGlob::new(pattern).unwrap().is_match(path),
                expected,
                "{pattern:?} on {path:?}"
            );
        }
    }

    #[test]
    fn globstars_respect_directory_boundaries_and_empty_paths() {
        for (pattern, path, expected) in [
            ("**/*.rs", "lib.rs", true),
            ("**/*.rs", "a/b/lib.rs", true),
            ("src/**/lib.rs", "src/lib.rs", true),
            ("src/**/lib.rs", "src/a/b/lib.rs", true),
            ("src/**/lib.rs", "src/alib.rs", false),
            ("src/**", "src", true),
            ("src/**", "src/", true),
            ("src/**", "src/a/b", true),
            ("src/**", "src-other/file", false),
            ("src/**/**", "src/a", true),
            ("**", "", true),
            ("**/**", "a/b/c", true),
            ("a**b", "a/x/b", false),
            ("***", "a/b", false),
            ("a/*/b", "a/b", false),
            ("a/*/b", "a//b", true),
        ] {
            assert_eq!(
                FilenameGlob::new(pattern).unwrap().is_match(path),
                expected,
                "{pattern:?} on {path:?}"
            );
        }
    }

    #[test]
    fn alternatives_classes_and_unicode_are_supported() {
        for (pattern, path, expected) in [
            ("**/*.{rs,toml}", "a/b/Cargo.toml", true),
            ("{**/*.rs,**/Cargo.toml}", "a/Cargo.toml", true),
            ("{foo,{bar,baz}}.txt", "baz.txt", true),
            ("test{,-debug}.rs", "test.rs", true),
            ("test{,-debug}.rs", "test-debug.rs", true),
            ("test{,-debug}.rs", "test-release.rs", false),
            ("example.[0-9]", "example.4", true),
            ("example.[!0-9]", "example.x", true),
            ("example.[^0-9]", "example.4", false),
            ("[[]id[]].rs", "[id].rs", true),
            ("[{}*,?]", "*", true),
            ("[-a]", "-", true),
            ("[a-]", "-", true),
            ("[!a]", "/", false),
            ("?.rs", "界.rs", true),
            ("?.rs", "😀.rs", true),
            ("?.rs", "ab.rs", false),
            ("?.rs", "/.rs", false),
        ] {
            assert_eq!(
                FilenameGlob::new(pattern).unwrap().is_match(path),
                expected,
                "{pattern:?} on {path:?}"
            );
        }
        let glob = FilenameGlob::new("[A-C]onfig.{JSON,YAML}").unwrap();
        assert!(!glob.is_match("config.json"));
        assert!(glob.is_match_ignore_ascii_case("config.json"));
        assert!(!FilenameGlob::new("[!a]")
            .unwrap()
            .is_match_ignore_ascii_case("A"));
    }

    #[test]
    fn malformed_and_excessive_patterns_return_precise_errors() {
        for (pattern, kind) in [
            ("a{b", FilenameGlobErrorKind::UnclosedGroup),
            ("a}b", FilenameGlobErrorKind::UnexpectedGroupEnd),
            ("a[b", FilenameGlobErrorKind::UnclosedCharacterClass),
            ("a]b", FilenameGlobErrorKind::UnexpectedCharacterClassEnd),
            ("[z-a]", FilenameGlobErrorKind::ReversedCharacterRange),
            ("[a/b]", FilenameGlobErrorKind::PathSeparatorInClass),
            ("a\\b", FilenameGlobErrorKind::BackslashInPattern),
        ] {
            assert_eq!(
                FilenameGlob::new(pattern).unwrap_err().kind,
                kind,
                "{pattern}"
            );
        }
        assert_eq!(
            FilenameGlob::new(&"a".repeat(MAX_PATTERN_BYTES + 1))
                .unwrap_err()
                .kind,
            FilenameGlobErrorKind::PatternTooLong
        );
        let nested = format!(
            "{}a{}",
            "{".repeat(MAX_GROUP_DEPTH + 1),
            "}".repeat(MAX_GROUP_DEPTH + 1)
        );
        assert_eq!(
            FilenameGlob::new(&nested).unwrap_err().kind,
            FilenameGlobErrorKind::TooManyGroups
        );
        assert_eq!(FilenameGlob::new("界[").unwrap_err().offset, 3);
    }

    #[test]
    fn combinations_remain_linear_in_compiled_size() {
        let alternatives = "{a,b}".repeat(64);
        let glob = FilenameGlob::new(&alternatives).unwrap();
        assert!(glob.states.len() < alternatives.len() * 4);
        assert!(glob.is_match(&"a".repeat(64)));
        assert!(!glob.is_match(&format!("{}c", "a".repeat(63))));
        let ambiguous = format!("{}b", "*a".repeat(100));
        let glob = FilenameGlob::new(&ambiguous).unwrap();
        assert!(!glob.is_match(&"a".repeat(1000)));
    }
}
