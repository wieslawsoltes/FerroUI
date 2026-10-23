//! The mappings of a locator (`ProcessClass` and the mapping stages of
//! `StaticViewLocatorGenerator.cs` and `StaticViewLocatorGenerator.Adapters.cs`).
//!
//! Mapping sources are applied in the reference's order, from the highest priority to the lowest:
//!
//! 1. `#[static_view_mapping]` (`[StaticViewMapping]`);
//! 2. the configured contracts (`view_model_mapping_contracts`);
//! 3. the views that implement `IControlViewFor` when `generate_view_locator = true` (the
//!    automatic `ReactiveUI.IViewFor<TViewModel>` inference);
//! 4. the namespace and type name conventions.
//!
//! What the reference decides at run time and the port at build time is described in
//! `docs/porting/static-view-locator.md`: the generated adapters have no run-time type tests,
//! the fallback precedence is flattened into the table of the views for every type of the crate.

use crate::diagnostics::{self, Diagnostic, Location};
use crate::emit;
use crate::options::{GeneratorOptions, ReplacementRule, DEFAULT_VIEW_BASE_TYPES};
use crate::source::{SourceLocator, SourceMethod};
use crate::universe::{Kind, TypeDecl, TypeRef, Universe};
use std::collections::{BTreeMap, BTreeSet};

/// The fallback precedence of the generated adapters (`ViewResolutionMode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ResolutionMode {
    Exact,
    ExactThenBaseTypes,
    ExactThenInterfaces,
    #[default]
    ExactThenBaseTypesThenInterfaces,
    ExactThenInterfacesThenBaseTypes,
}

impl ResolutionMode {
    fn of(name: &str) -> Option<Self> {
        Some(match name {
            "Exact" => ResolutionMode::Exact,
            "ExactThenBaseTypes" => ResolutionMode::ExactThenBaseTypes,
            "ExactThenInterfaces" => ResolutionMode::ExactThenInterfaces,
            "ExactThenBaseTypesThenInterfaces" => ResolutionMode::ExactThenBaseTypesThenInterfaces,
            "ExactThenInterfacesThenBaseTypes" => ResolutionMode::ExactThenInterfacesThenBaseTypes,
            _ => return None,
        })
    }
}

/// A view model mapped to its view (`ResolvedViewMapping`).
#[derive(Clone, Debug)]
pub struct Mapping {
    pub view_model: TypeRef,
    pub view: TypeRef,
    /// What creates the view, without `()`: its constructor without parameters.
    pub constructor: String,
}

/// How a view model type is named by generated source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelKey {
    /// `TypeId::of::<T>()`; `object_model` for a class of the object model, keyed by `Ref<T>`.
    Type { code: String, object_model: bool },
    /// The path of an open generic type definition (the generic tables of the legacy fallback).
    GenericDefinition(String),
    /// A trait: `TypeId::of::<dyn Trait>` when the trait can be the type of a trait object (the
    /// text names `dyn Trait`), else no entry; its mappings are also flattened onto the types of
    /// the crate that implement it.
    Interface(Option<String>),
}

/// What the emitter writes for a locator.
#[derive(Debug, Default)]
pub struct LocatorPlan {
    pub name: String,
    pub module: String,
    pub generics: String,
    pub generic_arguments: String,
    pub where_clause: String,
    /// `s_views`, sorted by the display string of the view model.
    pub views: Vec<(ModelKey, Mapping)>,
    /// `s_missingViews`, when it is generated.
    pub missing_views: Option<Vec<(ModelKey, String)>>,
    /// The legacy fallback, when it is generated: the types of the crate whose nearest mapped base
    /// class has a view or a missing-view message, and the ones whose mapped trait has.
    pub legacy: Option<LegacyTables>,
    /// The resolver of the generated adapters.
    pub resolver: bool,
    /// The view model types a generated `match_` accepts; `None` without `data_template_match_types`.
    pub match_types: Option<Vec<ModelKey>>,
    pub try_create_view_exact: bool,
    pub view_locator: Option<ViewLocatorPlan>,
    pub legacy_build: bool,
    pub data_template_build: Option<DataTemplateBuildPlan>,
    pub data_template_match: Option<DataTemplateMatchPlan>,
}

/// The tables of the legacy fallback.
#[derive(Debug, Default)]
pub struct LegacyTables {
    pub base_views: Vec<(ModelKey, Mapping)>,
    pub interface_views: Vec<(ModelKey, Mapping)>,
    pub base_missing_views: Vec<(ModelKey, String)>,
    pub interface_missing_views: Vec<(ModelKey, String)>,
}

/// The generated view locator of ReactiveUI.
#[derive(Debug, Default)]
pub struct ViewLocatorPlan {
    /// The mappings whose view implements `IControlViewFor`.
    pub view_fors: Vec<(ModelKey, Mapping)>,
    pub implement_view_locator: bool,
    pub implement_enable_logger: bool,
    pub resolve_view_for: bool,
    pub resolve_view_default: bool,
}

/// The generated `build` of the data template and its hooks.
#[derive(Debug, Default)]
pub struct DataTemplateBuildPlan {
    pub with_view_locator: bool,
    pub build_resolved_view: bool,
    pub build_fallback_view: bool,
    pub build_invalid_view: bool,
    pub build_missing_view: bool,
}

/// The generated `match_` of the data template.
#[derive(Debug, Default)]
pub struct DataTemplateMatchPlan {
    pub implement_data_template: bool,
    pub match_data_template: bool,
}

/// The options of the attribute of a locator, with their defaults.
struct Settings {
    generate_view_factory_methods: bool,
    generate_runtime_type_fallback_methods: bool,
    generate_view_locator: bool,
    generate_data_template: bool,
    mode: ResolutionMode,
}

/// What the locator declares itself: its inherent methods and the traits it implements.
struct Members {
    methods: Vec<SourceMethod>,
    traits: Vec<String>,
}

impl Members {
    fn method(&self, name: &str) -> Option<&SourceMethod> {
        self.methods.iter().find(|method| method.name == name)
    }

    fn implements(&self, trait_name: &str) -> bool {
        let suffix = format!("::{trait_name}");
        self.traits.iter().any(|key| key.ends_with(&suffix) || key == trait_name)
    }
}

/// Whether the trait `key` is `IControlViewFor` of the ReactiveUI controls (or its re-export by
/// ferroui-static-view-locator).
pub fn is_control_view_for(key: &str) -> bool {
    key.ends_with("::IControlViewFor")
        && (key.starts_with("::ferroui_reactiveui_controls::") || key.starts_with("::ferroui_static_view_locator::"))
}

/// Plans the generated source of `locator`; the diagnostics are added to `diagnostics`.
pub fn plan_locator(
    universe: &Universe<'_>,
    options: &GeneratorOptions,
    locator: &SourceLocator,
    reactiveui_available: bool,
    diagnostics: &mut Vec<Diagnostic>,
) -> LocatorPlan {
    Planner { universe, options, locator, diagnostics, module: &locator.module }.plan(reactiveui_available)
}

struct Planner<'a, 'u> {
    universe: &'a Universe<'u>,
    options: &'a GeneratorOptions,
    locator: &'a SourceLocator,
    diagnostics: &'a mut Vec<Diagnostic>,
    module: &'a str,
}

impl Planner<'_, '_> {
    fn plan(&mut self, reactiveui_available: bool) -> LocatorPlan {
        let attribute = &self.locator.attribute;
        let mode = match attribute.generated_adapter_resolution_mode.as_deref() {
            None => ResolutionMode::default(),
            Some(name) => ResolutionMode::of(name).unwrap_or_else(|| {
                self.diagnostics.push(Diagnostic::error(diagnostics::INVALID_OPTION, format!("'{name}' is not a view resolution mode"), &self.locator.location()));
                ResolutionMode::default()
            }),
        };
        let settings = Settings {
            generate_view_factory_methods: attribute.generate_view_factory_methods.unwrap_or(false),
            generate_runtime_type_fallback_methods: attribute.generate_runtime_type_fallback_methods.unwrap_or(true),
            generate_view_locator: attribute.generate_view_locator.unwrap_or(false),
            generate_data_template: attribute.generate_data_template.unwrap_or(false),
            mode,
        };
        let members = self.members();

        let build_method_exists = members.method("build").is_some();
        let mut generate_view_locator = settings.generate_view_locator;
        if generate_view_locator && !reactiveui_available {
            self.diagnostics.push(Diagnostic::error(
                diagnostics::MISSING_ADAPTER_TYPE,
                "'generate_view_locator' requires a reference containing 'ferroui_reactiveui::IViewLocator': the feature `reactiveui` of ferroui-static-view-locator",
                &self.locator.location(),
            ));
            generate_view_locator = false;
        }
        let generate_data_template = settings.generate_data_template;
        // An inherent `build` or `match_` would shadow the methods of the data template on a call
        // (`locator.build(..)`): the member a C# class declares with the signature of the adapter.
        let inherent_build = generate_data_template && self.has_inherent_adapter_member(&members, "build", "ITemplateWithParam::build(&self, &Option<BoxedValue>)");
        let inherent_match = generate_data_template && self.has_inherent_adapter_member(&members, "match_", "IDataTemplate::match_(&self, Option<&BoxedValue>)");
        let data_template_build_exists = generate_data_template && (members.implements("ITemplateWithParam") || inherent_build);
        let data_template_match_exists = generate_data_template && (members.implements("IDataTemplate") || inherent_match);
        let missing_view_hook_exists = generate_data_template && !data_template_build_exists && self.has_hook(&members, "build_missing_view", 2, "build_missing_view(&self, Option<&BoxedValue>, RuntimeType)");
        let should_generate_runtime_fallback_methods = (!build_method_exists && !generate_data_template)
            || ((if generate_data_template { data_template_build_exists } else { build_method_exists }) && settings.generate_runtime_type_fallback_methods);
        let should_generate_missing_views =
            should_generate_runtime_fallback_methods || (generate_data_template && !data_template_build_exists && !missing_view_hook_exists);

        let view_base_types = self.view_base_types();
        let explicit = self.explicit_mappings();
        let mut ambiguous: BTreeSet<String> = BTreeSet::new();
        let mut inferred = self.inferred_mappings(&view_base_types, &explicit, &mut ambiguous);
        if generate_view_locator {
            for (canonical, mapping) in self.control_view_for_mappings(&view_base_types, &inferred, &explicit, &mut ambiguous) {
                inferred.entry(canonical).or_insert(mapping);
            }
        }
        for (canonical, mapping) in &explicit {
            inferred.insert(canonical.clone(), mapping.clone());
        }

        // The relevant view models: the candidates of the conventions, then the mapped ones.
        let mut relevant: Vec<TypeRef> = Vec::new();
        let mut seen: BTreeSet<String> = BTreeSet::new();
        for decl in self.universe.types() {
            if !self.is_view_model_candidate(decl) || ambiguous.contains(&decl.key) {
                continue;
            }
            let type_ref = self.universe.type_ref_of(&decl.key);
            if seen.insert(type_ref.canonical.clone()) {
                relevant.push(type_ref);
            }
        }
        for (view_model, _) in inferred.values() {
            if seen.insert(view_model.canonical.clone()) {
                relevant.push(view_model.clone());
            }
        }
        relevant.sort_by(|left, right| left.display().cmp(right.display()));

        let mut resolved: Vec<Mapping> = Vec::new();
        let mut resolved_view_models: BTreeSet<String> = BTreeSet::new();
        for view_model in &relevant {
            let view = match inferred.get(&view_model.canonical) {
                Some((_, view)) => Some(view.clone()),
                None => self.derived_view(view_model).map(|key| self.universe.type_ref_of(&key)),
            };
            let Some(view) = view else { continue };
            let decl = self.universe.get(&view.key);
            let trusted = decl.is_none() && explicit.contains_key(&view_model.canonical);
            if !trusted && !decl.is_some_and(|decl| self.is_supported_view(decl, &view_base_types) && !decl.is_generic()) {
                continue;
            }
            let constructor = match decl {
                Some(decl) => match self.constructor_of(decl) {
                    Some(constructor) => constructor,
                    None => {
                        self.diagnostics.push(Diagnostic::error(
                            diagnostics::VIEW_CANNOT_BE_CONSTRUCTED,
                            format!(
                                "View '{}' mapped to '{}' is not accessible or has no accessible constructor callable without arguments",
                                view.display(),
                                view_model.display()
                            ),
                            &location_of(decl),
                        ));
                        continue;
                    }
                },
                // A view the scanner does not know, mapped explicitly: the compiler checks it.
                None => format!("{}::new", view.code),
            };
            resolved_view_models.insert(view_model.canonical.clone());
            resolved.push(Mapping { view_model: view_model.clone(), view, constructor });
        }

        if generate_view_locator || generate_data_template {
            self.add_compile_time_fallback_mappings(&relevant, &mut resolved, &mut resolved_view_models, settings.mode);
        }

        let mut plan = LocatorPlan {
            name: self.locator.name.clone(),
            module: self.locator.module.clone(),
            generics: self.locator.generics.clone(),
            generic_arguments: self.locator.generic_arguments.clone(),
            where_clause: self.locator.where_clause.clone(),
            ..LocatorPlan::default()
        };
        resolved.sort_by(|left, right| left.view_model.display().cmp(right.view_model.display()));
        plan.views = resolved.iter().map(|mapping| (self.model_key(&mapping.view_model), mapping.clone())).collect();

        let mut missing: Vec<(TypeRef, String)> = Vec::new();
        if should_generate_missing_views {
            for view_model in &relevant {
                if resolved_view_models.contains(&view_model.canonical) {
                    continue;
                }
                let (view_name, view_decl) = match inferred.get(&view_model.canonical) {
                    Some((_, view)) => (view.display().to_string(), self.universe.get(&view.key)),
                    None => {
                        let derived = self.derived_view_name(view_model);
                        let decl = derived.as_deref().and_then(|name| self.find_view(view_model, name));
                        (derived.unwrap_or_else(|| view_model.display().to_string()), decl)
                    }
                };
                if let Some(decl) = view_decl {
                    if self.is_supported_view(decl, &view_base_types) && (decl.is_generic() || self.constructor_of(decl).is_some()) {
                        continue;
                    }
                }
                missing.push((view_model.clone(), format!("Not Found: {}", view_name.trim_start_matches("::"))));
            }
            plan.missing_views = Some(missing.iter().map(|(view_model, message)| (self.model_key(view_model), message.clone())).collect());
        }

        if should_generate_runtime_fallback_methods {
            plan.legacy = Some(self.legacy_tables(&resolved, &missing));
        }

        plan.resolver = generate_view_locator || generate_data_template;
        plan.try_create_view_exact =
            (settings.generate_view_factory_methods || generate_view_locator || generate_data_template) && !self.has_try_create_view_exact(&members);

        if generate_view_locator {
            plan.view_locator = Some(ViewLocatorPlan {
                view_fors: resolved
                    .iter()
                    .filter(|mapping| self.universe.get(&mapping.view.key).is_some_and(|decl| decl.contracts.iter().any(|contract| is_control_view_for(&contract.contract))))
                    .map(|mapping| (self.model_key(&mapping.view_model), mapping.clone()))
                    .filter(|(key, _)| matches!(key, ModelKey::Type { .. }))
                    .collect(),
                implement_view_locator: !members.implements("IViewLocator"),
                implement_enable_logger: !members.implements("IEnableLogger"),
                resolve_view_for: !self.has_resolve_view(&members, "resolve_view_for", 1),
                resolve_view_default: !self.has_resolve_view(&members, "resolve_view_default", 0),
            });
        }

        plan.legacy_build = !build_method_exists && !generate_data_template;

        if generate_data_template && !data_template_build_exists {
            plan.data_template_build = Some(DataTemplateBuildPlan {
                with_view_locator: generate_view_locator,
                build_resolved_view: !self.has_hook(&members, "build_resolved_view", 1, "build_resolved_view(&self, Option<&BoxedValue>)"),
                build_fallback_view: !self.has_hook(&members, "build_fallback_view", 1, "build_fallback_view(&self, Option<&BoxedValue>)"),
                build_invalid_view: !self.has_hook(&members, "build_invalid_view", 1, "build_invalid_view(&self, Option<&BoxedValue>)"),
                build_missing_view: !missing_view_hook_exists,
            });
        }

        if generate_data_template && !data_template_match_exists {
            let match_data_template = !self.has_boolean_hook(&members, "match_data_template");
            if match_data_template && !self.locator.attribute.data_template_match_types.is_empty() {
                let match_types: Vec<TypeRef> =
                    self.locator.attribute.data_template_match_types.iter().map(|text| self.universe.resolve(self.module, text)).collect();
                plan.match_types = Some(
                    resolved
                        .iter()
                        .filter(|mapping| match_types.iter().any(|match_type| self.is_assignable(&mapping.view_model, match_type)))
                        .map(|mapping| self.model_key(&mapping.view_model))
                        .filter(|key| matches!(key, ModelKey::Type { .. }))
                        .collect(),
                );
            }
            plan.data_template_match = Some(DataTemplateMatchPlan { implement_data_template: true, match_data_template });
        }
        plan
    }

    /// The inherent methods of the locator and the traits it implements, from the sources.
    fn members(&self) -> Members {
        let key = format!("::{}::{}", self.locator.module, self.locator.name);
        let mut methods = Vec::new();
        let mut traits = Vec::new();
        for block in &self.universe.source.impls {
            if self.universe.resolve(&block.module, &block.self_type).key != key {
                continue;
            }
            match &block.trait_path {
                None => methods.extend(block.methods.iter().cloned()),
                Some(path) => traits.push(self.universe.resolve(&block.module, path).key),
            }
        }
        Members { methods, traits }
    }

    /// A hook declared by the locator suppresses the generated one (`HasControlHook`); one that
    /// cannot stand for it (not `&self`, another number of parameters) is reported.
    fn has_hook(&mut self, members: &Members, name: &str, parameters: usize, expected: &str) -> bool {
        let Some(method) = members.method(name) else { return false };
        if method.receiver.as_deref() != Some("&self") || method.parameters.len() != parameters {
            self.incompatible(method, expected, "a method of `&self` whose return type converts into Option<Ref<Control>>");
        }
        true
    }

    fn has_inherent_adapter_member(&mut self, members: &Members, name: &str, generated: &str) -> bool {
        let Some(method) = members.method(name) else { return false };
        let expected = format!("an implementation of the trait for the locator, not an inherent method `{name}`");
        self.incompatible(method, generated, &expected);
        true
    }

    fn has_boolean_hook(&mut self, members: &Members, name: &str) -> bool {
        let Some(method) = members.method(name) else { return false };
        let returns_bool = method.return_type.as_deref().is_some_and(|returned| returned.trim() == "bool");
        if method.receiver.as_deref() != Some("&self") || method.parameters.len() != 1 || !returns_bool {
            self.incompatible(method, "match_data_template(&self, Option<&BoxedValue>)", "a method of `&self` returning bool");
        }
        true
    }

    fn has_try_create_view_exact(&mut self, members: &Members) -> bool {
        let Some(method) = members.method("try_create_view_exact") else { return false };
        let returns_option = method.return_type.as_deref().is_some_and(|returned| returned.trim_start().starts_with("Option"));
        let receiver_ok = matches!(method.receiver.as_deref(), None | Some("&self"));
        if !returns_option || !receiver_ok || method.parameters.len() != 1 {
            self.incompatible(method, "try_create_view_exact(TypeId)", "a function returning Option<Ref<Control>>");
        }
        true
    }

    fn has_resolve_view(&mut self, members: &Members, name: &str, parameters: usize) -> bool {
        let Some(method) = members.method(name) else { return false };
        if method.receiver.as_deref() != Some("&self") || method.parameters.len() != parameters {
            let expected = format!("{name}<VM>(&self{})", if parameters == 1 { ", Option<&str>" } else { "" });
            self.incompatible(method, &expected, "a public method of `&self` returning Option<Rc<dyn IViewForOf<VM>>>");
        }
        true
    }

    fn incompatible(&mut self, method: &SourceMethod, generated: &str, expected: &str) {
        let location = Location::new(&self.locator.file, method.line);
        let location = if method.line == 0 { self.locator.location() } else { location };
        self.diagnostics.push(Diagnostic::error(
            diagnostics::INCOMPATIBLE_ADAPTER_MEMBER,
            format!("Member '{}::{}' conflicts with generated adapter member '{generated}'; expected {expected}", self.locator.name, method.name),
            &location,
        ));
    }

    /// The keys of the view base types (`GetViewBaseTypes`).
    fn view_base_types(&self) -> BTreeSet<String> {
        let root = &self.universe.crate_name;
        DEFAULT_VIEW_BASE_TYPES
            .iter()
            .map(|path| path.to_string())
            .chain(self.options.additional_view_base_types.iter().cloned())
            .map(|path| self.universe.resolve(root, &path).key)
            .collect()
    }

    /// `[StaticViewMapping]`: the canonical text of the view model, with the two types. A type
    /// the scanner does not know is named as written: the generated source is in the module of
    /// the locator, where the attribute names it.
    fn explicit_mappings(&self) -> BTreeMap<String, (TypeRef, TypeRef)> {
        let mut mappings = BTreeMap::new();
        for (view_model_text, view_text) in &self.locator.mappings {
            let mut view_model = self.universe.resolve(self.module, view_model_text);
            let mut view = self.universe.resolve(self.module, view_text);
            if self.universe.get(&view.key).is_none() {
                view.code = view_text.clone();
            }
            if !view_model.resolved {
                view_model.code = view_model_text.clone();
            }
            mappings.insert(view_model.canonical.clone(), (view_model, view));
        }
        mappings
    }

    /// The configured contracts (`GetViewModelMappingContracts`): the keys of the valid ones.
    fn contracts(&mut self) -> BTreeSet<String> {
        let mut contracts = BTreeSet::new();
        for written in self.locator.attribute.view_model_mapping_contracts.clone() {
            let contract = self.universe.resolve(self.module, &written);
            let valid = !contract.constructed
                && match self.universe.get(&contract.key) {
                    Some(decl) => {
                        decl.kind == Kind::Interface && (decl.generics.len() == 1 || (decl.generics.is_empty() && decl.associated_types == 1))
                    }
                    // A trait of a crate without a model: its `impl` blocks tell.
                    None => true,
                };
            if valid {
                contracts.insert(contract.key);
            } else {
                self.diagnostics.push(Diagnostic::error(
                    diagnostics::INVALID_MAPPING_CONTRACT,
                    format!("Mapping contract '{}' must be a trait with exactly one view model type: one generic parameter, or one associated type", written.replace(' ', "")),
                    &self.locator.location(),
                ));
            }
        }
        contracts
    }

    /// `GetInferredMappings`: the views of the crate that implement a configured contract.
    fn inferred_mappings(
        &mut self,
        view_base_types: &BTreeSet<String>,
        explicit: &BTreeMap<String, (TypeRef, TypeRef)>,
        ambiguous: &mut BTreeSet<String>,
    ) -> BTreeMap<String, (TypeRef, TypeRef)> {
        let contracts = self.contracts();
        if contracts.is_empty() {
            return BTreeMap::new();
        }
        self.contract_mappings(view_base_types, &|key| contracts.contains(key), &BTreeMap::new(), explicit, ambiguous)
    }

    /// `GetReactiveUIMappings`: the views of the crate that implement `IControlViewFor`.
    fn control_view_for_mappings(
        &mut self,
        view_base_types: &BTreeSet<String>,
        configured: &BTreeMap<String, (TypeRef, TypeRef)>,
        explicit: &BTreeMap<String, (TypeRef, TypeRef)>,
        ambiguous: &mut BTreeSet<String>,
    ) -> BTreeMap<String, (TypeRef, TypeRef)> {
        self.contract_mappings(view_base_types, &is_control_view_for, configured, explicit, ambiguous)
    }

    fn contract_mappings(
        &mut self,
        view_base_types: &BTreeSet<String>,
        is_contract: &dyn Fn(&str) -> bool,
        configured: &BTreeMap<String, (TypeRef, TypeRef)>,
        explicit: &BTreeMap<String, (TypeRef, TypeRef)>,
        ambiguous: &mut BTreeSet<String>,
    ) -> BTreeMap<String, (TypeRef, TypeRef)> {
        let mut mappings: BTreeMap<String, (TypeRef, TypeRef)> = BTreeMap::new();
        let views: Vec<&TypeDecl> = self
            .universe
            .types()
            .filter(|decl| decl.local && decl.kind == Kind::Class && !decl.is_generic() && self.is_supported_view(decl, view_base_types))
            .collect();
        for view_decl in views {
            let Some(view_model) = self.mapped_view_model(view_decl, is_contract) else { continue };
            if ambiguous.contains(&view_model.canonical) || configured.contains_key(&view_model.canonical) || explicit.contains_key(&view_model.canonical) {
                continue;
            }
            let view = self.universe.type_ref_of(&view_decl.key);
            if let Some((_, existing)) = mappings.get(&view_model.canonical) {
                if existing.key != view.key {
                    self.diagnostics.push(Diagnostic::error(
                        diagnostics::AMBIGUOUS_VIEW_MAPPING,
                        format!(
                            "View model '{}' is implemented by both '{}' and '{}'; add #[static_view_mapping] to select one view",
                            view_model.display(),
                            existing.display(),
                            view.display()
                        ),
                        &location_of(view_decl),
                    ));
                    mappings.remove(&view_model.canonical);
                    ambiguous.insert(view_model.canonical.clone());
                }
                continue;
            }
            mappings.insert(view_model.canonical.clone(), (view_model, view));
        }
        mappings
    }

    /// `TryGetMappedViewModelType`: the view model of the first contract the view or one of its
    /// base classes implements.
    fn mapped_view_model(&self, view: &TypeDecl, is_contract: &dyn Fn(&str) -> bool) -> Option<TypeRef> {
        let mut owners = vec![view.key.clone()];
        owners.extend(self.universe.base_chain(&view.key));
        owners.iter().filter_map(|owner| self.universe.get(owner)).find_map(|decl| {
            decl.contracts.iter().filter(|contract| is_contract(&contract.contract)).find_map(|contract| contract.view_model.clone())
        })
    }

    /// `IsViewModelCandidate`.
    fn is_view_model_candidate(&self, decl: &TypeDecl) -> bool {
        if !decl.name.ends_with("ViewModel") {
            return false;
        }
        if !decl.local && (!self.options.include_referenced_crates || !self.universe.is_accessible(decl, self.module)) {
            return false;
        }
        if self.options.namespace_prefixes.is_empty() {
            return true;
        }
        self.options.namespace_prefixes.iter().any(|prefix| {
            let prefix = prefix.trim().trim_start_matches("::");
            let prefix = match prefix.strip_prefix("crate") {
                Some(rest) if rest.is_empty() || rest.starts_with("::") => format!("{}{rest}", self.universe.crate_name),
                _ => prefix.to_string(),
            };
            !prefix.is_empty() && crate::source::is_within(&decl.module, &prefix)
        })
    }

    /// The name of the view of a view model by the conventions: the module and the name with the
    /// replacement rules applied, the interface prefix stripped (the full path, without `::`).
    fn derived_view_name(&self, view_model: &TypeRef) -> Option<String> {
        let decl = self.universe.get(&view_model.key)?;
        let module = apply_rules(&decl.module, &self.options.effective_namespace_rules());
        let mut name = apply_rules(&decl.name, &self.options.effective_type_name_rules());
        if decl.kind == Kind::Interface {
            name = strip_interface_prefix(&name, &self.options.interface_prefixes_to_strip);
        }
        Some(format!("{module}::{name}"))
    }

    /// The type a derived view name names, with the generic parameters the options allow.
    fn find_view(&self, view_model: &TypeRef, name: &str) -> Option<&TypeDecl> {
        let decl = self.universe.get(&format!("::{name}"))?;
        let view_model_generics = self.universe.get(&view_model.key).map(|decl| decl.generics.len()).unwrap_or_default();
        if !self.options.strip_generic_arity_from_view_name && view_model_generics > 0 && decl.generics.len() != view_model_generics {
            return None;
        }
        Some(decl)
    }

    /// The key of the view of a view model by the conventions (`GetTypeByMetadataName`).
    fn derived_view(&self, view_model: &TypeRef) -> Option<String> {
        let name = self.derived_view_name(view_model)?;
        self.find_view(view_model, &name).map(|decl| decl.key.clone())
    }

    /// `IsSupportedView`: the view or one of its base classes is a view base type.
    fn is_supported_view(&self, decl: &TypeDecl, view_base_types: &BTreeSet<String>) -> bool {
        decl.kind == Kind::Class
            && (view_base_types.contains(&decl.key) || self.universe.base_chain(&decl.key).iter().any(|base| view_base_types.contains(base)))
    }

    /// `CanInstantiateView`: the constructor generated source calls, when the view can be
    /// created from the module of the locator.
    fn constructor_of(&self, decl: &TypeDecl) -> Option<String> {
        if decl.kind != Kind::Class || decl.is_generic() || !self.universe.is_accessible(decl, self.module) {
            return None;
        }
        if !self.universe.is_constructor_accessible(decl, self.module) {
            return None;
        }
        decl.constructor.as_ref().map(|(code, _)| code.clone())
    }

    /// How generated source names a view model type.
    fn model_key(&self, view_model: &TypeRef) -> ModelKey {
        match self.universe.get(&view_model.key) {
            Some(decl) if decl.kind == Kind::Interface => {
                ModelKey::Interface((decl.dyn_compatible && !view_model.constructed).then(|| format!("dyn {}", view_model.code)))
            }
            Some(decl) if decl.is_generic() && !view_model.constructed => ModelKey::GenericDefinition(view_model.key.trim_start_matches("::").to_string()),
            Some(decl) => ModelKey::Type { code: view_model.code.clone(), object_model: decl.object_model },
            None => ModelKey::Type { code: view_model.code.clone(), object_model: false },
        }
    }

    /// Whether `view_model` is `target`, derives from it or implements it (`data is T`).
    fn is_assignable(&self, view_model: &TypeRef, target: &TypeRef) -> bool {
        if view_model.canonical == target.canonical {
            return true;
        }
        if target.constructed {
            return false;
        }
        view_model.key == target.key && !view_model.constructed
            || self.universe.base_chain(&view_model.key).contains(&target.key)
            || self.universe.all_interfaces(&view_model.key).contains(&target.key)
    }

    /// `AddCompileTimeFallbackMappings`: the views of the view models that have none of their own,
    /// by the precedence of `mode`, flattened into the table of the views. The reference flattens
    /// the relevant view models and tests the other types at run time; the port flattens every
    /// type of the crate (a Rust value cannot be tested for a trait at run time).
    fn add_compile_time_fallback_mappings(
        &mut self,
        relevant: &[TypeRef],
        resolved: &mut Vec<Mapping>,
        resolved_view_models: &mut BTreeSet<String>,
        mode: ResolutionMode,
    ) {
        let universe = self.universe;
        let mut mapped: BTreeMap<String, Mapping> = resolved.iter().map(|mapping| (mapping.view_model.canonical.clone(), mapping.clone())).collect();
        let relevant_keys: BTreeSet<&str> = relevant.iter().map(|type_ref| type_ref.canonical.as_str()).collect();

        // Open generic view models: a closed instance has a `TypeId` of its own, and the adapters
        // test no contract at run time (SVL0008), unless the mapped traits the mode would select
        // are ambiguous already (SVL0007, `ValidateOpenGenericAdapterMapping`).
        for view_model in relevant {
            let Some(decl) = universe.get(&view_model.key) else { continue };
            if decl.kind == Kind::Class && decl.is_generic() && !view_model.constructed && mapped.contains_key(&view_model.canonical) {
                let candidates = self.nearest_mapped_interfaces(&view_model.key, &mapped);
                let mut views: Vec<&str> = candidates.iter().map(|candidate| mapped[candidate].view.key.as_str()).collect();
                views.dedup();
                views.sort();
                views.dedup();
                let base_view = universe.base_chain(&view_model.key).iter().any(|base| mapped.contains_key(base));
                let interface_selected = match mode {
                    ResolutionMode::ExactThenInterfaces => true,
                    ResolutionMode::ExactThenInterfacesThenBaseTypes => !candidates.is_empty(),
                    ResolutionMode::ExactThenBaseTypesThenInterfaces => !base_view,
                    _ => false,
                };
                if interface_selected && views.len() > 1 {
                    self.diagnostics.push(Diagnostic::error(
                        diagnostics::AMBIGUOUS_FALLBACK_MAPPING,
                        format!(
                            "View model '{}' inherits multiple mapped interfaces ({}); add #[static_view_mapping] to select one view",
                            view_model.display(),
                            candidates.iter().map(|key| key.trim_start_matches("::")).collect::<Vec<_>>().join(", ")
                        ),
                        &location_of(decl),
                    ));
                    continue;
                }
                self.diagnostics.push(Diagnostic::error(
                    diagnostics::UNSUPPORTED_OPEN_GENERIC_ADAPTER_MAPPING,
                    format!(
                        "Open generic view model '{}' requires a mapping of each instantiation for generated adapter resolution: a generated adapter finds a view by the exact type of a view model",
                        view_model.display()
                    ),
                    &location_of(decl),
                ));
            }
        }
        if mode == ResolutionMode::Exact {
            return;
        }

        let mut candidates: Vec<TypeRef> = relevant.to_vec();
        for decl in universe.types().filter(|decl| decl.local && decl.kind == Kind::Class && !decl.is_generic()) {
            let type_ref = universe.type_ref_of(&decl.key);
            if !relevant_keys.contains(type_ref.canonical.as_str()) {
                candidates.push(type_ref);
            }
        }
        for view_model in candidates {
            let Some(decl) = universe.get(&view_model.key) else { continue };
            if resolved_view_models.contains(&view_model.canonical) || decl.kind != Kind::Class || (decl.is_generic() && !view_model.constructed) {
                continue;
            }
            let is_relevant = relevant_keys.contains(view_model.canonical.as_str());
            let base_view = universe.base_chain(&view_model.key).iter().find_map(|base| mapped.get(base).cloned());

            let interface_candidates = self.nearest_mapped_interfaces(&view_model.key, &mapped);

            let mut interface_ambiguous = false;
            let mut select_interface = |planner: &mut Self| -> Option<Mapping> {
                if interface_candidates.is_empty() {
                    return None;
                }
                let mut views: Vec<&Mapping> = Vec::new();
                for candidate in &interface_candidates {
                    let mapping = &mapped[candidate];
                    if !views.iter().any(|view| view.view.key == mapping.view.key) {
                        views.push(mapping);
                    }
                }
                if views.len() == 1 {
                    return Some(views[0].clone());
                }
                interface_ambiguous = true;
                if is_relevant {
                    planner.diagnostics.push(Diagnostic::error(
                        diagnostics::AMBIGUOUS_FALLBACK_MAPPING,
                        format!(
                            "View model '{}' inherits multiple mapped interfaces ({}); add #[static_view_mapping] to select one view",
                            view_model.display(),
                            interface_candidates.iter().map(|key| key.trim_start_matches("::")).collect::<Vec<_>>().join(", ")
                        ),
                        &location_of(decl),
                    ));
                    return None;
                }
                // A type the reference would test at run time: the first of the generated
                // interface branches (the deepest interface, then by name).
                let mut ordered = interface_candidates.clone();
                ordered.sort_by(|left, right| {
                    let depth = |key: &String| universe.all_interfaces(key).len();
                    depth(right).cmp(&depth(left)).then_with(|| left.cmp(right))
                });
                ordered.first().map(|key| mapped[key].clone())
            };

            let selected = match mode {
                ResolutionMode::Exact => None,
                ResolutionMode::ExactThenBaseTypes => base_view,
                ResolutionMode::ExactThenInterfaces => select_interface(self),
                ResolutionMode::ExactThenBaseTypesThenInterfaces => base_view.or_else(|| select_interface(self)),
                ResolutionMode::ExactThenInterfacesThenBaseTypes => {
                    let selected = select_interface(self);
                    if selected.is_none() && !interface_ambiguous {
                        base_view
                    } else {
                        selected
                    }
                }
            };
            let Some(selected) = selected else { continue };
            let mapping = Mapping { view_model: view_model.clone(), view: selected.view.clone(), constructor: selected.constructor.clone() };
            resolved_view_models.insert(view_model.canonical.clone());
            mapped.insert(view_model.canonical.clone(), mapping.clone());
            resolved.push(mapping);
        }
    }

    /// The mapped traits of a type that no other mapped trait of the type extends, by name.
    fn nearest_mapped_interfaces(&self, key: &str, mapped: &BTreeMap<String, Mapping>) -> Vec<String> {
        let universe = self.universe;
        let all_interfaces = universe.all_interfaces(key);
        let mut candidates: Vec<String> = all_interfaces
            .iter()
            .filter(|candidate| mapped.contains_key(*candidate))
            .filter(|candidate| {
                !all_interfaces.iter().any(|other| other != *candidate && mapped.contains_key(other) && universe.all_interfaces(other).contains(candidate))
            })
            .cloned()
            .collect();
        candidates.sort_by(|left, right| left.trim_start_matches("::").cmp(right.trim_start_matches("::")));
        candidates
    }

    /// The tables of the legacy fallback: for each type of the crate without an entry of its own,
    /// the entry of its nearest base class (`TryGetFactory` and `TryGetMissingView` walk
    /// `BaseType`) and of its last trait whose name ends with `ViewModel`
    /// (`TryGetFactoryFromInterfaces` walks `GetInterfaces()` from the end).
    fn legacy_tables(&self, resolved: &[Mapping], missing: &[(TypeRef, String)]) -> LegacyTables {
        let universe = self.universe;
        let views: BTreeMap<&str, &Mapping> = resolved.iter().map(|mapping| (mapping.view_model.canonical.as_str(), mapping)).collect();
        let missing: BTreeMap<&str, &String> = missing.iter().map(|(view_model, message)| (view_model.canonical.as_str(), message)).collect();
        let mut tables = LegacyTables::default();
        for decl in universe.types().filter(|decl| decl.local && decl.kind == Kind::Class && !decl.is_generic()) {
            if views.contains_key(decl.key.as_str()) {
                continue;
            }
            let type_ref = universe.type_ref_of(&decl.key);
            let key = self.model_key(&type_ref);
            let bases = universe.base_chain(&decl.key);
            if let Some(mapping) = bases.iter().find_map(|base| views.get(base.as_str())) {
                tables.base_views.push((key.clone(), Mapping { view_model: type_ref.clone(), ..(*mapping).clone() }));
            }
            if !missing.contains_key(decl.key.as_str()) {
                if let Some(message) = bases.iter().find_map(|base| missing.get(base.as_str())) {
                    tables.base_missing_views.push((key.clone(), (*message).clone()));
                }
            }
            let interfaces: Vec<String> = universe.all_interfaces(&decl.key).into_iter().rev().filter(|interface| interface.ends_with("ViewModel")).collect();
            if let Some(mapping) = interfaces.iter().find_map(|interface| views.get(interface.as_str())) {
                tables.interface_views.push((key.clone(), Mapping { view_model: type_ref.clone(), ..(*mapping).clone() }));
            }
            if let Some(message) = interfaces.iter().find_map(|interface| missing.get(interface.as_str())) {
                tables.interface_missing_views.push((key, (*message).clone()));
            }
        }
        tables
    }
}

fn location_of(decl: &TypeDecl) -> Location {
    decl.location.clone()
}

/// `ApplyReplacementRules`.
pub fn apply_rules(input: &str, rules: &[ReplacementRule]) -> String {
    let mut value = input.to_string();
    for rule in rules {
        value = value.replace(&rule.from, &rule.to);
    }
    value
}

/// `StripInterfacePrefix`: the first prefix the name starts with, when the name is longer.
pub fn strip_interface_prefix(name: &str, prefixes: &[String]) -> String {
    for prefix in prefixes.iter().filter(|prefix| !prefix.is_empty()) {
        if name.starts_with(prefix.as_str()) && name.len() > prefix.len() {
            return name[prefix.len()..].to_string();
        }
    }
    name.to_string()
}

/// The generated source of a locator.
pub fn generate_locator(
    universe: &Universe<'_>,
    options: &GeneratorOptions,
    locator: &SourceLocator,
    reactiveui_available: bool,
    diagnostics: &mut Vec<Diagnostic>,
) -> String {
    let plan = plan_locator(universe, options, locator, reactiveui_available, diagnostics);
    emit::emit(&plan)
}
