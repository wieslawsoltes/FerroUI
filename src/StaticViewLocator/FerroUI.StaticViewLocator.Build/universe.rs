//! The types the generator knows (the `Compilation` of the reference): the structs, enumerations
//! and traits of the crate, with what the scanner of the framework knows of its classes (base
//! class, constructor), the traits each type implements and the contracts of its trait `impl`
//! blocks, and the types of the models of the crates it depends on.
//!
//! A type is known by its key: the absolute path of its declaration without generic arguments
//! (`::my_app::views::SettingsView`), made canonical through the models of the dependencies, so
//! that every spelling of a path to one type is one key.

use crate::diagnostics::Location;
use crate::source::{ItemKind, SourceCrate, Visibility};
use ferroui_build_scan::model::{AssemblyModel, TypeKind};
use ferroui_build_scan::model_set::ModelSet;
use ferroui_build_scan::scanner::Scan;
use std::collections::{BTreeMap, BTreeSet};

/// Whether a type is a class (a struct, an enumeration, a class of the object model) or an
/// interface (a trait).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Class,
    Interface,
}

/// A contract a type implements: a trait `impl` block with the type its single generic argument
/// or its single associated type names.
#[derive(Clone, Debug)]
pub struct ContractImpl {
    /// The key of the trait.
    pub contract: String,
    /// The type of the generic argument or of the associated type; `None` when the block has
    /// not exactly one of them.
    pub view_model: Option<TypeRef>,
}

/// A type as the sources name it: its key, its canonical text (generic arguments included) and
/// the text generated source names it by.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypeRef {
    /// The key of the declaration (generic arguments left out).
    pub key: String,
    /// The canonical text of the type, generic arguments included: the identity of a
    /// constructed generic type (`::my_app::Widget<i32>`).
    pub canonical: String,
    /// The text generated source names the type by, from the module of the locator.
    pub code: String,
    /// Whether the type has generic arguments (a constructed generic type).
    pub constructed: bool,
    /// Whether the scanner resolved every path of the type.
    pub resolved: bool,
}

impl TypeRef {
    /// The display string the reference sorts by (`ToDisplayString()`): the canonical text
    /// without the leading `::`.
    pub fn display(&self) -> &str {
        self.canonical.trim_start_matches("::")
    }
}

/// A type the generator knows.
#[derive(Clone, Debug)]
pub struct TypeDecl {
    pub key: String,
    pub name: String,
    /// The absolute module path of the declaration (`my_app::views`).
    pub module: String,
    pub kind: Kind,
    /// The names of the generic parameters.
    pub generics: Vec<String>,
    /// The number of associated types of a trait of the crate.
    pub associated_types: usize,
    /// Whether a trait of the crate can be the type of a trait object (`dyn Trait`).
    pub dyn_compatible: bool,
    /// Declared in the crate the generator runs for.
    pub local: bool,
    pub visibility: Visibility,
    /// The path other crates name the type by, for a type of a dependency.
    pub public_path: Option<String>,
    /// A class of the object model (`ferro_class!`): a value of it is a `Ref<T>`.
    pub object_model: bool,
    /// The key of the base class.
    pub base: Option<String>,
    /// The keys of the traits the type implements directly, in the order of the sources, and of
    /// the supertraits of a trait.
    pub interfaces: Vec<String>,
    /// The constructor without parameters: the text generated source calls (without `()`), with
    /// the module and the visibility of the function when the sources state them.
    pub constructor: Option<(String, Option<(String, Visibility)>)>,
    pub contracts: Vec<ContractImpl>,
    pub location: Location,
}

impl TypeDecl {
    pub fn is_generic(&self) -> bool {
        !self.generics.is_empty()
    }
}

/// The types the generator knows.
pub struct Universe<'a> {
    pub crate_name: String,
    scan: &'a Scan,
    models: &'a ModelSet,
    pub source: &'a SourceCrate,
    types: BTreeMap<String, TypeDecl>,
    /// The crates the crate depends on directly: the ones generated source can name.
    direct_crates: BTreeSet<String>,
}

impl<'a> Universe<'a> {
    pub fn new(crate_name: &str, scan: &'a Scan, models: &'a ModelSet, source: &'a SourceCrate, direct_crates: BTreeSet<String>) -> Self {
        let mut universe = Self { crate_name: crate_name.to_string(), scan, models, source, types: BTreeMap::new(), direct_crates };
        universe.read_sources();
        universe.read_dependencies();
        universe
    }

    /// The type of the key `key`.
    pub fn get(&self, key: &str) -> Option<&TypeDecl> {
        self.types.get(key)
    }

    /// Every type, by key.
    pub fn types(&self) -> impl Iterator<Item = &TypeDecl> {
        self.types.values()
    }

    /// The type `text`, written in the module `module` of the crate (`FindTypeByMetadataName` of a
    /// type of the sources, `typeof(..)` of an attribute).
    pub fn resolve(&self, module: &str, text: &str) -> TypeRef {
        let (normalised, resolved) = match self.scan.normalise(module, text) {
            Some(type_) => (type_.text.clone(), type_.is_resolved()),
            None => (text.replace(' ', ""), false),
        };
        self.type_ref(&normalised, resolved)
    }

    /// The reference of the normalised type text `normalised`.
    fn type_ref(&self, normalised: &str, resolved: bool) -> TypeRef {
        let canonical = self.models.canonical(normalised);
        let key = strip_generic_arguments(&canonical);
        TypeRef { constructed: key.len() != canonical.len(), key, code: self.code_of(normalised), canonical, resolved }
    }

    /// The reference of the type with the key `key` (a type the universe knows).
    pub fn type_ref_of(&self, key: &str) -> TypeRef {
        let code = match self.types.get(key) {
            Some(decl) if !decl.local => decl.public_path.clone().unwrap_or_else(|| key.to_string()),
            _ => self.code_of(key),
        };
        TypeRef { key: key.to_string(), canonical: key.to_string(), code, constructed: false, resolved: true }
    }

    /// `text` with the absolute paths into the crate written from `crate`.
    pub fn code_of(&self, text: &str) -> String {
        let prefix = format!("::{}::", self.crate_name);
        let mut code = String::with_capacity(text.len());
        let mut index = 0;
        while let Some(found) = text[index..].find(&prefix) {
            let at = index + found;
            // `a::my_app::b` is a module of that name, not the crate.
            let joined = text[..at].chars().last().is_some_and(|character| character.is_alphanumeric() || character == '_');
            code.push_str(&text[index..at]);
            code.push_str(if joined { &prefix } else { "crate::" });
            index = at + prefix.len();
        }
        code.push_str(&text[index..]);
        code
    }

    /// The key of a type written as an absolute path of a model, canonical.
    fn canonical_key(&self, text: &str) -> String {
        strip_generic_arguments(&self.models.canonical(text))
    }

    /// Whether the type of `key` is visible from the module `from` of the crate, with every module
    /// on its path (`IsSymbolAccessibleWithin`).
    pub fn is_accessible(&self, decl: &TypeDecl, from: &str) -> bool {
        if decl.local {
            return self.source.is_accessible(&decl.module, &decl.visibility, from);
        }
        decl.public_path.as_deref().is_some_and(|path| {
            let crate_name = path.trim_start_matches("::").split("::").next().unwrap_or_default();
            self.direct_crates.contains(crate_name)
        })
    }

    /// Whether the constructor of `decl` can be called from the module `from`.
    pub fn is_constructor_accessible(&self, decl: &TypeDecl, from: &str) -> bool {
        match &decl.constructor {
            Some((_, Some((module, visibility)))) => self.source.is_accessible(module, visibility, from),
            Some((_, None)) => true,
            None => false,
        }
    }

    /// The type and its base classes, nearest first, as far as the universe knows them; the key
    /// of a base class the universe does not know ends the chain.
    pub fn base_chain(&self, key: &str) -> Vec<String> {
        let mut chain = Vec::new();
        let mut current = self.types.get(key).and_then(|decl| decl.base.clone());
        while let Some(base) = current {
            if chain.contains(&base) || chain.len() > 64 {
                break;
            }
            current = self.types.get(&base).and_then(|decl| decl.base.clone());
            chain.push(base);
        }
        chain
    }

    /// Every trait the type implements (`AllInterfaces`): its own, the ones of its base classes,
    /// and their supertraits, each once, in that order.
    pub fn all_interfaces(&self, key: &str) -> Vec<String> {
        let mut all: Vec<String> = Vec::new();
        let mut pending: Vec<String> = Vec::new();
        let mut owners = vec![key.to_string()];
        owners.extend(self.base_chain(key));
        for owner in &owners {
            if let Some(decl) = self.types.get(owner) {
                pending.extend(decl.interfaces.iter().cloned());
            }
        }
        let mut index = 0;
        while index < pending.len() {
            let interface = pending[index].clone();
            index += 1;
            if all.contains(&interface) || interface == key {
                continue;
            }
            if let Some(decl) = self.types.get(&interface) {
                pending.extend(decl.interfaces.iter().cloned());
            }
            all.push(interface);
        }
        all
    }

    fn read_sources(&mut self) {
        for type_ in &self.source.types {
            let key = format!("::{}::{}", type_.module, type_.name);
            self.types.entry(key.clone()).or_insert_with(|| TypeDecl {
                key,
                name: type_.name.clone(),
                module: type_.module.clone(),
                kind: if type_.kind == ItemKind::Trait { Kind::Interface } else { Kind::Class },
                generics: type_.generics.clone(),
                associated_types: type_.associated_types,
                dyn_compatible: type_.dyn_compatible,
                local: true,
                visibility: type_.visibility.clone(),
                public_path: None,
                object_model: false,
                base: None,
                interfaces: Vec::new(),
                constructor: None,
                contracts: Vec::new(),
                location: Location::new(&type_.file, type_.line),
            });
        }
        // The supertraits of the traits.
        for type_ in self.source.types.iter().filter(|type_| type_.kind == ItemKind::Trait) {
            let key = format!("::{}::{}", type_.module, type_.name);
            let supertraits: Vec<String> = type_.supertraits.iter().map(|supertrait| self.resolve(&type_.module, supertrait).key).collect();
            if let Some(decl) = self.types.get_mut(&key) {
                decl.interfaces = supertraits;
            }
        }
        // What the scanner knows of the classes and markup types of the crate. A class declared by
        // a macro of the crate is known to the scanner only, which expands those macros.
        for model_type in &self.scan.model.types {
            if !model_type.rust_path.is_resolved() {
                continue;
            }
            let key = self.canonical_key(&model_type.rust_path.text);
            if model_type.object_model && model_type.kind == TypeKind::Class && !self.types.contains_key(&key) {
                let name = key.rsplit("::").next().unwrap_or_default().to_string();
                self.types.insert(
                    key.clone(),
                    TypeDecl {
                        key: key.clone(),
                        name,
                        module: model_type.module.clone(),
                        kind: Kind::Class,
                        generics: Vec::new(),
                        associated_types: 0,
                        dyn_compatible: false,
                        local: true,
                        visibility: Visibility::Public,
                        public_path: None,
                        object_model: true,
                        base: None,
                        interfaces: Vec::new(),
                        constructor: None,
                        contracts: Vec::new(),
                        location: Location::default(),
                    },
                );
            }
            let base = model_type.base.as_ref().filter(|base| base.is_resolved()).map(|base| self.canonical_key(&base.text));
            let constructor = model_type.default_constructor.as_ref().and_then(|callable| callable.resolved.clone()).map(|path| self.code_of(&path));
            if let Some(decl) = self.types.get_mut(&key) {
                decl.object_model |= model_type.object_model;
                if decl.base.is_none() {
                    decl.base = base;
                }
                if decl.constructor.is_none() {
                    decl.constructor = constructor.map(|path| (path, None));
                }
            }
        }
        // The `impl` blocks.
        for block in &self.source.impls {
            let self_type = self.resolve(&block.module, &block.self_type);
            if !self.types.contains_key(&self_type.key) {
                continue;
            }
            match &block.trait_path {
                None => {
                    let new = block.methods.iter().find(|method| {
                        method.name == "new"
                            && method.receiver.is_none()
                            && method.parameters.is_empty()
                            && method.return_type.as_deref().is_some_and(|returned| returned.replace(' ', "").starts_with("Ref<"))
                    });
                    if let Some(new) = new {
                        let code = format!("{}::new", self.type_ref_of(&self_type.key).code);
                        let decl = self.types.get_mut(&self_type.key).expect("the type is known");
                        decl.constructor = Some((code, Some((block.module.clone(), new.visibility.clone()))));
                    }
                }
                Some(trait_path) => {
                    let contract = self.resolve(&block.module, trait_path).key;
                    let generic_parameters: BTreeSet<&str> = block.generics.iter().map(String::as_str).collect();
                    let mentions_parameter = |text: &str| {
                        text.split(|character: char| !character.is_alphanumeric() && character != '_').any(|word| generic_parameters.contains(word))
                    };
                    let view_model = match (block.trait_arguments.as_slice(), block.associated_types.as_slice()) {
                        ([argument], []) => Some(argument.clone()),
                        ([], [(_, type_)]) => Some(type_.clone()),
                        _ => None,
                    };
                    let view_model = view_model.filter(|text| !mentions_parameter(text)).map(|text| self.resolve(&block.module, &text));
                    let decl = self.types.get_mut(&self_type.key).expect("the type is known");
                    if !decl.interfaces.contains(&contract) {
                        decl.interfaces.push(contract.clone());
                    }
                    decl.contracts.push(ContractImpl { contract, view_model });
                }
            }
        }
    }

    fn read_dependencies(&mut self) {
        let crate_name = self.crate_name.clone();
        let models: Vec<&AssemblyModel> = self.models.models().iter().filter(|model| model.crate_name != crate_name).collect();
        for model in models {
            for model_type in &model.types {
                if !model_type.rust_path.is_resolved() {
                    continue;
                }
                let kind = match model_type.kind {
                    TypeKind::Class | TypeKind::Struct | TypeKind::Enum => Kind::Class,
                    TypeKind::Interface => Kind::Interface,
                    TypeKind::Static => continue,
                };
                let key = self.canonical_key(&model_type.rust_path.text);
                if self.types.contains_key(&key) {
                    continue;
                }
                let public_path = model_type.public_path.clone();
                let constructor = match (&public_path, &model_type.default_constructor) {
                    (Some(public_path), Some(callable)) => callable
                        .path
                        .as_deref()
                        .and_then(|path| path.rsplit("::").next())
                        .filter(|name| name.chars().all(|character| character.is_alphanumeric() || character == '_'))
                        .map(|name| (format!("{public_path}::{name}"), None)),
                    _ => None,
                };
                let name = key.rsplit("::").next().unwrap_or_default().to_string();
                let decl = TypeDecl {
                    key: key.clone(),
                    name,
                    module: model_type.module.clone(),
                    kind,
                    // A generic type of a model is an instantiation (`stated_path`), not a definition.
                    generics: Vec::new(),
                    associated_types: 0,
                    dyn_compatible: false,
                    local: false,
                    visibility: Visibility::Public,
                    public_path,
                    object_model: model_type.object_model,
                    base: model_type.base.as_ref().filter(|base| base.is_resolved()).map(|base| self.canonical_key(&base.text)),
                    interfaces: Vec::new(),
                    constructor,
                    contracts: Vec::new(),
                    location: Location::default(),
                };
                self.types.insert(key, decl);
            }
        }
    }
}

/// `text` without the generic arguments of its last segment (`::a::Widget` of
/// `::a::Widget<::b::C>`).
pub fn strip_generic_arguments(text: &str) -> String {
    match text.find('<') {
        Some(at) => text[..at].to_string(),
        None => text.to_string(),
    }
}
