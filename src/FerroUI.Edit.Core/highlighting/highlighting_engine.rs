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
    Rule(usize, usize, usize, usize),
}
impl Candidate {
    fn start(&self) -> usize {
        match self {
            Self::End(s, _) | Self::Span(_, _, s, _) | Self::Rule(_, _, s, _) => *s,
        }
    }
    fn end(&self) -> usize {
        match self {
            Self::End(_, e) | Self::Span(_, _, _, e) | Self::Rule(_, _, _, e) => *e,
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
                let boundary = candidate.as_ref().map_or(text.len(), |c| c.start());
                scanned.clear();
                self.find_rule_candidate(
                    text,
                    position,
                    ruleset,
                    boundary,
                    &mut scanned,
                    &mut candidate,
                )?;
            }
            let Some(candidate) = candidate else {
                self.push(&mut sections, &utf16, position, text.len(), offset, ambient);
                break;
            };
            self.push(
                &mut sections,
                &utf16,
                position,
                candidate.start(),
                offset,
                ambient.clone(),
            );
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
                Candidate::Rule(set, index, start, end) => {
                    let color = self.definition.rule_sets[set].rules[index].color.as_ref();
                    self.push(
                        &mut sections,
                        &utf16,
                        start,
                        end,
                        offset,
                        overlay(ambient, color),
                    );
                    if start == end {
                        if end == text.len() {
                            break;
                        }
                        let next = end + text[end..].chars().next().unwrap().len_utf8();
                        self.push(
                            &mut sections,
                            &utf16,
                            end,
                            next,
                            offset,
                            self.ambient(&state)?,
                        );
                        position = next;
                        continue;
                    }
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
    fn find_rule_candidate(
        &self,
        text: &str,
        position: usize,
        set: usize,
        boundary: usize,
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
        for (index, rule) in ruleset.rules.iter().enumerate() {
            if let Some(m) = rule
                .regex
                .find_from_pos(text, position)
                .map_err(|e| HighlightingError::MatchBudget(e.to_string()))?
            {
                if m.start() < boundary && candidate.as_ref().is_none_or(|c| m.start() < c.start())
                {
                    *candidate = Some(Candidate::Rule(
                        set,
                        index,
                        m.start(),
                        m.end().min(boundary),
                    ));
                }
            }
        }
        for import in &ruleset.imports {
            self.find_rule_candidate(text, position, *import, boundary, scanned, candidate)?;
        }
        Ok(())
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
        assert!(engine
            .highlight_line("x", 1, 0, &HighlightingState::default())
            .is_err());
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
}
