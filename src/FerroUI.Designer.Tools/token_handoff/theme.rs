use super::ValueResult;
use crate::components::{escape, framework, key_attribute, XAML};
use ferroui_designer_core::{Document, MarkupValue, Node, NodeKind};
use std::{collections::BTreeMap, fmt::Write};

/// Resolve the target's namespace, including a literal x:Type with local aliases.
pub(super) fn target(document: &Document, theme: &Node) -> Option<(String, String)> {
    let namespaces = document.ast().namespaces(theme.id);
    let target = match MarkupValue::parse(theme.attribute("TargetType")?.trim()).ok()? {
        MarkupValue::Literal(target) => target,
        MarkupValue::Extension(extension) => {
            let (prefix, "Type") = extension.name.split_once(':')? else {
                return None;
            };
            let [argument] = extension.arguments.as_slice() else {
                return None;
            };
            if namespaces.get(prefix).map(String::as_str) != Some(XAML) || argument.name.is_some() {
                return None;
            }
            let MarkupValue::Literal(target) = &argument.value else {
                return None;
            };
            target.clone()
        }
    };
    let (prefix, name) = target.split_once(':').unwrap_or(("", &target));
    Some((namespaces.get(prefix)?.clone(), name.into()))
}

pub(super) fn literal_setters<'a>(
    document: &'a Document,
    theme: &'a Node,
    properties: &[&str],
) -> ValueResult<BTreeMap<&'a str, &'a str>> {
    if theme.attributes.iter().any(|attribute| {
        !attribute.is_namespace()
            && Some(attribute) != key_attribute(theme)
            && attribute.name != "TargetType"
    }) {
        return Err("Export a keyed ControlTheme with only a literal TargetType".into());
    }
    let mut setters = BTreeMap::new();
    for child in theme
        .children
        .iter()
        .filter_map(|id| document.ast().node(*id))
        .filter(|child| !trivia(child))
    {
        if child.local_name() != "Setter"
            || !framework(child)
            || child.attributes.iter().any(|attribute| {
                !attribute.is_namespace()
                    && !matches!(attribute.name.as_str(), "Property" | "Value")
            })
            || child
                .children
                .iter()
                .filter_map(|id| document.ast().node(*id))
                .any(|child| !trivia(child))
        {
            return Err("Export direct literal Setter elements".into());
        }
        let property = child
            .attribute("Property")
            .ok_or("A Setter needs a Property")?;
        let value = child
            .attribute("Value")
            .ok_or("A Setter needs a literal Value")?
            .trim();
        if !properties.contains(&property) || setters.insert(property, value).is_some() {
            return Err(format!("Unexpected or repeated theme setter {property}"));
        }
    }
    Ok(setters)
}

pub(super) fn trivia(node: &Node) -> bool {
    node.kind == NodeKind::Comment
        || matches!(node.kind, NodeKind::Text | NodeKind::CData)
            && node
                .text
                .as_deref()
                .is_some_and(|text| text.trim().is_empty())
}

pub(super) fn write<'a>(
    key: &str,
    target: &str,
    setters: impl IntoIterator<Item = (&'a str, String)>,
) -> String {
    let key = escape(key);
    let target = escape(target);
    let mut theme = format!("    <ControlTheme x:Key=\"{key}\" TargetType=\"{target}\">\n");
    for (property, value) in setters {
        let value = escape(&value);
        let _ = writeln!(
            theme,
            "        <Setter Property=\"{property}\" Value=\"{value}\"/>"
        );
    }
    theme.push_str("    </ControlTheme>\n");
    theme
}
