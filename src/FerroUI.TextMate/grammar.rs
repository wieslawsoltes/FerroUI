use crate::regex_source::{
    resolve_back_references, resolve_scope_name, Captures, RegexCache, RegexInput,
};
use crate::{Error, RawGrammar, RawRule, RegexBackend, Result, ScopeSelector, SelectorPriority};
use std::{
    cell::{OnceCell, RefCell},
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
        let external: BTreeMap<_, _> = grammars
            .iter()
            .filter_map(|(scope, grammar)| {
                grammar
                    .injection_selector
                    .as_deref()
                    .map(|selector| (scope.as_str(), (selector, result.roots[scope])))
            })
            .collect();
        for (host, root) in &result.roots {
            let list = result
                .injections
                .get_mut(root)
                .expect("registered root injections");
            let append = |&(selector, rule): &(&str, RuleId)| -> Result<()> {
                for selector in ScopeSelector::parse_many(selector)? {
                    list.push((selector, rule));
                }
                Ok(())
            };
            if let Some(targets) = injection_targets.get(host) {
                targets
                    .iter()
                    .filter_map(|scope| external.get(scope.as_str()))
                    .try_for_each(append)?;
            } else {
                external.values().try_for_each(append)?;
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
        let [begin_captures, end_captures, while_captures] =
            [&raw.begin_captures, &raw.end_captures, &raw.while_captures].map(|overrides| {
                overrides
                    .as_ref()
                    .map_or_else(|| match_captures.clone(), &mut captures)
            });
        // Choose a single scanning form before storing the rule's delimiters.
        let (begin, end, while_pattern) = match (&raw.match_pattern, &raw.begin, &raw.while_pattern) {
            (None, Some(begin), Some(test)) => (Some(begin.clone()), None, Some(test.clone())),
            (None, Some(begin), None) => (Some(begin.clone()), raw.end.clone(), None),
            _ => (None, None, None),
        };
        self.nodes[id] = Node {
            owner,
            name: raw.name.clone(),
            content_name: raw.content_name.clone(),
            match_pattern: raw.match_pattern.clone(),
            begin,
            end,
            while_pattern,
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
    fn resolve_include(&self, node: &Node, include: &str, base: RuleId) -> Option<RuleId> {
        match include {
            "$self" => Some(node.owner),
            "$base" => Some(base),
            name if name.starts_with('#') => node.repository.get(&name[1..]).copied(),
            name => {
                if let Some((scope, fragment)) = name.split_once('#') {
                    let root = *self.roots.get(scope)?;
                    self.nodes[root].repository.get(fragment).copied()
                } else {
                    self.roots.get(name).copied()
                }
            }
        }
    }

    fn unavailable_patterns(&self, base: RuleId) -> Vec<bool> {
        let mut dependents = vec![Vec::new(); self.nodes.len()];
        let mut remaining = vec![0; self.nodes.len()];
        let mut pending = Vec::new();
        for (id, node) in self.nodes.iter().enumerate() {
            let target;
            let alternatives = if let Some(include) = &node.include {
                target = self.resolve_include(node, include, base);
                target.as_slice()
            } else if node.match_pattern.is_none() && !node.patterns.is_empty() {
                node.patterns.as_slice()
            } else {
                // A match, or a rule declared with no children, is complete.
                continue;
            };
            remaining[id] = alternatives.len();
            if alternatives.is_empty() {
                pending.push(id);
            }
            for &child in alternatives {
                dependents[child].push(id);
            }
        }
        // Remove a parent only when its last alternative disappears. This
        // keeps legitimate recursive rules and does not recurse into cycles.
        let mut unavailable = vec![false; self.nodes.len()];
        while let Some(id) = pending.pop() {
            unavailable[id] = true;
            for &parent in &dependents[id] {
                remaining[parent] -= 1;
                if remaining[parent] == 0 {
                    pending.push(parent);
                }
            }
        }
        unavailable
    }

    fn collect(
        &self,
        id: RuleId,
        base: RuleId,
        unavailable: &[bool],
        output: &mut Vec<RuleId>,
        visiting: &mut HashSet<RuleId>,
    ) {
        let node = &self.nodes[id];
        if node.disabled || unavailable[id] || !visiting.insert(id) {
            return;
        }
        if let Some(include) = &node.include {
            if let Some(target) = self.resolve_include(node, include, base) {
                self.collect(target, base, unavailable, output, visiting);
            }
        } else if node.match_pattern.is_some() || node.begin.is_some() {
            output.push(id);
        } else {
            for child in &node.patterns {
                self.collect(*child, base, unavailable, output, visiting);
            }
        }
        visiting.remove(&id);
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
    // These positions are local to one scan and excluded from continuation equality.
    enter_offset: Option<usize>,
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
    /// Number of active nested rules, excluding the root grammar.
    pub fn depth(&self) -> usize {
        self.frames.len().saturating_sub(1)
    }

    /// Identifies the active compiled rule within this grammar.
    /// IDs are local to a compiled program and must not be persisted.
    pub fn rule_id(&self) -> usize {
        self.frames.last().expect("root frame").rule
    }

    /// Active end expression, including resolved captures from the begin rule.
    pub fn end_rule(&self) -> Option<&str> {
        self.frames.last().and_then(|frame| frame.end.as_deref())
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
    name: Option<Rc<str>>,
    file_types: Rc<[String]>,
    unavailable_patterns: Rc<OnceCell<Vec<bool>>>,
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
    pub(crate) fn new(program: Rc<Program>, source: &RawGrammar) -> Result<Self> {
        let scope = &source.scope_name;
        let root = program
            .roots
            .get(scope)
            .copied()
            .ok_or_else(|| Error::MissingGrammar(scope.into()))?;
        Ok(Self {
            program,
            root,
            identity: Rc::new(()),
            scope_name: scope.clone(),
            name: source.name.as_deref().map(Rc::from),
            file_types: source
                .file_types
                .iter()
                .map(|extension| extension.strip_prefix('.').unwrap_or(extension).to_owned())
                .collect(),
            unavailable_patterns: Rc::new(OnceCell::new()),
            flattened: Rc::new(RefCell::new(HashMap::new())),
        })
    }
    pub fn scope_name(&self) -> &str {
        &self.scope_name
    }
    pub(crate) fn same_instance(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.identity, &other.identity)
    }
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
    /// The source extensions with one leading dot removed from each entry.
    pub fn file_types(&self) -> &[String] {
        &self.file_types
    }
    pub fn backend(&self) -> RegexBackend {
        RegexCache::backend()
    }
    fn expanded(&self, id: RuleId, include_self: bool) -> Rc<Vec<RuleId>> {
        if let Some(value) = self.flattened.borrow().get(&(id, include_self)) {
            return value.clone();
        }
        let unavailable = self
            .unavailable_patterns
            .get_or_init(|| self.program.unavailable_patterns(self.root));
        let mut result = Vec::new();
        if include_self {
            self.program
                .collect(id, self.root, unavailable, &mut result, &mut HashSet::new());
        } else {
            for child in &self.program.nodes[id].patterns {
                self.program
                    .collect(*child, self.root, unavailable, &mut result, &mut HashSet::new());
            }
        }
        let result = Rc::new(result);
        self.flattened
            .borrow_mut()
            .insert((id, include_self), result.clone());
        result
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
                enter_offset: None,
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
            frame.enter_offset = None;
            frame.parent_anchor = None;
        }
        let text = format!("{line}\n");
        let input = RegexInput::new(&text);
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
                    &input,
                    cursor,
                    state.first_line,
                    anchor == Some(cursor),
                )?;
                let Some(captures) = captures.filter(|c| c.first().is_some_and(Option::is_some)) else {
                    state.frames.truncate(index);
                    break;
                };
                let (start, end) = captures[0].expect("a regex result has capture zero");
                // An unanchored continuation may match ahead of the cursor.
                // Its content scopes cover the intervening text and the mark.
                emit(&mut byte_tokens, cursor, start, &frame.content_scopes);
                self.emit_captures(
                    &text,
                    start,
                    end,
                    &frame.content_scopes,
                    &self.program.nodes[frame.rule].while_captures,
                    &captures,
                    state.first_line,
                    &mut work,
                    0,
                    &mut byte_tokens,
                )?;
                cursor = cursor.max(end);
                anchor = Some(end);
            }
            index += 1;
        }
        self.scan(
            &input,
            cursor,
            &mut state.frames,
            state.first_line,
            anchor,
            &mut work,
            0,
            None,
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
        input: &RegexInput<'_>,
        mut cursor: usize,
        frames: &mut Vec<Frame>,
        mut first_line: bool,
        mut anchor: Option<usize>,
        work: &mut Work<'_>,
        capture_depth: usize,
        capture_base_scopes: Option<&[String]>,
        output: &mut Vec<ByteToken>,
    ) -> Result<()> {
        let text = input.text();
        let limit = text.len();
        while cursor <= limit && work.step() {
            let frame = frames.last().expect("root frame").clone();
            let node = &self.program.nodes[frame.rule];
            let mut best: Option<Match> = None;
            if let Some(pattern) = &frame.end {
                if work.step() {
                    if let Some(captures) = self.program.cache.search(
                        pattern,
                        input,
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
            if let Some(candidate) = self.match_candidates(
                frame.rule,
                false,
                input,
                cursor,
                first_line,
                anchor == Some(cursor),
                work,
            )? {
                if best.as_ref().is_none_or(|other| {
                    (candidate.start, candidate.precedence) < (other.start, other.precedence)
                }) {
                    best = Some(candidate);
                }
            }
            let mut injected: Option<(Match, SelectorPriority)> = None;
            if let Some(injections) = self.program.injections.get(&self.root) {
                for (selector, rule) in injections {
                    if !selector.matches(frame.content_scopes.as_slice()) {
                        continue;
                    }
                    // A match at the cursor wins immediately. When searching ahead,
                    // a later injection can replace one at the same offset.
                    let Some(candidate) = self.match_candidates(
                        *rule,
                        true,
                        input,
                        cursor,
                        first_line,
                        anchor == Some(cursor),
                        work,
                    )?
                    else {
                        continue;
                    };
                    if injected
                        .as_ref()
                        .is_some_and(|(matched, _)| matched.start < candidate.start)
                    {
                        continue;
                    }
                    let at_cursor = candidate.start == cursor;
                    injected = Some((candidate, selector.priority));
                    if at_cursor {
                        break;
                    }
                }
            }
            if work.stopped {
                break;
            }
            if let Some((candidate, priority)) = injected {
                if best.as_ref().is_none_or(|matched| {
                    candidate.start < matched.start
                        || (candidate.start == matched.start && priority == SelectorPriority::Left)
                }) {
                    best = Some(candidate);
                }
            }
            let Some(matched) = best else {
                emit(output, cursor, limit, &frame.content_scopes);
                return Ok(());
            };
            emit(output, cursor, matched.start, &frame.content_scopes);
            let rule = &self.program.nodes[matched.rule];
            let advanced = matched.end > cursor;
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
                if !advanced && frame.enter_offset == Some(cursor) {
                    // Keep the closing delimiter's scopes when this rule
                    // opens and closes without consuming any text.
                    let closing = frames.last_mut().expect("root frame");
                    closing.content_scopes = closing.name_scopes.clone();
                    emit(output, cursor, limit, &closing.content_scopes);
                    return Ok(());
                }
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
                    if !advanced
                        && frames
                            .iter()
                            .rev()
                            .take_while(|frame| frame.enter_offset == Some(cursor))
                            .any(|frame| frame.rule == matched.rule)
                    {
                        emit(output, cursor, limit, &frame.content_scopes);
                        return Ok(());
                    }
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
                        enter_offset: Some(cursor),
                        parent_anchor: anchor,
                    });
                    anchor = Some(matched.end);
                } else if !advanced {
                    let scopes = if frames.len() > 1 {
                        frames.pop();
                        frames.last().expect("root frame").content_scopes.as_slice()
                    } else {
                        capture_base_scopes.unwrap_or(frame.content_scopes.as_slice())
                    };
                    emit(output, cursor, limit, scopes);
                    return Ok(());
                }
            }
            if advanced {
                first_line = false;
                cursor = matched.end;
            }
        }
        if let Some(frame) = frames.last() {
            emit(output, cursor, limit, &frame.content_scopes);
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn match_candidates(
        &self,
        source: RuleId,
        injection: bool,
        input: &RegexInput<'_>,
        cursor: usize,
        first_line: bool,
        allow_g: bool,
        work: &mut Work<'_>,
    ) -> Result<Option<Match>> {
        let rules = self.expanded(source, injection);
        let precedence = |index| if injection { 0 } else { index + 2 };
        #[cfg(all(
            not(feature = "portable-regex"),
            any(target_arch = "wasm32", feature = "native-onig")
        ))]
        if work.cancelled.is_none() && rules.len() > 1 && rules.len() <= work.remaining {
            let patterns = || {
                rules.iter().map(|&rule| {
                    self.rule_pattern(rule)
                        .expect("expanded rules have a pattern")
                })
            };
            if let Some(matched) = self.program.cache.search_group(
                (self.root, source, injection),
                patterns,
                input,
                cursor,
                first_line,
                allow_g,
            ) {
                let mut steps = rules.len();
                let best = matched?.and_then(|(index, captures)| {
                    let (start, end) = captures[0].filter(|(_, end)| *end <= input.text().len())?;
                    if injection && start == cursor {
                        steps = index + 1;
                    }
                    Some(Match {
                        rule: rules[index],
                        captures,
                        start,
                        end,
                        is_end: false,
                        precedence: precedence(index),
                    })
                });
                work.remaining -= steps;
                return Ok(best);
            }
        }
        let mut best = None;
        for (index, &rule) in rules.iter().enumerate() {
            if !work.step() {
                break;
            }
            self.consider(
                rule,
                precedence(index),
                input,
                cursor,
                first_line,
                allow_g,
                &mut best,
            )?;
            if injection && best.as_ref().is_some_and(|matched| matched.start == cursor) {
                break;
            }
        }
        Ok(best)
    }

    fn rule_pattern(&self, id: RuleId) -> Option<&str> {
        let rule = &self.program.nodes[id];
        rule.match_pattern.as_deref().or(rule.begin.as_deref())
    }

    #[allow(clippy::too_many_arguments)]
    fn consider(
        &self,
        id: RuleId,
        precedence: usize,
        input: &RegexInput<'_>,
        cursor: usize,
        first_line: bool,
        allow_g: bool,
        best: &mut Option<Match>,
    ) -> Result<()> {
        let Some(pattern) = self.rule_pattern(id) else {
            return Ok(());
        };
        if let Some(captures) = self
            .program
            .cache
            .search(pattern, input, cursor, first_line, allow_g)?
        {
            if let Some((start, end)) = captures[0].filter(|(_, end)| *end <= input.text().len()) {
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
        let mut cursor = start;
        let mut active: Vec<(usize, Vec<String>)> = Vec::new();
        let mut inner = Vec::new();
        for (index, rule_id) in rules {
            let Some(Some((capture_start, capture_end))) = captures.get(*index) else {
                continue;
            };
            let capture_start = (*capture_start).max(start);
            let capture_end = (*capture_end).min(end);
            if capture_start >= capture_end {
                continue;
            }
            while active.last().is_some_and(|(end, _)| *end <= capture_start) {
                let (end, scopes) = active.pop().unwrap();
                emit_until(output, &mut cursor, end, &scopes);
            }
            let scopes = active.last().map_or(base, |(_, scopes)| scopes.as_slice());
            emit_until(output, &mut cursor, capture_start, scopes);

            let rule = &self.program.nodes[*rule_id];
            let retokenize = !rule.patterns.is_empty();
            if retokenize && capture_end <= cursor {
                continue;
            }
            if !retokenize || depth >= work.options.max_capture_depth {
                work.stopped |= retokenize;
                if rule.name.is_some() {
                    let scoped = add_scopes(scopes, rule.name.as_deref(), text, captures);
                    active.push((capture_end, scoped));
                }
                continue;
            }
            // Plain captures use a separate stack and resume after this range.
            let name = add_scopes(base, rule.name.as_deref(), text, captures);
            let content = add_scopes(&name, rule.content_name.as_deref(), text, captures);
            let mut frames = vec![Frame {
                rule: *rule_id,
                name_scopes: Rc::new(name),
                content_scopes: Rc::new(content),
                end: None,
                while_pattern: None,
                begin_at_eol: false,
                enter_offset: Some(capture_start),
                parent_anchor: None,
            }];
            // Capture scans start at the capture offset, but only a begin
            // match inside the scan establishes its \G anchor.
            self.scan(
                &RegexInput::new(&text[..capture_end]),
                capture_start,
                &mut frames,
                first_line && capture_start == 0,
                None,
                work,
                depth + 1,
                Some(base),
                &mut inner,
            )?;
            for token in inner.drain(..) {
                emit(output, cursor.max(token.start), token.end, &token.scopes);
                cursor = cursor.max(token.end);
            }
        }
        for (end, scopes) in active.into_iter().rev() {
            emit_until(output, &mut cursor, end, &scopes);
        }
        emit(output, cursor, end, base);
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
fn emit_until(output: &mut Vec<ByteToken>, cursor: &mut usize, end: usize, scopes: &[String]) {
    emit(output, *cursor, end, scopes);
    *cursor = (*cursor).max(end);
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
