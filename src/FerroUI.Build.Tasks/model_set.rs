//! The models of several crates read together: the model of a crate and the
//! models of the crates it is built on (docs/porting/xaml.md, 9.5.1 and
//! 9.6.3).
//!
//! One crate's model names the types of another by the paths its own sources
//! spell (`::ferroui_base::Ref`, a re-export), the other crate's model by the
//! module that declares them (`::ferroui_base::type_system::Ref`). The export
//! tables of the models ([`AssemblyModel::exports`]) make the two one text
//! ([`ModelSet::canonical`]), so that a type, a handle or the accessor of a
//! property is found whichever crate names it.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::model::{AssemblyModel, RegisteredModel, RegistrationModel, TypeModel};

/// How many times a path is followed through export tables (a crate exports a type of
/// another crate again) and an accessor through the accessors it is an owner or an alias of.
const MAX_HOPS: usize = 8;

/// The models of a set of crates, with what is looked up across them.
pub struct ModelSet {
    models: Vec<AssemblyModel>,
    /// Every public path of every model, with the path of the declaring module.
    exports: HashMap<String, String>,
    /// The types by the text of their Rust type, the first of a text.
    types: HashMap<String, (usize, usize)>,
}

impl ModelSet {
    /// The set of `models`. Of two types with one Rust type the first is the type.
    pub fn new(models: Vec<AssemblyModel>) -> Self {
        let mut exports = HashMap::new();
        for model in &models {
            for export in &model.exports {
                exports.entry(export.path.clone()).or_insert_with(|| export.declared.clone());
            }
        }
        let mut set = Self { models, exports, types: HashMap::new() };
        let mut types = HashMap::new();
        for (model_index, model) in set.models.iter().enumerate() {
            for (type_index, type_) in model.types.iter().enumerate() {
                types.entry(set.canonical(&type_.rust_path.text)).or_insert((model_index, type_index));
            }
        }
        set.types = types;
        set
    }

    pub fn models(&self) -> &[AssemblyModel] {
        &self.models
    }

    pub fn is_empty(&self) -> bool {
        self.models.is_empty()
    }

    /// The type at the position `(model, type)` of the set.
    pub fn type_at(&self, position: (usize, usize)) -> &TypeModel {
        &self.models[position.0].types[position.1]
    }

    /// The path `path` (`::ferroui_base::Border::child_property`) with its longest prefix
    /// that a model exports replaced by the path of the declaring module; `path` itself
    /// when no model exports a prefix of it.
    pub fn canonical_path(&self, path: &str) -> String {
        let mut current = path.to_string();
        for _ in 0..MAX_HOPS {
            match self.declared(&current) {
                Some(next) if next != current => current = next,
                _ => break,
            }
        }
        current
    }

    fn declared(&self, path: &str) -> Option<String> {
        let mut end = path.len();
        while end > 0 {
            if let Some(declared) = self.exports.get(&path[..end]) {
                return Some(format!("{declared}{}", &path[end..]));
            }
            end = path[..end].rfind("::")?;
        }
        None
    }

    /// The normalised type text `text` with every absolute path made canonical
    /// ([`canonical_path`](Self::canonical_path)): the text the model of the declaring
    /// crate has for the same type.
    pub fn canonical(&self, text: &str) -> String {
        if self.exports.is_empty() {
            return text.to_string();
        }
        let is_word = |character: char| character.is_alphanumeric() || character == '_';
        let mut result = String::with_capacity(text.len());
        let mut rest = text;
        // The last character written: a path starts at `::` only where nothing it could
        // continue stands before it (`Vec<T>::Item`, `a::B`).
        let mut before: Option<char> = None;
        while !rest.is_empty() {
            let starts = rest.starts_with("::") && !before.is_some_and(|character| is_word(character) || character == '>');
            if starts {
                let mut end = 0;
                loop {
                    let tail = &rest[end..];
                    let Some(name) = tail.strip_prefix("::") else { break };
                    let length: usize = name.chars().take_while(|character| is_word(*character)).map(char::len_utf8).sum();
                    if length == 0 {
                        break;
                    }
                    end += 2 + length;
                }
                if end > 0 {
                    result.push_str(&self.canonical_path(&rest[..end]));
                    before = rest[..end].chars().next_back();
                    rest = &rest[end..];
                    continue;
                }
            }
            let Some(character) = rest.chars().next() else { break };
            result.push(character);
            before = Some(character);
            rest = &rest[character.len_utf8()..];
        }
        result
    }

    /// The type with the Rust type text `rust_path`, in any spelling of its paths: its
    /// position in the set.
    pub fn position_of_rust_type(&self, rust_path: &str) -> Option<(usize, usize)> {
        if let Some(found) = self.types.get(rust_path) {
            return Some(*found);
        }
        self.types.get(&self.canonical(rust_path)).copied()
    }

    /// The type with the Rust type text `rust_path` (its path, for a type a crate
    /// declares), in whichever model has it, with that model.
    pub fn find_rust_type(&self, rust_path: &str) -> Option<(&AssemblyModel, &TypeModel)> {
        let (model, type_) = self.position_of_rust_type(rust_path)?;
        Some((&self.models[model], &self.models[model].types[type_]))
    }

    /// The first type with the namespace-qualified name `full_name`, with its model.
    pub fn find_type(&self, full_name: &str) -> Option<(&AssemblyModel, &TypeModel)> {
        self.models.iter().find_map(|model| model.find_type(full_name).map(|type_| (model, type_)))
    }

    /// The accessor of a registered property with the absolute path `path`
    /// (`::ferroui_base::Decorator::child_property`, in any spelling), with its type.
    pub fn find_accessor(&self, path: &str) -> Option<(&TypeModel, &RegisteredModel)> {
        let path = self.canonical_path(path);
        let (type_path, accessor) = path.rsplit_once("::")?;
        let type_ = self.type_at(*self.types.get(type_path)?);
        type_.registered.iter().find(|registered| registered.accessor == accessor).map(|registered| (type_, registered))
    }

    /// The declaration of the property `registered` is an accessor of: itself when it
    /// declares the property, else the accessor its body calls, followed to the
    /// declaration through the models. Nothing when an accessor on the way is in no model.
    pub fn declaration_of<'a>(&'a self, registered: &'a RegisteredModel) -> Option<&'a RegisteredModel> {
        let mut current = registered;
        for _ in 0..MAX_HOPS {
            if current.registration == RegistrationModel::Declared {
                return Some(current);
            }
            let source = current.source.as_ref()?.resolved.as_deref()?;
            current = self.find_accessor(source)?.1;
        }
        None
    }

    /// The name of the property `registered` is an accessor of: its own, else the one of
    /// its declaration.
    pub fn name_of<'a>(&'a self, registered: &'a RegisteredModel) -> Option<&'a str> {
        match &registered.name {
            Some(name) => Some(name.as_str()),
            None => self.declaration_of(registered)?.name.as_deref(),
        }
    }

    /// The names the module `module` of a crate of the set gives another crate
    /// (`::ferroui_base`, `::ferroui_base::media`), each with the path of the declaring
    /// module: what a glob import of the module brings.
    pub fn exported_names<'a>(&'a self, module: &'a str) -> impl Iterator<Item = (&'a str, &'a str)> + 'a {
        self.exports.iter().filter_map(move |(path, declared)| {
            let name = path.strip_prefix(module)?.strip_prefix("::")?;
            (!name.contains("::")).then_some((name, declared.as_str()))
        })
    }

    /// The models of the `.xamlmeta` files `paths` and, transitively, of the files they
    /// name (relative to the naming file; `DEP_*` metadata exists for direct dependencies
    /// only), each assembly once, in the order of `paths`, with every file that was read.
    /// A file of format 1 is a model without types.
    pub fn read(paths: &[PathBuf]) -> Result<(Vec<AssemblyModel>, Vec<PathBuf>), String> {
        let mut models: Vec<AssemblyModel> = Vec::new();
        let mut files = Vec::new();
        let mut pending: Vec<PathBuf> = paths.iter().rev().cloned().collect();
        while let Some(path) = pending.pop() {
            if files.contains(&path) {
                continue;
            }
            let text = fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
            let model = AssemblyModel::parse(&text).map_err(|error| format!("{}: {error}", path.display()))?;
            let directory = path.parent().map(Path::to_path_buf).unwrap_or_default();
            files.push(path);
            if models.iter().any(|known| known.name == model.name) {
                continue;
            }
            for dependency in model.dependencies.iter().rev() {
                pending.push(directory.join(dependency));
            }
            models.push(model);
        }
        Ok((models, files))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{CallableModel, ExportModel, RegisteredKind, RustType, TypeKind};

    fn export(path: &str, declared: &str) -> ExportModel {
        ExportModel { path: path.to_string(), declared: declared.to_string() }
    }

    fn registered(name: Option<&str>, accessor: &str, registration: RegistrationModel, source: Option<&str>) -> RegisteredModel {
        RegisteredModel {
            name: name.map(str::to_string),
            kind: RegisteredKind::Styled,
            value_type: RustType::resolved("f64"),
            owner: None,
            host: None,
            accessor: accessor.to_string(),
            visibility: "pub".to_string(),
            registration,
            source: source.map(|path| CallableModel { path: Some(path.to_string()), resolved: Some(path.to_string()) }),
            assign_binding: false,
            inherits: false,
            read_only: false,
            added_owners: Vec::new(),
        }
    }

    /// `base` declares `Shape` (re-exported at its root, with the property `Fill`) and
    /// exports `Rc` of another crate again; `controls` declares `Path`, an owner of `Fill`.
    fn set() -> ModelSet {
        let mut base = AssemblyModel::new("Base", "base");
        base.exports = vec![
            export("::base::Shape", "::base::shapes::shape::Shape"),
            export("::base::shapes::Shape", "::base::shapes::shape::Shape"),
            export("::base::Handle", "::other::rc::Rc"),
            export("::base::media::Brush", "::base::media::Brush"),
        ];
        let mut shape = TypeModel::new("Shape", TypeKind::Class, RustType::resolved("::base::shapes::shape::Shape"), "base::shapes::shape");
        shape.namespace = "Base.Shapes".to_string();
        shape.registered = vec![registered(Some("Fill"), "fill_property", RegistrationModel::Declared, None)];
        base.types = vec![shape];

        let mut other = AssemblyModel::new("Other", "other");
        other.exports = vec![export("::other::rc::Rc", "::other::alloc::Rc")];

        let mut controls = AssemblyModel::new("Controls", "controls");
        let mut path = TypeModel::new("Path", TypeKind::Class, RustType::resolved("::controls::path::Path"), "controls::path");
        path.registered = vec![
            registered(None, "fill_property", RegistrationModel::AddedOwner, Some("::base::Shape::fill_property")),
            registered(None, "paint_property", RegistrationModel::Alias, Some("::controls::path::Path::fill_property")),
            registered(None, "lost_property", RegistrationModel::AddedOwner, Some("::missing::Type::lost_property")),
        ];
        controls.types = vec![path];
        ModelSet::new(vec![base, other, controls])
    }

    /// Not from upstream: a path is canonical by the longest exported prefix, through the
    /// crates that export it again; a path nothing exports stays as it is.
    #[test]
    fn paths_are_made_canonical_through_the_export_tables() {
        let set = set();
        assert_eq!(set.canonical_path("::base::Shape"), "::base::shapes::shape::Shape");
        assert_eq!(set.canonical_path("::base::shapes::Shape::fill_property"), "::base::shapes::shape::Shape::fill_property");
        assert_eq!(set.canonical_path("::base::Handle"), "::other::alloc::Rc");
        assert_eq!(set.canonical_path("::base::media::Brush"), "::base::media::Brush");
        assert_eq!(set.canonical_path("::base::Unknown"), "::base::Unknown");
        assert_eq!(set.canonical_path("::std::rc::Rc"), "::std::rc::Rc");
        assert_eq!(
            set.canonical("Option<::base::Handle<dyn ::base::Shape>>"),
            "Option<::other::alloc::Rc<dyn ::base::shapes::shape::Shape>>"
        );
        assert_eq!(set.canonical("&'static ::base::Shape"), "&'static ::base::shapes::shape::Shape");
        assert_eq!(set.canonical("(f64, ::base::shapes::Shape)"), "(f64, ::base::shapes::shape::Shape)");
        // `::` after a name or `>` continues what is before it.
        assert_eq!(set.canonical("Vec<f64>::base::Shape"), "Vec<f64>::base::Shape");
        assert_eq!(set.canonical("String"), "String");
        assert_eq!(ModelSet::new(Vec::new()).canonical("::base::Shape"), "::base::Shape");
    }

    /// Not from upstream: a type and the accessor of a property are found by any spelling,
    /// and an owner added in another crate has the name of the declaration.
    #[test]
    fn types_and_accessors_are_found_across_models() {
        let set = set();
        let found = set.find_rust_type("::base::Shape").map(|(model, type_)| (model.name.as_str(), type_.name.as_str()));
        assert_eq!(found, Some(("Base", "Shape")));
        assert_eq!(set.position_of_rust_type("::base::shapes::shape::Shape"), Some((0, 0)));
        assert_eq!(set.position_of_rust_type("::controls::path::Path"), Some((2, 0)));
        assert_eq!(set.position_of_rust_type("::base::Missing"), None);
        assert_eq!(set.find_type("Base.Shapes.Shape").map(|(_, type_)| type_.name.as_str()), Some("Shape"));

        let path = set.type_at((2, 0));
        assert_eq!(set.name_of(&path.registered[0]), Some("Fill"));
        assert_eq!(set.name_of(&path.registered[1]), Some("Fill"));
        assert_eq!(set.name_of(&path.registered[2]), None);
        assert_eq!(set.declaration_of(&path.registered[1]).map(|declared| declared.accessor.as_str()), Some("fill_property"));
        assert_eq!(set.find_accessor("::base::Shape::fill_property").map(|(type_, _)| type_.name.as_str()), Some("Shape"));

        let mut names: Vec<(&str, &str)> = set.exported_names("::base").collect();
        names.sort();
        assert_eq!(names, vec![("Handle", "::other::rc::Rc"), ("Shape", "::base::shapes::shape::Shape")]);
    }
}
