//! The options of the generator for a crate (the MSBuild properties `StaticViewLocator*` of the
//! reference, `GeneratorOptions`), set by the build script.
//!
//! A multi-valued option is a list, or one text whose values are separated by commas, as the
//! reference reads its properties; a replacement rule is `from=to`.

/// A replacement applied to a path or a name (`ReplacementRule`): every occurrence of `from` is
/// replaced by `to`, which may be empty.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplacementRule {
    pub from: String,
    pub to: String,
}

impl ReplacementRule {
    pub fn new(from: &str, to: &str) -> Self {
        Self { from: from.to_string(), to: to.to_string() }
    }
}

/// The namespace rules that are always applied after the configured ones: `view_models=views`
/// (`ViewModels=Views` of the reference, for a module of view models beside a module of views)
/// and `_view_model=_view` (an addition, for a module per type: `settings_view_model` beside
/// `settings_view`).
pub const DEFAULT_NAMESPACE_RULES: &[(&str, &str)] = &[("view_models", "views"), ("_view_model", "_view")];

/// The type name rule that is always applied after the configured ones (`ViewModel=View`).
pub const DEFAULT_TYPE_NAME_RULES: &[(&str, &str)] = &[("ViewModel", "View")];

/// The view base types: a view derives from one of them (`UserControl` and `Window`, the defaults
/// of the reference).
pub const DEFAULT_VIEW_BASE_TYPES: &[&str] = &["::ferroui_controls::UserControl", "::ferroui_controls::Window"];

/// The options of the generator (`GeneratorOptions`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratorOptions {
    /// The module paths view models are taken from (`StaticViewLocatorViewModelNamespacePrefixes`):
    /// a view model is taken when its module is one of them or below one; empty, from every
    /// module. A path is written from the crate (`crate::view_models`, or the name of the crate).
    pub namespace_prefixes: Vec<String>,
    /// `StaticViewLocatorIncludeInternalViewModels`. Kept for the options of the reference and
    /// without an effect: Rust has no `InternalsVisibleTo`, and a view model of another crate is
    /// taken only when it is public.
    pub include_internal_view_models: bool,
    /// Whether the view models of the crates the crate depends on are taken
    /// (`StaticViewLocatorIncludeReferencedAssemblies`): the types of their models (the
    /// `.xamlmeta` of the crates that export one) whose name ends with `ViewModel`.
    pub include_referenced_crates: bool,
    /// More view base types (`StaticViewLocatorAdditionalViewBaseTypes`), as Rust paths from the
    /// crate (`crate::controls::ToolWindowBase`) or of a dependency (`my_controls::Panel`).
    pub additional_view_base_types: Vec<String>,
    /// Applied in order to the module path of a view model to find the module of its view
    /// (`StaticViewLocatorNamespaceReplacementRules`), then [`DEFAULT_NAMESPACE_RULES`].
    pub namespace_replacement_rules: Vec<ReplacementRule>,
    /// Applied in order to the name of a view model to find the name of its view
    /// (`StaticViewLocatorTypeNameReplacementRules`), then [`DEFAULT_TYPE_NAME_RULES`].
    pub type_name_replacement_rules: Vec<ReplacementRule>,
    /// Whether the view of a generic view model is found by the name alone, whatever the generic
    /// parameters of the view (`StaticViewLocatorStripGenericArityFromViewName`, default `true`).
    pub strip_generic_arity_from_view_name: bool,
    /// Removed from the name of a trait view model before its view is looked up
    /// (`StaticViewLocatorInterfacePrefixesToStrip`, default `I`).
    pub interface_prefixes_to_strip: Vec<String>,
}

impl Default for GeneratorOptions {
    fn default() -> Self {
        Self {
            namespace_prefixes: Vec::new(),
            include_internal_view_models: false,
            include_referenced_crates: false,
            additional_view_base_types: Vec::new(),
            namespace_replacement_rules: Vec::new(),
            type_name_replacement_rules: Vec::new(),
            strip_generic_arity_from_view_name: true,
            interface_prefixes_to_strip: vec!["I".to_string()],
        }
    }
}

impl GeneratorOptions {
    /// The namespace rules applied in order: the configured ones, then the defaults
    /// (`GetReplacementRules`).
    pub fn effective_namespace_rules(&self) -> Vec<ReplacementRule> {
        let mut rules = self.namespace_replacement_rules.clone();
        rules.extend(DEFAULT_NAMESPACE_RULES.iter().map(|(from, to)| ReplacementRule::new(from, to)));
        rules
    }

    /// The type name rules applied in order: the configured ones, then the default.
    pub fn effective_type_name_rules(&self) -> Vec<ReplacementRule> {
        let mut rules = self.type_name_replacement_rules.clone();
        rules.extend(DEFAULT_TYPE_NAME_RULES.iter().map(|(from, to)| ReplacementRule::new(from, to)));
        rules
    }

    /// Sets the option `name` from its text, as the reference reads an MSBuild property: the
    /// names are the ones of the setters of [`StaticViewLocatorBuild`](crate::StaticViewLocatorBuild)
    /// (`view_model_namespace_prefixes`, `include_internal_view_models`,
    /// `include_referenced_crates`, `additional_view_base_types`, `namespace_replacement_rules`,
    /// `type_name_replacement_rules`, `strip_generic_arity_from_view_name`,
    /// `interface_prefixes_to_strip`), and the properties of the reference
    /// (`StaticViewLocatorViewModelNamespacePrefixes`, ..) are accepted for them. A boolean that
    /// is not `true` or `false` is the default, as `bool.TryParse` fails.
    pub fn set(&mut self, name: &str, value: &str) -> Result<(), String> {
        let name = match name.strip_prefix("StaticViewLocator") {
            Some(property) => snake_case(property),
            None => name.to_string(),
        };
        match name.as_str() {
            "view_model_namespace_prefixes" => self.namespace_prefixes = parse_list(value),
            "include_internal_view_models" => self.include_internal_view_models = parse_bool(value).unwrap_or(false),
            "include_referenced_crates" | "include_referenced_assemblies" => {
                self.include_referenced_crates = parse_bool(value).unwrap_or(false)
            }
            "additional_view_base_types" => self.additional_view_base_types = parse_list(value),
            "namespace_replacement_rules" => self.namespace_replacement_rules = parse_rules(value),
            "type_name_replacement_rules" => self.type_name_replacement_rules = parse_rules(value),
            "strip_generic_arity_from_view_name" => self.strip_generic_arity_from_view_name = parse_bool(value).unwrap_or(true),
            "interface_prefixes_to_strip" => {
                let prefixes = parse_list(value);
                self.interface_prefixes_to_strip = if prefixes.is_empty() { vec!["I".to_string()] } else { prefixes };
            }
            _ => return Err(format!("`{name}` is not an option of the generator")),
        }
        Ok(())
    }
}

/// `Foo` of `StaticViewLocatorFoo` in snake case (`ViewModelNamespacePrefixes` to
/// `view_model_namespace_prefixes`).
fn snake_case(name: &str) -> String {
    let mut text = String::with_capacity(name.len() + 8);
    for (index, character) in name.chars().enumerate() {
        if character.is_ascii_uppercase() {
            if index > 0 {
                text.push('_');
            }
            text.push(character.to_ascii_lowercase());
        } else {
            text.push(character);
        }
    }
    text
}

/// The values of a list: separated by commas (and semicolons, which the reference accepts where
/// MSBuild keeps them), trimmed, the empty ones left out.
pub fn parse_list(value: &str) -> Vec<String> {
    value.split([',', ';']).map(str::trim).filter(|part| !part.is_empty()).map(str::to_string).collect()
}

/// The rules of a list of `from=to` values; a value without `=` or with an empty `from` is left
/// out.
pub fn parse_rules(value: &str) -> Vec<ReplacementRule> {
    parse_list(value)
        .iter()
        .filter_map(|part| {
            let (from, to) = part.split_once('=')?;
            let from = from.trim();
            (!from.is_empty()).then(|| ReplacementRule::new(from, to.trim()))
        })
        .collect()
}

/// `bool.TryParse`: `true` or `false`, ignoring case and the white space around.
pub fn parse_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}
