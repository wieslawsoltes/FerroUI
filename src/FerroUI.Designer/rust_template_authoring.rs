use crate::{DesignerSession, NodeId};
use ferroui_designer_core::SourceSpan;
use ferroui_designer_rsx::{RsxItemTemplate, RsxProjection, RsxSourceEdit, RsxTemplateBody};
use ferroui_designer_tools::{ItemTemplateChange, ItemTemplateMember, nearest_table};

impl DesignerSession {
    pub(crate) fn has_rust_template_owner(&self, node: NodeId) -> bool {
        self.rust_map().is_some()
            && self
                .document()
                .ast()
                .node(node)
                .is_some_and(|node| ferroui_designer_rsx::is_item_template_owner(node.local_name()))
    }

    pub(crate) fn rust_item_templates(&self, node: NodeId) -> Vec<RsxItemTemplate> {
        self.rust
            .as_ref()
            .and_then(|rust| rust.projection().ok())
            .map_or_else(Vec::new, |projection| {
                self.rust_template_slots(&projection, node)
            })
    }

    pub(crate) fn rust_template_slots(
        &self,
        projection: &RsxProjection<'_>,
        node: NodeId,
    ) -> Vec<RsxItemTemplate> {
        let mut slots = projection.item_templates(self.document(), node);
        if !self.can_manipulate() || self.ensure_layer_manipulable(node).is_err() {
            for slot in &mut slots {
                slot.body = None;
                slot.can_create = false;
                slot.can_reset = false;
                slot.reason = Some("Open a current preview and unlock the owning layer.");
            }
        }
        slots
    }

    pub(crate) fn rust_template_body(
        &self,
        projection: &RsxProjection<'_>,
        owner: NodeId,
        property: ItemTemplateMember,
    ) -> Result<RsxTemplateBody, String> {
        self.rust_template_slots(projection, owner)
            .into_iter()
            .find(|slot| slot.property == property)
            .and_then(|slot| slot.body)
            .ok_or_else(|| "Open a mapped Rust template in the current preview".into())
    }

    pub(crate) fn rust_template_plan(
        &self,
        projection: &RsxProjection<'_>,
        owner: NodeId,
        property: ItemTemplateMember,
        change: &ItemTemplateChange,
        row_type: Option<&str>,
    ) -> Result<RsxSourceEdit, String> {
        self.ensure_writable().map_err(|error| error.to_string())?;
        if !self.can_manipulate() {
            return Err("Wait for the current editable Rust preview".into());
        }
        self.ensure_layer_manipulable(owner)?;
        projection.plan_item_template(self.document(), owner, property, change, row_type)
    }

    pub(crate) fn edit_rust_template(
        &mut self,
        owner: NodeId,
        property: ItemTemplateMember,
        change: ItemTemplateChange,
        row_type: Option<&str>,
    ) -> Result<(), String> {
        let rust = self.rust.as_ref().ok_or("Open a Rust view")?;
        let edit =
            self.rust_template_plan(&rust.projection()?, owner, property, &change, row_type)?;
        self.apply_rust_template_plan(change.label(), edit)
    }

    pub(crate) fn apply_rust_template_plan(
        &mut self,
        label: &str,
        edit: RsxSourceEdit,
    ) -> Result<(), String> {
        if !self
            .commit_rust_declaration(label, edit)
            .map_err(|error| error.to_string())?
            .is_empty()
        {
            self.changed(true);
        }
        Ok(())
    }

    pub(crate) fn preview_rust_template(
        &mut self,
        owner: NodeId,
        property: ItemTemplateMember,
    ) -> Result<(), String> {
        let rust = self.rust.as_ref().ok_or("Open a Rust view")?;
        let body = self.rust_template_body(&rust.projection()?, owner, property)?;
        if body.location.uri.is_some() {
            return Err("Open this template in its companion Rust module".into());
        }
        self.enter_rust_template(owner, body.location.span, property)
    }

    pub(crate) fn enter_rust_template(
        &mut self,
        owner: NodeId,
        body: SourceSpan,
        property: ItemTemplateMember,
    ) -> Result<(), String> {
        let invocation = self.rust_map().ok_or("Open a mapped Rust view")?.invocation;
        let owner = self
            .authored_span(owner)
            .or_else(|| {
                nearest_table(self.document(), owner).and_then(|table| self.authored_span(table))
            })
            .map_or(invocation.start, |span| span.start);
        self.rust
            .as_mut()
            .expect("Rust view is open")
            .enter_template(&mut self.document, body, owner, property)
            .map_err(|error| error.to_string())?;
        self.finish_rust_template_preview();
        Ok(())
    }

    pub(crate) fn enter_companion_rust_template(
        &mut self,
        body: SourceSpan,
        consumer: String,
        property: ItemTemplateMember,
    ) -> Result<(), String> {
        self.rust
            .as_mut()
            .ok_or("Open the Rust template module")?
            .enter_companion_template(&mut self.document, body, consumer, property)
            .map_err(|error| error.to_string())?;
        self.finish_rust_template_preview();
        Ok(())
    }

    fn finish_rust_template_preview(&mut self) {
        self.reset_rust_view_state();
        self.changed(true);
        self.select([self.document.ast().root()]);
    }

    pub(crate) fn rust_template_property(&self) -> Option<ItemTemplateMember> {
        self.rust.as_ref()?.template_property()
    }

    pub(crate) fn exit_rust_template(&mut self) -> Result<(), String> {
        let owner = self
            .rust
            .as_mut()
            .ok_or("Open a Rust view")?
            .leave_template(&mut self.document)
            .map_err(|error| error.to_string())?;
        self.reset_rust_view_state();
        self.changed(true);
        let node = self
            .document
            .ast()
            .elements()
            .find(|node| {
                self.authored_span(node.id)
                    .is_some_and(|span| Some(span.start) == owner)
            })
            .map_or(self.document.ast().root(), |node| node.id);
        self.select([node]);
        Ok(())
    }
}
