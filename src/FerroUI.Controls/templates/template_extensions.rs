use crate::primitives::TemplatedControl;
use crate::Control;
use ferroui_base::{FerroObject, Ref, Visual};

impl TemplatedControl {
    /// Gets the list of all control descendants that are part of the template of this
    /// control: the template descendants that are controls.
    #[deprecated(note = "Use get_template_descendants")]
    pub fn get_template_children(&self) -> Vec<Ref<Control>> {
        self.get_template_descendants().into_iter().filter_map(|child| child.cast::<Control>()).collect()
    }

    /// Gets a visual tree's template descendants: the visual descendants
    /// whose templated parent is this control.
    pub fn get_template_descendants(&self) -> Vec<Ref<Visual>> {
        let templated_parent: Ref<FerroObject> = self.to_ref().upcast();
        let mut result = Vec::new();
        collect_template_descendants(self, &templated_parent, &mut result);
        result
    }
}

fn collect_template_descendants(control: &Visual, templated_parent: &Ref<FerroObject>, result: &mut Vec<Ref<Visual>>) {
    let Some(children) = control.visual_children_snapshot() else { return };

    for child in children.iter() {
        let child_templated_parent = child.templated_parent();

        if child_templated_parent.as_ref() == Some(templated_parent) {
            result.push(child.clone());
        }

        if child_templated_parent.is_some() {
            collect_template_descendants(child, templated_parent, result);
        }
    }
}
