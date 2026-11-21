use super::{RustDocument, carry_anchor, invalid};
use ferroui_designer_core::{DesignerError, Document, SourceSpan, TextPatch};
use ferroui_designer_tools::ItemTemplateMember;

#[derive(Clone)]
pub(super) struct TemplateScope {
    return_to: TemplateReturn,
    body: usize,
    property: ItemTemplateMember,
}

#[derive(Clone)]
enum TemplateReturn {
    Local {
        invocation: usize,
        owner: usize,
    },
    Companion {
        uri: String,
        previous: Option<usize>,
    },
}

impl TemplateReturn {
    fn invocation(&self) -> Option<usize> {
        match self {
            Self::Local { invocation, .. } => Some(*invocation),
            Self::Companion { previous, .. } => *previous,
        }
    }

    fn carry(&mut self, patch: &TextPatch) -> bool {
        let position = match self {
            Self::Local { invocation, owner } => {
                *owner = carry_anchor(patch, *owner).0;
                invocation
            }
            Self::Companion {
                previous: Some(previous),
                ..
            } => previous,
            Self::Companion { previous: None, .. } => return true,
        };
        let (next, replaced) = carry_anchor(patch, *position);
        *position = next;
        !replaced
    }
}

impl RustDocument {
    pub(crate) fn template_property(&self) -> Option<ItemTemplateMember> {
        self.template_scopes.last().map(|scope| scope.property)
    }

    pub(crate) fn template_consumer(&self) -> Option<&str> {
        match &self.template_scopes.last()?.return_to {
            TemplateReturn::Companion { uri, .. } => Some(uri),
            TemplateReturn::Local { .. } => None,
        }
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
        if body == invocation {
            return Err(invalid("The template is already open"));
        }
        self.enter_template_scope(
            document,
            body,
            property,
            TemplateReturn::Local {
                invocation: invocation.start,
                owner,
            },
        )
    }

    pub(crate) fn enter_companion_template(
        &mut self,
        document: &mut Document,
        body: SourceSpan,
        consumer: String,
        property: ItemTemplateMember,
    ) -> Result<(), DesignerError> {
        let previous = self.map().map(|map| map.invocation.start);
        self.enter_template_scope(
            document,
            body,
            property,
            TemplateReturn::Companion {
                uri: consumer,
                previous,
            },
        )
    }

    fn enter_template_scope(
        &mut self,
        document: &mut Document,
        body: SourceSpan,
        property: ItemTemplateMember,
        return_to: TemplateReturn,
    ) -> Result<(), DesignerError> {
        if self.template_scopes.iter().any(|scope| {
            scope.body == body.start || scope.return_to.invocation() == Some(body.start)
        }) {
            return Err(invalid(
                "The template is already in the current navigation path",
            ));
        }
        if let Some(map) = self.map() {
            if !map.blocks.contains(&body) {
                return Err(invalid("The authored Rust template changed"));
            }
            self.navigate_to_invocation(document, body.start)?;
        } else {
            self.project_into(
                document,
                self.source.clone(),
                body.start,
                "Open Rust template",
                true,
                None,
            )?;
        }
        self.template_scopes.push(TemplateScope {
            return_to,
            body: body.start,
            property,
        });
        Ok(())
    }

    pub(crate) fn leave_template(
        &mut self,
        document: &mut Document,
    ) -> Result<Option<usize>, DesignerError> {
        let scope = self
            .template_scopes
            .last()
            .ok_or_else(|| invalid("Open a Rust template first"))?;
        let owner = match &scope.return_to {
            TemplateReturn::Local { owner, .. } => Some(*owner),
            TemplateReturn::Companion { .. } => None,
        };
        if let Some(invocation) = scope.return_to.invocation() {
            self.navigate_to_invocation(document, invocation)?;
        }
        self.template_scopes.pop();
        Ok(owner)
    }

    pub(super) fn carry_template_scopes(&mut self, patch: &TextPatch) {
        let anchors_retained = self.template_scopes.iter_mut().all(|scope| {
            let (body, body_replaced) = carry_anchor(patch, scope.body);
            scope.body = body;
            !body_replaced && scope.return_to.carry(patch)
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
                    .filter_map(|scope| scope.return_to.invocation())
                    .any(|start| !map.blocks.iter().any(|block| block.start == start))
            {
                self.template_scopes.clear();
            }
        }
    }
}
