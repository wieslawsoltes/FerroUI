use crate::DesignerSession;
use ferroui_base::TypeInfo;
use ferroui_designer_core::NodeId;
use ferroui_designer_rsx::{RsxAnimationEdit, RsxAnimationPlan};
use ferroui_markup::markup::parsers::SelectorParser;

/// Check the same selector grammar and type lookup as the declarative view builder.
pub fn validate_rsx_animation_selector(selector: &str) -> Result<(), String> {
    let parser =
        SelectorParser::new(|_, name| TypeInfo::find_by_name(name)).unknown_types_by_name();
    parser
        .parse(selector)
        .map_err(|error| error.to_string())?
        .map(|_| ())
        .ok_or_else(|| "Enter a native style selector".into())
}

impl DesignerSession {
    pub fn can_create_animation_program(&self, owner: NodeId) -> bool {
        if !self.can_manipulate() {
            return false;
        }
        let Some(rust) = &self.rust else { return true };
        self.document
            .ast()
            .node(owner)
            .zip(rust.map())
            .is_some_and(|(node, map)| map.can_create_animation(node.span.start))
    }

    pub(crate) fn create_rust_animation(
        &mut self,
        name: &str,
        owner: NodeId,
        selector: &str,
        duration: f64,
    ) -> Result<NodeId, String> {
        validate_rsx_animation_selector(selector)?;
        let rust = self.rust.as_ref().ok_or("Open the current Rust view")?;
        let plan = rust.create_animation(&self.document, owner, name, selector, duration)?;
        self.apply_rust_animation_plan("Create Rust animation", plan)?
            .ok_or_else(|| "The authored animation has no selection identity".into())
    }

    pub(crate) fn edit_rust_animation(
        &mut self,
        label: &str,
        program: NodeId,
        edit: RsxAnimationEdit<'_>,
    ) -> Result<Option<NodeId>, String> {
        let rust = self.rust.as_ref().ok_or("Open the current Rust view")?;
        let plan = rust.edit_animation(&self.document, program, edit)?;
        self.apply_rust_animation_plan(label, plan)
    }

    fn apply_rust_animation_plan(
        &mut self,
        label: &str,
        plan: RsxAnimationPlan,
    ) -> Result<Option<NodeId>, String> {
        if !self.can_manipulate() {
            return Err("Open an editable Rust view".into());
        }
        if !plan.patches.is_empty() {
            self.commit_rust_patches(label, plan.patches)
                .map_err(|error| error.to_string())?;
            self.changed(true);
        }
        if let Some(key) = plan.selected_key {
            let program = self
                .animation_programs()
                .iter()
                .find(|program| program.source_key.as_deref() == Some(key.as_str()))
                .ok_or("The authored Rust animation was not found in the preview")?
                .id;
            if self.active_animation_program() != Some(program) {
                self.select_animation_program(program)?;
            }
            return Ok(Some(program));
        }
        Ok(None)
    }
}
