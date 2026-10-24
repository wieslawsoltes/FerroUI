use crate::{
    Color, EncodedTokenAttributes, Error, Grammar, Result, ScopeSelector, StandardTokenType,
    StateStack, Theme,
};
use std::collections::{BTreeMap, HashMap};

/// Stable theme color indices. Index zero is reserved for an absent color.
#[derive(Clone, Debug, Default)]
pub struct ColorMap {
    colors: Vec<Color>,
    indices: HashMap<Color, u16>,
}
impl ColorMap {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn get_id(&mut self, color: Option<Color>) -> Result<u16> {
        let Some(color) = color else {
            return Ok(0);
        };
        if let Some(index) = self.indices.get(&color) {
            return Ok(*index);
        }
        if self.colors.len() >= 511 {
            return Err(Error::InvalidGrammar(
                "encoded token color map exceeds 511 colors".into(),
            ));
        }
        self.colors.push(color);
        let index = self.colors.len() as u16;
        self.indices.insert(color, index);
        Ok(index)
    }
    pub fn get_color(&self, id: u16) -> Option<Color> {
        id.checked_sub(1)
            .and_then(|index| self.colors.get(index as usize))
            .copied()
    }
    pub fn colors(&self) -> &[Color] {
        &self.colors
    }
}

#[derive(Clone, Debug, Default)]
pub struct GrammarConfiguration {
    pub initial_language: u8,
    /// TextMate scope prefix -> host language ID.
    pub embedded_languages: BTreeMap<String, u8>,
    pub token_types: Vec<(ScopeSelector, StandardTokenType)>,
    pub balanced_bracket_selectors: Vec<ScopeSelector>,
    pub unbalanced_bracket_selectors: Vec<ScopeSelector>,
}

#[derive(Clone, Debug)]
pub struct TokenizeLineResult2 {
    /// Alternating UTF-16 start offset and packed token metadata.
    pub tokens: Vec<u32>,
    pub rule_stack: StateStack,
    pub stopped_early: bool,
}

impl Grammar {
    pub fn tokenize_line2(
        &self,
        line: &str,
        previous: Option<&StateStack>,
        theme: &Theme,
        colors: &mut ColorMap,
        configuration: &GrammarConfiguration,
    ) -> Result<TokenizeLineResult2> {
        let result = self.tokenize_line(line, previous)?;
        let mut encoded = Vec::new();
        for token in result.tokens {
            let style = theme.match_scopes(&token.scopes);
            let mut language = configuration.initial_language;
            for scope in &token.scopes {
                if let Some((_, id)) = configuration
                    .embedded_languages
                    .iter()
                    .filter(|(prefix, _)| {
                        scope == *prefix
                            || scope
                                .strip_prefix(prefix.as_str())
                                .is_some_and(|tail| tail.starts_with('.'))
                    })
                    .max_by_key(|(prefix, _)| prefix.len())
                {
                    language = *id;
                }
            }
            let mut token_type = EncodedTokenAttributes::infer_token_type(&token.scopes);
            for (selector, value) in &configuration.token_types {
                if selector.matches(&token.scopes) {
                    token_type = *value;
                }
            }
            let balanced = configuration
                .balanced_bracket_selectors
                .iter()
                .any(|s| s.matches(&token.scopes))
                && !configuration
                    .unbalanced_bracket_selectors
                    .iter()
                    .any(|s| s.matches(&token.scopes));
            let foreground = colors.get_id(style.foreground)?;
            let background = u8::try_from(colors.get_id(style.background)?).map_err(|_| {
                Error::InvalidGrammar("encoded token background color index exceeds 255".into())
            })?;
            let metadata = EncodedTokenAttributes::new(
                language,
                token_type,
                balanced,
                style.font_style,
                foreground,
                background,
            )
            .0;
            if encoded.last().copied() != Some(metadata) {
                encoded.push(u32::try_from(token.start_index).map_err(|_| {
                    Error::InvalidGrammar("line exceeds encoded token index range".into())
                })?);
                encoded.push(metadata);
            }
        }
        Ok(TokenizeLineResult2 {
            tokens: encoded,
            rule_stack: result.rule_stack,
            stopped_early: result.stopped_early,
        })
    }
}
