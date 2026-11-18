use crate::{native_fragment, AnimationError, AnimationPrograms, Result};
use ferroui_designer_core::{Document, Edit, NodeId};

/// Compose a codec's standalone replacement into the original namespace
/// scope. The resulting edits form one document transaction.
pub fn compose_animation_edit(
    document: &Document,
    id: NodeId,
    generated: &str,
) -> Result<Vec<Edit>> {
    let mut source = native_fragment(document, id, generated)?;
    if document
        .source()
        .split_once('\n')
        .is_some_and(|(first_line, _)| first_line.ends_with('\r'))
    {
        source = source.replace("\r\n", "\n").replace('\n', "\r\n");
    }
    let mut edits = AnimationPrograms::replace_source(document, id, &source)?;
    let mut staged = document.clone();
    staged
        .apply("Stage animation namespace composition", edits.clone())
        .map_err(|error| AnimationError(error.to_string()))?;
    let original_start = document
        .ast()
        .node(id)
        .ok_or_else(|| AnimationError("Program source disappeared".into()))?
        .span
        .start;
    let rewritten = staged
        .ast()
        .elements()
        .find(|node| node.span.start == original_start && node.local_name() == "Animation")
        .ok_or_else(|| AnimationError("Rewritten animation disappeared".into()))?;
    let parent_scope = rewritten
        .parent
        .map(|parent| staged.ast().namespaces(parent))
        .unwrap_or_default();
    let root = staged.ast().root();
    let mut root_scope = staged.ast().namespaces(root);
    for attribute in rewritten
        .attributes
        .iter()
        .filter(|attribute| attribute.is_namespace())
    {
        let prefix = attribute.name.strip_prefix("xmlns:").unwrap_or("");
        if parent_scope.get(prefix) == Some(&attribute.value) {
            edits.push(Edit::RemoveAttribute {
                node: rewritten.id,
                name: attribute.name.clone(),
            });
        } else if !prefix.is_empty() && !root_scope.contains_key(prefix) {
            edits.push(Edit::SetAttribute {
                node: root,
                name: attribute.name.clone(),
                value: attribute.value.clone(),
            });
            edits.push(Edit::RemoveAttribute {
                node: rewritten.id,
                name: attribute.name.clone(),
            });
            root_scope.insert(prefix.to_owned(), attribute.value.clone());
        }
    }
    Ok(edits)
}
