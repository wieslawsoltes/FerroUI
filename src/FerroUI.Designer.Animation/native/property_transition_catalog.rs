//! Turn registered native properties into qualified XAML transition targets.
use super::native_property_transition_types;
use crate::XamlTransitionTarget;
use ferroui_base::{FerroProperty, FerroPropertyRegistry, TypeInfo};
use ferroui_designer_core::{Document, NodeId};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

const XAML: &str = "http://schemas.microsoft.com/winfx/2006/xaml";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransitionPropertyChoice {
    pub property: String,
    pub label: String,
    pub type_name: &'static str,
}

pub struct NativeTransitionChoice {
    pub description: TransitionPropertyChoice,
    property: &'static FerroProperty,
    qualified: bool,
}

/// Registered properties and their compatible built-in transition classes.
pub fn transition_choices(type_: &'static TypeInfo) -> Vec<NativeTransitionChoice> {
    let regular = type_.properties();
    let attached = FerroPropertyRegistry::instance().get_registered_attached(type_);
    let mut seen = BTreeSet::new();
    let mut choices = Vec::new();
    for &property in regular.iter().chain(attached.iter()) {
        let qualified =
            type_.find_property(property.name()).map(FerroProperty::id) != Some(property.id());
        if qualified && !property.is_attached() {
            continue;
        }
        if !seen.insert(property.id()) {
            continue;
        }
        let owner = property.owner_type();
        if qualified && owner.namespace().is_empty() {
            continue;
        }
        let (key, label) = if qualified {
            (
                format!("{}.{}", owner.full_name(), property.name()),
                format!("{}.{}", owner.name(), property.name()),
            )
        } else {
            (property.name().to_owned(), property.name().to_owned())
        };
        for type_name in native_property_transition_types(property) {
            choices.push(NativeTransitionChoice {
                description: TransitionPropertyChoice {
                    property: key.clone(),
                    label: label.clone(),
                    type_name,
                },
                property,
                qualified,
            });
        }
    }
    choices.sort_by(|left, right| left.description.label.cmp(&right.description.label));
    choices
}

/// Resolve an exact choice or prefer the class already authored on an entry.
pub fn choose_transition(
    type_: &'static TypeInfo,
    property: &str,
    requested: Option<&str>,
    preferred: Option<&str>,
) -> Option<NativeTransitionChoice> {
    let mut matches = transition_choices(type_).into_iter().filter(|choice| {
        choice.description.property == property
            && requested.is_none_or(|name| choice.description.type_name == name)
    });
    let first = matches.next()?;
    if let Some(name) = preferred.filter(|name| *name != first.description.type_name) {
        if let Some(choice) = matches.find(|choice| choice.description.type_name == name) {
            return Some(choice);
        }
    }
    Some(first)
}

impl NativeTransitionChoice {
    /// Qualify the property using the host's registered XML namespace resolver.
    pub fn xaml_target(
        &self,
        document: &Document,
        at: NodeId,
        static_property: bool,
        resolve_type: impl Fn(&str, &str) -> Option<&'static TypeInfo>,
    ) -> XamlTransitionTarget {
        let mut scope = document.ast().namespaces(at);
        let mut namespaces = Vec::new();
        let property = if self.qualified || static_property {
            let owner = qualify_owner(
                self.property.owner_type(),
                &mut scope,
                &mut namespaces,
                resolve_type,
            );
            let name = self.property.name();
            if static_property {
                let prefix = scope
                    .iter()
                    .find_map(|(prefix, uri)| {
                        (uri == XAML && !prefix.is_empty()).then(|| prefix.clone())
                    })
                    .unwrap_or_else(|| declare("x", XAML.into(), &mut scope, &mut namespaces));
                format!("{{{prefix}:Static {owner}.{name}Property}}")
            } else {
                format!("{owner}.{name}")
            }
        } else {
            self.property.name().to_owned()
        };
        XamlTransitionTarget {
            property,
            namespaces,
            type_name: self.description.type_name,
        }
    }
}

fn qualify_owner(
    owner: &'static TypeInfo,
    scope: &mut BTreeMap<String, String>,
    namespaces: &mut Vec<(String, String)>,
    resolve_type: impl Fn(&str, &str) -> Option<&'static TypeInfo>,
) -> String {
    let name = owner.name();
    let prefix = scope
        .iter()
        .find_map(|(prefix, uri)| {
            resolve_type(uri, name)
                .is_some_and(|found| std::ptr::eq(found, owner))
                .then(|| prefix.clone())
        })
        .unwrap_or_else(|| {
            declare(
                "p",
                format!("using:{}", owner.namespace()),
                scope,
                namespaces,
            )
        });
    if prefix.is_empty() {
        name.to_owned()
    } else {
        format!("{prefix}:{name}")
    }
}

fn declare(
    base: &str,
    uri: String,
    scope: &mut BTreeMap<String, String>,
    namespaces: &mut Vec<(String, String)>,
) -> String {
    let prefix = if !scope.contains_key(base) {
        base.to_owned()
    } else {
        (1..)
            .map(|index| format!("{base}{index}"))
            .find(|prefix| !scope.contains_key(prefix))
            .unwrap()
    };
    scope.insert(prefix.clone(), uri.clone());
    namespaces.push((prefix.clone(), uri));
    prefix
}
