//! Plan edits inside one setter, including attribute values and inline objects.
use super::{children, Patches};
use crate::xaml::{literal_value, setter_object};
use crate::{native_fragment, AnimationError, AnimationValue, Result};
use ferroui_designer_core::{Document, Node, NodeKind, SourceSpan, TextPatch};

pub(super) fn plan(
    document: &Document,
    setter: &Node,
    old: &AnimationValue,
    next: &AnimationValue,
    output: &mut Patches,
) -> Result<()> {
    match (setter.attribute_info("Value"), setter_object(next)) {
        (Some(attribute), None) => {
            let mut value = literal_value(next);
            if attribute.value.starts_with("{}") && !value.starts_with("{}") {
                value.insert_str(0, "{}");
            }
            output.attribute(document, setter, "Value", Some(value))
        }
        (Some(_), Some(object)) => {
            output.attribute(document, setter, "Value", None)?;
            let object = native_fragment(document, setter.id, &object)?;
            let (span, replacement) = if setter.self_closing {
                let end = setter.opening_span.end;
                (
                    SourceSpan::new(end - 2, end),
                    format!(">{object}</{}>", setter.name),
                )
            } else {
                let end = setter.closing_span.ok_or_else(unmapped)?.start;
                (SourceSpan::new(end, end), object)
            };
            output.text.push(TextPatch { span, replacement });
            Ok(())
        }
        (None, None) => {
            let object = object_child(document, setter).ok_or_else(unmapped)?;
            output.attribute(document, setter, "Value", Some(literal_value(next)))?;
            output.text.push(TextPatch {
                span: object.span,
                replacement: comments(document, object),
            });
            Ok(())
        }
        (None, Some(next)) => plan_object(
            document,
            setter,
            &setter_object(old).ok_or_else(unmapped)?,
            &next,
            output,
        ),
    }
}

fn plan_object(
    document: &Document,
    setter: &Node,
    previous: &str,
    next: &str,
    output: &mut Patches,
) -> Result<()> {
    let previous = Document::new(previous).map_err(super::core_error)?;
    let next = Document::new(next).map_err(super::core_error)?;
    let old = previous
        .ast()
        .node(previous.ast().root())
        .expect("value root");
    let new = next.ast().node(next.ast().root()).expect("value root");
    let actual = object_child(document, setter).ok_or_else(unmapped)?;
    let mut attributes = Vec::new();
    if same_shape(
        document,
        actual,
        &previous,
        old,
        &next,
        new,
        &mut attributes,
    ) {
        for (node, name, value) in attributes {
            output.attribute(document, node, name, value)?;
        }
        return Ok(());
    }

    // Keep a common Setter.Value wrapper, including its prefix and surrounding
    // trivia, when the value changes shape or the number of gradient stops changes.
    let (actual, new) =
        if actual.local_name() == "Setter.Value" && new.local_name() == "Setter.Value" {
            (
                object_child(document, actual).ok_or_else(unmapped)?,
                object_child(&next, new).ok_or_else(unmapped)?,
            )
        } else {
            (actual, new)
        };
    let markup = &next.source()[new.span.range()];
    let parent = actual.parent.ok_or_else(unmapped)?;
    let mut replacement = comments(document, actual);
    replacement.push_str(&native_fragment(document, parent, markup)?);
    output.text.push(TextPatch {
        span: actual.span,
        replacement,
    });
    Ok(())
}

type FieldChange<'a, 'b> = (&'a Node, &'b str, Option<String>);

fn same_shape<'a, 'b>(
    document: &'a Document,
    actual: &'a Node,
    before: &'b Document,
    old: &'b Node,
    after: &'b Document,
    new: &'b Node,
    edits: &mut Vec<FieldChange<'a, 'b>>,
) -> bool {
    if old.local_name() != new.local_name() || actual.local_name() != old.local_name() {
        return false;
    }
    for attr in new.attributes.iter().filter(|attr| !attr.is_namespace()) {
        if old.attribute(&attr.name) != Some(&attr.value) {
            edits.push((actual, &attr.name, Some(attr.value.clone())));
        }
    }
    for attr in old.attributes.iter().filter(|attr| !attr.is_namespace()) {
        if new.attribute(&attr.name).is_none() {
            edits.push((actual, &attr.name, None));
        }
    }
    let actual = value_children(document, actual);
    let old = value_children(before, old);
    let new = value_children(after, new);
    actual.len() == old.len()
        && old.len() == new.len()
        && actual
            .into_iter()
            .zip(old)
            .zip(new)
            .all(|((actual, old), new)| {
                same_shape(document, actual, before, old, after, new, edits)
            })
}

fn object_child<'a>(document: &'a Document, node: &Node) -> Option<&'a Node> {
    let mut children = children(document, node.id);
    let value = children.next()?;
    children.next().is_none().then_some(value)
}

fn value_children<'a>(document: &'a Document, node: &Node) -> Vec<&'a Node> {
    let direct = children(document, node.id).collect::<Vec<_>>();
    if let [value] = direct.as_slice() {
        if value.local_name().ends_with(".GradientStops") {
            return children(document, value.id).collect();
        }
    }
    direct
}

fn comments(document: &Document, node: &Node) -> String {
    document
        .ast()
        .subtree(node.id)
        .filter(|child| {
            matches!(
                child.kind,
                NodeKind::Comment | NodeKind::ProcessingInstruction
            )
        })
        .map(|child| &document.source()[child.span.range()])
        .collect()
}

fn unmapped() -> AnimationError {
    AnimationError("The authored setter value changed; wait for the current preview".into())
}
