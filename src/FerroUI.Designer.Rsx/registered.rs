//! Syntax alone cannot resolve a Rust import or prove that an application builder
//! matches a XAML type. A package can declare the correspondence in its exported
//! MarkupAssembly metadata. Ordinary Rust name resolution remains rust-analyzer's.
use crate::{catalog, literal::Literal};
use ferroui_designer_language::Schema;
use ferroui_designer_project::{
    rsx_mapping::{assembly_path, RsxBuilderMapping, PREFIX}, MarkupCatalog, MarkupControl,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, OnceLock};
use syn::ext::IdentExt;

#[path = "registered/animation.rs"]
mod animation;
#[path = "registered/control.rs"]
mod control;
pub(crate) use control::Class;

#[derive(Clone, Default)]
pub struct RsxCatalog {
    registered: Vec<Arc<RegisteredClass>>,
    animation_properties: BTreeMap<String, crate::animation::Property>,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct RegisteredClass {
    name: String,
    namespace: String,
    aliases: BTreeSet<String>,
    rust_path: String,
    base: Option<String>,
    properties: BTreeMap<String, MappedProperty>,
}

#[derive(Debug, PartialEq, Eq)]
struct MappedProperty {
    setter: String,
    value_type: String,
    codec: Literal,
}

impl RsxCatalog {
    pub fn built_in() -> &'static Self {
        static BUILTIN: OnceLock<RsxCatalog> = OnceLock::new();
        BUILTIN.get_or_init(Self::default)
    }

    pub fn from_project(catalog: &MarkupCatalog, schema: &Schema, package: Option<&str>) -> Self {
        let package = package.map(|name| name.replace('-', "_"));
        let mut candidates = BTreeMap::new();
        for control in catalog.controls() {
            let key = format!("{PREFIX}{}", control.class.full_name());
            let Some(value) = unique(
                control
                    .assembly
                    .metadata
                    .iter()
                    .filter(|(name, _)| name == &key),
            ) else {
                continue;
            };
            let Ok(declaration) = RsxBuilderMapping::parse(&value.1) else {
                continue;
            };
            if let Some(class) =
                registered(catalog, schema, control, package.as_deref(), declaration)
            {
                let path = class.rust_path.clone();
                let entry = candidates
                    .entry(path)
                    .or_insert_with(|| (class.clone(), false));
                entry.1 |= entry.0 != class;
            }
        }
        let registered = candidates
            .into_values()
            .filter_map(|(class, ambiguous)| (!ambiguous).then_some(class))
            .collect();
        let animation_properties = animation::properties(catalog, package.as_deref());
        Self { registered, animation_properties }
    }

    pub(crate) fn animation_property(&self, path: &str) -> Option<&crate::animation::Property> {
        self.animation_properties.get(path)
    }

    pub(crate) fn animation_getter(
        &self,
        name: &str,
        owner: Option<&crate::animation::TargetType>,
    ) -> Option<(&str, crate::animation::ValueShape)> {
        let (path, property) = unique(self.animation_properties.iter().filter(|(_, property)| {
            property.name == name && owner.is_none_or(|owner| property.owner.as_ref() == Some(owner))
        }))?;
        Some((path, property.shape))
    }

    pub(crate) fn rust(&self, path: &syn::Path) -> Option<Class> {
        if path
            .segments
            .iter()
            .any(|segment| !segment.arguments.is_empty())
        {
            return None;
        }
        let class = path.segments.last()?.ident.unraw().to_string();
        let text = path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        let module = text.rsplit_once("::").map_or("", |(module, _)| module);
        let built_in_module = matches!(
            module,
            "" | "ferroui_controls" | "ferroui_view::prelude" | "ferroui_view::generated::builders"
        ) || (class == "TreeDataGrid"
            && matches!(module, "ferroui_controls_treedatagrid" | "ferroui_view::generated::treedatagrid"));
        if built_in_module && catalog::children(&class).is_some() {
            return Some(Class::Builtin(class));
        }
        self.registered
            .iter()
            .find(|class| class.rust_path.trim_start_matches("::") == text)
            .map(|class| Class::Registered(class.clone()))
    }

    pub(crate) fn xaml(&self, name: &str, namespace: Option<&str>) -> Option<Class> {
        if catalog::framework_type(name, namespace) {
            return Some(Class::Builtin(name.into()));
        }
        let namespace = namespace?;
        let mut exact = self
            .registered
            .iter()
            .filter(|class| class.name == name && class.namespace == namespace);
        if let Some(class) = exact.next() {
            return exact
                .next()
                .is_none()
                .then(|| Class::Registered(class.clone()));
        }
        unique(
            self.registered
                .iter()
                .filter(|class| class.name == name && class.aliases.contains(namespace)),
        )
        .map(|class| Class::Registered(class.clone()))
    }
}

fn registered(
    catalog: &MarkupCatalog,
    schema: &Schema,
    control: MarkupControl<'_>,
    package: Option<&str>,
    declaration: RsxBuilderMapping,
) -> Option<Arc<RegisteredClass>> {
    let rust_path = assembly_path(&declaration.path, &control.assembly.crate_name, package)?;
    let namespace = control.namespace();
    let base = declaration
        .inherit
        .then(|| inherited_control(catalog, &control))
        .flatten();
    if declaration.inherit && base.is_none() {
        return None;
    }
    let properties: BTreeMap<_, _> = declaration
        .properties
        .into_iter()
        .map(|(name, setter)| {
            let ident = syn::parse_str::<syn::Ident>(&setter).ok()?;
            if ident != setter.as_str() || setter.starts_with("on_") || name.contains('.') {
                return None;
            }
            let info = schema.property(&control.class.name, Some(&namespace), &name)?;
            if info.attached {
                return None;
            }
            let codec = catalog::property_codec(&info)?;
            Some((
                name,
                MappedProperty {
                    setter,
                    value_type: info.type_name,
                    codec,
                },
            ))
        })
        .collect::<Option<_>>()?;
    let setters: BTreeSet<_> = properties
        .values()
        .map(|property| property.setter.as_str())
        .collect();
    if setters.len() != properties.len() {
        return None;
    }
    let aliases = std::iter::once(format!("using:{}", control.class.namespace))
        .chain(
            control
                .assembly
                .xmlns_definitions
                .iter()
                .filter(|definition| definition.namespace == control.class.namespace)
                .map(|definition| definition.xml_namespace.clone()),
        )
        .collect();
    Some(Arc::new(RegisteredClass {
        name: control.class.name.clone(),
        namespace,
        aliases,
        rust_path,
        base,
        properties,
    }))
}

fn inherited_control(catalog: &MarkupCatalog, control: &MarkupControl<'_>) -> Option<String> {
    let mut base = control.class.base.as_ref();
    let mut seen = BTreeSet::new();
    while let Some(path) = base {
        if !seen.insert(path.text.as_str()) {
            break;
        }
        let (assembly, class) = catalog.models().find_rust_type(&path.text)?;
        if assembly.crate_name == "ferroui_controls" && catalog::children(&class.name).is_some() {
            return Some(class.name.clone());
        }
        base = class.base.as_ref();
    }
    None
}

fn unique<T>(mut matches: impl Iterator<Item = T>) -> Option<T> {
    let value = matches.next()?;
    matches.next().is_none().then_some(value)
}
