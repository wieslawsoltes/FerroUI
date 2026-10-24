use crate::{grammar::Program, Error, Grammar, RawGrammar, Result, Theme};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet, VecDeque},
    rc::Rc,
};

/// Host resource loading. Browser hosts can preload resources asynchronously and
/// implement these synchronous lookups without introducing I/O into the engine.
pub trait IRegistryOptions {
    fn get_grammar(&self, scope_name: &str) -> Result<Option<RawGrammar>>;
    fn get_injections(&self, _scope_name: &str) -> Result<Vec<String>> {
        Ok(Vec::new())
    }
    /// Returns JSON/JSONC or XML plist theme source.
    fn get_theme(&self, _name: &str) -> Result<Option<String>> {
        Ok(None)
    }
    fn get_default_theme(&self) -> Result<Option<String>> {
        Ok(None)
    }
}

#[derive(Default)]
pub struct DefaultLocator;
impl IRegistryOptions for DefaultLocator {
    fn get_grammar(&self, _scope_name: &str) -> Result<Option<RawGrammar>> {
        Ok(None)
    }
}

/// Existing grammar sources and explicitly registered injection dependencies.
pub trait IGrammarRepository {
    fn lookup(&self, scope_name: &str) -> Option<RawGrammar>;
    fn injections(&self, target_scope: &str) -> Vec<String>;
}

pub trait IThemeProvider {
    fn theme(&self) -> &Theme;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrammarLoadDiagnostic {
    pub scope_name: String,
    pub error: Error,
}

/// Owns grammar sources and compiled registries. Replacing a source invalidates
/// future loads; existing grammar handles retain their immutable program.
#[derive(Default)]
pub struct Registry {
    grammars: RefCell<BTreeMap<String, RawGrammar>>,
    injection_targets: RefCell<BTreeMap<String, Vec<String>>>,
    program: RefCell<Option<Rc<Program>>>,
    compiled: RefCell<BTreeMap<String, Grammar>>,
    theme: Theme,
    options: Option<Rc<dyn IRegistryOptions>>,
    loading: RefCell<BTreeSet<String>>,
    diagnostics: RefCell<Vec<GrammarLoadDiagnostic>>,
}
impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_options(options: Rc<dyn IRegistryOptions>) -> Result<Self> {
        let mut registry = Self {
            options: Some(options.clone()),
            ..Self::default()
        };
        if let Some(source) = options.get_default_theme()? {
            registry.theme = registry.parse_theme(&source)?;
        }
        Ok(registry)
    }

    pub fn options(&self) -> Option<Rc<dyn IRegistryOptions>> {
        self.options.clone()
    }

    pub fn add_grammar_json(&mut self, source: &str) -> Result<()> {
        self.add_grammar(RawGrammar::from_json(source)?);
        Ok(())
    }
    pub fn add_grammar(&mut self, grammar: RawGrammar) {
        self.register(grammar);
    }
    fn register(&self, grammar: RawGrammar) {
        self.grammars
            .borrow_mut()
            .insert(grammar.scope_name.clone(), grammar);
        self.invalidate_program();
    }
    fn invalidate_program(&self) {
        *self.program.borrow_mut() = None;
        self.compiled.borrow_mut().clear();
    }

    pub fn set_injections(
        &mut self,
        host_scope: impl Into<String>,
        injections: impl IntoIterator<Item = String>,
    ) {
        self.injection_targets
            .get_mut()
            .insert(host_scope.into(), injections.into_iter().collect());
        self.invalidate_program();
    }

    /// Owned names allow the iterator to outlive a subsequent lazy load.
    pub fn scope_names(&self) -> impl Iterator<Item = String> {
        self.grammars
            .borrow()
            .keys()
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
    }
    pub fn lookup(&self, scope: &str) -> Option<RawGrammar> {
        self.grammars.borrow().get(scope).cloned()
    }
    pub fn injections(&self, scope: &str) -> Vec<String> {
        self.injection_targets
            .borrow()
            .get(scope)
            .cloned()
            .unwrap_or_default()
    }
    pub fn dependencies(&self, scope: &str) -> Option<Vec<String>> {
        self.lookup(scope).map(|grammar| {
            let mut scopes: BTreeSet<_> = grammar.dependencies().into_iter().collect();
            scopes.extend(self.injections(scope));
            scopes.into_iter().collect()
        })
    }
    pub fn diagnostics(&self) -> Vec<GrammarLoadDiagnostic> {
        self.diagnostics.borrow().clone()
    }

    pub fn load_grammar(&self, scope: &str) -> Result<Grammar> {
        if let Some(grammar) = self.compiled.borrow().get(scope) {
            return Ok(grammar.clone());
        }
        self.load_dependencies(scope)?;
        let program = self.program.borrow().clone();
        let program = match program {
            Some(program) => program,
            None => {
                let program = Rc::new(Program::compile(
                    &self.grammars.borrow(),
                    &self.injection_targets.borrow(),
                )?);
                *self.program.borrow_mut() = Some(program.clone());
                program
            }
        };
        let grammar = Grammar::new(program, scope)?;
        self.compiled
            .borrow_mut()
            .insert(scope.into(), grammar.clone());
        Ok(grammar)
    }

    fn load_dependencies(&self, initial_scope: &str) -> Result<()> {
        if !self.loading.borrow_mut().insert(initial_scope.into()) {
            return Err(Error::ResourceCycle(initial_scope.into()));
        }
        struct LoadingScope<'a> {
            scopes: &'a RefCell<BTreeSet<String>>,
            name: &'a str,
        }
        impl Drop for LoadingScope<'_> {
            fn drop(&mut self) {
                self.scopes.borrow_mut().remove(self.name);
            }
        }
        let _guard = LoadingScope {
            scopes: &self.loading,
            name: initial_scope,
        };
        let mut pending = VecDeque::from([initial_scope.to_owned()]);
        let mut seen = BTreeSet::new();
        while let Some(scope) = pending.pop_front() {
            if !seen.insert(scope.clone()) {
                continue;
            }
            let load = || -> Result<Vec<String>> {
                let existing = self.lookup(&scope);
                let grammar = match existing {
                    Some(grammar) => grammar,
                    None => {
                        let grammar = match &self.options {
                            Some(options) => options.get_grammar(&scope)?,
                            None => None,
                        }
                        .ok_or_else(|| Error::MissingGrammar(scope.clone()))?;
                        if grammar.scope_name != scope {
                            return Err(Error::InvalidGrammar(format!(
                                "loader returned {} for {scope}",
                                grammar.scope_name
                            )));
                        }
                        self.register(grammar.clone());
                        grammar
                    }
                };
                // Drop each interior borrow before invoking application code.
                let configured = self.injection_targets.borrow().contains_key(&scope);
                if !configured {
                    if let Some(options) = &self.options {
                        let injections = options.get_injections(&scope)?;
                        self.injection_targets
                            .borrow_mut()
                            .insert(scope.clone(), injections);
                        self.invalidate_program();
                    }
                }
                let mut dependencies = grammar.dependencies();
                dependencies.extend(self.injections(&scope));
                Ok(dependencies)
            };
            match load() {
                Ok(dependencies) => pending.extend(dependencies),
                Err(error) if scope == initial_scope => return Err(error),
                Err(error) => {
                    let diagnostic = GrammarLoadDiagnostic {
                        scope_name: scope,
                        error,
                    };
                    let mut diagnostics = self.diagnostics.borrow_mut();
                    if !diagnostics.contains(&diagnostic) {
                        diagnostics.push(diagnostic);
                    }
                }
            }
        }
        Ok(())
    }

    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
    }
    pub fn set_theme_json(&mut self, source: &str) -> Result<()> {
        self.theme = self.parse_theme(source)?;
        Ok(())
    }
    pub fn load_theme(&self, name: &str) -> Result<Theme> {
        let source = self
            .options
            .as_ref()
            .ok_or_else(|| Error::MissingResource(name.into()))?
            .get_theme(name)?
            .ok_or_else(|| Error::MissingResource(name.into()))?;
        self.parse_theme(&source)
    }
    fn parse_theme(&self, source: &str) -> Result<Theme> {
        Theme::from_json_with_resolver(source, &|name| {
            self.options
                .as_ref()
                .ok_or_else(|| Error::MissingResource(name.into()))?
                .get_theme(name)?
                .ok_or_else(|| Error::MissingResource(name.into()))
        })
    }
    pub fn theme(&self) -> &Theme {
        &self.theme
    }
}
impl IGrammarRepository for Registry {
    fn lookup(&self, scope_name: &str) -> Option<RawGrammar> {
        self.lookup(scope_name)
    }
    fn injections(&self, target_scope: &str) -> Vec<String> {
        self.injections(target_scope)
    }
}
impl IThemeProvider for Registry {
    fn theme(&self) -> &Theme {
        self.theme()
    }
}

impl Grammar {
    /// Builds a self-contained program from a raw grammar and its repository.
    pub fn from_repository(
        grammar: RawGrammar,
        repository: &dyn IGrammarRepository,
    ) -> Result<Self> {
        let scope = grammar.scope_name.clone();
        let mut registry = Registry::new();
        let mut pending = VecDeque::from([grammar]);
        let mut seen = BTreeSet::new();
        while let Some(grammar) = pending.pop_front() {
            if !seen.insert(grammar.scope_name.clone()) {
                continue;
            }
            let injections = repository.injections(&grammar.scope_name);
            for dependency in grammar
                .dependencies()
                .into_iter()
                .chain(injections.iter().cloned())
            {
                if !seen.contains(&dependency) {
                    if let Some(grammar) = repository.lookup(&dependency) {
                        pending.push_back(grammar);
                    }
                }
            }
            registry.set_injections(grammar.scope_name.clone(), injections);
            registry.add_grammar(grammar);
        }
        registry.load_grammar(&scope)
    }
}
