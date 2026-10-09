//! The choice of a call form for every callable of a model
//! (docs/porting/xaml.md, 9.5.3): how generated code calls a constructor, a
//! method, a static field, an event, `Parse` or the accessor of a plain
//! property that a declaration states by a callable.
//!
//! | Form | When | [`CallForm`] |
//! |---|---|---|
//! | A, structural | the callable is a path to a function a declaration macro writes for a type: the accessor of a routed event (`ferro_routed_event!`) or of a registered property, public, of a type with a public path; or the closure `\|\| *Type::accessor()` that dereferences what such a function returns (the form of the static field of a routed event) | `Structural` |
//! | B, typed path | the callable is a path to a function of an inherent `impl` block the scanner read, which takes the arguments the declaration macro calls it with, is `pub`, and whose type has a public path | `Path` |
//! | B, in the crate | the same with a function that is `pub(crate)`: for code generated into the declaring crate | `CratePath` |
//! | C, invoker | everything else: a closure, a path the scanner did not resolve, a free function, a function of a trait, a private function, a function of a type no public path leads to | `Invoker` |
//!
//! The registered properties, the members of enumerations and the default
//! constructor of a class (`new:`) have no callable of their own to choose
//! for: they are form A by their declaration.
//!
//! The functions of the crates the crate is built on are read from their
//! models ([`AssemblyModel::functions`]); the scan writes the public
//! functions of its own crate there for the crates built on it.
//!
//! What the declaration macros pass to a callable, which a function must
//! take to be called by its path (the instance is passed by reference):
//!
//! | Member | Arguments |
//! |---|---|
//! | constructor | its parameters |
//! | instance method, static method | the instance and its parameters; its parameters |
//! | static field | none |
//! | event | the instance and the handler |
//! | getter, setter of a property | the instance; the instance and the value |
//! | getter, setter of a static property | none; the value |
//! | getter, setter of an indexer | the instance and the indices; the same and the value |
//! | `Parse` | the text |

use std::collections::{BTreeMap, HashMap};

use crate::model::{AccessorModel, AssemblyModel, CallForm, CallableModel, FunctionModel, FunctionsModel, TypeModel};
use crate::scanner::InherentFunction;

/// What the choice found, in numbers: the callables by form, and why a callable is
/// called through the invoker.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CallFormStatistics {
    /// Form A.
    pub structural: usize,
    /// Form B, by a public path.
    pub path: usize,
    /// Form B for the declaring crate only.
    pub crate_path: usize,
    /// Form C, the sum of the reasons below.
    pub invoker: usize,
    /// The callable is not a path (a closure).
    pub closures: usize,
    /// The callable is a path the scanner did not resolve.
    pub unresolved: usize,
    /// The path names no function of an inherent `impl` block of the scanned crates: a
    /// free function, a function of a trait, or a function a macro writes.
    pub not_inherent: usize,
    /// The function does not take the arguments the declaration passes.
    pub signature: usize,
    /// The function is private to its module (or visible to a part of its crate).
    pub private: usize,
    /// The function is public, and no public path leads to its type.
    pub no_public_path: usize,
}

impl CallFormStatistics {
    /// The number of callables a form was chosen for.
    pub fn total(&self) -> usize {
        self.structural + self.path + self.crate_path + self.invoker
    }

    /// The numbers as text, one line.
    pub fn summary(&self) -> String {
        format!(
            "call forms: {} callables; A (structural) {}, B (typed path) {} and {} within the crate, C (invoker) {} ({} closures, {} unresolved paths, {} paths to no inherent function, {} with other arguments, {} private functions, {} functions of a type without a public path)",
            self.total(),
            self.structural,
            self.path,
            self.crate_path,
            self.invoker,
            self.closures,
            self.unresolved,
            self.not_inherent,
            self.signature,
            self.private,
            self.no_public_path
        )
    }
}

/// A function a callable may name.
struct Candidate<'a> {
    receiver: bool,
    parameters: usize,
    /// `pub`, `pub(crate)`, or anything else for a function that is neither.
    visibility: &'a str,
    /// Written by a declaration macro.
    declared: bool,
    /// The public path of the type, when one leads to it.
    public_path: Option<&'a str>,
}

/// The functions a callable is looked up in: by the Rust path of the type and the name.
struct Functions<'a> {
    by_path: HashMap<(&'a str, &'a str), Vec<Candidate<'a>>>,
}

impl<'a> Functions<'a> {
    fn new(model: &'a AssemblyModel, functions: &'a [InherentFunction], public: &'a BTreeMap<String, String>, dependencies: &'a [AssemblyModel]) -> Self {
        let mut by_path: HashMap<(&str, &str), Vec<Candidate>> = HashMap::new();
        for function in functions {
            by_path.entry((function.owner.as_str(), function.name.as_str())).or_default().push(Candidate {
                receiver: function.receiver,
                parameters: function.parameters.len(),
                visibility: function.visibility.as_str(),
                declared: function.declared_by.is_some(),
                public_path: public.get(&function.owner).map(String::as_str),
            });
        }
        // The accessors of the registered properties: functions the declaration macro
        // writes for the type the accessor is a function of.
        for type_ in &model.types {
            let public_path = type_.public_path.as_deref();
            for registered in &type_.registered {
                let (owner, public_path) = match &registered.function_of {
                    Some(owner) => (owner.as_str(), public.get(owner).map(String::as_str)),
                    None => (type_.rust_path.text.as_str(), public_path),
                };
                by_path.entry((owner, registered.accessor.as_str())).or_default().push(Candidate {
                    receiver: false,
                    parameters: 0,
                    visibility: registered.visibility.as_str(),
                    declared: true,
                    public_path,
                });
            }
        }
        for dependency in dependencies {
            for functions in &dependency.functions {
                for function in &functions.functions {
                    by_path.entry((functions.owner.as_str(), function.name.as_str())).or_default().push(Candidate {
                        receiver: function.receiver,
                        parameters: function.parameters,
                        visibility: "pub",
                        declared: function.declared_by.is_some(),
                        public_path: Some(functions.public_path.as_str()),
                    });
                }
            }
            for type_ in &dependency.types {
                for registered in type_.registered.iter().filter(|registered| registered.visibility == "pub") {
                    let public_path = match &registered.function_of {
                        Some(owner) => dependency.exports.iter().filter(|export| export.declared == *owner).map(|export| export.path.as_str()).min_by_key(|path| (path.matches("::").count(), *path)),
                        None => type_.public_path.as_deref(),
                    };
                    let owner = registered.function_of.as_deref().unwrap_or(type_.rust_path.text.as_str());
                    by_path.entry((owner, registered.accessor.as_str())).or_default().push(Candidate {
                        receiver: false,
                        parameters: 0,
                        visibility: "pub",
                        declared: true,
                        public_path,
                    });
                }
            }
        }
        Self { by_path }
    }

    /// The form of the callable `callable`, which the declaration calls with `arguments`
    /// arguments.
    fn choose(&self, callable: &CallableModel, arguments: usize, statistics: &mut CallFormStatistics) -> CallForm {
        let mut invoker = |reason: fn(&mut CallFormStatistics) -> &mut usize| {
            statistics.invoker += 1;
            *reason(statistics) += 1;
            CallForm::Invoker
        };
        if callable.path.is_none() {
            // A closure that dereferences what the accessor of a routed event returns is
            // that accessor, called by its path and dereferenced.
            let declared = callable.dereferenced.as_deref().and_then(|path| path.rsplit_once("::")).and_then(|key| self.by_path.get(&key)).is_some_and(|candidates| {
                arguments == 0
                    && candidates.iter().all(|candidate| {
                        candidate.declared && candidate.visibility == "pub" && candidate.public_path.is_some() && !candidate.receiver && candidate.parameters == 0
                    })
            });
            if declared {
                statistics.structural += 1;
                return CallForm::Structural;
            }
            return invoker(|statistics| &mut statistics.closures);
        }
        let Some(resolved) = callable.resolved.as_deref() else {
            return invoker(|statistics| &mut statistics.unresolved);
        };
        let Some((owner, name)) = resolved.rsplit_once("::") else {
            return invoker(|statistics| &mut statistics.not_inherent);
        };
        let Some(candidates) = self.by_path.get(&(owner, name)) else {
            return invoker(|statistics| &mut statistics.not_inherent);
        };
        // Two functions of one name are the function under two `cfg` conditions: the call
        // is by path when each of them can be called so.
        let fits = |candidate: &Candidate| usize::from(candidate.receiver) + candidate.parameters == arguments;
        if !candidates.iter().all(fits) {
            return invoker(|statistics| &mut statistics.signature);
        }
        if !candidates.iter().all(|candidate| matches!(candidate.visibility, "pub" | "pub(crate)")) {
            return invoker(|statistics| &mut statistics.private);
        }
        let Some(public_path) = candidates[0].public_path.filter(|path| candidates.iter().all(|candidate| candidate.public_path == Some(path))) else {
            return invoker(|statistics| &mut statistics.no_public_path);
        };
        let path = format!("{public_path}::{name}");
        if candidates.iter().any(|candidate| candidate.visibility == "pub(crate)") {
            statistics.crate_path += 1;
            return CallForm::CratePath(path);
        }
        if candidates.iter().all(|candidate| candidate.declared) {
            statistics.structural += 1;
            return CallForm::Structural;
        }
        statistics.path += 1;
        CallForm::Path(path)
    }
}

/// Chooses the call form of every callable of `model` and writes the public functions of
/// the crate into it ([`AssemblyModel::functions`]). `functions` are the functions of the
/// inherent `impl` blocks of the crate, `public` the public path of every item of the
/// crate by its path in its declaring module, `dependencies` the models of the crates the
/// crate is built on.
pub(crate) fn choose(model: &mut AssemblyModel, functions: &[InherentFunction], public: &BTreeMap<String, String>, dependencies: &[AssemblyModel]) -> CallFormStatistics {
    let mut statistics = CallFormStatistics::default();
    let mut chosen: Vec<Vec<CallForm>> = Vec::with_capacity(model.types.len());
    {
        let known = Functions::new(model, functions, public, dependencies);
        for type_ in &model.types {
            let mut forms = Vec::new();
            visit(type_, &mut |callable, arguments| forms.push(known.choose(callable, arguments, &mut statistics)));
            chosen.push(forms);
        }
    }
    for (type_, forms) in model.types.iter_mut().zip(chosen) {
        let mut forms = forms.into_iter();
        assign(type_, &mut || forms.next());
    }

    let mut exported: Vec<FunctionsModel> = Vec::new();
    let mut position: HashMap<&str, usize> = HashMap::new();
    for function in functions.iter().filter(|function| function.visibility == "pub") {
        let Some(public_path) = public.get(&function.owner) else { continue };
        let index = *position.entry(function.owner.as_str()).or_insert_with(|| {
            exported.push(FunctionsModel { owner: function.owner.clone(), public_path: public_path.clone(), functions: Vec::new() });
            exported.len() - 1
        });
        let entry = FunctionModel {
            name: function.name.clone(),
            receiver: function.receiver,
            parameters: function.parameters.len(),
            declared_by: function.declared_by.clone(),
        };
        if !exported[index].functions.contains(&entry) {
            exported[index].functions.push(entry);
        }
    }
    model.functions = exported;
    statistics
}

/// Calls `visit` with every callable of the type a form is chosen for and the number of
/// arguments its declaration passes, in a fixed order ([`assign`] follows the same).
fn visit(type_: &TypeModel, visit: &mut dyn FnMut(&CallableModel, usize)) {
    if let Some(parse) = &type_.parse {
        visit(parse, 1);
    }
    for constructor in &type_.constructors {
        visit(&constructor.callable, constructor.parameters.len());
    }
    for method in &type_.methods {
        visit(&method.callable, usize::from(!method.is_static) + method.parameters.len());
    }
    for field in &type_.fields {
        visit(&field.callable, 0);
    }
    for event in &type_.events {
        visit(&event.callable, 2);
    }
    let mut accessors = |getter: &Option<AccessorModel>, setter: &Option<AccessorModel>, instance: usize| {
        if let Some(getter) = getter {
            visit(&getter.callable, instance);
        }
        if let Some(setter) = setter {
            visit(&setter.callable, instance + 1);
        }
    };
    for property in &type_.properties {
        accessors(&property.getter, &property.setter, 1);
    }
    for property in &type_.static_properties {
        accessors(&property.getter, &property.setter, 0);
    }
    for indexer in &type_.indexers {
        accessors(&indexer.getter, &indexer.setter, 1 + indexer.parameters.len());
    }
}

/// Writes the forms `next` yields into the type, in the order of [`visit`].
fn assign(type_: &mut TypeModel, next: &mut dyn FnMut() -> Option<CallForm>) {
    if type_.parse.is_some() {
        type_.parse_call = next();
    }
    for member in type_.constructors.iter_mut().chain(&mut type_.methods).chain(&mut type_.fields).chain(&mut type_.events) {
        member.call = next();
    }
    for property in type_.properties.iter_mut().chain(&mut type_.static_properties).chain(&mut type_.indexers) {
        if let Some(getter) = &mut property.getter {
            getter.call = next();
        }
        if let Some(setter) = &mut property.setter {
            setter.call = next();
        }
    }
}
