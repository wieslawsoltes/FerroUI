use super::{MarkupCatalog, MarkupControl};
use ferroui_build_scan::model::{AssemblyModel, MemberModel, TypeModel};
use std::collections::BTreeSet;

const INTERACTIVE: &str = "::ferroui_base::interactivity::Interactive";
const ROUTED_EVENT: &str = "::ferroui_base::interactivity::RoutedEvent";
const SENDER: &str = "Option<::ferroui_base::BoxedValue>";
const ROUTED_ARGUMENTS: &str = "::std::rc::Rc<dyn ::ferroui_base::interactivity::IRoutedEventArgs>";

/// A known two-part event contract (control member and the Rust callback parameters).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarkupEventSignature {
    pub owner: String,
    pub member: String,
    pub parameters: Vec<String>,
}

impl MarkupCatalog {
    /// Resolve an XML type with the build export's namespace and assembly mappings.
    pub fn xml_type(&self, namespace: &str, name: &str) -> Option<MarkupControl<'_>> {
        unique(self.models().models().iter().flat_map(|assembly| {
            assembly.types.iter().filter_map(move |class| {
                (class.name == name
                    && class.generic.is_none()
                    && !class.unregistered
                    && namespace_matches(assembly, &class.namespace, namespace))
                .then_some(MarkupControl { assembly, class })
            })
        }))
    }

    /// An authored class is resolved only among the selected package's crate targets.
    pub fn code_behind_type(
        &self,
        full_name: &str,
        crate_names: &BTreeSet<String>,
    ) -> Option<MarkupControl<'_>> {
        let (namespace, name) = full_name.rsplit_once('.').unwrap_or(("", full_name));
        unique(
            self.models()
                .models()
                .iter()
                .filter(|assembly| crate_names.contains(&assembly.crate_name))
                .flat_map(|assembly| {
                    assembly.types.iter().filter_map(move |class| {
                        (class.name == name
                            && class.namespace == namespace
                            && class.object_model
                            && class.generic.is_none()
                            && !class.unregistered)
                            .then_some(MarkupControl { assembly, class })
                    })
                }),
        )
    }

    /// Resolve declared and inherited events as the XAML compiler does. The non-generic
    /// routed delegate is registered for every routed event by the compiler and loader.
    pub fn event_signature(
        &self,
        target: &TypeModel,
        owner: &TypeModel,
        member: &str,
        crate_name: &str,
    ) -> Option<MarkupEventSignature> {
        let field = format!("{member}Event");
        let mut classes = self.ancestry(owner);
        if let Some((declaring, value)) = classes.iter().find_map(|class| {
            let value = class
                .fields
                .iter()
                .find(|value| value.is_static && value.name == field)?;
            Some((*class, value))
        }) {
            if self.routed_event_argument(value).is_none()
                || !self.derives_from(target, INTERACTIVE)
            {
                return None;
            }
            return Some(signature(
                declaring,
                member,
                vec![SENDER.into(), ROUTED_ARGUMENTS.into()],
            ));
        }
        if !self.derives_from(target, &owner.rust_path.text) {
            return None;
        }
        for class in classes.drain(..) {
            if let Some(event) = unique(class.events.iter().filter(|event| event.name == member)) {
                let parameters = event
                    .parameters
                    .iter()
                    .map(|p| {
                        p.type_
                            .is_resolved()
                            .then(|| self.models().source_type(&p.type_.text, crate_name))
                    })
                    .collect::<Option<Vec<_>>>()?;
                return Some(signature(class, member, parameters));
            }
            if class.events.iter().any(|event| event.name == member) {
                return None;
            }
        }
        None
    }

    pub fn has_markup_method(&self, class: &TypeModel, name: &str) -> bool {
        self.ancestry(class)
            .iter()
            .any(|owner| owner.methods.iter().any(|method| method.name == name))
    }

    /// The display type of a metadata field that declares a routed event.
    pub fn routed_event_argument(&self, field: &MemberModel) -> Option<String> {
        if !field.is_static || !field.name.ends_with("Event") {
            return None;
        }
        let ty = field.return_type.as_ref().filter(|ty| ty.is_resolved())?;
        let routed = self.models().canonical_path(ROUTED_EVENT);
        let actual = self.models().expanded(&ty.text);
        let arguments = actual.strip_prefix(&routed)?;
        if arguments.is_empty() {
            return Some("RoutedEventArgs".into());
        }
        arguments
            .strip_prefix('<')?
            .strip_suffix('>')
            .map(str::to_owned)
    }

    fn ancestry<'a>(&'a self, class: &'a TypeModel) -> Vec<&'a TypeModel> {
        let mut result = Vec::new();
        let mut seen = BTreeSet::new();
        let mut next = Some(class);
        while let Some(class) = next {
            if !seen.insert(&class.rust_path.text) {
                break;
            }
            result.push(class);
            next = class
                .base
                .as_ref()
                .and_then(|base| self.models().find_rust_type(&base.text))
                .map(|(_, class)| class);
        }
        result
    }
}

fn signature(owner: &TypeModel, member: &str, parameters: Vec<String>) -> MarkupEventSignature {
    MarkupEventSignature {
        owner: owner.name.clone(),
        member: member.into(),
        parameters,
    }
}

fn namespace_matches(assembly: &AssemblyModel, namespace: &str, xml: &str) -> bool {
    if let Some(value) = xml.strip_prefix("using:") {
        return value == namespace;
    }
    if let Some(value) = xml.strip_prefix("clr-namespace:") {
        return value
            .split_once(";assembly=")
            .map_or(value == namespace, |(declared, name)| {
                declared == namespace && name.eq_ignore_ascii_case(&assembly.name)
            });
    }
    assembly
        .xmlns_definitions
        .iter()
        .any(|mapping| mapping.xml_namespace == xml && mapping.namespace == namespace)
}

fn unique<T>(mut items: impl Iterator<Item = T>) -> Option<T> {
    let first = items.next()?;
    items.next().is_none().then_some(first)
}
