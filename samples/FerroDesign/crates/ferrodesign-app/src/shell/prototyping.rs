//! Native interaction authoring. A form commits one validated rule at a time;
//! rebuilding a focused modal keeps all old callbacks inert until it is ready.

use super::*;
use ferroui_base::CornerRadius;

const TRIGGERS: &[(&str, core::Trigger)] = &[
    ("On click", core::Trigger::Click),
    ("On hover", core::Trigger::Hover),
    ("On press", core::Trigger::Press),
    ("On drag", core::Trigger::Drag),
    ("After delay", core::Trigger::AfterDelay),
    ("Key shortcut", core::Trigger::Key),
    ("While hovering", core::Trigger::WhileHover),
    ("While pressing", core::Trigger::WhilePress),
    ("Mouse leaves", core::Trigger::MouseLeave),
    ("On release", core::Trigger::Release),
];
const ACTIONS: &[&str] = &[
    "Navigate to",
    "Open overlay",
    "Close overlay",
    "Back",
    "Open URL",
    "Set variable",
    "Scroll to",
];
const TRANSITIONS: &[(&str, core::TransitionKind)] = &[
    ("Instant", core::TransitionKind::Instant),
    ("Dissolve", core::TransitionKind::Dissolve),
    ("Smart animate", core::TransitionKind::SmartAnimate),
    ("Slide left", core::TransitionKind::SlideLeft),
    ("Slide right", core::TransitionKind::SlideRight),
    ("Slide up", core::TransitionKind::SlideUp),
    ("Slide down", core::TransitionKind::SlideDown),
    ("Push", core::TransitionKind::Push),
    ("Move in", core::TransitionKind::MoveIn),
    ("Move out", core::TransitionKind::MoveOut),
];
const EASINGS: &[(&str, core::Easing)] = &[
    ("Linear", core::Easing::Linear),
    ("Ease in", core::Easing::EaseIn),
    ("Ease out", core::Easing::EaseOut),
    ("Ease in and out", core::Easing::EaseInOut),
    ("Spring", core::Easing::Spring),
];

struct UpdatingInteractions<'a>(&'a Cell<bool>, bool);
impl Drop for UpdatingInteractions<'_> {
    fn drop(&mut self) {
        self.0.set(self.1);
    }
}

struct InteractionForm {
    source: NodeId,
    editing: Option<usize>,
    expected: Option<core::Interaction>,
    trigger: Ref<ComboBox>,
    action: Ref<ComboBox>,
    target: Ref<ComboBox>,
    targets: Vec<NodeId>,
    delay: Ref<TextBox>,
    key: Ref<TextBox>,
    url: Ref<TextBox>,
    overlay_x: Ref<TextBox>,
    overlay_y: Ref<TextBox>,
    variable: Ref<ComboBox>,
    variables: Vec<core::VariableId>,
    value_kind: Ref<ComboBox>,
    value: Ref<TextBox>,
    alias: Ref<ComboBox>,
    transition: Ref<ComboBox>,
    duration: Ref<TextBox>,
    easing: Ref<ComboBox>,
    target_row: Ref<StackPanel>,
    delay_row: Ref<StackPanel>,
    key_row: Ref<StackPanel>,
    url_row: Ref<StackPanel>,
    overlay_row: Ref<Grid>,
    variable_row: Ref<Grid>,
    value_row: Ref<StackPanel>,
    alias_row: Ref<StackPanel>,
    animation_note: Ref<TextBlock>,
    scroll_note: Ref<TextBlock>,
    error: Ref<TextBlock>,
}

impl InteractionForm {
    fn update_visibility(&self) {
        let action = self.action.selected_index();
        let trigger = self.trigger.selected_index();
        self.target_row.set_is_visible(matches!(action, 0 | 1 | 6));
        self.overlay_row.set_is_visible(action == 1);
        self.url_row.set_is_visible(action == 4);
        self.variable_row.set_is_visible(action == 5);
        self.value_row
            .set_is_visible(action == 5 && self.value_kind.selected_index() != 4);
        self.alias_row
            .set_is_visible(action == 5 && self.value_kind.selected_index() == 4);
        self.delay_row.set_is_visible(trigger == 4);
        self.key_row.set_is_visible(trigger == 5);
        self.animation_note
            .set_is_visible(self.transition.selected_index() > 0);
        self.scroll_note.set_is_visible(action == 6);
        self.error.set_is_visible(false);
    }

    fn read(&self, document: &core::Document) -> Result<core::Interaction, String> {
        let trigger = TRIGGERS
            .get(self.trigger.selected_index() as usize)
            .map(|entry| entry.1)
            .ok_or("Choose a trigger.")?;
        let target = || {
            self.targets
                .get(self.target.selected_index() as usize)
                .copied()
                .ok_or_else(|| "Choose a destination layer.".to_string())
        };
        let action = match self.action.selected_index() {
            0 => core::PrototypeAction::NavigateTo { target: target()? },
            1 => core::PrototypeAction::OpenOverlay {
                target: target()?,
                position: Point::new(
                    interaction_number(&self.overlay_x, "Overlay X")?,
                    interaction_number(&self.overlay_y, "Overlay Y")?,
                ),
            },
            2 => core::PrototypeAction::CloseOverlay,
            3 => core::PrototypeAction::Back,
            4 => core::PrototypeAction::OpenUrl {
                url: self.url.text().unwrap_or_default().trim().to_string(),
            },
            5 => {
                let variable = self
                    .variables
                    .get(self.variable.selected_index() as usize)
                    .copied()
                    .ok_or("Create or choose a variable before adding this action.")?;
                let text = self.value.text().unwrap_or_default();
                let value = match self.value_kind.selected_index() {
                    0 => core::VariableValue::Color(
                        Color::from_hex(&text)
                            .ok_or("Enter a hexadecimal color, such as #7B61FF or #7B61FF80.")?,
                    ),
                    1 => core::VariableValue::Number(interaction_number(
                        &self.value,
                        "Variable value",
                    )?),
                    2 => core::VariableValue::String(text),
                    3 => core::VariableValue::Boolean(match text.trim().to_lowercase().as_str() {
                        "true" | "1" => true,
                        "false" | "0" => false,
                        _ => return Err("Enter true or false for a Boolean value.".into()),
                    }),
                    4 => core::VariableValue::Alias(
                        self.variables
                            .get(self.alias.selected_index() as usize)
                            .copied()
                            .ok_or("Choose an existing variable to reference.")?,
                    ),
                    _ => return Err("Choose a value type.".into()),
                };
                core::PrototypeAction::SetVariable { variable, value }
            }
            6 => core::PrototypeAction::ScrollTo { target: target()? },
            _ => return Err("Choose an action.".into()),
        };
        let rule = core::Interaction {
            trigger,
            action,
            delay_ms: if trigger == core::Trigger::AfterDelay {
                interaction_milliseconds(&self.delay, "Delay")?
            } else {
                0
            },
            key: if trigger == core::Trigger::Key {
                Some(
                    core::normalize_prototype_key(&self.key.text().unwrap_or_default())
                        .ok_or("Enter a key or chord, such as Enter, Escape, or Ctrl+Shift+A.")?,
                )
            } else {
                None
            },
            transition: core::Transition {
                kind: TRANSITIONS
                    .get(self.transition.selected_index() as usize)
                    .map(|entry| entry.1)
                    .ok_or("Choose a transition.")?,
                duration_ms: interaction_milliseconds(&self.duration, "Transition duration")?,
                easing: EASINGS
                    .get(self.easing.selected_index() as usize)
                    .map(|entry| entry.1)
                    .ok_or("Choose an easing curve.")?,
            },
        };
        core::validate_prototype_interaction(document, self.source, &rule)?;
        Ok(rule)
    }

    fn show_error(&self, error: &str) {
        self.error.set_text(Some(error));
        self.error.set_is_visible(true);
    }
}

impl EditorShell {
    pub(super) fn interactions_dialog(self: &Rc<Self>) {
        let source = {
            let editor = self.editor.borrow();
            (editor.selection.len() == 1).then(|| editor.selection[0])
        };
        if let Some(source) = source {
            self.interactions_dialog_for(source, None);
        } else {
            self.toast("Select one layer to edit its prototype interactions.");
        }
    }

    fn interactions_dialog_for(self: &Rc<Self>, source: NodeId, editing: Option<usize>) {
        let (name, interactions, targets, target_labels, variables, variable_labels, draft) = {
            let editor = self.editor.borrow();
            let document = &editor.document;
            let Some(node) = document.nodes.get(&source) else {
                return;
            };
            if document.effective_locked(source) {
                drop(editor);
                self.toast("Unlock the source layer before editing its interactions.");
                return;
            }
            let mut targets: Vec<_> = document.nodes.keys().copied().collect();
            targets.sort_by_key(|id| (document.nodes[id].kind != NodeKind::Frame, *id));
            let target_labels: Vec<_> = targets
                .iter()
                .map(|id| interaction_layer_label(document, *id))
                .collect();
            let variables: Vec<_> = document.variables.keys().copied().collect();
            let variable_labels: Vec<_> = variables
                .iter()
                .map(|id| {
                    let variable = &document.variables[id];
                    format!("{} / {}", variable.collection, variable.name)
                })
                .collect();
            let draft = editing
                .and_then(|index| node.interactions.get(index).cloned())
                .unwrap_or(core::Interaction {
                    trigger: core::Trigger::Click,
                    action: core::PrototypeAction::NavigateTo {
                        target: targets
                            .iter()
                            .copied()
                            .find(|id| *id != source)
                            .unwrap_or(source),
                    },
                    transition: core::Transition {
                        kind: core::TransitionKind::Instant,
                        ..core::Transition::default()
                    },
                    delay_ms: 300,
                    key: None,
                });
            (
                node.name.clone(),
                node.interactions.clone(),
                targets,
                target_labels,
                variables,
                variable_labels,
                draft,
            )
        };
        let _updating = UpdatingInteractions(&self.updating, self.updating.replace(true));
        self.modal(
            "Prototype interactions",
            &format!(
                "{name} · Changes are saved when you add or update a rule, with one undo step."
            ),
        );
        let host = self.control::<StackPanel>("ModalContent");
        host.set_spacing(12.0);
        self.prototype_scroll_controls(source, &host);
        if interactions.is_empty() {
            host.children().add(interaction_text(
                "No interactions yet. Choose a trigger and action below.",
                true,
            ));
        }
        for (index, rule) in interactions.iter().enumerate() {
            let card = Border::new();
            card.set_name(Some(format!("InteractionCard{index}")));
            card.set_padding(Thickness::uniform(10.0));
            card.set_corner_radius(CornerRadius::uniform(7.0));
            card.set_background(Some(icons::brush(if editing == Some(index) {
                "#F0EAFE"
            } else {
                "#F7F7F7"
            })));
            let row = interaction_grid("*,54,72");
            let summary = interaction_summary(&self.editor.borrow().document, rule);
            row.children().add(interaction_text(
                &format!("{}. {summary}", index + 1),
                false,
            ));
            let edit =
                self.interaction_button("Edit", &format!("InteractionEdit{index}"), move |shell| {
                    shell.interactions_dialog_for(source, Some(index))
                });
            Grid::set_column(&edit, 1);
            row.children().add(edit);
            let expected = rule.clone();
            let remove = self.interaction_button(
                "Remove",
                &format!("InteractionRemove{index}"),
                move |shell| {
                    let mut editor = shell.editor.borrow_mut();
                    let valid = editor.selection.as_slice() == [source]
                        && !editor.document.effective_locked(source)
                        && editor
                            .document
                            .nodes
                            .get(&source)
                            .and_then(|node| node.interactions.get(index))
                            == Some(&expected);
                    if !valid {
                        drop(editor);
                        shell.toast("This interaction changed. Reopen it before removing it.");
                        return;
                    }
                    editor.transact("Remove prototype interaction", |document| {
                        document
                            .nodes
                            .get_mut(&source)
                            .unwrap()
                            .interactions
                            .remove(index);
                    });
                    drop(editor);
                    shell.changed();
                    shell.interactions_dialog_for(source, None);
                },
            );
            Grid::set_column(&remove, 2);
            row.children().add(remove);
            card.set_child(row);
            host.children().add(card);
        }
        host.children().add(interaction_text(
            if editing.is_some() {
                "Edit interaction"
            } else {
                "New interaction"
            },
            false,
        ));
        let form = Rc::new(self.interaction_form(
            source,
            editing,
            &draft,
            targets,
            target_labels,
            variables,
            variable_labels,
        ));
        for choice in [
            &form.trigger,
            &form.action,
            &form.value_kind,
            &form.transition,
        ] {
            let weak_form = Rc::downgrade(&form);
            let weak_shell = Rc::downgrade(self);
            choice.selection_changed(move |_, _| {
                if let (Some(form), Some(shell)) = (weak_form.upgrade(), weak_shell.upgrade()) {
                    if !shell.updating.get() {
                        form.update_visibility();
                    }
                }
            });
        }
        form.update_visibility();
        let actions = self.control::<StackPanel>("ModalActions");
        if editing.is_some() {
            actions.children().add(self.interaction_button(
                "New rule",
                "InteractionNew",
                move |shell| shell.interactions_dialog_for(source, None),
            ));
        }
        let save = self.interaction_button(
            if editing.is_some() {
                "Update interaction"
            } else {
                "Add interaction"
            },
            "InteractionSave",
            move |shell| {
                let mut editor = shell.editor.borrow_mut();
                if editor.selection.as_slice() != [form.source] {
                    form.show_error("The selected source changed. Reopen its interactions.");
                    return;
                }
                let rule = match form.read(&editor.document) {
                    Ok(rule) => rule,
                    Err(error) => {
                        form.show_error(&error);
                        return;
                    }
                };
                if let Some(index) = form.editing {
                    if editor
                        .document
                        .nodes
                        .get(&form.source)
                        .and_then(|node| node.interactions.get(index))
                        != form.expected.as_ref()
                    {
                        form.show_error("This interaction changed. Reopen it before saving.");
                        return;
                    }
                }
                editor.transact(
                    if form.editing.is_some() {
                        "Update prototype interaction"
                    } else {
                        "Add prototype interaction"
                    },
                    |document| {
                        let node = document.nodes.get_mut(&form.source).unwrap();
                        if let Some(index) = form.editing {
                            node.interactions[index] = rule;
                        } else {
                            node.interactions.push(rule);
                        }
                    },
                );
                drop(editor);
                shell.changed();
                shell.interactions_dialog_for(form.source, None);
            },
        );
        save.classes().add("primary");
        actions.children().add(save);
        actions
            .children()
            .add(self.interaction_button("Done", "InteractionDone", |shell| shell.close_modal()));
    }

    fn interaction_form(
        &self,
        source: NodeId,
        editing: Option<usize>,
        draft: &core::Interaction,
        targets: Vec<NodeId>,
        target_labels: Vec<String>,
        variables: Vec<core::VariableId>,
        variable_labels: Vec<String>,
    ) -> InteractionForm {
        let host = self.control::<StackPanel>("ModalContent");
        let (trigger_row, trigger) = interaction_choice(
            "Trigger",
            "InteractionTrigger",
            &TRIGGERS.iter().map(|entry| entry.0).collect::<Vec<_>>(),
            TRIGGERS
                .iter()
                .position(|entry| entry.1 == draft.trigger)
                .unwrap_or(0),
        );
        let (action_row, action) = interaction_choice(
            "Action",
            "InteractionAction",
            ACTIONS,
            interaction_action_index(&draft.action),
        );
        let first = interaction_grid("*,*");
        first.children().add(trigger_row);
        Grid::set_column(&action_row, 1);
        first.children().add(action_row);
        host.children().add(first);
        host.children().add(interaction_text(
            "While hovering and while pressing restore the previous preview state on pointer exit or release.",
            true,
        ));
        let selected_target = match draft.action {
            core::PrototypeAction::NavigateTo { target }
            | core::PrototypeAction::OpenOverlay { target, .. }
            | core::PrototypeAction::ScrollTo { target } => Some(target),
            _ => None,
        };
        let (target_row, target) = interaction_choice(
            "Destination layer",
            "InteractionTarget",
            &target_labels.iter().map(String::as_str).collect::<Vec<_>>(),
            selected_target
                .and_then(|target| targets.iter().position(|id| *id == target))
                .unwrap_or(0),
        );
        host.children().add(target_row.clone());
        let (delay_row, delay) = interaction_field(
            "Delay · milliseconds",
            "InteractionDelay",
            &draft.delay_ms.to_string(),
        );
        host.children().add(delay_row.clone());
        let (key_row, key) = interaction_field(
            "Key or chord",
            "InteractionKey",
            draft.key.as_deref().unwrap_or("Enter"),
        );
        key.set_placeholder_text(Some("Enter, Escape, Ctrl+Shift+A…"));
        host.children().add(key_row.clone());
        let (url_row, url) = interaction_field(
            "URL",
            "InteractionUrl",
            match &draft.action {
                core::PrototypeAction::OpenUrl { url } => url,
                _ => "",
            },
        );
        url.set_placeholder_text(Some("https://example.com"));
        host.children().add(url_row.clone());
        let position = match draft.action {
            core::PrototypeAction::OpenOverlay { position, .. } => position,
            _ => Point::default(),
        };
        let (x_row, overlay_x) =
            interaction_field("Overlay X", "InteractionOverlayX", &number(position.x));
        let (y_row, overlay_y) =
            interaction_field("Overlay Y", "InteractionOverlayY", &number(position.y));
        let overlay_row = interaction_grid("*,*");
        overlay_row.children().add(x_row);
        Grid::set_column(&y_row, 1);
        overlay_row.children().add(y_row);
        host.children().add(overlay_row.clone());
        let (selected_variable, value_kind_index, value_text, selected_alias) = match &draft.action
        {
            core::PrototypeAction::SetVariable { variable, value } => {
                let (kind, value, alias) = match value {
                    core::VariableValue::Color(color) => (0, color.hex(), None),
                    core::VariableValue::Number(value) => (1, value.to_string(), None),
                    core::VariableValue::String(value) => (2, value.clone(), None),
                    core::VariableValue::Boolean(value) => (3, value.to_string(), None),
                    core::VariableValue::Alias(value) => (4, String::new(), Some(*value)),
                };
                (Some(*variable), kind, value, alias)
            }
            _ => (None, 0, "#7B61FF".to_string(), None),
        };
        let labels: Vec<_> = variable_labels.iter().map(String::as_str).collect();
        let (variable_panel, variable) = interaction_choice(
            "Variable",
            "InteractionVariable",
            &labels,
            selected_variable
                .and_then(|selected| variables.iter().position(|id| *id == selected))
                .unwrap_or(0),
        );
        let (value_kind_panel, value_kind) = interaction_choice(
            "Value type",
            "InteractionValueKind",
            &["Color", "Number", "Text", "Boolean", "Alias"],
            value_kind_index,
        );
        let variable_row = interaction_grid("*,*");
        variable_row.children().add(variable_panel);
        Grid::set_column(&value_kind_panel, 1);
        variable_row.children().add(value_kind_panel);
        host.children().add(variable_row.clone());
        let (value_row, value) = interaction_field("Value", "InteractionValue", &value_text);
        host.children().add(value_row.clone());
        let (alias_row, alias) = interaction_choice(
            "Reference variable",
            "InteractionAlias",
            &labels,
            selected_alias
                .and_then(|selected| variables.iter().position(|id| *id == selected))
                .unwrap_or(0),
        );
        host.children().add(alias_row.clone());
        let (transition_panel, transition) = interaction_choice(
            "Transition",
            "InteractionTransition",
            &TRANSITIONS.iter().map(|entry| entry.0).collect::<Vec<_>>(),
            TRANSITIONS
                .iter()
                .position(|entry| entry.1 == draft.transition.kind)
                .unwrap_or(0),
        );
        let (duration_panel, duration) = interaction_field(
            "Duration · ms",
            "InteractionDuration",
            &draft.transition.duration_ms.to_string(),
        );
        let (easing_panel, easing) = interaction_choice(
            "Easing",
            "InteractionEasing",
            &EASINGS.iter().map(|entry| entry.0).collect::<Vec<_>>(),
            EASINGS
                .iter()
                .position(|entry| entry.1 == draft.transition.easing)
                .unwrap_or(0),
        );
        let motion = interaction_grid("*,92,*");
        motion.children().add(transition_panel);
        Grid::set_column(&duration_panel, 1);
        motion.children().add(duration_panel);
        Grid::set_column(&easing_panel, 2);
        motion.children().add(easing_panel);
        host.children().add(motion);
        let animation_note = interaction_text(
            "Smart animate matches layer names within the same hierarchy. Compatible geometry and paints interpolate; changed content dissolves.",
            true,
        );
        animation_note.set_name(Some("InteractionAnimationNote".into()));
        host.children().add(animation_note.clone());
        let scroll_note = interaction_text(
            "Enable overflow scrolling on the containing frame. Scroll to aligns this destination inside its enclosing scroll frames.",
            true,
        );
        host.children().add(scroll_note.clone());
        let error = interaction_text("", false);
        error.set_name(Some("InteractionError".into()));
        error.set_foreground(Some(icons::brush("#C92A2A")));
        host.children().add(error.clone());
        InteractionForm {
            source,
            editing,
            expected: editing.map(|_| draft.clone()),
            trigger,
            action,
            target,
            targets,
            delay,
            key,
            url,
            overlay_x,
            overlay_y,
            variable,
            variables,
            value_kind,
            value,
            alias,
            transition,
            duration,
            easing,
            target_row,
            delay_row,
            key_row,
            url_row,
            overlay_row,
            variable_row,
            value_row,
            alias_row,
            animation_note,
            scroll_note,
            error,
        }
    }

    fn prototype_scroll_controls(self: &Rc<Self>, source: NodeId, host: &Ref<StackPanel>) {
        let overflow = {
            let editor = self.editor.borrow();
            let Some(node) = editor.document.nodes.get(&source) else { return };
            if !node.kind.is_container() { return; }
            node.prototype_overflow
        };
        let selected = match overflow {
            core::PrototypeOverflow::None => 0,
            core::PrototypeOverflow::Horizontal => 1,
            core::PrototypeOverflow::Vertical => 2,
            core::PrototypeOverflow::Both => 3,
        };
        let (field, choice) = interaction_choice(
            "Frame overflow scrolling",
            "PrototypeOverflowMode",
            &["No scrolling", "Horizontal", "Vertical", "Both directions"],
            selected,
        );
        let row = interaction_grid("*,124");
        row.children().add(field);
        let apply = self.interaction_button("Apply scrolling", "PrototypeOverflowSave", move |shell| {
            let overflow = match choice.selected_index() {
                1 => core::PrototypeOverflow::Horizontal,
                2 => core::PrototypeOverflow::Vertical,
                3 => core::PrototypeOverflow::Both,
                _ => core::PrototypeOverflow::None,
            };
            let mut editor = shell.editor.borrow_mut();
            if editor.selection.as_slice() != [source] || editor.document.effective_locked(source) {
                drop(editor);
                shell.toast("Select and unlock this frame before changing its scrolling.");
                return;
            }
            editor.update_node(source, "Set prototype scrolling", |node| {
                node.prototype_overflow = overflow;
                if overflow != core::PrototypeOverflow::None { node.clip_content = true; }
            });
            drop(editor);
            shell.changed();
        });
        apply.set_vertical_alignment(VerticalAlignment::Bottom);
        Grid::set_column(&apply, 1);
        row.children().add(apply);
        host.children().add(row);
    }

    fn interaction_button(
        self: &Rc<Self>,
        caption: &str,
        name: &str,
        action: impl Fn(&Rc<Self>) + 'static,
    ) -> Ref<Button> {
        let button = Button::new();
        button.set_name(Some(name.to_string()));
        button.set_content(Some(Rc::new(caption.to_string())));
        button.set_height(30.0);
        let weak = Rc::downgrade(self);
        button.click(move |_, _| {
            if let Some(shell) = weak.upgrade() {
                if !shell.updating.get() {
                    action(&shell);
                }
            }
        });
        button
    }
}

fn interaction_field(label: &str, name: &str, value: &str) -> (Ref<StackPanel>, Ref<TextBox>) {
    let row = StackPanel::new();
    row.set_spacing(4.0);
    row.children().add(interaction_text(label, true));
    let field = TextBox::new();
    field.set_name(Some(name.to_string()));
    field.set_text(Some(value));
    field.set_min_height(30.0);
    row.children().add(field.clone());
    (row, field)
}

fn interaction_choice(
    label: &str,
    name: &str,
    labels: &[&str],
    selected: usize,
) -> (Ref<StackPanel>, Ref<ComboBox>) {
    let row = StackPanel::new();
    row.set_spacing(4.0);
    row.children().add(interaction_text(label, true));
    let choice = ComboBox::new();
    choice.set_name(Some(name.to_string()));
    for label in labels {
        choice.items().add(Some(Rc::new(label.to_string())));
    }
    choice.set_selected_index(if labels.is_empty() {
        -1
    } else {
        selected.min(labels.len() - 1) as i32
    });
    choice.set_min_height(30.0);
    row.children().add(choice.clone());
    (row, choice)
}

fn interaction_grid(columns: &str) -> Ref<Grid> {
    let grid = Grid::new();
    grid.set_column_definitions(
        ColumnDefinitions::parse(columns).expect("interaction form columns"),
    );
    grid.set_column_spacing(8.0);
    grid
}

fn interaction_text(value: &str, muted: bool) -> Ref<TextBlock> {
    let text = TextBlock::new();
    text.set_text(Some(value));
    text.set_font_size(if muted { 11.0 } else { 12.0 });
    text.set_foreground(Some(icons::brush(if muted {
        "#737373"
    } else {
        "#242424"
    })));
    text.set_text_wrapping(TextWrapping::Wrap);
    text
}

fn interaction_number(field: &TextBox, name: &str) -> Result<f64, String> {
    field
        .text()
        .unwrap_or_default()
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|number| number.is_finite())
        .ok_or_else(|| format!("{name} must be a finite number."))
}

fn interaction_milliseconds(field: &TextBox, name: &str) -> Result<u32, String> {
    field
        .text()
        .unwrap_or_default()
        .trim()
        .parse::<u32>()
        .map_err(|_| {
            format!(
                "{name} must be a whole number from 0 to {} milliseconds.",
                u32::MAX
            )
        })
}

fn interaction_action_index(action: &core::PrototypeAction) -> usize {
    match action {
        core::PrototypeAction::NavigateTo { .. } => 0,
        core::PrototypeAction::OpenOverlay { .. } => 1,
        core::PrototypeAction::CloseOverlay => 2,
        core::PrototypeAction::Back => 3,
        core::PrototypeAction::OpenUrl { .. } => 4,
        core::PrototypeAction::SetVariable { .. } => 5,
        core::PrototypeAction::ScrollTo { .. } => 6,
    }
}

fn interaction_layer_label(document: &core::Document, id: NodeId) -> String {
    let Some(node) = document.nodes.get(&id) else {
        return "Missing layer".into();
    };
    let page = document
        .page_of(id)
        .and_then(|id| document.page(id))
        .map(|page| page.name.as_str())
        .unwrap_or("Page");
    format!("{page} / {} · {}", node.name, node.kind.label())
}

fn interaction_summary(document: &core::Document, interaction: &core::Interaction) -> String {
    let trigger = match interaction.trigger {
        core::Trigger::AfterDelay => format!("After {} ms", interaction.delay_ms),
        core::Trigger::Key => format!("Key {}", interaction.key.as_deref().unwrap_or("not set")),
        trigger => TRIGGERS
            .iter()
            .find(|entry| entry.1 == trigger)
            .map(|entry| entry.0)
            .unwrap_or("Trigger")
            .to_string(),
    };
    let action = match &interaction.action {
        core::PrototypeAction::NavigateTo { target } => {
            format!("Navigate to {}", interaction_layer_label(document, *target))
        }
        core::PrototypeAction::OpenOverlay { target, position } => format!(
            "Open {} at {}, {}",
            interaction_layer_label(document, *target),
            number(position.x),
            number(position.y)
        ),
        core::PrototypeAction::CloseOverlay => "Close overlay".to_string(),
        core::PrototypeAction::Back => "Go back".to_string(),
        core::PrototypeAction::ScrollTo { target } => {
            format!("Scroll to {}", interaction_layer_label(document, *target))
        }
        core::PrototypeAction::OpenUrl { url } => format!("Open {url}"),
        core::PrototypeAction::SetVariable { variable, .. } => format!(
            "Set {}",
            document
                .variables
                .get(variable)
                .map(|variable| variable.name.as_str())
                .unwrap_or("missing variable")
        ),
    };
    format!("{trigger} → {action}")
}
