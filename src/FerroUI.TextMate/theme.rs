use crate::{raw::read_json_or_plist, Error, Result, ScopeSelector};
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

#[derive(Clone, Debug)]
struct Rule {
    layer: usize,
    selector: ScopeSelector,
    foreground: Option<Color>,
    background: Option<Color>,
    font_style: Option<FontStyle>,
}

/// Scope theme with deterministic specificity and explicit font-style resets.
#[derive(Clone, Debug)]
pub struct Theme {
    pub name: Option<String>,
    pub colors: BTreeMap<String, Color>,
    defaults: TokenStyle,
    rules: Vec<Rule>,
    next_layer: usize,
    cache: RefCell<HashMap<Vec<String>, TokenStyle>>,
}
impl Default for Theme {
    fn default() -> Self {
        Self {
            name: None,
            colors: BTreeMap::new(),
            defaults: TokenStyle {
                foreground: Color::parse("#000000"),
                background: Color::parse("#ffffff"),
                font_style: FontStyle::NONE,
            },
            rules: Vec::new(),
            next_layer: 0,
            cache: RefCell::new(HashMap::new()),
        }
    }
}
impl Theme {
    pub fn from_json(source: &str) -> Result<Self> {
        Self::from_json_with_resolver(source, &|path| Err(Error::MissingResource(path.into())))
    }
    pub fn from_json_with_resolver(
        source: &str,
        resolver: &dyn Fn(&str) -> Result<String>,
    ) -> Result<Self> {
        let mut theme = Self::default();
        theme.apply(source, resolver, &mut HashSet::new())?;
        Ok(theme)
    }
    fn apply(
        &mut self,
        source: &str,
        resolver: &dyn Fn(&str) -> Result<String>,
        seen: &mut HashSet<String>,
    ) -> Result<()> {
        let raw = read_json_or_plist(source)?;
        if let Some(include) = raw.get("include").and_then(Value::as_str) {
            if !seen.insert(include.into()) {
                return Err(Error::ResourceCycle(include.into()));
            }
            self.apply(&resolver(include)?, resolver, seen)?;
            seen.remove(include);
        }
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
        if let Some(color) = self.colors.get("editor.foreground") {
            self.defaults.foreground = Some(*color);
        }
        if let Some(color) = self.colors.get("editor.background") {
            self.defaults.background = Some(*color);
        }
        let entries = raw.get("tokenColors").or_else(|| raw.get("settings"));
        if let Some(Value::String(path)) = entries {
            if !seen.insert(path.clone()) {
                return Err(Error::ResourceCycle(path.clone()));
            }
            self.apply(&resolver(path)?, resolver, seen)?;
            seen.remove(path);
        }
        if let Some(entries) = entries.and_then(Value::as_array) {
            // The including theme takes precedence over its inherited theme,
            // even when an inherited rule matches a deeper scope.
            let layer = self.next_layer;
            self.next_layer += 1;
            for entry in entries {
                let Some(settings) = entry.get("settings") else {
                    continue;
                };
                let foreground = settings
                    .get("foreground")
                    .and_then(Value::as_str)
                    .and_then(Color::parse);
                let background = settings
                    .get("background")
                    .and_then(Value::as_str)
                    .and_then(Color::parse);
                let font_style = settings
                    .get("fontStyle")
                    .and_then(Value::as_str)
                    .map(FontStyle::parse);
                let scopes: Vec<&str> = match entry.get("scope") {
                    Some(Value::String(s)) => vec![s],
                    Some(Value::Array(a)) => a.iter().filter_map(Value::as_str).collect(),
                    _ => Vec::new(),
                };
                if scopes.is_empty() || scopes.iter().all(|s| s.trim().is_empty()) {
                    if foreground.is_some() {
                        self.defaults.foreground = foreground;
                    }
                    if background.is_some() {
                        self.defaults.background = background;
                    }
                    if let Some(style) = font_style {
                        self.defaults.font_style = style;
                    }
                } else {
                    for scope in scopes {
                        self.rules.push(Rule {
                            layer,
                            selector: ScopeSelector::parse(scope)?,
                            foreground,
                            background,
                            font_style,
                        });
                    }
                }
            }
        }
        Ok(())
    }
    pub fn defaults(&self) -> TokenStyle {
        self.defaults
    }
    pub fn match_scopes(&self, scopes: &[String]) -> TokenStyle {
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
        let mut style = self.defaults;
        for (_, _, _, rule) in matched {
            if rule.foreground.is_some() {
                style.foreground = rule.foreground;
            }
            if rule.background.is_some() {
                style.background = rule.background;
            }
            if let Some(font_style) = rule.font_style {
                style.font_style = font_style;
            }
        }
        let mut cache = self.cache.borrow_mut();
        if cache.len() >= 16384 {
            cache.clear();
        }
        cache.insert(scopes.to_vec(), style);
        style
    }
}
