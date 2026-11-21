use super::{RustDocument, carry_anchor, invalid};
use ferroui_designer_core::{DesignerError, Document, SourceSpan, TextPatch};
use ferroui_designer_tools::ItemTemplateMember;

#[derive(Clone)]
pub(super) struct TemplateScope {
    consumer: usize,
    owner: usize,
    body: usize,
    property: ItemTemplateMember,
}

impl RustDocument {
    pub(crate) fn template_property(&self) -> Option<ItemTemplateMember> {
        self.template_scopes.last().map(|scope| scope.property)
    }

    pub(crate) fn enter_template(
        &mut self,
        document: &mut Document,
        body: SourceSpan,
        owner: usize,
        property: ItemTemplateMember,
    ) -> Result<(), DesignerError> {
        let invocation = self
            .map()
            .ok_or_else(|| invalid("Wait for the current Rust preview"))?
            .invocation;
        if body.start <= invocation.start || body.end >= invocation.end {
            return Err(invalid("The template is outside the selected Rust view"));
        }
        self.navigate_to_invocation(document, body.start)?;
        self.template_scopes.push(TemplateScope {
            consumer: invocation.start,
            owner,
            body: body.start,
            property,
        });
        Ok(())
    }

    pub(crate) fn leave_template(
        &mut self,
        document: &mut Document,
    ) -> Result<usize, DesignerError> {
        let scope = self
            .template_scopes
            .last()
            .ok_or_else(|| invalid("Open a Rust template first"))?;
        let (consumer, owner) = (scope.consumer, scope.owner);
        self.navigate_to_invocation(document, consumer)?;
        self.template_scopes.pop();
        Ok(owner)
    }

    pub(super) fn carry_template_scopes(&mut self, patch: &TextPatch) {
        let anchors_retained = self.template_scopes.iter_mut().all(|scope| {
            let (consumer, consumer_replaced) = carry_anchor(patch, scope.consumer);
            let (body, body_replaced) = carry_anchor(patch, scope.body);
            scope.consumer = consumer;
            scope.body = body;
            scope.owner = carry_anchor(patch, scope.owner).0;
            !consumer_replaced && !body_replaced
        });
        if !anchors_retained {
            self.template_scopes.clear();
            return;
        }
        if let (Some(scope), Some(map)) = (self.template_scopes.last(), self.map.as_ref()) {
            if map.invocation.start != scope.body
                || self
                    .template_scopes
                    .iter()
                    .any(|scope| !map.blocks.iter().any(|block| block.start == scope.consumer))
            {
                self.template_scopes.clear();
            }
        }
    }
}
