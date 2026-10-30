//! Stable development identities and authored-value lookup.
use crate::ReloadError;
pub const KEY_NAMESPACE: &str = "https://github.com/ferroui/hot-reload";
const XAML_NAMESPACE: &str = "http://schemas.microsoft.com/winfx/2006/xaml";
const PREFIX: &str = "__ferroui_reload_";
fn explicit_key<'a, 'input>(node: roxmltree::Node<'a, 'input>) -> Option<&'a str> {
    node.attribute((KEY_NAMESPACE, "Key")).map(|key| key.strip_prefix("{}").unwrap_or(key))
}
fn declared_name<'a, 'input>(node: roxmltree::Node<'a, 'input>) -> Option<&'a str> {
    node.attribute((XAML_NAMESPACE, "Name"))
        .or_else(|| node.attribute("Name"))
}
fn generated_name(key: &str) -> String {
    let mut name = PREFIX.to_owned();
    for byte in key.as_bytes() {
        use std::fmt::Write;
        write!(name, "{byte:02x}").unwrap();
    }
    name
}
fn runtime_name(node: roxmltree::Node<'_, '_>) -> Option<String> {
    explicit_key(node)
        .map(generated_name)
        .or_else(|| declared_name(node).map(str::to_owned))
}
pub(crate) fn control_name(element: &ferroui_base::StyledElement) -> Option<String> {
    ferroui_markup_xaml::reload_identity::key(element)
        .map(|key| generated_name(&key))
        .or_else(|| element.name())
}
/// Validate development identity syntax. Compiler/runtime metadata supplies keys
/// without injecting public Name assignments or changing parser coordinates.
pub(crate) fn runtime_source(source: &str) -> Result<String, ReloadError> {
    let document =
        roxmltree::Document::parse(source).map_err(|e| ReloadError::Failed(e.to_string()))?;
    for node in document.descendants().filter(|node| node.is_element()) {
        if declared_name(node).is_some_and(|name| name.starts_with(PREFIX)) {
            return Err(ReloadError::Failed(
                "Name uses the reserved hot-reload prefix".into(),
            ));
        }
        if let Some(key) = explicit_key(node) {
            if key.is_empty() {
                return Err(ReloadError::Failed(
                    "Development key must not be empty".into(),
                ));
            }
        }
    }
    Ok(source.to_owned())
}
pub(crate) fn identity(source: &str, name: &str) -> Option<String> {
    let document = roxmltree::Document::parse(source).ok()?;
    let keys: std::collections::BTreeSet<_> = document
        .descendants()
        .filter(|node| node.is_element())
        .filter(|node| runtime_name(*node).as_deref() == Some(name))
        .map(explicit_key)
        .collect();
    match keys.len() {
        0 => None,
        1 => match keys.first().unwrap() {
            Some(key) => Some(format!("key:{key}")),
            None => Some(format!("name:{name}")),
        },
        _ => None,
    }
}
pub(crate) fn authored(source: &str, name: &str, property: &str) -> Option<String> {
    let document = roxmltree::Document::parse(source).ok()?;
    let values: std::collections::BTreeSet<_> = document
        .descendants()
        .filter(|node| node.is_element())
        .filter(|node| runtime_name(*node).as_deref() == Some(name))
        .map(|node| {
            node.attribute(property).map(str::to_owned).or_else(|| {
                node.children()
                    .find(|child| {
                        child.is_element()
                            && child
                                .tag_name()
                                .name()
                                .rsplit_once('.')
                                .is_some_and(|(_, member)| member == property)
                    })
                    .map(|child| source[child.range()].to_owned())
            })
        })
        .collect();
    match values.len() {
        0 => None,
        1 => values.into_iter().next().flatten(),
        _ => Some("{ambiguous}".into()),
    }
}
