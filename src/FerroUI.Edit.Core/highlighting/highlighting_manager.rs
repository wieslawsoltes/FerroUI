use super::xshd::XshdSyntaxDefinition;
use super::{HighlightingDefinition, HighlightingError};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::rc::Rc;

/// Lazily compiles embedded and application-provided XSHD definitions. Only the
/// transitive dependencies of the requested language are compiled.
pub struct HighlightingManager {
    sources: BTreeMap<String, Rc<XshdSyntaxDefinition>>,
    extensions: HashMap<String, String>,
    cache: RefCell<HashMap<String, Rc<HighlightingDefinition>>>,
}
impl Default for HighlightingManager {
    fn default() -> Self {
        Self::new()
    }
}
impl HighlightingManager {
    pub fn new() -> Self {
        let mut manager = Self::empty();
        for source in BUILTINS {
            manager
                .register_highlighting(source)
                .expect("embedded highlighting metadata is validated by tests");
        }
        manager
    }
    pub fn empty() -> Self {
        Self {
            sources: BTreeMap::new(),
            extensions: HashMap::new(),
            cache: RefCell::new(HashMap::new()),
        }
    }
    pub fn highlighting_names(&self) -> impl ExactSizeIterator<Item = &str> {
        self.sources.keys().map(String::as_str)
    }
    pub fn register_highlighting(&mut self, source: &str) -> Result<String, HighlightingError> {
        self.register_syntax_definition(XshdSyntaxDefinition::load(source)?)
    }

    /// Registers an editable definition without compiling its expressions.
    /// Replacing a name invalidates cached dependents and its old extensions.
    pub fn register_syntax_definition(
        &mut self,
        syntax: XshdSyntaxDefinition,
    ) -> Result<String, HighlightingError> {
        let name = syntax
            .name
            .as_deref()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                HighlightingError::InvalidDefinition("Registered definitions need a name".into())
            })?
            .to_owned();
        self.extensions.retain(|_, value| value != &name);
        for extension in syntax.extensions.iter().filter(|s| !s.is_empty()) {
            self.extensions
                .insert(extension.to_lowercase(), name.clone());
        }
        self.sources.insert(name.clone(), Rc::new(syntax));
        self.cache.borrow_mut().clear();
        Ok(name)
    }
    pub fn get_definition(
        &self,
        name: &str,
    ) -> Result<Option<Rc<HighlightingDefinition>>, HighlightingError> {
        if let Some(cached) = self.cache.borrow().get(name).cloned() {
            return Ok(Some(cached));
        }
        let Some(source) = self.sources.get(name) else {
            return Ok(None);
        };
        let mut visited = BTreeSet::new();
        let mut pending = vec![name.to_owned()];
        let mut dependencies = Vec::new();
        while let Some(current) = pending.pop() {
            if !visited.insert(current.clone()) {
                continue;
            }
            let current_source = self
                .sources
                .get(&current)
                .ok_or_else(|| HighlightingError::Reference(current.clone()))?;
            for dependency in current_source.external_definitions() {
                if !visited.contains(dependency) {
                    pending.push(dependency.to_owned());
                }
            }
            if current != name {
                dependencies.push(current_source.as_ref());
            }
        }
        let definition = Rc::new(source.compile_with_dependencies(&dependencies)?);
        self.cache
            .borrow_mut()
            .insert(name.to_owned(), definition.clone());
        Ok(Some(definition))
    }
    pub fn get_definition_by_extension(
        &self,
        extension: &str,
    ) -> Result<Option<Rc<HighlightingDefinition>>, HighlightingError> {
        let normalized = if extension.starts_with('.') {
            extension.to_lowercase()
        } else if let Some((_, ext)) = extension.rsplit_once('.') {
            format!(".{}", ext.to_lowercase())
        } else {
            format!(".{}", extension.to_lowercase())
        };
        match self.extensions.get(&normalized) {
            Some(name) => self.get_definition(name),
            None => Ok(None),
        }
    }
}

const BUILTINS: &[&str] = &[
    include_str!("resources/ASPX.xshd"),
    include_str!("resources/Boo.xshd"),
    include_str!("resources/Coco-Mode.xshd"),
    include_str!("resources/CPP-Mode.xshd"),
    include_str!("resources/CSharp-Mode.xshd"),
    include_str!("resources/CSS-Mode.xshd"),
    include_str!("resources/HTML-Mode.xshd"),
    include_str!("resources/Java-Mode.xshd"),
    include_str!("resources/JavaScript-Mode.xshd"),
    include_str!("resources/Json.xshd"),
    include_str!("resources/MarkDown-Mode.xshd"),
    include_str!("resources/MarkDownWithFontSize-Mode.xshd"),
    include_str!("resources/Patch-Mode.xshd"),
    include_str!("resources/PHP-Mode.xshd"),
    include_str!("resources/PowerShell.xshd"),
    include_str!("resources/Python-Mode.xshd"),
    include_str!("resources/Tex-Mode.xshd"),
    include_str!("resources/TSQL-Mode.xshd"),
    include_str!("resources/VB-Mode.xshd"),
    include_str!("resources/XML-Mode.xshd"),
    include_str!("resources/XmlDoc.xshd"),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::highlighting::{HighlightingEngine, HighlightingState};

    fn foreground(definition: Rc<HighlightingDefinition>, text: &str) -> Option<String> {
        HighlightingEngine::new(definition)
            .highlight_line(text, 1, 0, &HighlightingState::default())
            .unwrap()
            .sections
            .first()
            .and_then(|section| section.color.foreground.clone())
    }

    #[test]
    fn typed_registry_resolves_legacy_dependencies_and_extension_separators() {
        let mut manager = HighlightingManager::empty();
        manager.register_highlighting(r#"<SyntaxDefinition name="Dependency/Language"><RuleSet><Rule foreground="Red">x</Rule></RuleSet></SyntaxDefinition>"#).unwrap();
        manager.register_highlighting(r#"<SyntaxDefinition name="Legacy" extensions=".old|.alt;.third" extends="Dependency/Language"><RuleSets><RuleSet/></RuleSets></SyntaxDefinition>"#).unwrap();
        manager.register_highlighting(r#"<SyntaxDefinition name="Referenced"><RuleSets><RuleSet reference="Dependency/Language"/></RuleSets></SyntaxDefinition>"#).unwrap();
        for name in ["Legacy", "Referenced"] {
            assert_eq!(
                foreground(manager.get_definition(name).unwrap().unwrap(), "x"),
                Some("Red".into())
            );
        }
        for extension in [".old", ".alt", ".third", "file.OLD"] {
            assert_eq!(
                manager
                    .get_definition_by_extension(extension)
                    .unwrap()
                    .unwrap()
                    .name,
                "Legacy"
            );
        }
    }

    #[test]
    fn replacing_typed_definition_invalidates_dependents_and_preserves_lazy_compilation() {
        let mut manager = HighlightingManager::empty();
        manager.register_highlighting(r#"<SyntaxDefinition name="Base" extensions=".old"><RuleSet><Rule foreground="Red">x</Rule></RuleSet></SyntaxDefinition>"#).unwrap();
        manager.register_highlighting(r#"<SyntaxDefinition name="Dependent"><RuleSet><Import ruleSet="Base/"/></RuleSet></SyntaxDefinition>"#).unwrap();
        manager.register_highlighting(r#"<SyntaxDefinition name="Unrelated"><RuleSet><Rule>(</Rule></RuleSet></SyntaxDefinition>"#).unwrap();
        let first = manager.get_definition("Dependent").unwrap().unwrap();
        assert!(Rc::ptr_eq(
            &first,
            &manager.get_definition("Dependent").unwrap().unwrap()
        ));
        assert_eq!(foreground(first.clone(), "x"), Some("Red".into()));
        let replacement = XshdSyntaxDefinition::load(r#"<SyntaxDefinition name="Base" extensions=".new"><RuleSet><Rule foreground="Blue">x</Rule></RuleSet></SyntaxDefinition>"#).unwrap();
        manager.register_syntax_definition(replacement).unwrap();
        let second = manager.get_definition("Dependent").unwrap().unwrap();
        assert!(!Rc::ptr_eq(&first, &second));
        assert_eq!(foreground(second, "x"), Some("Blue".into()));
        assert!(manager
            .get_definition_by_extension(".old")
            .unwrap()
            .is_none());
        assert_eq!(
            manager
                .get_definition_by_extension(".new")
                .unwrap()
                .unwrap()
                .name,
            "Base"
        );
        assert!(matches!(
            manager.get_definition("Unrelated"),
            Err(HighlightingError::Pattern(_))
        ));
        assert!(manager
            .register_highlighting(r#"<SyntaxDefinition name="Base"><Unknown/></SyntaxDefinition>"#)
            .is_err());
        assert_eq!(
            foreground(manager.get_definition("Dependent").unwrap().unwrap(), "x"),
            Some("Blue".into())
        );
    }

    #[test]
    fn missing_external_colors_are_resolved_through_the_registry() {
        let mut manager = HighlightingManager::empty();
        manager.register_highlighting(r#"<SyntaxDefinition name="Consumer"><RuleSet><Span begin="\[" end="\]"><RuleSet><Rule color="Colors/Word">x</Rule></RuleSet></Span></RuleSet></SyntaxDefinition>"#).unwrap();
        assert!(matches!(
            manager.get_definition("Consumer"),
            Err(HighlightingError::Reference(_))
        ));
        manager.register_highlighting(r#"<SyntaxDefinition name="Colors"><Color name="Word" foreground="Green"/><RuleSet/></SyntaxDefinition>"#).unwrap();
        assert_eq!(
            foreground(manager.get_definition("Consumer").unwrap().unwrap(), "[x]"),
            Some("Green".into())
        );
    }

    #[test]
    fn all_bundled_definitions_compile_and_execute() {
        let manager = HighlightingManager::new();
        let mut failures = Vec::new();
        for name in manager.highlighting_names() {
            match manager.get_definition(name) {
                Ok(Some(definition)) => {
                    let engine = HighlightingEngine::new(definition);
                    for line in [
                        "",
                        "// comment",
                        "let value = \"😀\";",
                        "<tag a=\"x\">",
                        "/* comment */",
                    ] {
                        if let Err(error) =
                            engine.highlight_line(line, 1, 0, &HighlightingState::default())
                        {
                            failures.push(format!("{name}: {error}"));
                        }
                    }
                }
                Ok(None) => failures.push(format!("{name}: missing")),
                Err(error) => failures.push(format!("{name}: {error}")),
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
        assert_eq!(manager.highlighting_names().len(), 21);
    }
}
