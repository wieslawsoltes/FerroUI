//! The generated source of a locator: one `impl` block of the struct with the tables and the
//! members (the generated `partial class` of the reference), followed by the trait `impl` blocks
//! of the adapters.
//!
//! The names follow the reference in snake case (`s_views`, `try_create_view_exact`,
//! `build_resolved_view`); a C# `out` parameter is an `Option` result.

use crate::generator::{LocatorPlan, Mapping, ModelKey};
use std::fmt::Write;

/// The crate the generated source names.
const RUNTIME: &str = "::ferroui_static_view_locator";

/// The generated source of a locator.
pub fn emit(plan: &LocatorPlan) -> String {
    let mut members = Members::default();
    let r = RUNTIME;
    let generated = format!("{r}::generated");

    // `s_views`.
    let views: Vec<String> = plan
        .views
        .iter()
        .filter_map(|(key, mapping)| type_id(key).map(|type_id| format!("{r}::TypeEntry::new({type_id}, || {}),", factory(mapping))))
        .collect();
    members.table("The views of the view model types (`s_views`).", "s_views", &format!("{r}::ViewTable"), &views);

    // `s_missingViews`.
    if let Some(missing) = &plan.missing_views {
        let entries: Vec<String> = missing
            .iter()
            .filter_map(|(key, message)| type_id(key).map(|type_id| format!("{r}::TypeEntry::new({type_id}, {}),", string(message))))
            .collect();
        members.table(
            "The messages of the view model types whose view is not found (`s_missingViews`).",
            "s_missing_views",
            &format!("{r}::MissingViewTable"),
            &entries,
        );
    }

    if let Some(legacy) = &plan.legacy {
        let generic_views: Vec<String> = plan
            .views
            .iter()
            .filter_map(|(key, mapping)| match key {
                ModelKey::GenericDefinition(path) => Some(format!("({}, || {}),", string(path), factory(mapping))),
                _ => None,
            })
            .collect();
        members.name_table(
            "The views of the open generic view model types, by the path of the definition (the `s_views` entries of generic type definitions).",
            "s_generic_views",
            &format!("{r}::NameTable<{r}::ViewFactory>"),
            &generic_views,
        );
        let generic_missing: Vec<String> = plan
            .missing_views
            .iter()
            .flatten()
            .filter_map(|(key, message)| match key {
                ModelKey::GenericDefinition(path) => Some(format!("({}, {}),", string(path), string(message))),
                _ => None,
            })
            .collect();
        members.name_table(
            "The messages of the open generic view model types whose view is not found, by the path of the definition.",
            "s_generic_missing_views",
            &format!("{r}::NameTable<&'static str>"),
            &generic_missing,
        );
        let flattened = |entries: &[(ModelKey, Mapping)]| -> Vec<String> {
            entries
                .iter()
                .filter_map(|(key, mapping)| type_id(key).map(|type_id| format!("{r}::TypeEntry::new({type_id}, || {}),", factory(mapping))))
                .collect()
        };
        let flattened_missing = |entries: &[(ModelKey, String)]| -> Vec<String> {
            entries
                .iter()
                .filter_map(|(key, message)| type_id(key).map(|type_id| format!("{r}::TypeEntry::new({type_id}, {}),", string(message))))
                .collect()
        };
        members.table(
            "The views of the types whose nearest mapped base class has a view (the `BaseType` walk of `TryGetFactory`, flattened).",
            "s_base_views",
            &format!("{r}::ViewTable"),
            &flattened(&legacy.base_views),
        );
        members.table(
            "The views of the types whose last mapped trait has a view (the `GetInterfaces()` walk of `TryGetFactoryFromInterfaces`, flattened).",
            "s_interface_views",
            &format!("{r}::ViewTable"),
            &flattened(&legacy.interface_views),
        );
        members.table(
            "The missing-view messages of the types by their nearest base class (the `BaseType` walk of `TryGetMissingView`, flattened).",
            "s_base_missing_views",
            &format!("{r}::MissingViewTable"),
            &flattened_missing(&legacy.base_missing_views),
        );
        members.table(
            "The missing-view messages of the types by their last trait (the `GetInterfaces()` walk of `TryGetMissingViewFromInterfaces`, flattened).",
            "s_interface_missing_views",
            &format!("{r}::MissingViewTable"),
            &flattened_missing(&legacy.interface_missing_views),
        );
    }

    if plan.resolver {
        members.function(&format!(
            "    /// The factory of the view of `instance`: an exact lookup of its run-time type in the table of the
    /// views, where the fallback precedence is flattened (`TryGetResolvedViewFactory`).
    fn try_get_resolved_view_factory(instance: {generated}::Option<&{generated}::BoxedValue>) -> {generated}::Option<{r}::ViewFactory> {{
        let instance = instance?;
        Self::s_views().get({r}::RuntimeType::of_value(instance).id())
    }}

    /// The view of `instance` (`TryCreateResolvedView`).
    fn try_create_resolved_view(instance: {generated}::Option<&{generated}::BoxedValue>) -> {generated}::Option<{generated}::Ref<{generated}::Control>> {{
        Self::try_get_resolved_view_factory(instance).map(|factory| factory())
    }}
"
        ));
    }

    if let Some(match_types) = &plan.match_types {
        let entries: Vec<String> = match_types.iter().filter_map(|key| type_id(key).map(|type_id| format!("{r}::TypeEntry::new({type_id}, ()),"))).collect();
        members.table(
            "The resolved view model types that are of one of `data_template_match_types` (`data is T`, decided at build time).",
            "s_match_types",
            &format!("{r}::TypeSet"),
            &entries,
        );
    }

    if plan.try_create_view_exact {
        members.function(&format!(
            "    /// The view of the view model type `view_model_type`, by an exact lookup in the table of the views
    /// (`TryCreateViewExact`): no base class or trait is consulted.
    fn try_create_view_exact(view_model_type: {generated}::TypeId) -> {generated}::Option<{generated}::Ref<{generated}::Control>> {{
        Self::s_views().get(view_model_type).map(|factory| factory())
    }}
"
        ));
    }

    let mut trait_impls = String::new();
    let header = format!("impl{} {}{}{}", plan.generics_text(), plan.name, plan.generic_arguments_text(), plan.where_text());
    let trait_header = |trait_path: &str| format!("impl{} {trait_path} for {}{}{}", plan.generics_text(), plan.name, plan.generic_arguments_text(), plan.where_text());

    if let Some(view_locator) = &plan.view_locator {
        let rx = format!("{r}::reactiveui");
        let entries: Vec<String> = view_locator
            .view_fors
            .iter()
            .filter_map(|(key, mapping)| type_id(key).map(|type_id| format!("{r}::TypeEntry::new({type_id}, || {rx}::view_for({}())),", mapping.constructor)))
            .collect();
        members.table(
            "The views of ReactiveUI of the view model types whose view implements `IControlViewFor`.",
            "s_view_fors",
            &format!("{rx}::ViewForTable"),
            &entries,
        );
        if view_locator.resolve_view_for {
            members.function(&format!(
                "    /// A view of the view model type `VM` without a view model, under `contract` (`None` or empty for the
    /// default view; the generated map has no other contracts) (`ResolveView<TViewModel>(string?)`).
    pub fn resolve_view_for<VM: 'static>(&self, contract: {generated}::Option<&str>) -> {generated}::Option<{generated}::Rc<dyn {rx}::IViewForOf<VM>>> {{
        if !{rx}::is_default_contract(contract) {{
            return None;
        }}
        let factory = Self::s_view_fors().get({generated}::TypeId::of::<VM>())?;
        {rx}::view_for_of::<VM>(factory())
    }}
"
            ));
        }
        if view_locator.resolve_view_default {
            members.function(&format!(
                "    /// The default view of the view model type `VM` (`ResolveView<TViewModel>()`).
    pub fn resolve_view_default<VM: 'static>(&self) -> {generated}::Option<{generated}::Rc<dyn {rx}::IViewForOf<VM>>> {{
        self.resolve_view_for::<VM>(None)
    }}
"
            ));
        }
        if view_locator.implement_enable_logger {
            let _ = writeln!(trait_impls, "\n{} {{}}", trait_header(&format!("{rx}::IEnableLogger")));
        }
        if view_locator.implement_view_locator {
            let _ = write!(
                trait_impls,
                "
{} {{
    fn resolve_view_typed(&self, view_model: {generated}::Rc<dyn {generated}::Any>, view_model_type: &{rx}::ViewModelType, contract: {generated}::Option<&str>) -> {generated}::Option<{generated}::Rc<dyn {rx}::IViewFor>> {{
        if !{rx}::is_default_contract(contract) {{
            return None;
        }}
        let factory = Self::s_view_fors().get(view_model_type.id())?;
        let view = factory();
        {rx}::set_view_model(&view, view_model);
        Some(view)
    }}

    fn resolve_view(&self, view_model: {generated}::Option<{generated}::Rc<dyn {generated}::Any>>, contract: {generated}::Option<&str>) -> {generated}::Option<{generated}::Rc<dyn {rx}::IViewFor>> {{
        let view_model = view_model?;
        if !{rx}::is_default_contract(contract) {{
            return None;
        }}
        let factory = Self::s_view_fors().get({rx}::runtime_type_of(&view_model).id())?;
        let view = factory();
        {rx}::set_view_model(&view, view_model);
        Some(view)
    }}

    fn resolve_view_unsafe(&self, view_model: {generated}::Option<{generated}::Rc<dyn {generated}::Any>>, contract: {generated}::Option<&str>) -> {generated}::Option<{generated}::Rc<dyn {rx}::IViewFor>> {{
        {rx}::IViewLocator::resolve_view(self, view_model, contract)
    }}
}}
",
                trait_header(&format!("{rx}::IViewLocator"))
            );
        }
    }

    if plan.legacy_build {
        members.function(&format!(
            "    /// The view of `data` (`Build`): the view of its type, of its nearest base class or of its generic
    /// type definition, then of its traits; else the text of its missing view.
    ///
    /// # Panics
    /// Panics for a view model whose type has neither (`Unable to create view for type`).
    pub fn build(&self, data: {generated}::Option<&{generated}::BoxedValue>) -> {generated}::Option<{generated}::Ref<{generated}::Control>> {{
        let data = data?;
        let type_ = {r}::RuntimeType::of_value(data);
        if let Some(factory) = Self::try_get_factory(Some(&type_)).or_else(|| Self::try_get_factory_from_interfaces(&type_)) {{
            return Some(factory());
        }}
        if let Some(missing_view) = Self::try_get_missing_view(Some(&type_)).or_else(|| Self::try_get_missing_view_from_interfaces(&type_)) {{
            return Some({r}::text_view(missing_view));
        }}
        {r}::unable_to_create_view(&type_)
    }}
"
        ));
    }

    if let Some(build) = &plan.data_template_build {
        let _ = write!(
            trait_impls,
            "
{} {{
    fn build(&self, param: &{generated}::Option<{generated}::BoxedValue>) -> {generated}::Option<{generated}::Ref<{generated}::Control>> {{
        let Some(data) = param.as_ref() else {{
            return {generated}::Option::from(self.build_invalid_view(None));
        }};
        let view_model_type = {r}::RuntimeType::of_value(data);
        let control: {generated}::Option<{generated}::Ref<{generated}::Control>> = self.build_resolved_view(Some(data)).into();
        if control.is_some() {{
            return control;
        }}
        let control: {generated}::Option<{generated}::Ref<{generated}::Control>> = self.build_fallback_view(Some(data)).into();
        if control.is_some() {{
            return control;
        }}
        self.build_missing_view(Some(data), view_model_type).into()
    }}
}}
",
            trait_header(&format!("{generated}::ITemplateWithParam<{generated}::Option<{generated}::BoxedValue>, {generated}::Option<{generated}::Ref<{generated}::Control>>>"))
        );
        if build.build_resolved_view {
            let body = if build.with_view_locator {
                let rx = format!("{r}::reactiveui");
                format!(
                    "        let data = param?;
        if let Some(factory) = Self::s_view_fors().get({r}::RuntimeType::of_value(data).id()) {{
            let view = factory();
            {rx}::set_view_model(&view, {rx}::view_model_of(data));
            return {rx}::control_of(&view);
        }}
        let control = Self::try_create_resolved_view(Some(data))?;
        {r}::set_data_context(&control, data);
        Some(control)
"
                )
            } else {
                format!(
                    "        let data = param?;
        let control = Self::try_create_resolved_view(Some(data))?;
        {r}::set_data_context(&control, data);
        Some(control)
"
                )
            };
            members.function(&format!(
                "    /// The view of a resolved view model, with the view model as its data context (`BuildResolvedView`).
    fn build_resolved_view(&self, param: {generated}::Option<&{generated}::BoxedValue>) -> {generated}::Option<{generated}::Ref<{generated}::Control>> {{
{body}    }}
"
            ));
        }
        if build.build_fallback_view {
            members.function(&format!(
                "    /// The view of a view model without a resolved view, before the missing view (`BuildFallbackView`).
    fn build_fallback_view(&self, param: {generated}::Option<&{generated}::BoxedValue>) -> {generated}::Option<{generated}::Ref<{generated}::Control>> {{
        None
    }}
"
            ));
        }
        if build.build_invalid_view {
            members.function(&format!(
                "    /// The view of no view model (`BuildInvalidView`).
    fn build_invalid_view(&self, param: {generated}::Option<&{generated}::BoxedValue>) -> {generated}::Option<{generated}::Ref<{generated}::Control>> {{
        Some({r}::text_view(\"Invalid view model type.\"))
    }}
"
            ));
        }
        if build.build_missing_view {
            members.function(&format!(
                "    /// The view of a view model whose view is not found (`BuildMissingView`).
    fn build_missing_view(&self, param: {generated}::Option<&{generated}::BoxedValue>, view_model_type: {r}::RuntimeType) -> {generated}::Option<{generated}::Ref<{generated}::Control>> {{
        let message = match Self::s_missing_views().get(view_model_type.id()) {{
            Some(missing_view) => missing_view.to_string(),
            None => {r}::view_not_found_text(&view_model_type),
        }};
        Some({r}::text_view(&message))
    }}
"
            ));
        }
    }

    if let Some(match_plan) = &plan.data_template_match {
        if match_plan.implement_data_template {
            let _ = write!(
                trait_impls,
                "
{} {{
    fn match_(&self, data: {generated}::Option<&{generated}::BoxedValue>) -> bool {{
        self.match_data_template(data)
    }}

    fn as_any(&self) -> {generated}::Option<&dyn {generated}::Any> {{
        Some(self)
    }}
}}
",
                trait_header(&format!("{generated}::IDataTemplate"))
            );
        }
        if match_plan.match_data_template {
            let body = if plan.match_types.is_some() {
                format!(
                    "        let Some(data) = data else {{
            return false;
        }};
        let view_model_type = {r}::RuntimeType::of_value(data).id();
        Self::s_views().contains(view_model_type) && Self::s_match_types().contains(view_model_type)
"
                )
            } else {
                "        Self::try_get_resolved_view_factory(data).is_some()\n".to_string()
            };
            members.function(&format!(
                "    /// Whether the locator builds the view of `data`: only a view model with a resolved view matches
    /// (`MatchDataTemplate`).
    fn match_data_template(&self, data: {generated}::Option<&{generated}::BoxedValue>) -> bool {{
{body}    }}
"
            ));
        }
    }

    if plan.legacy.is_some() {
        members.function(&format!(
            "    /// The factory of the view of `type_` or of its nearest mapped base class (`TryGetFactory`).
    fn try_get_factory(type_: {generated}::Option<&{r}::RuntimeType>) -> {generated}::Option<{r}::ViewFactory> {{
        let type_ = type_?;
        Self::try_get_factory_for_type(type_).or_else(|| Self::s_base_views().get(type_.id()))
    }}

    /// The factory of the view of the last mapped trait of `type_` (`TryGetFactoryFromInterfaces`).
    fn try_get_factory_from_interfaces(type_: &{r}::RuntimeType) -> {generated}::Option<{r}::ViewFactory> {{
        Self::s_interface_views().get(type_.id())
    }}

    /// The factory of the view of `type_`, or of its generic type definition (`TryGetFactoryForType`).
    fn try_get_factory_for_type(type_: &{r}::RuntimeType) -> {generated}::Option<{r}::ViewFactory> {{
        Self::s_views()
            .get(type_.id())
            .or_else(|| type_.generic_definition().and_then(|definition| Self::s_generic_views().get(definition)))
    }}

    /// The missing-view message of `type_` or of its nearest base class (`TryGetMissingView`).
    fn try_get_missing_view(type_: {generated}::Option<&{r}::RuntimeType>) -> {generated}::Option<&'static str> {{
        let type_ = type_?;
        Self::try_get_missing_view_for_type(type_).or_else(|| Self::s_base_missing_views().get(type_.id()))
    }}

    /// The missing-view message of the last trait of `type_` (`TryGetMissingViewFromInterfaces`).
    fn try_get_missing_view_from_interfaces(type_: &{r}::RuntimeType) -> {generated}::Option<&'static str> {{
        Self::s_interface_missing_views().get(type_.id())
    }}

    /// The missing-view message of `type_`, or of its generic type definition (`TryGetMissingViewForType`).
    fn try_get_missing_view_for_type(type_: &{r}::RuntimeType) -> {generated}::Option<&'static str> {{
        Self::s_missing_views()
            .get(type_.id())
            .or_else(|| type_.generic_definition().and_then(|definition| Self::s_generic_missing_views().get(definition)))
    }}
"
        ));
    }

    let mut text = String::new();
    let _ = writeln!(text, "// @generated by ferroui-static-view-locator-build for the locator `{}` ({}).", plan.name, plan.module);
    let _ = writeln!(text, "// The build script of the crate writes this file again; do not edit it.");
    let _ = writeln!(text);
    let _ = writeln!(text, "#[allow(dead_code, unused_variables, clippy::all)]");
    let _ = writeln!(text, "{header} {{");
    text.push_str(&members.text);
    text.push_str("}\n");
    text.push_str(&trait_impls);
    text
}

impl LocatorPlan {
    fn generics_text(&self) -> String {
        compact(&self.generics)
    }

    fn generic_arguments_text(&self) -> String {
        compact(&self.generic_arguments)
    }

    fn where_text(&self) -> String {
        if self.where_clause.is_empty() {
            String::new()
        } else {
            format!(" {}", self.where_clause)
        }
    }
}

/// Tokens printed by `quote` without the space before `<` and `>`.
fn compact(text: &str) -> String {
    text.replace(" <", "<").replace("< ", "<").replace(" >", ">").replace(" ,", ",")
}

#[derive(Default)]
struct Members {
    text: String,
}

impl Members {
    fn separate(&mut self) {
        if !self.text.is_empty() {
            self.text.push('\n');
        }
    }

    fn function(&mut self, function: &str) {
        self.separate();
        self.text.push_str(function);
    }

    fn table(&mut self, documentation: &str, name: &str, type_: &str, entries: &[String]) {
        self.separate();
        let _ = writeln!(self.text, "    /// {documentation}");
        let _ = writeln!(self.text, "    fn {name}() -> &'static {type_} {{");
        // `NameTable<V>::new` is `NameTable::<V>::new` in an expression.
        let constructor = type_.replacen('<', "::<", 1);
        if entries.is_empty() {
            let _ = writeln!(self.text, "        static TABLE: {type_} = {constructor}::new(&[]);");
        } else {
            let _ = writeln!(self.text, "        static TABLE: {type_} = {constructor}::new(&[");
            for entry in entries {
                let _ = writeln!(self.text, "            {entry}");
            }
            let _ = writeln!(self.text, "        ]);");
        }
        let _ = writeln!(self.text, "        &TABLE");
        let _ = writeln!(self.text, "    }}");
    }

    fn name_table(&mut self, documentation: &str, name: &str, type_: &str, entries: &[String]) {
        self.table(documentation, name, type_, entries);
    }
}

/// `TypeId::of::<T>` of a view model type; `None` for a trait and an open generic type.
fn type_id(key: &ModelKey) -> Option<String> {
    match key {
        ModelKey::Type { code, object_model: false } => Some(format!("{RUNTIME}::generated::TypeId::of::<{code}>")),
        ModelKey::Type { code, object_model: true } => Some(format!("{RUNTIME}::generated::TypeId::of::<{RUNTIME}::generated::Ref<{code}>>")),
        ModelKey::Interface(Some(code)) => Some(format!("{RUNTIME}::generated::TypeId::of::<{code}>")),
        ModelKey::GenericDefinition(_) | ModelKey::Interface(None) => None,
    }
}

/// The factory of the view of a mapping.
fn factory(mapping: &Mapping) -> String {
    format!("{RUNTIME}::upcast({}())", mapping.constructor)
}

/// A Rust string literal.
fn string(text: &str) -> String {
    format!("{text:?}")
}
