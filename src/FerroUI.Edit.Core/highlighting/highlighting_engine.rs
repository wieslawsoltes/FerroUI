use super::{
    HighlightedLine, HighlightedSection, HighlightingColor, HighlightingDefinition,
    HighlightingError, HighlightingSpan,
};
use std::collections::HashSet;
use std::rc::Rc;

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct HighlightingState {
    stack: Vec<(usize, usize)>,
}

#[derive(Clone)]
pub struct HighlightingEngine {
    definition: Rc<HighlightingDefinition>,
    pub operation_limit: usize,
}
enum Candidate {
    End(usize, usize),
    Span(usize, usize, usize, usize),
}

#[derive(Clone, Copy)]
enum RuleMatch {
    Unsearched,
    NotFound,
    Found(usize, usize),
}
impl Candidate {
    fn start(&self) -> usize {
        match self {
            Self::End(s, _) | Self::Span(_, _, s, _) => *s,
        }
    }
    fn end(&self) -> usize {
        match self {
            Self::End(_, e) | Self::Span(_, _, _, e) => *e,
        }
    }
}
impl HighlightingEngine {
    pub fn new(definition: Rc<HighlightingDefinition>) -> Self {
        Self {
            definition,
            operation_limit: 100_000,
        }
    }
    pub fn definition(&self) -> &Rc<HighlightingDefinition> {
        &self.definition
    }
    /// Returns the colors of active spans in inside-out order. Uncolored spans
    /// do not contribute entries, but every frame is validated against this engine.
    pub fn get_color_stack(
        &self,
        state: &HighlightingState,
    ) -> Result<Vec<HighlightingColor>, HighlightingError> {
        let mut colors = Vec::new();
        for frame in state.stack.iter().rev() {
            if let Some(color) = &self.span(*frame)?.span_color {
                colors.push(color.clone());
            }
        }
        Ok(colors)
    }
    pub fn highlight_line(
        &self,
        text: &str,
        line_number: usize,
        offset: usize,
        initial: &HighlightingState,
    ) -> Result<HighlightedLine, HighlightingError> {
        let mut state = initial.clone();
        for frame in &state.stack {
            self.span(*frame)?;
        }
        let mut sections = Vec::new();
        let mut position = 0;
        let mut iterations = 0;
        let mut utf16 = Vec::with_capacity(text.chars().count() + 1);
        let mut units = 0;
        for (byte, ch) in text.char_indices() {
            utf16.push((byte, units));
            units += ch.len_utf16();
        }
        utf16.push((text.len(), units));
        let mut visited = HashSet::new();
        while position <= text.len() {
            iterations += 1;
            if iterations > self.operation_limit {
                return Err(HighlightingError::MatchBudget(
                    "line operation count".into(),
                ));
            }
            if !visited.insert((position, state.clone())) {
                return Err(HighlightingError::MatchBudget(
                    "zero-width span cycle".into(),
                ));
            }
            let current = state.stack.last().map(|f| self.span(*f)).transpose()?;
            let ambient = self.ambient(&state)?;
            let mut candidate = if let Some(span) = current {
                span.end_expression
                    .find_from_pos(text, position)
                    .map_err(|e| HighlightingError::MatchBudget(e.to_string()))?
                    .map(|m| Candidate::End(m.start(), m.end()))
            } else {
                None
            };
            let ruleset = if let Some(span) = current {
                span.rule_set
            } else {
                Some(self.definition.main_rule_set)
            };
            let mut scanned = HashSet::new();
            if let Some(ruleset) = ruleset {
                self.find_candidates(text, position, ruleset, &mut scanned, &mut candidate)?;
            }
            let boundary = candidate.as_ref().map_or(text.len(), |c| c.start());
            if let Some(ruleset) = ruleset {
                self.highlight_non_spans(
                    text,
                    position..boundary,
                    ruleset,
                    &utf16,
                    offset,
                    ambient.clone(),
                    &mut sections,
                    &mut iterations,
                )?;
            } else {
                self.push(
                    &mut sections,
                    &utf16,
                    position,
                    boundary,
                    offset,
                    ambient.clone(),
                );
            }
            let Some(candidate) = candidate else {
                break;
            };
            let end = candidate.end();
            match candidate {
                Candidate::End(start, end) => {
                    let span = current.expect("end candidate requires open span");
                    self.push(
                        &mut sections,
                        &utf16,
                        start,
                        end,
                        offset,
                        overlay(ambient, span.end_color.as_ref()),
                    );
                    state.stack.pop();
                }
                Candidate::Span(set, index, start, end) => {
                    let span = self.span((set, index))?;
                    let color = overlay(
                        overlay(ambient, span.span_color.as_ref()),
                        span.start_color.as_ref(),
                    );
                    self.push(&mut sections, &utf16, start, end, offset, color);
                    state.stack.push((set, index));
                }
            }
            if end > position {
                visited.clear();
            }
            position = end;
        }
        // A non-multiline outer span also closes all its nested spans.
        if let Some(index) = state
            .stack
            .iter()
            .position(|f| self.span(*f).is_ok_and(|s| !s.multiline))
        {
            state.stack.truncate(index);
        }
        Ok(HighlightedLine {
            line_number,
            sections,
            end_state: state,
        })
    }
    fn span(&self, frame: (usize, usize)) -> Result<&HighlightingSpan, HighlightingError> {
        self.definition
            .rule_sets
            .get(frame.0)
            .and_then(|set| set.spans.get(frame.1))
            .ok_or(HighlightingError::InvalidState)
    }
    fn ambient(
        &self,
        state: &HighlightingState,
    ) -> Result<Option<HighlightingColor>, HighlightingError> {
        let mut color = None;
        for frame in &state.stack {
            color = overlay(color, self.span(*frame)?.span_color.as_ref());
        }
        Ok(color)
    }
    fn find_candidates(
        &self,
        text: &str,
        position: usize,
        set: usize,
        scanned: &mut HashSet<usize>,
        candidate: &mut Option<Candidate>,
    ) -> Result<(), HighlightingError> {
        if !scanned.insert(set) {
            return Ok(());
        }
        let ruleset = self
            .definition
            .rule_sets
            .get(set)
            .ok_or(HighlightingError::InvalidState)?;
        for (index, span) in ruleset.spans.iter().enumerate() {
            if let Some(m) = span
                .start_expression
                .find_from_pos(text, position)
                .map_err(|e| HighlightingError::MatchBudget(e.to_string()))?
            {
                if candidate.as_ref().is_none_or(|c| {
                    m.start() < c.start()
                        || (m.start() == c.start() && matches!(c, Candidate::End(_, _)))
                }) {
                    *candidate = Some(Candidate::Span(set, index, m.start(), m.end()));
                }
            }
        }
        for import in &ruleset.imports {
            self.find_candidates(text, position, *import, scanned, candidate)?;
        }
        Ok(())
    }
    fn collect_rules(
        &self,
        set: usize,
        scanned: &mut HashSet<usize>,
        rules: &mut Vec<(usize, usize)>,
    ) -> Result<(), HighlightingError> {
        if !scanned.insert(set) {
            return Ok(());
        }
        let ruleset = self
            .definition
            .rule_sets
            .get(set)
            .ok_or(HighlightingError::InvalidState)?;
        rules.extend((0..ruleset.rules.len()).map(|index| (set, index)));
        for import in &ruleset.imports {
            self.collect_rules(*import, scanned, rules)?;
        }
        Ok(())
    }
    #[allow(clippy::too_many_arguments)]
    fn highlight_non_spans(
        &self,
        text: &str,
        region: std::ops::Range<usize>,
        set: usize,
        utf16: &[(usize, usize)],
        offset: usize,
        ambient: Option<HighlightingColor>,
        sections: &mut Vec<HighlightedSection>,
        iterations: &mut usize,
    ) -> Result<(), HighlightingError> {
        if region.is_empty() {
            return Ok(());
        }
        let mut rules = Vec::new();
        self.collect_rules(set, &mut HashSet::new(), &mut rules)?;
        // An evaluated miss remains cached until the next span boundary. A successful match is
        // recomputed only once another rule has advanced beyond its start.
        let mut matches = vec![RuleMatch::Unsearched; rules.len()];
        let mut position = region.start;
        loop {
            let mut first: Option<(usize, usize, usize)> = None;
            for (index, (set, rule)) in rules.iter().enumerate() {
                if matches!(matches[index], RuleMatch::Unsearched)
                    || matches!(matches[index], RuleMatch::Found(start, _) if start < position)
                {
                    // Regex.Match(input, beginning, length) in upstream hides
                    // both sides of the region from anchors and lookarounds.
                    // Matching the whole line and truncating a successful
                    // match would accept expressions whose suffix cannot fit.
                    matches[index] = self.definition.rule_sets[*set].rules[*rule]
                        .regex
                        .find(&text[position..region.end])
                        .map_err(|e| HighlightingError::MatchBudget(e.to_string()))?
                        .map_or(RuleMatch::NotFound, |m| {
                            RuleMatch::Found(position + m.start(), position + m.end())
                        });
                }
                if let RuleMatch::Found(start, end) = matches[index] {
                    if first.is_none_or(|(_, first_start, _)| start < first_start) {
                        first = Some((index, start, end));
                    }
                }
            }
            let Some((index, start, end)) = first else {
                self.push(sections, utf16, position, region.end, offset, ambient);
                return Ok(());
            };
            let (set, rule) = rules[index];
            let rule = &self.definition.rule_sets[set].rules[rule];
            if start == end {
                return Err(HighlightingError::InvalidDefinition(format!(
                    "A highlighting rule matched 0 characters: {}",
                    rule.regex.as_str()
                )));
            }
            *iterations += 1;
            if *iterations > self.operation_limit {
                return Err(HighlightingError::MatchBudget(
                    "line operation count".into(),
                ));
            }
            self.push(sections, utf16, position, start, offset, ambient.clone());
            self.push(
                sections,
                utf16,
                start,
                end,
                offset,
                overlay(ambient.clone(), rule.color.as_ref()),
            );
            position = end;
        }
    }
    fn push(
        &self,
        sections: &mut Vec<HighlightedSection>,
        positions: &[(usize, usize)],
        start: usize,
        end: usize,
        offset: usize,
        color: Option<HighlightingColor>,
    ) {
        let Some(color) = color else {
            return;
        };
        if start >= end {
            return;
        }
        let start16 = positions[positions
            .binary_search_by_key(&start, |p| p.0)
            .expect("regex matches UTF-8 boundaries")]
        .1;
        let end16 = positions[positions
            .binary_search_by_key(&end, |p| p.0)
            .expect("regex matches UTF-8 boundaries")]
        .1;
        let offset = offset + start16;
        let length = end16 - start16;
        if let Some(last) = sections.last_mut() {
            if last.offset + last.length == offset && last.color == color {
                last.length += length;
                return;
            }
        }
        sections.push(HighlightedSection {
            offset,
            length,
            color,
        });
    }
}
fn overlay(
    base: Option<HighlightingColor>,
    overlay: Option<&HighlightingColor>,
) -> Option<HighlightingColor> {
    match (base, overlay) {
        (Some(base), Some(overlay)) => Some(base.merge_with(overlay)),
        (None, Some(overlay)) => Some(overlay.clone()),
        (base, None) => base,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const DEFINITION: &str = r##"<SyntaxDefinition name="sample"><Color name="Keyword" foreground="#ff0000"/><Color name="Comment" foreground="Green"/><RuleSet><Keywords color="Keyword"><Word>let</Word></Keywords><Span color="Comment" begin="/\*" end="\*/" multiline="true"/><Rule foreground="Blue">\b\d+(?=;)</Rule></RuleSet></SyntaxDefinition>"##;
    #[test]
    fn multiline_state_lookahead_and_utf16_offsets() {
        let definition = Rc::new(HighlightingDefinition::load(DEFINITION).unwrap());
        let engine = HighlightingEngine::new(definition);
        let first = engine
            .highlight_line("let 😀 = 42; /* hi", 1, 0, &HighlightingState::default())
            .unwrap();
        assert_eq!(first.sections[0].length, 3);
        assert_eq!(first.sections[1].offset, 9);
        let second = engine
            .highlight_line("end */ let", 2, 20, &first.end_state)
            .unwrap();
        assert_eq!(second.sections[0].offset, 20);
        assert_eq!(second.sections[0].length, 6);
        assert_eq!(second.end_state, HighlightingState::default());
    }
    #[test]
    fn zero_width_recursive_span_is_bounded() {
        let definition=Rc::new(HighlightingDefinition::load(r#"<SyntaxDefinition><RuleSet name="recursive"><Span begin="" end="z" ruleSet="recursive"/></RuleSet><RuleSet><Import ruleSet="recursive"/></RuleSet></SyntaxDefinition>"#).unwrap());
        let mut engine = HighlightingEngine::new(definition);
        engine.operation_limit = 32;
        assert!(
            engine
                .highlight_line("x", 1, 0, &HighlightingState::default())
                .is_err()
        );
    }
    #[test]
    fn greedy_rule_cannot_consume_span_boundaries() {
        let definition=Rc::new(HighlightingDefinition::load(r#"<SyntaxDefinition><RuleSet><Rule foreground="Red">.+</Rule><Span begin="/\*" end="\*/" foreground="Green" multiline="true"><RuleSet><Rule foreground="Blue">.+</Rule></RuleSet></Span></RuleSet></SyntaxDefinition>"#).unwrap());
        let engine = HighlightingEngine::new(definition);
        let line = engine
            .highlight_line("a /* b */ c", 1, 0, &HighlightingState::default())
            .unwrap();
        assert_eq!(line.end_state, HighlightingState::default());
        assert_eq!(
            line.sections
                .iter()
                .map(|s| (s.offset, s.length, s.color.foreground.as_deref().unwrap()))
                .collect::<Vec<_>>(),
            vec![
                (0, 2, "Red"),
                (2, 2, "Green"),
                (4, 3, "Blue"),
                (7, 2, "Green"),
                (9, 2, "Red")
            ]
        );
    }

    #[test]
    fn required_rule_suffixes_cannot_cross_a_span_boundary() {
        let definition = Rc::new(HighlightingDefinition::load(
            r#"<SyntaxDefinition><RuleSet><Rule foreground="Red">a.*z</Rule><Span begin="\[" end="\]" foreground="Green"><RuleSet><Rule foreground="Blue">x.*\]</Rule></RuleSet></Span></RuleSet></SyntaxDefinition>"#,
        ).unwrap());
        let line = HighlightingEngine::new(definition)
            .highlight_line("a[x]z", 1, 0, &HighlightingState::default())
            .unwrap();
        assert_eq!(line.sections.len(), 1);
        assert_eq!((line.sections[0].offset, line.sections[0].length), (1, 3));
        assert_eq!(line.sections[0].color.foreground.as_deref(), Some("Green"));
    }

    #[test]
    fn region_anchors_and_lookarounds_use_both_boundaries_and_utf16_offsets() {
        let definition = Rc::new(HighlightingDefinition::load(
            r#"<SyntaxDefinition><RuleSet><Rule foreground="Red">a(?=\[)|(?&lt;=\])b</Rule><Rule foreground="Blue">^b$</Rule><Span begin="\[" end="\]" foreground="Green"><RuleSet><Rule foreground="Red">(?&lt;=\[)x|x(?=\])</Rule><Rule foreground="Blue">^x$</Rule></RuleSet></Span></RuleSet></SyntaxDefinition>"#,
        ).unwrap());
        let line = HighlightingEngine::new(definition)
            .highlight_line("😀a[x]b[y]", 1, 10, &HighlightingState::default())
            .unwrap();
        assert_eq!(
            line.sections
                .iter()
                .map(|section| (
                    section.offset,
                    section.length,
                    section.color.foreground.as_deref().unwrap()
                ))
                .collect::<Vec<_>>(),
            [
                (13, 1, "Green"),
                (14, 1, "Blue"),
                (15, 1, "Green"),
                (16, 1, "Blue"),
                (17, 3, "Green")
            ]
        );
    }

    #[test]
    fn bounded_region_caches_failed_rules_and_future_matches() {
        let definition = Rc::new(HighlightingDefinition::load(
            r#"<SyntaxDefinition><RuleSet><Rule foreground="Red">a</Rule><Rule foreground="Blue">^b</Rule><Rule foreground="Green">(?&lt;=a)b</Rule></RuleSet></SyntaxDefinition>"#,
        ).unwrap());
        let line = HighlightingEngine::new(definition)
            .highlight_line("ab", 1, 0, &HighlightingState::default())
            .unwrap();
        assert_eq!(line.sections.len(), 2);
        assert_eq!(line.sections[0].color.foreground.as_deref(), Some("Red"));
        assert_eq!(line.sections[1].color.foreground.as_deref(), Some("Green"));
    }

    #[test]
    fn zero_length_rules_are_errors_in_nonempty_regions_including_the_end() {
        for pattern in ["", "$", "a|$", "(?=b)"] {
            let definition = Rc::new(HighlightingDefinition::load(&format!(
                r#"<SyntaxDefinition><RuleSet><Rule foreground="Red">{pattern}</Rule></RuleSet></SyntaxDefinition>"#,
            )).unwrap());
            let engine = HighlightingEngine::new(definition);
            assert!(
                matches!(
                    engine.highlight_line("ab", 1, 0, &HighlightingState::default()),
                    Err(HighlightingError::InvalidDefinition(message)) if message.contains("0 characters")
                ),
                "{pattern}"
            );
            // Upstream does not evaluate any rules for an initially empty region.
            assert!(
                engine
                    .highlight_line("", 1, 0, &HighlightingState::default())
                    .unwrap()
                    .sections
                    .is_empty()
            );
        }
    }
}
