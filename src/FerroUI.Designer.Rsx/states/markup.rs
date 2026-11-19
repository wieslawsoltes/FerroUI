use crate::registered::Class;
use crate::tree::{escape_xml, Element};
use ferroui_designer_animation::VisualStateGroup;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn anchor(owner: &Element, index: usize) -> String {
    let mut name = format!("ferroui_projected_state_owner_{index}");
    let mut nodes = Vec::new();
    collect(owner, &mut nodes);
    while nodes.iter().any(|node| {
        node.properties
            .get("Classes")
            .is_some_and(|value| value.split_ascii_whitespace().any(|class| class == name))
    }) {
        name.push('_');
    }
    name
}

pub(super) fn group_markup(
    group: &VisualStateGroup,
    owner: &Element,
    anchor: &str,
) -> Option<String> {
    let mut nodes = Vec::new();
    for child in &owner.children {
        collect(child, &mut nodes);
    }
    let mut namespaces = BTreeMap::new();
    let owner_selector = format!("{}.{anchor}", selector(&owner.class, &mut namespaces));
    let mut targets = BTreeMap::new();
    let names = group
        .states
        .iter()
        .flat_map(|state| &state.setters)
        .map(|setter| setter.target.as_str())
        .filter(|name| !name.is_empty())
        .collect::<BTreeSet<_>>();
    for name in names {
        let mut matches = nodes.iter().filter(|node| {
            node.properties
                .get("Name")
                .is_some_and(|value| value == name)
        });
        let target = matches.next()?;
        if matches.next().is_some() {
            return None;
        }
        let tag = selector(&target.class, &mut namespaces);
        targets.insert(name, format!("{tag}#{name}"));
    }
    let mut source = group
        .to_xaml_for_selectors(&owner_selector, |name| targets[name].clone())
        .ok()?;
    for (uri, prefix) in namespaces {
        source.insert_str(
            "<Style".len(),
            &format!(" xmlns:{prefix}=\"{}\"", escape_xml(uri, Some('"'))),
        );
    }
    Some(source)
}

fn selector<'a>(class: &'a Class, namespaces: &mut BTreeMap<&'a str, String>) -> String {
    let Some(uri) = class.namespace() else {
        return class.name().to_owned();
    };
    let count = namespaces.len();
    let prefix = namespaces
        .entry(uri)
        .or_insert_with(|| format!("state{count}"));
    format!("{prefix}|{}", class.name())
}

fn collect<'a>(element: &'a Element, nodes: &mut Vec<&'a Element>) {
    nodes.push(element);
    for child in &element.children {
        collect(child, nodes);
    }
}
