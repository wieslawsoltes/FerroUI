use super::model::*;
use crate::highlighting::highlighting_definition::parse_font_weight;
use crate::highlighting::{
    HighlightingColor, HighlightingDefinition, HighlightingDefinitionResolver, HighlightingError,
    HighlightingFontStyle, HighlightingRule, HighlightingRuleSet, HighlightingSpan,
};
use fancy_regex::{Regex, RegexBuilder};
use std::collections::{hash_map::Entry, HashMap, HashSet};
use std::rc::Rc;

/// Resolves syntax trees and expands imports where they occur. This preserves
/// source precedence when local and imported expressions match at the same place.
pub fn compile_xshd(
    syntax: &XshdSyntaxDefinition,
    dependencies: &[&XshdSyntaxDefinition],
) -> Result<HighlightingDefinition, HighlightingError> {
    compile_with_resolver(syntax, dependencies, None, true)
}

pub(in crate::highlighting) fn compile_with_resolver(
    syntax: &XshdSyntaxDefinition,
    dependencies: &[&XshdSyntaxDefinition],
    resolver: Option<&dyn HighlightingDefinitionResolver>,
    qualify_root: bool,
) -> Result<HighlightingDefinition, HighlightingError> {
    let mut compiler = Compiler::new(
        std::iter::once(syntax)
            .chain(dependencies.iter().copied())
            .collect(),
        resolver,
        qualify_root,
    )?;
    compiler.compile()?;
    let mut properties = HashMap::new();
    for element in &syntax.elements {
        if let XshdElement::Property(property) = element {
            if properties
                .insert(property.name.clone(), property.value.clone())
                .is_some()
            {
                return Err(property
                    .position
                    .error(format!("Duplicate property {}", property.name)));
            }
        }
    }
    let colors = compiler
        .colors
        .iter()
        .filter(|((definition, _), _)| *definition == 0)
        .map(|((_, name), color)| ((*name).to_owned(), runtime_color(color)))
        .collect();
    Ok(HighlightingDefinition {
        name: syntax.name.clone().unwrap_or_default(),
        extensions: syntax.extensions.clone(),
        properties,
        colors,
        main_rule_set: compiler.main[0],
        rule_sets: compiler.output,
    })
}

#[derive(Clone, Copy)]
struct SetInput<'a> {
    definition: usize,
    set: &'a XshdRuleSet,
}

struct ExternalDefinition {
    definition: Rc<HighlightingDefinition>,
    rule_set_offset: Option<usize>,
}

struct Compiler<'a> {
    definitions: Vec<&'a XshdSyntaxDefinition>,
    definition_names: HashMap<&'a str, usize>,
    inputs: Vec<SetInput<'a>>,
    identities: HashMap<usize, usize>,
    names: HashMap<(usize, &'a str), usize>,
    colors: HashMap<(usize, &'a str), &'a XshdColor>,
    main: Vec<usize>,
    output: Vec<HighlightingRuleSet>,
    status: Vec<u8>,
    resolver: Option<&'a dyn HighlightingDefinitionResolver>,
    external: HashMap<String, ExternalDefinition>,
    compile_depth: usize,
}

/// How deep rule sets may import each other: compilation recurses once per level.
const MAX_RULE_SET_DEPTH: usize = 256;
/// The rules and spans one rule set may hold after its imports. Two imports of the next set at
/// each level would otherwise double the set per level.
const MAX_RULES_PER_SET: usize = 1 << 16;

impl<'a> Compiler<'a> {
    fn new(
        definitions: Vec<&'a XshdSyntaxDefinition>,
        resolver: Option<&'a dyn HighlightingDefinitionResolver>,
        qualify_root: bool,
    ) -> Result<Self, HighlightingError> {
        let mut result = Self {
            definitions,
            definition_names: HashMap::new(),
            inputs: Vec::new(),
            identities: HashMap::new(),
            names: HashMap::new(),
            colors: HashMap::new(),
            main: Vec::new(),
            output: Vec::new(),
            status: Vec::new(),
            resolver,
            external: HashMap::new(),
            compile_depth: 0,
        };
        for definition in 0..result.definitions.len() {
            let syntax = result.definitions[definition];
            if let Some(name) = syntax
                .name
                .as_deref()
                .filter(|_| definition != 0 || qualify_root)
            {
                if result.definition_names.insert(name, definition).is_some() {
                    return Err(
                        XshdPosition::default().error(format!("Duplicate definition name {name}"))
                    );
                }
            }
            let mut main = None;
            for element in &syntax.elements {
                result.register_element(definition, element)?;
                if let XshdElement::RuleSet(set) = element {
                    if set.name.is_none() {
                        if main.replace(result.index(set)).is_some() {
                            return Err(set.position.error("Duplicate main RuleSet"));
                        }
                    }
                }
            }
            result.main.push(
                main.ok_or_else(|| XshdPosition::default().error("A main RuleSet is required"))?,
            );
        }
        result
            .output
            .resize_with(result.inputs.len(), HighlightingRuleSet::default);
        result.status.resize(result.inputs.len(), 0);
        Ok(result)
    }

    fn register_element(
        &mut self,
        definition: usize,
        element: &'a XshdElement,
    ) -> Result<(), HighlightingError> {
        match element {
            XshdElement::RuleSet(set) => self.register_set(definition, set),
            XshdElement::Color(color) => self.register_color(definition, color),
            XshdElement::Keywords(keywords) => self.register_color_ref(definition, &keywords.color),
            XshdElement::Span(span) => {
                self.register_color_ref(definition, &span.begin_color)?;
                self.register_color_ref(definition, &span.span_color)?;
                self.register_color_ref(definition, &span.end_color)?;
                self.register_set_ref(definition, &span.rule_set)
            }
            XshdElement::Import(import) => self.register_set_ref(definition, &import.rule_set),
            XshdElement::Rule(rule) => self.register_color_ref(definition, &rule.color),
            XshdElement::Property(_) => Ok(()),
        }
    }

    fn register_set(
        &mut self,
        definition: usize,
        set: &'a XshdRuleSet,
    ) -> Result<(), HighlightingError> {
        let index = self.inputs.len();
        self.inputs.push(SetInput { definition, set });
        self.identities
            .insert(set as *const XshdRuleSet as usize, index);
        if let Some(name) = set.name.as_deref() {
            validate_name(name, set.position)?;
            if self.names.insert((definition, name), index).is_some() {
                return Err(set
                    .position
                    .error(format!("Duplicate rule set name {name}")));
            }
        }
        for element in &set.elements {
            self.register_element(definition, element)?;
        }
        Ok(())
    }

    fn register_set_ref(
        &mut self,
        definition: usize,
        reference: &'a XshdReference<XshdRuleSet>,
    ) -> Result<(), HighlightingError> {
        if let XshdReference::Inline(set) = reference {
            self.register_set(definition, set)?;
        }
        Ok(())
    }

    fn register_color(
        &mut self,
        definition: usize,
        color: &'a XshdColor,
    ) -> Result<(), HighlightingError> {
        if color
            .font_weight
            .as_deref()
            .is_some_and(|value| parse_font_weight(value).is_none())
        {
            return Err(color
                .position
                .error("fontWeight must be a named weight or a number in 1..=1000"));
        }
        if color
            .font_style
            .as_deref()
            .is_some_and(|value| HighlightingFontStyle::parse(value).is_none())
        {
            return Err(color
                .position
                .error("fontStyle must be Normal, Italic or Oblique"));
        }
        if color
            .font_size
            .is_some_and(|size| !size.is_finite() || size <= 0.0)
        {
            return Err(color.position.error("fontSize must be finite and positive"));
        }
        if let Some(name) = color.name.as_deref() {
            validate_name(name, color.position)?;
            if self.colors.insert((definition, name), color).is_some() {
                return Err(color.position.error(format!("Duplicate color name {name}")));
            }
        }
        Ok(())
    }

    fn register_color_ref(
        &mut self,
        definition: usize,
        reference: &'a XshdReference<XshdColor>,
    ) -> Result<(), HighlightingError> {
        if let XshdReference::Inline(color) = reference {
            self.register_color(definition, color)?;
        }
        Ok(())
    }

    fn index(&self, set: &XshdRuleSet) -> usize {
        self.identities[&(set as *const XshdRuleSet as usize)]
    }

    fn compile(&mut self) -> Result<(), HighlightingError> {
        for definition in 0..self.definitions.len() {
            let syntax = self.definitions[definition];
            for element in &syntax.elements {
                if let XshdElement::RuleSet(set) = element {
                    self.compile_set(self.index(set), false)?;
                }
            }
        }
        Ok(())
    }

    fn compile_set(&mut self, index: usize, inherited_case: bool) -> Result<(), HighlightingError> {
        let SetInput { definition, set } = self.inputs[index];
        match self.status[index] {
            2 => return Ok(()),
            1 => return Err(set.position.error("RuleSet contains a cyclic Import")),
            _ => {}
        }
        if self.compile_depth >= MAX_RULE_SET_DEPTH {
            return Err(set.position.error("RuleSet imports are nested too deeply"));
        }
        self.compile_depth += 1;
        self.status[index] = 1;
        let ignore_case = set.ignore_case.unwrap_or(inherited_case);
        let mut output = HighlightingRuleSet {
            name: if definition == 0 {
                set.name.clone()
            } else {
                None
            },
            ..HighlightingRuleSet::default()
        };
        for element in &set.elements {
            match element {
                XshdElement::Rule(rule) => output.rules.push(HighlightingRule {
                    regex: compile_regex(
                        rule.regex.as_deref(),
                        rule.regex_type,
                        ignore_case,
                        rule.position,
                    )?,
                    color: self.color(definition, &rule.color, rule.position)?,
                }),
                XshdElement::Keywords(keywords) => {
                    let expression = keyword_regex(keywords)?;
                    output.rules.push(HighlightingRule {
                        regex: compile_regex(
                            Some(&expression),
                            XshdRegexType::Default,
                            ignore_case,
                            keywords.position,
                        )?,
                        color: self.color(definition, &keywords.color, keywords.position)?,
                    });
                }
                XshdElement::Span(span) => {
                    if span.begin.as_deref().unwrap_or("").is_empty()
                        && span.end.as_deref().unwrap_or("").is_empty()
                    {
                        return Err(span.position.error("Span has no start/end regex"));
                    }
                    let rule_set = self.resolve_set(definition, &span.rule_set, span.position)?;
                    if let XshdReference::Inline(_) = span.rule_set {
                        self.compile_set(
                            rule_set.expect("registered inline rule set"),
                            ignore_case,
                        )?;
                    }
                    let end = if span.multiline {
                        span.end.clone()
                    } else {
                        Some(match span.end.as_deref() {
                            None => "$".to_owned(),
                            // Re-enable extended mode before the terminator so
                            // comments and explicit (?-x) patterns both retain
                            // their original end-of-expression semantics.
                            Some(end) => format!("(?:$|{end}(?x)\n)"),
                        })
                    };
                    output.spans.push(HighlightingSpan {
                        start_expression: compile_regex(
                            span.begin.as_deref(),
                            span.begin_type,
                            ignore_case,
                            span.position,
                        )?,
                        end_expression: compile_regex(
                            end.as_deref(),
                            span.end_type,
                            ignore_case,
                            span.position,
                        )?,
                        rule_set,
                        span_color: self.color(definition, &span.span_color, span.position)?,
                        start_color: self.color(definition, &span.begin_color, span.position)?,
                        end_color: self.color(definition, &span.end_color, span.position)?,
                        span_color_includes_start: true,
                        span_color_includes_end: true,
                        multiline: span.multiline,
                    });
                }
                XshdElement::Import(import) => {
                    let target = self
                        .resolve_set(definition, &import.rule_set, import.position)?
                        .ok_or_else(|| import.position.error("Import needs a rule set"))?;
                    if let Some(input) = self.inputs.get(target) {
                        // A different definition has its own lexical root context.
                        self.compile_set(target, input.definition == definition && ignore_case)?;
                    }
                    self.append_import(&mut output, target, import.position)?;
                }
                XshdElement::RuleSet(child) => {
                    let target = self.index(child);
                    self.compile_set(target, ignore_case)?;
                    self.append_import(&mut output, target, child.position)?;
                }
                XshdElement::Color(_) | XshdElement::Property(_) => {}
            }
        }
        self.output[index] = output;
        self.status[index] = 2;
        self.compile_depth -= 1;
        Ok(())
    }

    fn append_import(
        &self,
        output: &mut HighlightingRuleSet,
        target: usize,
        position: XshdPosition,
    ) -> Result<(), HighlightingError> {
        let mut seen = HashSet::new();
        let mut pending = vec![target];
        while let Some(index) = pending.pop() {
            if seen.insert(index) {
                let set = &self.output[index];
                let total = output.rules.len() + output.spans.len() + set.rules.len() + set.spans.len();
                if total > MAX_RULES_PER_SET {
                    return Err(position.error("RuleSet imports expand to too many rules"));
                }
                output.rules.extend(set.rules.iter().cloned());
                output.spans.extend(set.spans.iter().cloned());
                pending.extend(set.imports.iter().rev().copied());
            }
        }
        Ok(())
    }

    fn external_definition(
        &mut self,
        name: &str,
        position: XshdPosition,
    ) -> Result<&mut ExternalDefinition, HighlightingError> {
        let missing = || position.reference_error(format!("Could not find definition {name}"));
        match self.external.entry(name.to_owned()) {
            Entry::Occupied(entry) => Ok(entry.into_mut()),
            Entry::Vacant(entry) => {
                let definition = self
                    .resolver
                    .ok_or_else(missing)?
                    .resolve(name)?
                    .ok_or_else(missing)?;
                Ok(entry.insert(ExternalDefinition {
                    definition,
                    rule_set_offset: None,
                }))
            }
        }
    }

    fn external_rule_set(
        &mut self,
        name: &str,
        element: &str,
        position: XshdPosition,
    ) -> Result<usize, HighlightingError> {
        let external = self.external_definition(name, position)?;
        let target = external
            .definition
            .named_rule_set_index(element)
            .ok_or_else(|| {
                position.reference_error(format!(
                    "Could not find rule set {element} in definition {name}"
                ))
            })?;
        if let Some(offset) = external.rule_set_offset {
            return Ok(offset + target);
        }

        let definition = external.definition.clone();
        let offset = self.output.len();
        let relocate = |index: usize| {
            if index < definition.rule_sets.len() {
                Ok(offset + index)
            } else {
                Err(position.reference_error(format!(
                    "Invalid rule set index {index} in definition {name}"
                )))
            }
        };
        let mut sets = definition.rule_sets.clone();
        for set in &mut sets {
            // Keep transitive helper sets private to their own definition.
            set.name = None;
            for span in &mut set.spans {
                span.rule_set = span.rule_set.map(relocate).transpose()?;
            }
            for import in &mut set.imports {
                *import = relocate(*import)?;
            }
        }
        self.output.extend(sets);
        self.external
            .get_mut(name)
            .expect("resolved definition")
            .rule_set_offset = Some(offset);
        Ok(offset + target)
    }

    fn resolve_set(
        &mut self,
        current: usize,
        reference: &XshdReference<XshdRuleSet>,
        position: XshdPosition,
    ) -> Result<Option<usize>, HighlightingError> {
        match reference {
            XshdReference::None => Ok(None),
            XshdReference::Inline(set) => Ok(Some(self.index(set))),
            XshdReference::Named {
                definition,
                element,
            } => {
                let target = match definition {
                    None => current,
                    Some(name) => match self.definition_names.get(name.as_str()) {
                        Some(&target) => target,
                        None => return self.external_rule_set(name, element, position).map(Some),
                    },
                };
                if element.is_empty() {
                    Ok(Some(self.main[target]))
                } else {
                    self.names
                        .get(&(target, element.as_str()))
                        .copied()
                        .map(Some)
                        .ok_or_else(|| {
                            position.reference_error(format!("Could not find rule set {element}"))
                        })
                }
            }
        }
    }

    fn color(
        &mut self,
        current: usize,
        reference: &XshdReference<XshdColor>,
        position: XshdPosition,
    ) -> Result<Option<HighlightingColor>, HighlightingError> {
        match reference {
            XshdReference::None => Ok(None),
            XshdReference::Inline(color) => {
                Ok((color.has_style() || color.name.is_some()).then(|| runtime_color(color)))
            }
            XshdReference::Named {
                definition,
                element,
            } => {
                let target = match definition {
                    None => current,
                    Some(name) => match self.definition_names.get(name.as_str()) {
                        Some(&target) => target,
                        None => {
                            let definition = &self.external_definition(name, position)?.definition;
                            return definition
                                .colors
                                .get(element)
                                .cloned()
                                .map(Some)
                                .ok_or_else(|| {
                                    position.reference_error(format!(
                                        "Could not find color {element} in definition {name}"
                                    ))
                                });
                        }
                    },
                };
                self.colors
                    .get(&(target, element.as_str()))
                    .map(|color| Some(runtime_color(color)))
                    .ok_or_else(|| {
                        position.reference_error(format!("Could not find color {element}"))
                    })
            }
        }
    }
}

fn validate_name(name: &str, position: XshdPosition) -> Result<(), HighlightingError> {
    if name.is_empty() || name.contains('/') {
        Err(position.error("Element names must be nonempty and cannot contain '/'"))
    } else {
        Ok(())
    }
}

fn runtime_color(color: &XshdColor) -> HighlightingColor {
    let font_weight = color.font_weight.as_deref().and_then(parse_font_weight);
    let font_style = color
        .font_style
        .as_deref()
        .and_then(HighlightingFontStyle::parse);
    HighlightingColor {
        name: color.name.clone(),
        foreground: color.foreground.clone(),
        background: color.background.clone(),
        bold: font_weight.map(|weight| weight >= 700),
        italic: font_style.map(|style| style != HighlightingFontStyle::Normal),
        underline: color.underline,
        strikethrough: color.strikethrough,
        font_family: color.font_family.clone(),
        font_weight,
        font_style,
        font_size: color.font_size,
    }
}

fn keyword_regex(keywords: &XshdKeywords) -> Result<String, HighlightingError> {
    if keywords.words.is_empty() || keywords.words.iter().any(String::is_empty) {
        return Err(keywords
            .position
            .error("Keywords must contain nonempty words"));
    }
    let simple = |word: &str| {
        word.chars().next().is_some_and(char::is_alphanumeric)
            && word.chars().next_back().is_some_and(char::is_alphanumeric)
    };
    if keywords.words.iter().all(|word| simple(word)) {
        let mut words: Vec<_> = keywords.words.iter().collect();
        words.sort_by_key(|word| std::cmp::Reverse(word.encode_utf16().count()));
        Ok(format!(
            r"\b(?>{})\b",
            words
                .into_iter()
                .map(|word| regex::escape(word))
                .collect::<Vec<_>>()
                .join("|")
        ))
    } else {
        Ok(format!(
            "(?:{})",
            keywords
                .words
                .iter()
                .map(|word| format!(
                    "{}{}{}",
                    if word.chars().next().is_some_and(char::is_alphanumeric) {
                        r"\b"
                    } else {
                        ""
                    },
                    regex::escape(word),
                    if word.chars().next_back().is_some_and(char::is_alphanumeric) {
                        r"\b"
                    } else {
                        ""
                    },
                ))
                .collect::<Vec<_>>()
                .join("|")
        ))
    }
}

fn compile_regex(
    pattern: Option<&str>,
    kind: XshdRegexType,
    ignore_case: bool,
    position: XshdPosition,
) -> Result<Regex, HighlightingError> {
    let pattern = pattern.ok_or_else(|| position.error("Regex missing"))?;
    let pattern = crate::regex_syntax::normalize(
        pattern,
        true,
        kind == XshdRegexType::IgnorePatternWhitespace,
    )
    .map_err(|error| position.pattern_error(error))?;
    RegexBuilder::new(&pattern.expression)
        .case_insensitive(ignore_case)
        .ignore_whitespace(kind == XshdRegexType::IgnorePatternWhitespace)
        .seek(pattern.seek_candidates())
        .backtrack_limit(1_000_000)
        .delegate_size_limit(16 * 1024 * 1024)
        .build()
        .map_err(|error| position.pattern_error(error))
}
