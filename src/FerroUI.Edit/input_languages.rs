use crate::bracket_balance::{editing_brackets, BracketSyntax};
use ferroui_textmate::{
    EnterRules, Error, FoldingRules, GrammarConfiguration, IndentationRules, LanguageConfiguration,
    LanguagePairs, Result, StandardTokenType, Token,
};
use std::collections::BTreeMap;

pub(crate) struct EditingRules {
    pub(crate) configuration: LanguageConfiguration,
    pub(crate) enter: EnterRules,
    pub(crate) indentation: Option<IndentationRules>,
    pub(crate) enter_brackets: Vec<(String, String)>,
    pub(crate) pairs: LanguagePairs,
}

#[derive(Default)]
pub(crate) struct InputLanguages {
    pub(crate) grammar: GrammarConfiguration,
    pub(crate) brackets: Option<BracketSyntax>,
    pub(crate) folding: FoldingRules,
    rules: BTreeMap<u8, EditingRules>,
}

impl InputLanguages {
    pub(crate) fn new<'a>(
        grammar: GrammarConfiguration,
        root: Option<&'a LanguageConfiguration>,
        embedded: impl IntoIterator<Item = (u8, &'a LanguageConfiguration)>,
    ) -> Result<Self> {
        let mut configurations = BTreeMap::new();
        if let Some(root) = root {
            configurations.insert(grammar.initial_language, root);
        }
        for (id, configuration) in embedded {
            if id == grammar.initial_language || configurations.insert(id, configuration).is_some()
            {
                return Err(Error::InvalidGrammar(format!(
                    "duplicate editing configuration for language ID {id}"
                )));
            }
        }
        let brackets = BracketSyntax::new(
            configurations
                .iter()
                .map(|(&id, &configuration)| (id, configuration)),
        );
        let folding = root
            .map(LanguageConfiguration::folding_rules)
            .transpose()?
            .unwrap_or_default();
        let rules = configurations
            .into_iter()
            .map(|(id, configuration)| {
                Ok((
                    id,
                    EditingRules {
                        configuration: configuration.clone(),
                        enter: configuration.enter_rules()?,
                        indentation: configuration.editing_indentation_rules()?,
                        enter_brackets: editing_brackets(configuration)
                            .into_iter()
                            .filter(|(open, close)| !open.is_empty() && !close.is_empty())
                            .map(|(open, close)| (open.to_owned(), close.to_owned()))
                            .collect(),
                        pairs: configuration.language_pairs()?,
                    },
                ))
            })
            .collect::<Result<_>>()?;
        Ok(Self {
            grammar,
            brackets,
            folding,
            rules,
        })
    }

    pub(crate) fn get(&self, id: u8) -> Option<&EditingRules> {
        self.rules.get(&id)
    }

    pub(crate) fn has_enter_rules(&self) -> bool {
        self.rules.values().any(|rules| {
            !rules.enter.is_empty()
                || !rules.enter_brackets.is_empty()
                || rules.indentation.is_some()
        })
    }

    pub(crate) fn has_indentation_rules(&self) -> bool {
        self.rules.values().any(|rules| rules.indentation.is_some())
    }

    pub(crate) fn has_pairs(&self) -> bool {
        self.rules.values().any(|rules| !rules.pairs.is_empty())
    }

    pub(crate) fn can_open_pair(&self, typed: char) -> bool {
        self.rules.values().any(|rules| {
            rules
                .pairs
                .auto_closing
                .iter()
                .any(|pair| pair.open.ends_with(typed))
        })
    }

    pub(crate) fn can_surround(&self, typed: &str) -> bool {
        self.rules.values().any(|rules| {
            rules
                .pairs
                .surrounding
                .iter()
                .any(|pair| pair.open == typed)
        })
    }

    pub(crate) fn tokens(&self, tokens: &[Token]) -> Vec<InputToken> {
        tokens
            .iter()
            .map(|token| {
                let brackets = (self.grammar.balanced_bracket_selectors.is_empty()
                    || self
                        .grammar
                        .balanced_bracket_selectors
                        .iter()
                        .any(|selector| selector.matches(&token.scopes)))
                    && !self
                        .grammar
                        .unbalanced_bracket_selectors
                        .iter()
                        .any(|selector| selector.matches(&token.scopes));
                InputToken {
                    start: token.start_index,
                    end: token.end_index,
                    id: self.grammar.language_for_scopes(&token.scopes),
                    kind: self.grammar.token_type_for_scopes(&token.scopes),
                    brackets,
                }
            })
            .collect()
    }
}

#[derive(Clone, Copy)]
pub(crate) struct InputToken {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) id: u8,
    pub(crate) kind: StandardTokenType,
    pub(crate) brackets: bool,
}

#[derive(Clone, Copy)]
pub(crate) struct LanguageScope {
    pub(crate) id: u8,
    pub(crate) start: usize,
    pub(crate) end: usize,
}

impl LanguageScope {
    pub(crate) fn at(tokens: &[InputToken], root: u8, column: usize, length: usize) -> Self {
        let index = tokens
            .partition_point(|token| token.start <= column)
            .saturating_sub(1);
        let Some(token) = tokens.get(index) else {
            return Self {
                id: root,
                start: 0,
                end: length,
            };
        };
        let left = tokens[..index]
            .iter()
            .rev()
            .take_while(|other| other.id == token.id);
        let right = tokens[index + 1..]
            .iter()
            .take_while(|other| other.id == token.id);
        Self {
            id: token.id,
            start: left.last().map_or(token.start, |token| token.start),
            end: right.last().map_or(token.end, |token| token.end),
        }
    }
}
