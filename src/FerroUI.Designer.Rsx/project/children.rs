use super::State;
use crate::{
    animation::syntax::span,
    child_collection::{ChildEnvelope, CollectionSource, InlineChildren},
    registered::RsxCatalog,
    tree::Element,
};
use ferroui_designer_core::SourceSpan;
use ferroui_view_syntax::{Attribute, Node};

impl State<'_> {
    pub(super) fn set_child_collection(
        &mut self,
        index: usize,
        list: &InlineChildren,
        attribute: &Attribute,
    ) -> bool {
        let assignment = match attribute {
            Attribute::Value { name, value, .. } => {
                Some(SourceSpan::new(span(name).start, span(value).end))
            }
            Attribute::Call { name, .. } => self.delimiters.calls.get(&span(name).start).copied(),
        };
        let Some(assignment) = assignment else {
            return false;
        };
        let owner = &mut self.nodes[index];
        owner.child_collection = Some(CollectionSource {
            assignment,
            close: list.close,
            unterminated: list.unterminated,
            len: list.members.len(),
        });
        // A nested RSX expression establishes Element as the item type of the
        // homogeneous list. The app still evaluates its other expressions.
        owner.computed_child_setter |= list.members.iter().all(|entry| entry.control.is_none());
        true
    }

    pub(super) fn project_child_collection(
        &mut self,
        source: &str,
        index: usize,
        result: &mut Element,
        list: InlineChildren,
        depth: usize,
        catalog: &RsxCatalog,
    ) -> Result<(), String> {
        for member in list.members {
            match member.control.as_ref() {
                Some(child @ Node::Element(element)) if catalog.rust(&element.path).is_some() => {
                    let child_index = self.nodes.len();
                    result
                        .children
                        .push(self.element(source, child, depth + 1, catalog)?);
                    self.nodes[child_index].envelope =
                        Some(ChildEnvelope::Collection(member.source));
                    self.nodes[index].child_indices.push(child_index);
                }
                _ => {
                    self.nodes[index].opaque_children.push(member.source.start);
                    self.omit(format!("{}.Children expression", result.class));
                }
            }
        }
        Ok(())
    }
}
