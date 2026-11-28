use crate::{
    raw::read_json_or_plist,
    selector::{matches_path, parent_scope_lengths},
    Error, Result, ScopeSelector,
};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap, HashSet},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}
impl Color {
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
    pub fn parse(value: &str) -> Option<Self> {
        let hex = value.strip_prefix('#')?;
        let parse = |start, end| u8::from_str_radix(hex.get(start..end)?, 16).ok();
        match hex.len() {
            3 | 4 => Some(Self::rgba(
                parse(0, 1)? * 17,
                parse(1, 2)? * 17,
                parse(2, 3)? * 17,
                if hex.len() == 4 {
                    parse(3, 4)? * 17
                } else {
                    255
                },
            )),
            6 | 8 => Some(Self::rgba(
                parse(0, 2)?,
                parse(2, 4)?,
                parse(4, 6)?,
                if hex.len() == 8 { parse(6, 8)? } else { 255 },
            )),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct FontStyle(pub u8);
impl FontStyle {
    pub const NONE: Self = Self(0);
    pub const ITALIC: Self = Self(1);
    pub const BOLD: Self = Self(2);
    pub const UNDERLINE: Self = Self(4);
    pub const STRIKETHROUGH: Self = Self(8);
    pub const fn contains(self, flag: Self) -> bool {
        self.0 & flag.0 == flag.0
    }
    pub fn parse(value: &str) -> Self {
        Self(value.split_whitespace().fold(0, |bits, name| {
            bits | match name {
                "italic" => 1,
                "bold" => 2,
                "underline" => 4,
                "strikethrough" => 8,
                _ => 0,
            }
        }))
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TokenStyle {
    pub foreground: Option<Color>,
    pub background: Option<Color>,
    pub font_style: FontStyle,
}

/// Properties supplied by token rules. An absent field inherits the host style.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TokenStyleOverrides {
    pub foreground: Option<Color>,
    pub background: Option<Color>,
    /// `Some(FontStyle::NONE)` clears the inherited theme font flags.
    pub font_style: Option<FontStyle>,
}

impl TokenStyleOverrides {
    fn over(self, inherited: Self) -> Self {
        Self {
            foreground: self.foreground.or(inherited.foreground),
            background: self.background.or(inherited.background),
            font_style: self.font_style.or(inherited.font_style),
        }
    }

    fn resolved(self) -> TokenStyle {
        TokenStyle {
            foreground: self.foreground,
            background: self.background,
            font_style: self.font_style.unwrap_or_default(),
        }
    }
}

const DEFAULT_TOKEN_COLORS: TokenStyleOverrides = TokenStyleOverrides {
    foreground: Some(Color::rgba(0, 0, 0, 255)),
    background: Some(Color::rgba(255, 255, 255, 255)),
    font_style: None,
};
const SCOPE_CACHE_LIMIT: usize = 16_384;

#[derive(Clone, Debug)]
struct Rule {
    layer: usize,
    selector: ScopeSelector,
    style: TokenStyleOverrides,
}

#[derive(Clone, Debug, Default)]
struct EncodingLayer {
    scopes: HashMap<String, EncodingScope>,
}

#[derive(Clone, Debug, Default)]
struct EncodingScope {
    plain: Option<TokenStyleOverrides>,
    conditional: IndexMap<Vec<String>, TokenStyleOverrides>,
}

fn scope_prefixes(scope: &str) -> impl Iterator<Item = &str> {
    scope
        .match_indices('.')
        .map(move |(end, _)| &scope[..end])
        .chain(std::iter::once(scope))
}

fn has_blank_scope_alternative(scope: &str) -> bool {
    if !scope.contains(',') {
        return false;
    }
    // A nonempty branch containing only whitespace declares a token default.
    // Adjacent commas produce empty branches and groups keep their own commas.
    let blank = |start, end| start != end && scope[start..end].trim().is_empty();
    let mut start = 0;
    let mut depth = 0usize;
    for (index, character) in scope.char_indices() {
        match character {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                if blank(start, index) {
                    return true;
                }
                start = index + 1;
            }
            _ => {}
        }
    }
    blank(start, scope.len())
}

impl EncodingLayer {
    fn insert(&mut self, target: &str, parents: &[String], style: TokenStyleOverrides) {
        let scope = self.scopes.entry(target.to_owned()).or_default();
        if parents.is_empty() {
            scope.plain = Some(style.over(scope.plain.unwrap_or_default()));
        } else {
            let inherited = scope.conditional.shift_remove(parents).unwrap_or_default();
            scope
                .conditional
                .insert(parents.to_vec(), style.over(inherited));
        }
        for prefix in scope_prefixes(target).take_while(|prefix| *prefix != target) {
            self.scopes.entry(prefix.to_owned()).or_default();
        }
    }

    fn resolve_conditional_inheritance(&mut self) {
        let mut targets: Vec<_> = self
            .scopes
            .iter()
            .filter(|(_, scope)| !scope.conditional.is_empty())
            .map(|(target, _)| target.clone())
            .collect();
        // Every dotted prefix sorts before its descendants.
        targets.sort_unstable();
        for target in targets {
            let mut scope = self.scopes.remove(&target).unwrap();
            let prefixes = || scope_prefixes(&target).filter_map(|name| self.scopes.get(name));
            let inherited_plain = prefixes()
                .filter_map(|prefix| prefix.plain)
                .fold(TokenStyleOverrides::default(), |plain, style| {
                    style.over(plain)
                });
            let plain = scope.plain.unwrap_or_default().over(inherited_plain);
            for (path, style) in &mut scope.conditional {
                // An existing conditional keeps its fields when redeclared
                // at a descendant. A new condition starts with the plain style.
                let inherited = prefixes()
                    .filter_map(|prefix| prefix.conditional.get(path).copied())
                    .last()
                    .unwrap_or(plain);
                *style = style.over(inherited);
            }
            self.scopes.insert(target, scope);
        }
    }

    fn style(&self, scopes: &[String]) -> Option<TokenStyleOverrides> {
        let (target, parents) = scopes.split_last()?;
        let mut plain = (0, TokenStyleOverrides::default());
        let mut conditional = None;
        for (index, prefix) in scope_prefixes(target).enumerate() {
            let Some(scope) = self.scopes.get(prefix) else {
                break;
            };
            let depth = index + 1;
            if let Some(style) = scope.plain {
                plain = (depth, style.over(plain.1));
            }
            // At one target, ancestry length and then parent specificity
            // decide which conditional rule supplies the style.
            if let Some((_, style)) = scope
                .conditional
                .iter()
                .filter(|(path, _)| matches_path(path, parents))
                .max_by(|(first, _), (second, _)| {
                    first.len().cmp(&second.len())
                        .then_with(|| parent_scope_lengths(first).cmp(parent_scope_lengths(second)))
                })
            {
                conditional = Some((depth, *style));
            }
        }
        // The reference exposes a plain prefix on an unknown suffix only
        // when the prefix already defines or inherits a foreground.
        let plain =
            (plain.1.foreground.is_some() || self.scopes.contains_key(target)).then_some(plain);
        match (conditional, plain) {
            (Some((depth, style)), Some((plain_depth, _))) if depth >= plain_depth => Some(style),
            (_, Some((_, style))) | (Some((_, style)), None) => Some(style),
            (None, None) => None,
        }
    }
}

/// Parsed JSON/JSONC or XML plist theme values before resource resolution.
///
/// Edit the document and compile it with [`Theme::from_raw`] or
/// [`Theme::from_raw_with_resolver`]. Serde serialization retains the parsed
/// data, including rule names, unknown fields and nulls. Comments and source
/// formatting are discarded during parsing.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RawTheme {
    pub document: Value,
}

impl RawTheme {
    /// Reads a theme document without loading includes or external token files.
    pub fn parse(source: &str) -> Result<Self> {
        read_json_or_plist(source)
    }
}

/// Scope theme with deterministic specificity and explicit font-style resets.
#[derive(Clone, Debug)]
pub struct Theme {
    pub name: Option<String>,
    pub colors: BTreeMap<String, Color>,
    token_defaults: TokenStyleOverrides,
    encoded_defaults: TokenStyleOverrides,
    rules: Vec<Rule>,
    encoding_layers: Vec<EncodingLayer>,
    extended_encoding_rules: Vec<usize>,
    cache: RefCell<HashMap<Vec<String>, TokenStyleOverrides>>,
    encoding_cache: RefCell<HashMap<Vec<String>, TokenStyleOverrides>>,
}

enum ThemePhase {
    Include,
    TokenFile,
    Rules,
}

struct ThemeFile {
    resource: Option<String>,
    raw: Value,
    phase: ThemePhase,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            name: None,
            colors: BTreeMap::new(),
            token_defaults: TokenStyleOverrides::default(),
            encoded_defaults: TokenStyleOverrides::default(),
            rules: Vec::new(),
            encoding_layers: Vec::new(),
            extended_encoding_rules: Vec::new(),
            cache: RefCell::new(HashMap::new()),
            encoding_cache: RefCell::new(HashMap::new()),
        }
    }
}
impl Theme {
    pub fn from_json(source: &str) -> Result<Self> {
        Self::from_raw(RawTheme::parse(source)?)
    }
    pub fn from_json_with_resolver(
        source: &str,
        resolver: &dyn Fn(&str) -> Result<String>,
    ) -> Result<Self> {
        Self::from_raw_with_resolver(RawTheme::parse(source)?, &|path| {
            RawTheme::parse(&resolver(path)?)
        })
    }

    /// Compiles a self-contained theme document.
    pub fn from_raw(raw: RawTheme) -> Result<Self> {
        Self::from_raw_with_resolver(raw, &|path| Err(Error::MissingResource(path.into())))
    }

    /// Compiles a theme and resolves resource names as written in the document.
    pub fn from_raw_with_resolver(
        raw: RawTheme,
        resolver: &dyn Fn(&str) -> Result<RawTheme>,
    ) -> Result<Self> {
        let mut theme = Self::default();
        let mut pending = vec![ThemeFile {
            resource: None,
            raw: raw.document,
            phase: ThemePhase::Include,
        }];
        let mut active = HashSet::new();
        let mut defaults = TokenStyleOverrides::default();
        // A theme first imports its base, then applies workbench colors,
        // loads its external token table, and finally applies its own rules.
        while let Some(file) = pending.last_mut() {
            let dependency = match file.phase {
                ThemePhase::Include => {
                    file.phase = ThemePhase::TokenFile;
                    file.raw
                        .get("include")
                        .and_then(Value::as_str)
                        .filter(|path| !path.is_empty())
                }
                ThemePhase::TokenFile => {
                    file.phase = ThemePhase::Rules;
                    theme.apply_workbench_colors(&file.raw);
                    // Include defaults already contribute to rendering. Only a
                    // token file's defaults are returned to the parent layer.
                    defaults = TokenStyleOverrides::default();
                    file.raw
                        .get("tokenColors")
                        .or_else(|| file.raw.get("settings"))
                        .and_then(Value::as_str)
                }
                ThemePhase::Rules => {
                    defaults = theme.apply_rules(&file.raw, defaults)?;
                    if let Some(resource) = &file.resource {
                        active.remove(resource);
                    }
                    pending.pop();
                    continue;
                }
            };
            if let Some(path) = dependency {
                let resource = path.to_owned();
                if !active.insert(resource.clone()) {
                    return Err(Error::ResourceCycle(resource));
                }
                let raw = resolver(&resource)?;
                pending.push(ThemeFile {
                    resource: Some(resource),
                    raw: raw.document,
                    phase: ThemePhase::Include,
                });
            }
        }
        theme.encoded_defaults = defaults;
        Ok(theme)
    }

    fn apply_workbench_colors(&mut self, raw: &Value) {
        if let Some(name) = raw.get("name").and_then(Value::as_str) {
            self.name = Some(name.into());
        }
        if let Some(colors) = raw.get("colors").and_then(Value::as_object) {
            for (name, value) in colors {
                if let Some(color) = value.as_str().and_then(Color::parse) {
                    self.colors.insert(name.clone(), color);
                }
            }
        }
    }

    fn apply_rules(
        &mut self,
        raw: &Value,
        mut defaults: TokenStyleOverrides,
    ) -> Result<TokenStyleOverrides> {
        let settings = raw.get("settings");
        let token_colors = raw.get("tokenColors");
        // Both formats contribute to the same layer. The including theme
        // takes precedence even when an inherited rule matches a deeper scope.
        let layer = self.encoding_layers.len();
        self.encoding_layers.push(EncodingLayer::default());
        let [mut legacy_entries, mut token_entries] = [settings, token_colors].map(|entries| {
            entries
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter(|entry| entry.get("settings").is_some_and(Value::is_object))
        });
        loop {
            // Both lists start at index zero. At the same index, apply
            // tokenColors after settings so redefinitions have a stable order.
            let entries = [legacy_entries.next(), token_entries.next()];
            if entries.iter().all(Option::is_none) {
                break;
            }
            for entry in entries.into_iter().flatten() {
                self.apply_entry(layer, entry, &mut defaults)?;
            }
        }
        self.encoding_layers[layer].resolve_conditional_inheritance();
        self.token_defaults = defaults.over(self.token_defaults);
        Ok(defaults)
    }

    fn apply_entry(
        &mut self,
        layer: usize,
        entry: &Value,
        defaults: &mut TokenStyleOverrides,
    ) -> Result<()> {
        let settings = &entry["settings"];
        let style = TokenStyleOverrides {
            foreground: settings
                .get("foreground")
                .and_then(Value::as_str)
                .and_then(Color::parse),
            background: settings
                .get("background")
                .and_then(Value::as_str)
                .and_then(Color::parse),
            font_style: settings
                .get("fontStyle")
                .and_then(Value::as_str)
                .map(FontStyle::parse),
        };
        let scopes: Vec<&str> = match entry.get("scope") {
            Some(Value::String(scope)) if scope.is_empty() => Vec::new(),
            Some(Value::String(scope)) => vec![scope],
            Some(Value::Array(scopes)) => scopes.iter().filter_map(Value::as_str).collect(),
            _ => vec![""],
        };
        for scope in scopes {
            if scope.trim().is_empty() || has_blank_scope_alternative(scope) {
                *defaults = style.over(*defaults);
            }
            for selector in ScopeSelector::parse_many(scope)? {
                match selector.path().and_then(<[_]>::split_last) {
                    Some((target, parents)) if target != "*" => {
                        self.encoding_layers[layer].insert(target, parents, style);
                    }
                    _ => self.extended_encoding_rules.push(self.rules.len()),
                }
                self.rules.push(Rule {
                    layer,
                    selector,
                    style,
                });
            }
        }
        Ok(())
    }

    pub fn defaults(&self) -> TokenStyle {
        self.default_style().resolved()
    }
    pub fn match_scopes(&self, scopes: &[String]) -> TokenStyle {
        self.matching_rules(scopes)
            .over(self.default_style())
            .resolved()
    }

    /// Explicit token defaults and matching scope rules for a host renderer.
    pub fn style_overrides(&self, scopes: &[String]) -> TokenStyleOverrides {
        self.matching_rules(scopes).over(self.token_defaults)
    }

    pub(crate) fn encoded_style(&self, scopes: &[String]) -> TokenStyle {
        // Boolean and wildcard target selectors extend the reference format.
        // Keep their full-stack cascade, including exclusions against descendants.
        let has_extended_match = self
            .extended_encoding_rules
            .iter()
            .any(|&index| self.rules[index].selector.matches(scopes));
        let style = if has_extended_match {
            self.matching_rules(scopes)
        } else {
            self.encoded_rules(scopes)
        };
        style
            .over(self.encoded_defaults)
            .over(DEFAULT_TOKEN_COLORS)
            .resolved()
    }

    fn encoded_rules(&self, scopes: &[String]) -> TokenStyleOverrides {
        let mut cache = self.encoding_cache.borrow_mut();
        let (start, mut style) = (1..=scopes.len())
            .rev()
            .find_map(|depth| cache.get(&scopes[..depth]).map(|style| (depth, *style)))
            .unwrap_or_default();
        for depth in start + 1..=scopes.len() {
            let prefix = &scopes[..depth];
            if let Some(rule) = self
                .encoding_layers
                .iter()
                .rev()
                .find_map(|layer| layer.style(prefix))
            {
                style = rule.over(style);
            }
            if cache.len() >= SCOPE_CACHE_LIMIT {
                cache.clear();
            }
            cache.insert(prefix.to_vec(), style);
        }
        style
    }

    fn default_style(&self) -> TokenStyleOverrides {
        let palette = TokenStyleOverrides {
            foreground: self.colors.get("editor.foreground").copied(),
            background: self.colors.get("editor.background").copied(),
            font_style: None,
        };
        self.token_defaults.over(palette).over(DEFAULT_TOKEN_COLORS)
    }

    fn matching_rules(&self, scopes: &[String]) -> TokenStyleOverrides {
        if let Some(style) = self.cache.borrow().get(scopes) {
            return *style;
        }
        let mut matched: Vec<_> = self
            .rules
            .iter()
            .enumerate()
            .filter_map(|(index, rule)| {
                rule.selector
                    .specificity(scopes)
                    .map(|rank| (rule.layer, rank, index, rule))
            })
            .collect();
        matched.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
        let style = matched
            .into_iter()
            .fold(TokenStyleOverrides::default(), |style, (_, _, _, rule)| {
                rule.style.over(style)
            });
        let mut cache = self.cache.borrow_mut();
        if cache.len() >= SCOPE_CACHE_LIMIT {
            cache.clear();
        }
        cache.insert(scopes.to_vec(), style);
        style
    }
}
