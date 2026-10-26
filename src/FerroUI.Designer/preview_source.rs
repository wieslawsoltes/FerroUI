//! A Window owns a native top-level visual parent and cannot be inserted into an artboard.
//! The design projection uses a Border frame while the authored Window syntax stays untouched.
//! Replacements preserve Unicode character columns and line breaks, including masked chrome
//! settings, so runtime source locations still identify the original AST elements.
use ferroui_designer_core::{Document, Node, SourceSpan};

pub(crate) struct PreviewSource {
    pub source: String,
    pub notices: Vec<String>,
}
pub(crate) fn project(document: &Document) -> Result<PreviewSource, String> {
    let source = document.valid_source();
    let root = document
        .ast()
        .node(document.ast().root())
        .ok_or("The document has no root")?;
    let window = native(root) && root.local_name() == "Window";
    for node in document.ast().elements() {
        if node.id != root.id
            && native(node)
            && matches!(
                node.local_name(),
                "Window" | "TopLevel" | "EmbeddableControlRoot"
            )
        {
            return Err("A nested top-level control cannot be embedded in an artboard. Open that Window as a separate document.".into());
        }
    }
    let mut patches = Vec::<(SourceSpan, String)>::new();
    let mut notices = vec![];
    for attribute in &root.attributes {
        if attribute.namespace_uri.as_deref()
            == Some("http://schemas.microsoft.com/winfx/2006/xaml")
            && attribute.name.rsplit(':').next() == Some("Class")
        {
            patches.push((
                attribute.span,
                blank(&source[attribute.span.start..attribute.span.end]),
            ));
            notices.push("Code-behind class is retained in source; the artboard instantiates its declared control type".into());
        } else if window && chrome(attribute.name.rsplit(':').next().unwrap_or(&attribute.name)) {
            patches.push((
                attribute.span,
                blank(&source[attribute.span.start..attribute.span.end]),
            ));
        }
    }
    if window {
        rename(root, source, "Border", &mut patches);
        notices.push(
            "Window content preview; native title bar and window chrome are retained in source"
                .into(),
        );
        if root.attribute("Content").is_some() {
            return Err("Use a Window.Content property element with a control to preview a Window that currently uses a Content attribute".into());
        }
        for node in document.ast().elements() {
            if node.parent == Some(root.id) {
                if let Some(property) = node.local_name().strip_prefix("Window.") {
                    if chrome(property) {
                        patches.push((node.span, blank(&source[node.span.start..node.span.end])));
                    } else {
                        let replacement = if property == "Content" {
                            "Border.Child".to_owned()
                        } else {
                            format!("Border.{property}")
                        };
                        rename(node, source, &replacement, &mut patches);
                    }
                }
            }
            if node.local_name() == "Style" {
                if let Some(selector) = node.attribute_info("Selector") {
                    let raw = &source[selector.value_span.start..selector.value_span.end];
                    let projected =
                        replace_window_selector(raw, &document.ast().namespaces(node.id));
                    if projected != raw {
                        patches.push((selector.value_span, projected));
                    }
                }
            }
        }
    }
    patches.sort_by_key(|(span, _)| (span.start, std::cmp::Reverse(span.end)));
    let mut outer = Vec::<(SourceSpan, String)>::new();
    for patch in patches {
        if !outer
            .last()
            .is_some_and(|(span, _)| patch.0.start >= span.start && patch.0.end <= span.end)
        {
            outer.push(patch);
        }
    }
    let mut patches = outer;
    patches.sort_by_key(|(span, _)| std::cmp::Reverse(span.start));
    let mut result = source.to_owned();
    let mut previous_start = source.len();
    for (span, replacement) in patches {
        // A removed chrome subtree may contain styles; its outer mask owns the whole span.
        if span.end > previous_start {
            continue;
        }
        result.replace_range(span.start..span.end, &replacement);
        previous_start = span.start;
    }
    Ok(PreviewSource {
        source: result,
        notices,
    })
}
fn native(node: &Node) -> bool {
    matches!(
        node.namespace_uri.as_deref(),
        Some(
            "https://github.com/ferroui"
                | "using:FerroUI.Controls"
                | "clr-namespace:FerroUI.Controls;assembly=FerroUI.Controls"
        )
    )
}
fn blank(value: &str) -> String {
    value
        .chars()
        .map(|c| {
            if matches!(c, '\r' | '\n' | '\t') {
                c
            } else {
                ' '
            }
        })
        .collect()
}
fn rename(node: &Node, source: &str, new_local: &str, patches: &mut Vec<(SourceSpan, String)>) {
    let prefix = node
        .name
        .rsplit_once(':')
        .map(|(prefix, _)| format!("{prefix}:"))
        .unwrap_or_default();
    let replacement = format!("{prefix}{new_local}");
    let padding = " ".repeat(
        node.name
            .chars()
            .count()
            .saturating_sub(replacement.chars().count()),
    );
    patches.push((node.name_span, format!("{replacement}{padding}")));
    if let Some(span) = node.closing_span {
        let end = span.start + 2 + node.name.len();
        if source.get(span.start..span.start + 2) == Some("</") {
            patches.push((
                SourceSpan::new(span.start + 2, end),
                format!("{replacement}{padding}"),
            ));
        }
    }
}
fn chrome(name: &str) -> bool {
    matches!(
        name,
        "Title"
            | "Icon"
            | "WindowState"
            | "WindowStartupLocation"
            | "CanResize"
            | "CanMinimize"
            | "CanMaximize"
            | "ShowInTaskbar"
            | "SystemDecorations"
            | "ExtendClientAreaToDecorationsHint"
            | "ExtendClientAreaChromeHints"
            | "ExtendClientAreaTitleBarHeightHint"
            | "SizeToContent"
            | "Topmost"
            | "ShowActivated"
            | "TransparencyLevelHint"
            | "TransparencyBackgroundFallback"
            | "WindowDecorations"
            | "ClosingBehavior"
            | "Opened"
            | "Closed"
            | "Closing"
            | "Position"
            | "Owner"
            | "Screens"
    )
}
pub(crate) fn replace_window_selector(
    source: &str,
    namespaces: &std::collections::BTreeMap<String, String>,
) -> String {
    let mut result = String::with_capacity(source.len());
    let mut cursor = 0usize;
    let mut brackets = 0usize;
    let mut quote = None;
    let mut escaped = false;
    while cursor < source.len() {
        let c = source[cursor..].chars().next().expect("character boundary");
        if let Some(active) = quote {
            result.push(c);
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == active {
                quote = None;
            }
            cursor += c.len_utf8();
            continue;
        }
        if matches!(c, '\'' | '"') {
            quote = Some(c);
        }
        if c == '[' {
            brackets += 1;
        }
        if c == ']' {
            brackets = brackets.saturating_sub(1);
        }
        if c.is_alphanumeric() || c == '_' {
            let start = cursor;
            while cursor < source.len() {
                let next = source[cursor..].chars().next().expect("character boundary");
                if !(next.is_alphanumeric() || next == '_') {
                    break;
                }
                cursor += next.len_utf8();
            }
            let token = &source[start..cursor];
            let previous = source[..start].chars().next_back();
            let type_position = previous.is_none_or(|c| {
                c.is_whitespace() || matches!(c, '>' | '+' | '~' | ',' | '(' | '|')
            });
            let native = if previous == Some('|') {
                let before = &source[..start - 1];
                let prefix_start = before
                    .char_indices()
                    .rev()
                    .find(|(_, c)| !c.is_alphanumeric() && *c != '_')
                    .map_or(0, |(index, c)| index + c.len_utf8());
                namespaces
                    .get(&before[prefix_start..])
                    .is_some_and(|uri| uri == "https://github.com/ferroui")
            } else {
                namespaces
                    .get("")
                    .is_some_and(|uri| uri == "https://github.com/ferroui")
            };
            result.push_str(
                if token == "Window" && brackets == 0 && quote.is_none() && type_position && native
                {
                    "Border"
                } else {
                    token
                },
            );
        } else {
            result.push(c);
            cursor += c.len_utf8();
        }
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn window_selector_projection_changes_only_native_type_tokens() {
        let namespaces = std::collections::BTreeMap::from([
            (String::new(), "https://github.com/ferroui".into()),
            ("ui".into(), "https://github.com/ferroui".into()),
            ("custom".into(), "using:Application.Controls".into()),
        ]);
        assert_eq!(
            replace_window_selector(
                "Window#Window.Window:Window[Tag=Window], ui|Window > Window:not(.Window)",
                &namespaces
            ),
            "Border#Window.Window:Window[Tag=Window], ui|Border > Border:not(.Window)"
        );
        assert_eq!(
            replace_window_selector(
                ":is(Window, ui|Window, custom|Window)[Tag='Window']",
                &namespaces
            ),
            ":is(Border, ui|Border, custom|Window)[Tag='Window']"
        );
    }

    #[test]
    fn window_projection_preserves_source_columns_and_the_authored_document() {
        let source = "<Window xmlns=\"https://github.com/ferroui\" xmlns:x=\"http://schemas.microsoft.com/winfx/2006/xaml\" x:Class=\"App.View\" Title=\"Café\"><Window.Content><Button Name=\"Target\" /></Window.Content></Window>";
        let original = Document::new(source).unwrap();
        let projected = project(&original).unwrap();
        let parsed = Document::new(projected.source.clone()).unwrap();
        let before = source[..original
            .ast()
            .node(original.ast().find_name("Target").unwrap())
            .unwrap()
            .span
            .start]
            .chars()
            .count();
        let after = projected.source[..parsed
            .ast()
            .node(parsed.ast().find_name("Target").unwrap())
            .unwrap()
            .span
            .start]
            .chars()
            .count();
        assert_eq!(before, after);
        assert_eq!(original.source(), source);
        assert_eq!(
            parsed.ast().node(parsed.ast().root()).unwrap().local_name(),
            "Border"
        );
    }
}

/// Reuse or declare a root alias so newly generated native markup stays compact.
/// Authored nested and shadowed namespace declarations remain unchanged and are supported.
pub(crate) fn embed_native_fragment(
    document: &Document,
    source: &str,
) -> Result<(String, Option<ferroui_designer_core::Edit>), String> {
    const NS: &str = "https://github.com/ferroui";
    let namespaces = document.ast().namespaces(document.ast().root());
    let mut declaration = None;
    let prefix = if namespaces.get("").is_some_and(|uri| uri == NS) {
        String::new()
    } else if let Some((prefix, _)) = namespaces.iter().find(|(_, uri)| uri.as_str() == NS) {
        format!("{prefix}:")
    } else {
        let mut prefix = "ui".to_owned();
        let mut suffix = 1;
        while namespaces.contains_key(&prefix) {
            prefix = format!("ui{suffix}");
            suffix += 1;
        }
        declaration = Some(ferroui_designer_core::Edit::SetAttribute {
            node: document.ast().root(),
            name: format!("xmlns:{prefix}"),
            value: NS.into(),
        });
        format!("{prefix}:")
    };
    let fragment = Document::new(source).map_err(|e| e.to_string())?;
    let mut patches = vec![];
    for node in fragment.ast().elements() {
        if node.namespace_uri.as_deref().is_none_or(|uri| uri == NS) {
            let name = format!("{prefix}{}", node.local_name());
            if name != node.name {
                patches.push((node.name_span, name.clone()));
                if let Some(closing) = node.closing_span {
                    patches.push((
                        SourceSpan::new(closing.start + 2, closing.start + 2 + node.name.len()),
                        name,
                    ));
                }
            }
        }
        for attribute in node
            .attributes
            .iter()
            .filter(|attribute| attribute.is_namespace() && attribute.value == NS)
        {
            patches.push((attribute.span, String::new()));
        }
    }
    patches.sort_by_key(|(span, _)| std::cmp::Reverse(span.start));
    let mut result = source.to_owned();
    for (span, text) in patches {
        result.replace_range(span.start..span.end, &text);
    }
    Ok((result, declaration))
}
