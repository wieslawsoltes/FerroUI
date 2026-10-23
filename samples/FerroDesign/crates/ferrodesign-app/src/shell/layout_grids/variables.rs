//! Native typed binding controls for one guide, sharing the variable transaction
//! engine with the local-variable palette. The document retains no UI state.
use super::*;
use ferrodesign_core::{LayoutGuideVariableField, VariableEdit};

struct UpdatingGuideVariables<'a>(&'a Cell<bool>, bool);
impl Drop for UpdatingGuideVariables<'_> { fn drop(&mut self) { self.0.set(self.1); } }

impl EditorShell {
    pub(super) fn guide_variables_dialog(self: &Rc<Self>, selection: Vec<NodeId>, index: usize, requested: LayoutGuideVariableField) {
        if !self.flush_inspector_drafts() { return; }
        let snapshot = {
            let editor = self.editor.borrow();
            if editor.selection != selection { drop(editor); self.toast("The selection changed. Reopen guide settings before binding."); return; }
            let document = &editor.document;
            let source = selection.iter().filter_map(|id| document.nodes.get(id))
                .find(|node| LayoutGrid::supports(node.kind) && !document.effective_locked(node.id) && node.layout_guide(index).is_some());
            source.map(|source| {
                let grid = source.layout_guide(index).unwrap();
                let fields: Vec<_> = LayoutGuideVariableField::ALL.into_iter().filter(|field| field.supports(grid)).collect();
                let field = if fields.contains(&requested) { requested } else { LayoutGuideVariableField::Size };
                let key = field.key(index);
                let targets: Vec<_> = selection.iter().copied().filter(|id| document.nodes.get(id).is_some_and(|node|
                    !document.effective_locked(*id) && core::validate_layout_guide_binding_target(node, &key).is_ok())).collect();
                let kinds = document.variable_kinds(&document.variable_mode).unwrap_or_default();
                let variables: Vec<_> = document.variables.values().filter(|variable| kinds.get(&variable.id) == Some(&field.kind()))
                    .map(|variable| (variable.id, format!("{} / {}", variable.collection, variable.name))).collect();
                let binding = source.variable_bindings.get(&key).copied();
                let mixed = targets.iter().any(|id| document.nodes[id].variable_bindings.get(&key).copied() != binding);
                (fields, field, targets, variables, binding, mixed, document.variable_mode.clone())
            })
        };
        let Some((fields, field, targets, variables, binding, mixed, mode)) = snapshot else {
            self.toast("This guide is no longer available on an unlocked selected frame."); return;
        };
        let _updating = UpdatingGuideVariables(&self.updating, self.updating.replace(true));
        self.modal("Guide variable", &format!("Guide {} · {} matching unlocked frame(s) · preview mode {mode}. Missing or incompatible rows are skipped.", index + 1, targets.len()));
        let host = self.control::<StackPanel>("ModalContent"); host.set_spacing(10.0);
        host.children().add(style_label("Bindings resolve aliases and all document modes. Counts must be whole numbers from 1 to 1,000,000; size is positive, gutter and margin/offset are nonnegative. Unbind to keep the current value or enter Auto."));
        host.children().add(style_label("A value binding detaches the shared grid style. Visibility stays local and can remain linked to the style. Editing a literal field removes only that field’s binding."));
        if mixed { host.children().add(style_label("Mixed bindings · Apply assigns the chosen binding to all matching targets.")); }
        host.children().add(style_label("Guide property"));
        let properties = ComboBox::new(); properties.set_name(Some("GuideVariableProperty".into()));
        for item in &fields { properties.items().add(Some(Rc::new(item.label().to_string()))); }
        properties.set_selected_index(fields.iter().position(|candidate| *candidate == field).unwrap() as i32);
        let weak = Rc::downgrade(self); let weak_choice = properties.downgrade(); let selected = selection.clone();
        properties.selection_changed(move |_, _| {
            let (Some(shell), Some(choice)) = (weak.upgrade(), weak_choice.upgrade()) else { return; };
            if shell.updating.get() || !choice.is_attached_to_visual_tree() { return; }
            let Some(field) = usize::try_from(choice.selected_index()).ok().and_then(|choice| fields.get(choice)).copied() else { return; };
            shell.guide_variables_dialog(selected.clone(), index, field);
        });
        host.children().add(properties);
        host.children().add(style_label(&format!("{} variable", field.kind().label())));
        let values = ComboBox::new(); values.set_name(Some("GuideVariableValue".into()));
        values.items().add(Some(Rc::new("Unbound · keep current value".to_string())));
        for (_, label) in &variables { values.items().add(Some(Rc::new(label.clone()))); }
        values.set_selected_index(binding.and_then(|id| variables.iter().position(|(candidate, _)| *candidate == id)).map_or(0, |index| index as i32 + 1));
        host.children().add(values.clone());
        if variables.is_empty() { host.children().add(style_label("No compatible variables exist. Create one in Local variables, then return to this guide.")); }
        let error = style_label(""); error.set_name(Some("GuideVariableError".into()));
        error.set_foreground(Some(icons::brush("#C8372B"))); error.set_is_visible(false); host.children().add(error.clone());
        let actions = self.control::<StackPanel>("ModalActions");
        let weak_values = values.downgrade(); let weak_error = error.downgrade(); let selected = selection.clone();
        actions.children().add(self.style_button("Apply binding", "GuideVariableApply", !targets.is_empty(), move |shell| {
            let (Some(values), Some(error)) = (weak_values.upgrade(), weak_error.upgrade()) else { return; };
            if !shell.flush_inspector_drafts() { return; }
            let variable = if values.selected_index() <= 0 { None } else {
                variables.get(values.selected_index() as usize - 1).map(|(id, _)| *id)
            };
            let result = {
                let mut editor = shell.editor.borrow_mut();
                if editor.selection != selected { Err("The selection changed. Reopen guide settings before binding.".to_string()) }
                else { editor.edit_variables(VariableEdit::Bind { nodes: targets.clone(), property: field.key(index), variable })
                    .map_err(|error| error.to_string()) }
            };
            match result {
                Ok(result) => { if result.changed { shell.changed(); } shell.guide_variables_dialog(selected.clone(), index, field); }
                Err(message) => { error.set_text(Some(&message)); error.set_is_visible(true); }
            }
        }));
        actions.children().add(self.style_button("Local variables", "GuideVariableLibrary", true, |shell| shell.visual_variables_dialog()));
        actions.children().add(self.style_button("Back to guides", "GuideVariableBack", true, |shell| shell.layout_grid_dialog()));
    }
}
