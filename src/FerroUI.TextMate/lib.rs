//! TextMate grammar execution and incremental highlighting.
//!
//! Token positions use UTF-16 code units, matching editor document positions.
//! Grammar and theme loading takes strings. Tokenization and theme lookup use
//! no file I/O or clock services and can run on the UI thread, a worker, or WebAssembly.
//! Native builds use Oniguruma. WebAssembly builds use the bundled official
//! engine after `initialize_browser_oniguruma().await`. [`RegexBackend`]
//! identifies the active engine; `portable-regex` explicitly selects a
//! restricted pure Rust fallback.

#[cfg(target_arch = "wasm32")]
mod browser;
mod catalog;
mod encoded_tokens;
mod error;
mod filename_glob;
mod grammar;
mod language_configuration;
mod line_text;
mod metadata;
mod model;
mod model_contract;
mod model_events;
mod package;
mod raw;
mod regex_source;
mod registry;
mod selector;
mod theme;
#[cfg(target_arch = "wasm32")]
pub use browser::{browser_oniguruma_ready, initialize_browser_oniguruma};

pub use catalog::{BundledCatalog, Language, LanguageConfiguration, LanguagePackage, ThemeName};
pub use encoded_tokens::{ColorMap, GrammarConfiguration, TokenizeLineResult2};
pub use error::{Error, Result};
pub use filename_glob::{FilenameGlob, FilenameGlobError, FilenameGlobErrorKind};
pub use grammar::{Grammar, StateStack, Token, TokenizeLineResult, TokenizeOptions};
pub use language_configuration::{
    AutoClosingPairs, AutoPair, Comments, EnterAction, EnterRule, EnterRules, FoldMarker, Folding,
    FoldingRules, IndentAction,
    Indentation, IndentationMatch, IndentationRules, LanguagePairs, LanguageSnippet, LanguageSnippets,
    Markers, RegexPattern, WordRange,
};
pub use line_text::{ITokenizationSupport, LineText};
pub use metadata::{EncodedTokenAttributes, StandardTokenType};
pub use model::{LineTokens, TokenizationModel, TokenizationUpdate};
pub use model_contract::{IModelLines, ITMModel};
pub use model_events::{IModelTokensChangedListener, ModelSubscription, ModelTokensChangedEvent};
pub use package::{
    Contributes, Engines, GrammarContribution, GrammarDefinition, LanguageContribution, Repository,
    Scripts, SnippetContribution,
};
pub use raw::{read_json, RawGrammar, RawRule};
pub use regex_source::RegexBackend;
pub use registry::{
    DefaultLocator, GrammarLoadDiagnostic, IGrammarRepository, IRegistryOptions, IThemeProvider,
    Registry,
};
pub use selector::{ScopeSelector, SelectorPriority};
pub use theme::{Color, FontStyle, RawTheme, Theme, TokenStyle, TokenStyleOverrides};

#[cfg(test)]
mod tests;

#[cfg(test)]
#[path = "tests/oracle/dynamic_delimiters.rs"]
mod dynamic_delimiters;

#[cfg(test)]
#[path = "tests/oracle/while_conditions.rs"]
mod while_conditions;

#[cfg(test)]
#[path = "tests/oracle/zero_width.rs"]
mod zero_width;

#[cfg(all(test, feature = "bundled-grammars"))]
#[path = "tests/oracle/pinned_grammar_cases.rs"]
mod pinned_grammar_cases;
