//! Native component sets, variant axes, and typed exposed-property controls.
use super::*;
use ferrodesign_core::{ComponentPropertyDefinition, ComponentPropertyValue};

struct UpdatingComponents<'a>(&'a Cell<bool>, bool);
impl Drop for UpdatingComponents<'_> {
    fn drop(&mut self) { self.0.set(self.1); }
}

// Inspector menus need identity and property metadata, never copies of image
// bytes, vector geometry, effects, or unrelated text payloads from source layers.
struct ComponentOwner {
    id: NodeId,
    kind: NodeKind,
    name: String,
    component_id: Option<NodeId>,
    variant_properties: BTreeMap<String, String>,
    component_property_definitions: BTreeMap<String, ComponentPropertyDefinition>,
}
impl From<&Node> for ComponentOwner {
    fn from(node: &Node) -> Self {
        Self {
            id: node.id, kind: node.kind, name: node.name.clone(), component_id: node.component_id,
            variant_properties: node.variant_properties.clone(),
            component_property_definitions: node.component_property_definitions.clone(),
        }
    }
}
struct ComponentTarget { id: NodeId, kind: NodeKind, name: String }
impl From<&Node> for ComponentTarget {
    fn from(node: &Node) -> Self { Self { id: node.id, kind: node.kind, name: node.name.clone() } }
}
struct VariantChoice { id: NodeId, label: String }

impl EditorShell {
    pub(super) fn components_dialog(self: &Rc<Self>) {
        self.components_view(None, 0);
    }

    fn components_view(self: &Rc<Self>, requested: Option<NodeId>, new_kind: usize) {
        let (selected, owner, variants, masters, targets, views, set) = {
            let editor = self.editor.borrow();
            let document = &editor.document;
            let selected = editor.selection.clone();
            let owner = requested.or_else(|| selected.first().and_then(|id| document.component_owner(*id)))
                .and_then(|id| document.nodes.get(&id)).map(ComponentOwner::from);
            let set = owner.as_ref().and_then(|node| document.component_set_of(node.id));
            let variants: Vec<_> = set.map(|set| document.component_variants(set)).unwrap_or_default()
                .iter().filter_map(|id| document.nodes.get(id)).map(|node| VariantChoice {
                    id: node.id, label: core::variant_label(&node.variant_properties),
                }).collect();
            let masters: Vec<_> = document.nodes.values().filter(|node| node.kind == NodeKind::Component)
                .map(|node| (node.id, format!("{} · {}", node.name, core::variant_label(&node.variant_properties)))).collect();
            let targets: Vec<_> = owner.as_ref().filter(|node| node.kind == NodeKind::Component)
                .map(|node| document.descendants(node.id)).unwrap_or_default()
                .iter().filter_map(|id| document.nodes.get(id)).map(ComponentTarget::from).collect();
            let views = owner.as_ref().map(|node| document.component_property_views(node.id)).unwrap_or_default();
            (selected, owner, variants, masters, targets, views, set)
        };
        let _updating = UpdatingComponents(&self.updating, self.updating.replace(true));
        self.modal("Components and variants", "Main components define reusable designs. Instances keep their exposed values and matching layer overrides when variants change.");
        let host = self.control::<StackPanel>("ModalContent");
        host.set_spacing(12.0);
        let actions = self.control::<StackPanel>("ModalActions");
        actions.children().add(self.component_button("Done", "ComponentDone", true, |shell| shell.close_modal()));

        let can_combine = {
            let editor = self.editor.borrow();
            !selected.is_empty() && selected.iter().all(|id| editor.document.nodes.get(id)
                .is_some_and(|node| node.kind == NodeKind::Component) && editor.document.component_set_of(*id).is_none())
        };
        if can_combine {
            let row = component_grid("*,170");
            let name = component_input("ComponentSetName", "Component set");
            let weak_name = name.downgrade();
            row.children().add(name);
            let combine = self.component_button("Combine as variants", "ComponentCombine", true, move |shell| {
                let Some(name) = weak_name.upgrade() else { return };
                let name = name.text().unwrap_or_default();
                let ids = selected.clone();
                let result = shell.editor.borrow_mut().create_component_set(&ids, &name);
                match result {
                    Ok(id) => { shell.changed(); shell.components_view(Some(id), 0); }
                    Err(error) => shell.toast(&error.to_string()),
                }
            });
            Grid::set_column(&combine, 1);
            row.children().add(combine);
            host.children().add(row);
        }
        let Some(owner) = owner else {
            host.children().add(component_text("Select a main component, an instance, or a layer inside one to edit reusable properties.", true));
            actions.children().add(self.component_button("Create component", "ComponentCreate", true, |shell| {
                shell.command("component");
                shell.components_dialog();
            }));
            return;
        };
        let owner_id = owner.id;
        host.children().add(component_text(&owner.name, false));
        if !variants.is_empty() {
            let labels: Vec<_> = variants.iter().map(|node| node.label.clone()).collect();
            let current = if owner.kind == NodeKind::Instance { owner.component_id } else { Some(owner.id) };
            let index = variants.iter().position(|variant| Some(variant.id) == current).unwrap_or(0);
            let ids: Vec<_> = variants.iter().map(|node| node.id).collect();
            let instance = owner.kind == NodeKind::Instance;
            host.children().add(self.component_choice("Variant", "ComponentVariant", &labels, index, move |shell, index| {
                let Some(id) = ids.get(index).copied() else { return };
                if instance {
                    shell.apply_component_change(owner_id, true, |editor| editor.swap_instance_component(owner_id, id));
                } else {
                    shell.components_view(Some(id), 0);
                }
            }));
            let set = set.unwrap();
            let template = current.filter(|id| variants.iter().any(|variant| variant.id == *id)).unwrap_or(variants[0].id);
            actions.children().add(self.component_button("Add variant", "ComponentAddVariant", true, move |shell| {
                let result = shell.editor.borrow_mut().add_component_variant(set, template);
                match result {
                    Ok(id) => { shell.changed(); shell.components_view(Some(id), 0); }
                    Err(error) => shell.toast(&error.to_string()),
                }
            }));
        }
        match owner.kind {
            NodeKind::Instance => {
                let labels: Vec<_> = masters.iter().map(|(_, label)| label.clone()).collect();
                let selected = masters.iter().position(|(id, _)| Some(*id) == owner.component_id).unwrap_or(0);
                let ids: Vec<_> = masters.iter().map(|(id, _)| *id).collect();
                host.children().add(self.component_choice("Main component", "ComponentMaster", &labels, selected, move |shell, index| {
                    if let Some(component) = ids.get(index).copied() {
                        shell.apply_component_change(owner_id, true, |editor| editor.swap_instance_component(owner_id, component));
                    }
                }));
                if views.is_empty() { host.children().add(component_text("This main component has no exposed properties. Define them on its main component.", true)); }
                for (index, view) in views.into_iter().enumerate() {
                    let card = component_panel();
                    let key = view.key.clone();
                    let row = component_grid("*,100");
                    row.children().add(component_text(&view.definition.name, false));
                    let reset = self.component_button("Reset", &format!("ComponentReset{index}"), true, move |shell| {
                        shell.apply_component_change(owner_id, true, |editor| editor.set_instance_property(owner_id, &key, None));
                    });
                    Grid::set_column(&reset, 1);
                    row.children().add(reset);
                    card.children().add(row);
                    let key = view.key;
                    self.component_value_editor(&card, &format!("ComponentValue{index}"), &view.value, &masters,
                        move |shell, value| shell.try_component_change(owner_id, false,
                            |editor| editor.set_instance_property(owner_id, &key, Some(value))));
                    host.children().add(card);
                }
                if let Some(master) = owner.component_id {
                    actions.children().add(self.component_button("Edit main component", "ComponentEditMaster", true, move |shell| shell.components_view(Some(master), 0)));
                }
                actions.children().add(self.component_button("Detach instance", "ComponentDetach", true, move |shell| {
                    let changed = shell.editor.borrow_mut().detach_instance(owner_id);
                    if changed { shell.changed(); shell.components_dialog(); }
                }));
            }
            NodeKind::Component => {
                self.component_variant_axes(&host, &owner);
                for (index, (key, definition)) in owner.component_property_definitions.iter().enumerate() {
                    self.component_definition_card(&host, owner_id, index, key, definition, &targets, &masters);
                }
                self.component_new_property(&host, owner_id, new_kind, &targets);
                actions.children().add(self.component_button("Update instances", "ComponentSync", true, move |shell| {
                    shell.apply_component_change(owner_id, false, |editor| editor.refresh_component_instances(owner_id).map(|_| ()));
                }));
                actions.children().add(self.component_button("Create instance", "ComponentInstance", true, move |shell| {
                    let result = {
                        let mut editor = shell.editor.borrow_mut();
                        let bounds = editor.document.world_bounds(owner_id).unwrap_or_default();
                        editor.create_instance(owner_id, bounds.right() + 40.0, bounds.y)
                    };
                    if let Some(id) = result { shell.changed(); shell.components_view(Some(id), 0); }
                }));
            }
            _ => host.children().add(component_text("Choose a variant above to define its properties or add a new variant.", true)),
        }
    }

    fn component_variant_axes(self: &Rc<Self>, host: &Ref<StackPanel>, component: &ComponentOwner) {
        if self.editor.borrow().document.component_set_of(component.id).is_none() { return; }
        let id = component.id;
        host.children().add(component_text("Variant properties", false));
        for (index, (axis, value)) in component.variant_properties.iter().enumerate() {
            let row = component_grid("*,80");
            let key = axis.clone();
            row.children().add(self.component_field(axis, &format!("ComponentAxis{index}"), value, move |shell, value| {
                let mut properties = shell.editor.borrow().document.nodes.get(&id)
                    .map(|node| node.variant_properties.clone())
                    .ok_or_else(|| "This component no longer exists.".to_string())?;
                properties.insert(key.clone(), value.trim().into());
                shell.try_component_change(id, false, |editor| editor.set_variant_properties(id, properties))
            }));
            let key = axis.clone();
            let remove = self.component_button("Remove", &format!("ComponentRemoveAxis{index}"), component.variant_properties.len() > 1, move |shell| {
                let properties = shell.editor.borrow().document.nodes.get(&id).map(|node| node.variant_properties.clone());
                let Some(mut properties) = properties else { shell.toast("This component no longer exists."); return };
                properties.remove(&key);
                shell.apply_component_change(id, true, |editor| editor.set_variant_properties(id, properties));
            });
            Grid::set_column(&remove, 1);
            row.children().add(remove);
            host.children().add(row);
        }
        let row = component_grid("*,*,85");
        let axis = component_input("ComponentNewAxisName", "Size");
        let value = component_input("ComponentNewAxisValue", "Medium");
        let weak_axis = axis.downgrade();
        let weak_value = value.downgrade();
        Grid::set_column(&value, 1);
        row.children().add(axis);
        row.children().add(value);
        let add = self.component_button("Add axis", "ComponentAddAxis", true, move |shell| {
            let (Some(axis), Some(value)) = (weak_axis.upgrade(), weak_value.upgrade()) else { return };
            let properties = shell.editor.borrow().document.nodes.get(&id).map(|node| node.variant_properties.clone());
            let Some(mut properties) = properties else { shell.toast("This component no longer exists."); return };
            properties.insert(axis.text().unwrap_or_default().trim().into(), value.text().unwrap_or_default().trim().into());
            shell.apply_component_change(id, true, |editor| editor.set_variant_properties(id, properties));
        });
        Grid::set_column(&add, 2);
        row.children().add(add);
        host.children().add(row);
    }

    fn component_definition_card(self: &Rc<Self>, host: &Ref<StackPanel>, owner: NodeId, index: usize, key: &str,
        definition: &ComponentPropertyDefinition, targets: &[ComponentTarget], masters: &[(NodeId, String)]) {
        let card = component_panel();
        let property = key.to_string();
        card.children().add(self.component_field("Property name", &format!("ComponentPropertyName{index}"), &definition.name,
            move |shell, value| {
                let mut definition = shell.editor.borrow().document.nodes.get(&owner)
                    .and_then(|node| node.component_property_definitions.get(&property)).cloned()
                    .ok_or_else(|| "This component property no longer exists.".to_string())?;
                definition.name = value.trim().into();
                shell.try_component_change(owner, false, |editor| editor.set_component_property_definition(owner, &property, definition))
            }));
        card.children().add(component_text(definition.default_value.kind_name(), true));
        let property = key.to_string();
        self.component_value_editor(&card, &format!("ComponentDefault{index}"), &definition.default_value, masters, move |shell, value| {
            let definition = shell.editor.borrow().document.nodes.get(&owner)
                .and_then(|node| node.component_property_definitions.get(&property)).cloned();
            if let Some(mut definition) = definition {
                definition.default_value = value;
                shell.try_component_change(owner, false, |editor| editor.set_component_property_definition(owner, &property, definition))
            } else {
                Err("This component property no longer exists.".into())
            }
        });
        for (position, target) in definition.targets.iter().enumerate() {
            let row = component_grid("*,80");
            let name = targets.iter().find(|node| node.id == *target).map(|node| node.name.as_str()).unwrap_or("Missing layer");
            row.children().add(component_text(&format!("Bound to {name}"), true));
            let target = *target;
            let property = key.to_string();
            let remove = self.component_button("Unbind", &format!("ComponentUnbind{index}_{position}"), definition.targets.len() > 1, move |shell| {
                let definition = shell.editor.borrow().document.nodes.get(&owner)
                    .and_then(|node| node.component_property_definitions.get(&property)).cloned();
                if let Some(mut definition) = definition {
                    definition.targets.retain(|id| *id != target);
                    shell.apply_component_change(owner, true, |editor| editor.set_component_property_definition(owner, &property, definition));
                }
            });
            Grid::set_column(&remove, 1);
            row.children().add(remove);
            card.children().add(row);
        }
        let candidates: Vec<_> = targets.iter().filter(|node| component_target_matches(node, &definition.default_value)
            && !definition.targets.contains(&node.id)).map(|node| (node.id, node.name.clone())).collect();
        if !candidates.is_empty() {
            let row = component_grid("*,100");
            let choice = component_combo(&format!("ComponentBindTarget{index}"), &candidates.iter().map(|(_, name)| name.clone()).collect::<Vec<_>>(), 0);
            let weak_choice = choice.downgrade();
            row.children().add(choice);
            let property = key.to_string();
            let add = self.component_button("Bind layer", &format!("ComponentBind{index}"), true, move |shell| {
                let Some(choice) = weak_choice.upgrade() else { return };
                let Some((target, _)) = candidates.get(choice.selected_index().max(0) as usize) else { return };
                let definition = shell.editor.borrow().document.nodes.get(&owner)
                    .and_then(|node| node.component_property_definitions.get(&property)).cloned();
                if let Some(mut definition) = definition {
                    definition.targets.push(*target);
                    shell.apply_component_change(owner, true, |editor| editor.set_component_property_definition(owner, &property, definition));
                }
            });
            Grid::set_column(&add, 1);
            row.children().add(add);
            card.children().add(row);
        }
        let property = key.to_string();
        card.children().add(self.component_button("Remove property", &format!("ComponentRemoveProperty{index}"), true, move |shell| {
            shell.apply_component_change(owner, true, |editor| editor.remove_component_property(owner, &property).map(|_| ()));
        }));
        host.children().add(card);
    }

    fn component_new_property(self: &Rc<Self>, host: &Ref<StackPanel>, owner: NodeId, kind: usize, targets: &[ComponentTarget]) {
        let panel = component_panel();
        panel.children().add(component_text("Expose a property", false));
        panel.children().add(self.component_choice("Type", "ComponentNewPropertyType",
            &["Text".into(), "Boolean visibility".into(), "Instance swap".into()], kind,
            move |shell, kind| shell.components_view(Some(owner), kind)));
        let eligible: Vec<_> = targets.iter().filter(|node| match kind {
            0 => node.kind == NodeKind::Text, 1 => true, _ => node.kind == NodeKind::Instance,
        }).map(|node| (node.id, node.name.clone())).collect();
        let row = component_grid("*,*");
        let name = component_input("ComponentNewPropertyName", "New property");
        let target = component_combo("ComponentNewPropertyTarget", &eligible.iter().map(|(_, name)| name.clone()).collect::<Vec<_>>(), 0);
        let weak_name = name.downgrade();
        let weak_target = target.downgrade();
        Grid::set_column(&target, 1);
        row.children().add(name);
        row.children().add(target);
        panel.children().add(row);
        if eligible.is_empty() { panel.children().add(component_text("Add a compatible layer to this component before exposing this property type.", true)); }
        panel.children().add(self.component_button("Create exposed property", "ComponentAddProperty", !eligible.is_empty(), move |shell| {
            let (Some(name), Some(target)) = (weak_name.upgrade(), weak_target.upgrade()) else { return };
            let Some((target, _)) = eligible.get(target.selected_index().max(0) as usize) else { return };
            let prepared = {
                let editor = shell.editor.borrow();
                let Some(node) = editor.document.nodes.get(target) else { return };
                let value = match kind {
                    0 => ComponentPropertyValue::Text(node.text.clone()),
                    1 => ComponentPropertyValue::Boolean(node.visible),
                    _ => { let Some(master) = node.component_id else { return }; ComponentPropertyValue::InstanceSwap(master) }
                };
                let name = name.text().unwrap_or_default().trim().to_string();
                if !editor.document.nodes.contains_key(&owner) { return; }
                let family = editor.document.component_set_of(owner).map(|set| editor.document.component_variants(set)).unwrap_or_else(|| vec![owner]);
                let definitions: Vec<_> = family.iter().filter_map(|id| editor.document.nodes.get(id))
                    .flat_map(|node| node.component_property_definitions.iter()).collect();
                let key = definitions.iter().find(|(_, definition)| definition.name == name && definition.default_value.same_kind(&value))
                    .map(|(key, _)| (*key).clone()).unwrap_or_else(|| {
                        let mut number = 1;
                        while definitions.iter().any(|(key, _)| *key == &format!("property{number}")) { number += 1; }
                        format!("property{number}")
                    });
                (key, ComponentPropertyDefinition {
                    name, default_value: value, targets: vec![*target],
                })
            };
            shell.apply_component_change(owner, true, |editor| editor.set_component_property_definition(owner, &prepared.0, prepared.1));
        }));
        host.children().add(panel);
    }

    fn component_value_editor(self: &Rc<Self>, host: &Ref<StackPanel>, name: &str, value: &ComponentPropertyValue,
        masters: &[(NodeId, String)], apply: impl Fn(&Rc<Self>, ComponentPropertyValue) -> Result<(), String> + 'static) {
        match value {
            ComponentPropertyValue::Text(text) => host.children().add(self.component_field("Text", name, text, move |shell, value| {
                apply(shell, ComponentPropertyValue::Text(value.into()))
            })),
            ComponentPropertyValue::Boolean(visible) => {
                let check = CheckBox::new();
                check.set_name(Some(name.into()));
                check.set_content(Some(Rc::new("Visible".to_string())));
                check.set_is_checked(Some(*visible));
                let weak_shell = Rc::downgrade(self);
                let weak_check = check.downgrade();
                check.is_checked_changed(move |_, _| {
                    if let (Some(shell), Some(check)) = (weak_shell.upgrade(), weak_check.upgrade()) {
                        if !shell.updating.get() {
                            if let Err(error) = apply(&shell, ComponentPropertyValue::Boolean(check.is_checked() == Some(true))) { shell.toast(&error); }
                        }
                    }
                });
                host.children().add(check);
            }
            ComponentPropertyValue::InstanceSwap(component) => {
                let selected = masters.iter().position(|(id, _)| id == component).unwrap_or(0);
                let ids: Vec<_> = masters.iter().map(|(id, _)| *id).collect();
                let labels: Vec<_> = masters.iter().map(|(_, label)| label.clone()).collect();
                host.children().add(self.component_choice("Instance component", name, &labels, selected, move |shell, index| {
                    if let Some(id) = ids.get(index) {
                        if let Err(error) = apply(shell, ComponentPropertyValue::InstanceSwap(*id)) { shell.toast(&error); }
                    }
                }));
            }
        }
    }

    fn apply_component_change(self: &Rc<Self>, owner: NodeId, rebuild: bool,
        apply: impl FnOnce(&mut Editor) -> Result<(), core::DocumentError>) {
        if let Err(error) = self.try_component_change(owner, rebuild, apply) { self.toast(&error); }
    }

    fn try_component_change(self: &Rc<Self>, owner: NodeId, rebuild: bool,
        apply: impl FnOnce(&mut Editor) -> Result<(), core::DocumentError>) -> Result<(), String> {
        if self.updating.get() { return Ok(()); }
        // Release the editor before refresh can raise native focus/layout events.
        let result = { let mut editor = self.editor.borrow_mut(); apply(&mut editor) };
        result.map_err(|error| error.to_string())?;
        self.changed();
        if rebuild { self.components_view(Some(owner), 0); }
        Ok(())
    }

    fn component_button(self: &Rc<Self>, caption: &str, name: &str, enabled: bool,
        apply: impl Fn(&Rc<Self>) + 'static) -> Ref<Button> {
        let button = Button::new();
        button.set_name(Some(name.into()));
        button.set_content(Some(Rc::new(caption.to_string())));
        button.set_is_enabled(enabled);
        button.set_height(30.0);
        let weak_shell = Rc::downgrade(self);
        button.click(move |_, _| { if let Some(shell) = weak_shell.upgrade() { if !shell.updating.get() { apply(&shell); } } });
        button
    }

    fn component_choice(self: &Rc<Self>, label: &str, name: &str, labels: &[String], selected: usize,
        apply: impl Fn(&Rc<Self>, usize) + 'static) -> Ref<StackPanel> {
        let panel = component_panel();
        panel.children().add(component_text(label, true));
        let choice = component_combo(name, labels, selected);
        let weak_shell = Rc::downgrade(self);
        let weak_choice = choice.downgrade();
        let count = labels.len();
        choice.selection_changed(move |_, _| {
            if let (Some(shell), Some(choice)) = (weak_shell.upgrade(), weak_choice.upgrade()) {
                let selected = choice.selected_index();
                if !shell.updating.get() && selected >= 0 && (selected as usize) < count { apply(&shell, selected as usize); }
            }
        });
        panel.children().add(choice);
        panel
    }

    fn component_field(self: &Rc<Self>, label: &str, name: &str, value: &str,
        apply: impl Fn(&Rc<Self>, &str) -> Result<(), String> + 'static) -> Ref<StackPanel> {
        let panel = component_panel();
        panel.children().add(component_text(label, true));
        let field = component_input(name, value);
        let error = component_text("", true);
        error.set_foreground(Some(icons::brush("#C8372B")));
        error.set_is_visible(false);
        let committed = Rc::new(RefCell::new(value.to_string()));
        let committed_value = committed.clone();
        let weak_shell = Rc::downgrade(self);
        let weak_field = field.downgrade();
        let weak_error = error.downgrade();
        let commit = Rc::new(move || {
            let (Some(shell), Some(field), Some(error)) = (weak_shell.upgrade(), weak_field.upgrade(), weak_error.upgrade()) else { return };
            if shell.updating.get() || field.is_composing_text() { return; }
            let value = field.text().unwrap_or_default();
            if *committed_value.borrow() == value { return; }
            match apply(&shell, &value) {
                Ok(()) => { *committed_value.borrow_mut() = value; error.set_is_visible(false); }
                Err(message) => { error.set_text(Some(&message)); error.set_is_visible(true); }
            }
        });
        let blur = commit.clone();
        field.add_handler(InputElement::lost_focus_event(), move |_, _| blur());
        let draft_commit = commit.clone();
        field.add_handler(InputElement::key_down_event(), move |_, event| {
            if event.key == Key::Enter { commit(); event.set_handled(true); }
        });
        self.bind_inspector_draft(&field, committed, draft_commit, Some(&error));
        panel.children().add(field);
        panel.children().add(error);
        panel
    }
}

fn component_panel() -> Ref<StackPanel> {
    let panel = StackPanel::new(); panel.set_spacing(6.0); panel
}
fn component_text(value: &str, muted: bool) -> Ref<TextBlock> {
    let text = TextBlock::new();
    text.set_text(Some(value));
    text.set_font_size(if muted { 11.0 } else { 12.0 });
    text.set_foreground(Some(icons::brush(if muted { "#737373" } else { "#242424" })));
    text.set_text_wrapping(TextWrapping::Wrap);
    text
}
fn component_grid(columns: &str) -> Ref<Grid> {
    let grid = Grid::new();
    grid.set_column_definitions(ColumnDefinitions::parse(columns).expect("component columns"));
    grid.set_column_spacing(8.0); grid
}
fn component_input(name: &str, value: &str) -> Ref<TextBox> {
    let input = TextBox::new(); input.set_name(Some(name.into())); input.set_text(Some(value)); input
}
fn component_combo(name: &str, labels: &[String], selected: usize) -> Ref<ComboBox> {
    let choice = ComboBox::new(); choice.set_name(Some(name.into()));
    for label in labels { choice.items().add(Some(Rc::new(label.clone()))); }
    choice.set_selected_index(if labels.is_empty() { -1 } else { selected.min(labels.len() - 1) as i32 });
    choice.set_is_enabled(!labels.is_empty()); choice
}
fn component_target_matches(node: &ComponentTarget, value: &ComponentPropertyValue) -> bool {
    match value {
        ComponentPropertyValue::Text(_) => node.kind == NodeKind::Text,
        ComponentPropertyValue::Boolean(_) => true,
        ComponentPropertyValue::InstanceSwap(_) => node.kind == NodeKind::Instance,
    }
}
