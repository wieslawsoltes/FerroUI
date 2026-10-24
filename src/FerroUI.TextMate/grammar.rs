use crate::regex_source::{resolve_back_references, resolve_scope_name, Captures, RegexCache};
use crate::{Error, RawGrammar, RawRule, RegexBackend, Result, ScopeSelector, SelectorPriority};
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap, HashSet},
    rc::Rc,
};

type RuleId = usize;

#[derive(Clone, Debug, Default)]
pub(crate) struct Node {
    owner: RuleId,
    name: Option<String>,
    content_name: Option<String>,
    match_pattern: Option<String>,
    begin: Option<String>,
    end: Option<String>,
    while_pattern: Option<String>,
    include: Option<String>,
    patterns: Vec<RuleId>,
    repository: Rc<BTreeMap<String, RuleId>>,
    captures: Vec<(usize, RuleId)>,
    begin_captures: Vec<(usize, RuleId)>,
    end_captures: Vec<(usize, RuleId)>,
    while_captures: Vec<(usize, RuleId)>,
    end_last: bool,
    disabled: bool,
}

pub(crate) struct Program {
    nodes: Vec<Node>,
    roots: BTreeMap<String, RuleId>,
    injections: BTreeMap<RuleId, Vec<(ScopeSelector, RuleId)>>,
    cache: RegexCache,
}

impl Program {
    pub fn compile(
        grammars: &BTreeMap<String, RawGrammar>,
        injection_targets: &BTreeMap<String, Vec<String>>,
    ) -> Result<Self> {
        let mut result = Self {
            nodes: Vec::new(),
            roots: BTreeMap::new(),
            injections: BTreeMap::new(),
            cache: RegexCache::default(),
        };
        for scope in grammars.keys() {
            let id = result.reserve();
            result.roots.insert(scope.clone(), id);
        }
        for (scope, grammar) in grammars {
            let root = result.roots[scope];
            let raw = RawRule {
                name: Some(scope.clone()),
                patterns: grammar.patterns.clone(),
                repository: grammar.repository.clone(),
                ..RawRule::default()
            };
            result.fill(root, &raw, root, Rc::new(BTreeMap::new()));
            let repository = result.nodes[root].repository.clone();
            let mut injections = Vec::new();
            for (selector, raw) in &grammar.injections {
                let id = result.reserve();
                result.fill(id, raw, root, repository.clone());
                for selector in ScopeSelector::parse_many(selector)? {
                    injections.push((selector, id));
                }
            }
            result.injections.insert(root, injections);
        }
        // Registered injection grammars extend every host, according to their selector.
        let external: Vec<_> = grammars
            .iter()
            .filter_map(|(scope, grammar)| {
                grammar
                    .injection_selector
                    .as_ref()
                    .map(|s| (scope.clone(), s.clone(), result.roots[scope]))
            })
            .collect();
        for (host, root) in &result.roots {
            let list = result
                .injections
                .get_mut(root)
                .expect("registered root injections");
            for (scope, selector, rule) in &external {
                if let Some(targets) = injection_targets.get(host) {
                    if !targets.contains(scope) {
                        continue;
                    }
                }
                for selector in ScopeSelector::parse_many(selector)? {
                    list.push((selector, *rule));
                }
            }
            list.sort_by_key(|(selector, _)| selector.priority);
        }
        Ok(result)
    }
    fn reserve(&mut self) -> RuleId {
        let id = self.nodes.len();
        self.nodes.push(Node::default());
        id
    }
    fn fill(
        &mut self,
        id: RuleId,
        raw: &RawRule,
        owner: RuleId,
        inherited: Rc<BTreeMap<String, RuleId>>,
    ) {
        let repository = if raw.repository.is_empty() {
            inherited
        } else {
            let mut repository = (*inherited).clone();
            for name in raw.repository.keys() {
                repository.insert(name.clone(), self.reserve());
            }
            Rc::new(repository)
        };
        for (name, rule) in &raw.repository {
            self.fill(repository[name], rule, owner, repository.clone());
        }
        let mut patterns = Vec::new();
        for rule in &raw.patterns {
            let child = self.reserve();
            self.fill(child, rule, owner, repository.clone());
            patterns.push(child);
        }
        let mut captures = |source: &BTreeMap<String, RawRule>| {
            let mut result = Vec::new();
            for (index, rule) in source {
                if let Ok(index) = index.parse::<usize>() {
                    let child = self.reserve();
                    self.fill(child, rule, owner, repository.clone());
                    result.push((index, child));
                }
            }
            result.sort_by_key(|(index, _)| *index);
            result
        };
        let match_captures = captures(&raw.captures);
        let begin_captures = if raw.begin_captures.is_empty() {
            match_captures.clone()
        } else {
            captures(&raw.begin_captures)
        };
        let end_captures = if raw.end_captures.is_empty() {
            match_captures.clone()
        } else {
            captures(&raw.end_captures)
        };
        let while_captures = if raw.while_captures.is_empty() {
            match_captures.clone()
        } else {
            captures(&raw.while_captures)
        };
        self.nodes[id] = Node {
            owner,
            name: raw.name.clone(),
            content_name: raw.content_name.clone(),
            match_pattern: raw.match_pattern.clone(),
            begin: raw.begin.clone(),
            end: raw.end.clone(),
            while_pattern: raw.while_pattern.clone(),
            include: raw.include.clone(),
            patterns,
            repository,
            captures: match_captures,
            begin_captures,
            end_captures,
            while_captures,
            end_last: raw.apply_end_pattern_last,
            disabled: raw.disabled.as_ref().is_some_and(|v| {
                v.as_bool()
                    .unwrap_or_else(|| v.as_i64().is_some_and(|n| n != 0))
            }),
        };
    }
    fn collect(
        &self,
        id: RuleId,
        base: RuleId,
        output: &mut Vec<RuleId>,
        visiting: &mut HashSet<RuleId>,
    ) -> Result<()> {
        let node = &self.nodes[id];
        if node.disabled || !visiting.insert(id) {
            return Ok(());
        }
        if let Some(include) = &node.include {
            let target = if include == "$self" {
                Some(node.owner)
            } else if include == "$base" {
                Some(base)
            } else if let Some(name) = include.strip_prefix('#') {
                node.repository.get(name).copied()
            } else {
                let (scope, name) = include
                    .split_once('#')
                    .map_or((include.as_str(), None), |(scope, name)| {
                        (scope, Some(name))
                    });
                let root = self.roots.get(scope).copied();
                if let (Some(root), Some(name)) = (root, name) {
                    self.nodes[root].repository.get(name).copied()
                } else {
                    root
                }
            };
            // An absent include is optional upstream. It is observable through unresolved_includes.
            if let Some(target) = target {
                self.collect(target, base, output, visiting)?;
            }
        } else if node.match_pattern.is_some() || node.begin.is_some() {
            output.push(id);
        } else {
            for child in &node.patterns {
                self.collect(*child, base, output, visiting)?;
            }
        }
        visiting.remove(&id);
        Ok(())
    }
}

#[derive(Clone, Debug)]
struct Frame {
    rule: RuleId,
    name_scopes: Rc<Vec<String>>,
    content_scopes: Rc<Vec<String>>,
    end: Option<String>,
    while_pattern: Option<String>,
    begin_at_eol: bool,
    parent_anchor: Option<usize>,
}
impl PartialEq for Frame {
    fn eq(&self, other: &Self) -> bool {
        self.rule == other.rule
            && self.name_scopes == other.name_scopes
            && self.content_scopes == other.content_scopes
            && self.end == other.end
            && self.while_pattern == other.while_pattern
            && self.begin_at_eol == other.begin_at_eol
    }
}
impl Eq for Frame {}

/// Immutable continuation state. Equality is the incremental invalidation boundary.
#[derive(Clone, Debug)]
pub struct StateStack {
    identity: Rc<()>,
    frames: Vec<Frame>,
    first_line: bool,
}
impl PartialEq for StateStack {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.identity, &other.identity)
            && self.frames == other.frames
            && self.first_line == other.first_line
    }
}
impl Eq for StateStack {}
impl StateStack {
    pub fn depth(&self) -> usize {
        self.frames.len().saturating_sub(1)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Token {
    pub start_index: usize,
    pub end_index: usize,
    pub scopes: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct TokenizeLineResult {
    pub tokens: Vec<Token>,
    pub rule_stack: StateStack,
    pub stopped_early: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct TokenizeOptions {
    /// Maximum regex searches and scanner transitions for one line.
    pub max_steps: usize,
    pub max_stack_depth: usize,
    pub max_capture_depth: usize,
}
impl Default for TokenizeOptions {
    fn default() -> Self {
        Self {
            max_steps: 100_000,
            max_stack_depth: 256,
            max_capture_depth: 32,
        }
    }
}

/// Compiled grammar and shared regex cache; clone to share without recompilation.
#[derive(Clone)]
pub struct Grammar {
    program: Rc<Program>,
    root: RuleId,
    identity: Rc<()>,
    scope_name: String,
    flattened: Rc<RefCell<HashMap<(RuleId, bool), Rc<Vec<RuleId>>>>>,
}
impl std::fmt::Debug for Grammar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Grammar")
            .field("scope_name", &self.scope_name)
            .field("backend", &self.backend())
            .finish()
    }
}
impl Grammar {
    pub(crate) fn new(program: Rc<Program>, scope: &str) -> Result<Self> {
        let root = program
            .roots
            .get(scope)
            .copied()
            .ok_or_else(|| Error::MissingGrammar(scope.into()))?;
        Ok(Self {
            program,
            root,
            identity: Rc::new(()),
            scope_name: scope.into(),
            flattened: Rc::new(RefCell::new(HashMap::new())),
        })
    }
    pub fn scope_name(&self) -> &str {
        &self.scope_name
    }
    pub fn backend(&self) -> RegexBackend {
        RegexCache::backend()
    }
    fn expanded(&self, id: RuleId, include_self: bool) -> Result<Rc<Vec<RuleId>>> {
        if let Some(value) = self.flattened.borrow().get(&(id, include_self)) {
            return Ok(value.clone());
        }
        let mut result = Vec::new();
        if include_self {
            self.program
                .collect(id, self.root, &mut result, &mut HashSet::new())?;
        } else {
            for child in &self.program.nodes[id].patterns {
                self.program
                    .collect(*child, self.root, &mut result, &mut HashSet::new())?;
            }
        }
        let result = Rc::new(result);
        self.flattened
            .borrow_mut()
            .insert((id, include_self), result.clone());
        Ok(result)
    }
    pub fn initial_state(&self) -> StateStack {
        let scopes = Rc::new(vec![self.scope_name.clone()]);
        StateStack {
            identity: self.identity.clone(),
            frames: vec![Frame {
                rule: self.root,
                name_scopes: scopes.clone(),
                content_scopes: scopes,
                end: None,
                while_pattern: None,
                begin_at_eol: false,
                parent_anchor: None,
            }],
            first_line: true,
        }
    }
    pub fn tokenize_line(
        &self,
        line: &str,
        previous: Option<&StateStack>,
    ) -> Result<TokenizeLineResult> {
        self.tokenize_line_with_options(line, previous, TokenizeOptions::default(), None)
    }
    pub fn tokenize_line_with_options(
        &self,
        line: &str,
        previous: Option<&StateStack>,
        options: TokenizeOptions,
        cancelled: Option<&dyn Fn() -> bool>,
    ) -> Result<TokenizeLineResult> {
        let mut state = previous.cloned().unwrap_or_else(|| self.initial_state());
        if !Rc::ptr_eq(&state.identity, &self.identity) {
            return Err(Error::InvalidState);
        }
        for frame in &mut state.frames {
            frame.parent_anchor = None;
        }
        let text = format!("{line}\n");
        let mut work = Work {
            remaining: options.max_steps,
            stopped: false,
            options,
            cancelled,
        };
        let mut byte_tokens = Vec::new();
        let mut cursor = 0;
        let mut anchor = if state.frames.last().is_some_and(|f| f.begin_at_eol) {
            Some(0)
        } else {
            None
        };
        // While conditions are checked outside-in, before ordinary matching.
        let mut index = 1;
        while index < state.frames.len() {
            let frame = state.frames[index].clone();
            if let Some(pattern) = &frame.while_pattern {
                if !work.step() {
                    break;
                }
                let captures = self.program.cache.search(
                    pattern,
                    &text,
                    cursor,
                    state.first_line,
                    anchor == Some(cursor),
                )?;
                let Some(captures) = captures.filter(|c| {
                    c.first()
                        .copied()
                        .flatten()
                        .is_some_and(|(start, _)| start == cursor)
                }) else {
                    state.frames.truncate(index);
                    break;
                };
                let (start, end) = captures[0].expect("a regex result has capture zero");
                self.emit_captures(
                    &text,
                    start,
                    end,
                    &frame.name_scopes,
                    &self.program.nodes[frame.rule].while_captures,
                    &captures,
                    state.first_line,
                    &mut work,
                    0,
                    &mut byte_tokens,
                )?;
                cursor = end;
                anchor = Some(end);
            }
            index += 1;
        }
        self.scan(
            &text,
            text.len(),
            cursor,
            &mut state.frames,
            state.first_line,
            anchor,
            &mut work,
            0,
            &mut byte_tokens,
        )?;
        state.first_line = false;
        // Translate byte positions once, preserving astral characters' two code units.
        let mut offsets = vec![0; text.len() + 1];
        let mut utf16 = 0;
        for (byte, ch) in text.char_indices() {
            for value in &mut offsets[byte..byte + ch.len_utf8()] {
                *value = utf16;
            }
            utf16 += ch.len_utf16();
            offsets[byte + ch.len_utf8()] = utf16;
        }
        let mut tokens = Vec::<Token>::new();
        for token in byte_tokens {
            let start = token.start.min(line.len());
            let end = token.end.min(line.len());
            if start >= end {
                continue;
            }
            let token = Token {
                start_index: offsets[start],
                end_index: offsets[end],
                scopes: token.scopes,
            };
            if let Some(previous) = tokens
                .last_mut()
                .filter(|p| p.end_index == token.start_index && p.scopes == token.scopes)
            {
                previous.end_index = token.end_index;
            } else {
                tokens.push(token);
            }
        }
        if tokens.is_empty() {
            tokens.push(Token {
                start_index: 0,
                end_index: line.encode_utf16().count(),
                scopes: state
                    .frames
                    .last()
                    .map(|f| (*f.content_scopes).clone())
                    .unwrap_or_default(),
            });
        }
        Ok(TokenizeLineResult {
            tokens,
            rule_stack: state,
            stopped_early: work.stopped,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn scan(
        &self,
        text: &str,
        limit: usize,
        mut cursor: usize,
        frames: &mut Vec<Frame>,
        first_line: bool,
        mut anchor: Option<usize>,
        work: &mut Work<'_>,
        capture_depth: usize,
        output: &mut Vec<ByteToken>,
    ) -> Result<()> {
        let mut empty_states = HashSet::new();
        while cursor <= limit && work.step() {
            let frame = frames.last().expect("root frame").clone();
            let node = &self.program.nodes[frame.rule];
            let candidates = self.expanded(frame.rule, false)?;
            let mut best: Option<Match> = None;
            if let Some(pattern) = &frame.end {
                if work.step() {
                    if let Some(captures) = self.program.cache.search(
                        pattern,
                        text,
                        cursor,
                        first_line,
                        anchor == Some(cursor),
                    )? {
                        if let Some((start, end)) = captures[0].filter(|(_, end)| *end <= limit) {
                            best = Some(Match {
                                rule: frame.rule,
                                captures,
                                start,
                                end,
                                is_end: true,
                                precedence: if node.end_last { usize::MAX } else { 1 },
                            });
                        }
                    }
                }
            }
            let mut precedence = 2;
            for &rule in candidates.iter() {
                if !work.step() {
                    break;
                }
                self.consider(
                    rule, precedence, text, limit, cursor, first_line, anchor, &mut best,
                )?;
                precedence += 1;
            }
            if let Some(injections) = self.program.injections.get(&self.root) {
                for (selector, rule) in injections {
                    if !selector.matches(frame.content_scopes.as_slice()) {
                        continue;
                    }
                    let rules = self.expanded(*rule, true)?;
                    for &rule in rules.iter() {
                        if !work.step() {
                            break;
                        }
                        let rank = if selector.priority == SelectorPriority::Left {
                            0
                        } else {
                            precedence
                        };
                        self.consider(
                            rule, rank, text, limit, cursor, first_line, anchor, &mut best,
                        )?;
                        precedence += 1;
                    }
                }
            }
            if work.stopped {
                break;
            }
            let Some(matched) = best else {
                emit(output, cursor, limit, &frame.content_scopes);
                return Ok(());
            };
            emit(output, cursor, matched.start, &frame.content_scopes);
            let rule = &self.program.nodes[matched.rule];
            if matched.end == cursor {
                let key = (
                    cursor,
                    frames.iter().map(|f| f.rule).collect::<Vec<_>>(),
                    matched.rule,
                    matched.is_end,
                );
                if !empty_states.insert(key) {
                    emit(output, cursor, limit, &frame.content_scopes);
                    return Ok(());
                }
            } else {
                empty_states.clear();
            }
            if matched.is_end {
                self.emit_captures(
                    text,
                    matched.start,
                    matched.end,
                    &frame.name_scopes,
                    &rule.end_captures,
                    &matched.captures,
                    first_line,
                    work,
                    capture_depth,
                    output,
                )?;
                if frames.len() > 1 {
                    frames.pop();
                }
                anchor = frame.parent_anchor;
            } else {
                let names = add_scopes(
                    &frame.content_scopes,
                    rule.name.as_deref(),
                    text,
                    &matched.captures,
                );
                let is_begin = rule.begin.is_some();
                self.emit_captures(
                    text,
                    matched.start,
                    matched.end,
                    &names,
                    if is_begin {
                        &rule.begin_captures
                    } else {
                        &rule.captures
                    },
                    &matched.captures,
                    first_line,
                    work,
                    capture_depth,
                    output,
                )?;
                if is_begin {
                    if frames.len() >= work.options.max_stack_depth {
                        work.stopped = true;
                        cursor = matched.end;
                        break;
                    }
                    let content = add_scopes(
                        &names,
                        rule.content_name.as_deref(),
                        text,
                        &matched.captures,
                    );
                    frames.push(Frame {
                        rule: matched.rule,
                        name_scopes: Rc::new(names),
                        content_scopes: Rc::new(content),
                        end: rule
                            .end
                            .as_ref()
                            .map(|p| resolve_back_references(p, text, &matched.captures)),
                        while_pattern: rule
                            .while_pattern
                            .as_ref()
                            .map(|p| resolve_back_references(p, text, &matched.captures)),
                        begin_at_eol: matched.end == limit,
                        parent_anchor: anchor,
                    });
                    anchor = Some(matched.end);
                } else if matched.start == matched.end {
                    emit(output, matched.end, limit, &frame.content_scopes);
                    return Ok(());
                }
            }
            cursor = matched.end;
        }
        if let Some(frame) = frames.last() {
            emit(output, cursor, limit, &frame.content_scopes);
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn consider(
        &self,
        id: RuleId,
        precedence: usize,
        text: &str,
        limit: usize,
        cursor: usize,
        first_line: bool,
        anchor: Option<usize>,
        best: &mut Option<Match>,
    ) -> Result<()> {
        let rule = &self.program.nodes[id];
        let Some(pattern) = rule.match_pattern.as_ref().or(rule.begin.as_ref()) else {
            return Ok(());
        };
        if let Some(captures) =
            self.program
                .cache
                .search(pattern, text, cursor, first_line, anchor == Some(cursor))?
        {
            if let Some((start, end)) = captures[0].filter(|(_, end)| *end <= limit) {
                if best
                    .as_ref()
                    .is_none_or(|best| (start, precedence) < (best.start, best.precedence))
                {
                    *best = Some(Match {
                        rule: id,
                        captures,
                        start,
                        end,
                        is_end: false,
                        precedence,
                    });
                }
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_captures(
        &self,
        text: &str,
        start: usize,
        end: usize,
        base: &[String],
        rules: &[(usize, RuleId)],
        captures: &Captures,
        first_line: bool,
        work: &mut Work<'_>,
        depth: usize,
        output: &mut Vec<ByteToken>,
    ) -> Result<()> {
        if start >= end {
            return Ok(());
        }
        let mut spans = Vec::new();
        let mut boundaries = vec![start, end];
        for (index, rule) in rules {
            if let Some(Some((a, b))) = captures.get(*index) {
                let a = (*a).max(start);
                let b = (*b).min(end);
                if a < b {
                    spans.push((a, b, *index, *rule));
                    boundaries.push(a);
                    boundaries.push(b);
                }
            }
        }
        spans.sort_by_key(|(a, b, index, _)| (*a, std::cmp::Reverse(*b), *index));
        let mut inner = Vec::<ByteToken>::new();
        if depth < work.options.max_capture_depth {
            for (a, b, _, rule_id) in &spans {
                let rule = &self.program.nodes[*rule_id];
                if rule.patterns.is_empty() {
                    continue;
                }
                let mut scopes = base.to_vec();
                for (outer_a, outer_b, _, outer_id) in &spans {
                    if outer_a <= a && outer_b >= b {
                        scopes = add_scopes(
                            &scopes,
                            self.program.nodes[*outer_id].name.as_deref(),
                            text,
                            captures,
                        );
                    }
                    if outer_id == rule_id {
                        break;
                    }
                }
                scopes = add_scopes(&scopes, rule.content_name.as_deref(), text, captures);
                let scopes = Rc::new(scopes);
                let mut frames = vec![Frame {
                    rule: *rule_id,
                    name_scopes: scopes.clone(),
                    content_scopes: scopes,
                    end: None,
                    while_pattern: None,
                    begin_at_eol: false,
                    parent_anchor: None,
                }];
                let begin = inner.len();
                self.scan(
                    &text[..*b],
                    *b,
                    *a,
                    &mut frames,
                    first_line,
                    Some(*a),
                    work,
                    depth + 1,
                    &mut inner,
                )?;
                for token in &inner[begin..] {
                    boundaries.push(token.start);
                    boundaries.push(token.end);
                }
            }
        } else if spans
            .iter()
            .any(|(_, _, _, id)| !self.program.nodes[*id].patterns.is_empty())
        {
            work.stopped = true;
        }
        boundaries.sort_unstable();
        boundaries.dedup();
        for pair in boundaries.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            let mut scopes = base.to_vec();
            for (ca, cb, _, id) in &spans {
                if *ca <= a && *cb >= b {
                    scopes = add_scopes(
                        &scopes,
                        self.program.nodes[*id].name.as_deref(),
                        text,
                        captures,
                    );
                }
            }
            if let Some(token) = inner.iter().rev().find(|t| t.start <= a && t.end >= b) {
                scopes = token.scopes.clone();
            }
            emit(output, a, b, &scopes);
        }
        Ok(())
    }

    /// External includes unavailable in this registry. They are skipped as upstream does.
    pub fn unresolved_includes(&self) -> Vec<String> {
        let mut output = std::collections::BTreeSet::new();
        for node in &self.program.nodes {
            if let Some(include) = &node.include {
                if !include.starts_with('#') && !include.starts_with('$') {
                    let scope = include
                        .split_once('#')
                        .map_or(include.as_str(), |(scope, _)| scope);
                    if !self.program.roots.contains_key(scope) {
                        output.insert(include.clone());
                    }
                }
            }
        }
        output.into_iter().collect()
    }
}

struct Match {
    rule: RuleId,
    captures: Captures,
    start: usize,
    end: usize,
    is_end: bool,
    precedence: usize,
}
#[derive(Clone)]
struct ByteToken {
    start: usize,
    end: usize,
    scopes: Vec<String>,
}
fn emit(output: &mut Vec<ByteToken>, start: usize, end: usize, scopes: &[String]) {
    if start >= end {
        return;
    }
    if let Some(previous) = output
        .last_mut()
        .filter(|p| p.end == start && p.scopes == scopes)
    {
        previous.end = end;
    } else {
        output.push(ByteToken {
            start,
            end,
            scopes: scopes.to_vec(),
        });
    }
}
fn add_scopes(base: &[String], name: Option<&str>, text: &str, captures: &Captures) -> Vec<String> {
    let mut output = base.to_vec();
    if let Some(name) = name {
        output.extend(
            resolve_scope_name(name, text, captures)
                .split_whitespace()
                .map(str::to_owned),
        );
    }
    output
}
struct Work<'a> {
    remaining: usize,
    stopped: bool,
    options: TokenizeOptions,
    cancelled: Option<&'a dyn Fn() -> bool>,
}
impl Work<'_> {
    fn step(&mut self) -> bool {
        if self.remaining == 0 || self.cancelled.is_some_and(|f| f()) {
            self.stopped = true;
            false
        } else {
            self.remaining -= 1;
            true
        }
    }
}
