use super::behaviors::{add_picker, buttons};
use super::{description, field, text, DesignerWorkspace};
use crate::icons;
use crate::property_transition_session::PropertyTransitionCommand;
use crate::workspace::inspector_actions::{add_button, picker, SOURCE};
use ferroui_base::Ref;
use ferroui_controls::{Control, StackPanel, TextBox};
use ferroui_designer_animation::native::TransitionPropertyChoice;
use ferroui_designer_animation::{AuthoredPropertyTransition, Easing, PropertyTransitionTiming};
use ferroui_designer_core::{NodeId, SourceSpan};
use std::rc::{Rc, Weak};

impl DesignerWorkspace {
    pub(super) fn build_property_transition_panel(
        self: &Rc<Self>,
        panel: &StackPanel,
        owner: NodeId,
    ) {
        let session = self.session.borrow();
        let uri = session.document_uri().to_owned();
        let authoring = self.authoring.borrow();
        let target = authoring
            .property_transition_target
            .as_ref()
            .filter(|(document, node, _)| document == &uri && *node == owner)
            .map(|(_, _, target)| target.clone());
        let selected = authoring.property_transition;
        drop(authoring);
        let editor = session
            .property_transition_editor(owner, target.as_deref())
            .or_else(|_| session.property_transition_editor(owner, None));
        let Ok(editor) = editor else { return };
        let model = editor.list;
        let index = model
            .entries
            .iter()
            .position(|entry| Some(entry.id) == selected)
            .unwrap_or(0);
        let scope = TransitionScope {
            workspace: Rc::downgrade(self),
            owner,
            generation: session.generation(),
            target,
            index,
        };
        drop(session);
        if editor.requires_target && editor.can_edit {
            resource_target_field(panel, &scope, uri);
        }
        let entry = model.entries.get(index);
        if entry.is_some() {
            add_picker(
                panel,
                self,
                "Property transition",
                &model.entries,
                index,
                |entry| {
                    (
                        entry.id,
                        format!(
                            "{} · {}",
                            entry.property.as_deref().unwrap_or("Property"),
                            entry.type_name
                        ),
                    )
                },
                |authoring, id| authoring.property_transition = Some(id),
            );
        }
        let toolbar = if editor.can_edit && !editor.choices.is_empty() && model.can_add() {
            property_actions(panel, &scope, editor.choices, entry)
        } else {
            buttons(panel)
        };
        let Some(entry) = entry else {
            if let Some(source) = model.source {
                scope.reveal(&toolbar, source);
            }
            if !model.can_add() {
                description(panel, "Open the authored transition collection in source.");
            }
            return;
        };
        scope.reveal(&toolbar, entry.source);
        if editor.can_edit {
            for (destination, icon, label) in [
                (
                    index.checked_sub(1),
                    icons::MOVE_UP,
                    "Move transition earlier",
                ),
                (
                    (index + 1 < model.entries.len()).then_some(index + 1),
                    icons::MOVE_DOWN,
                    "Move transition later",
                ),
            ] {
                if let Some(index) = destination {
                    let transition = entry.id;
                    scope.add(&toolbar, icon, label, move || {
                        Ok(PropertyTransitionCommand::Move { transition, index })
                    });
                }
            }
            let transition = entry.id;
            scope.add(&toolbar, icons::REMOVE, "Remove transition", move || {
                Ok(PropertyTransitionCommand::Remove { transition })
            });
            if !entry.editable_timing.is_empty() {
                add_timing_fields(panel, &scope, entry);
            }
        }
    }
}

const RETARGET: &str = "F1 M8 1A7 7 0 1 1 8 15A7 7 0 1 1 8 1Z M8 3A5 5 0 1 0 8 13A5 5 0 1 0 8 3Z M8 5.5A2.5 2.5 0 1 1 8 10.5A2.5 2.5 0 1 1 8 5.5Z";

fn property_actions(
    panel: &StackPanel,
    scope: &TransitionScope,
    choices: Vec<TransitionPropertyChoice>,
    entry: Option<&AuthoredPropertyTransition>,
) -> Ref<StackPanel> {
    let labels: Vec<_> = choices
        .iter()
        .map(|choice| {
            if choices.iter().any(|other| {
                other.property == choice.property && other.type_name != choice.type_name
            }) {
                format!("{} · {}", choice.label, choice.type_name)
            } else {
                choice.label.clone()
            }
        })
        .collect();
    let index = entry
        .and_then(|entry| {
            choices.iter().position(|choice| {
                (entry.property.as_deref() == Some(choice.property.as_str())
                    || entry.property.as_deref() == Some(choice.label.as_str()))
                    && choice.type_name == entry.type_name
            })
        })
        .unwrap_or(0);
    description(panel, "Target property");
    let control = picker(
        "Transition target property",
        labels.iter().map(String::as_str),
        index,
    );
    panel.children().add(control.clone().upcast::<Control>());
    let toolbar = buttons(panel);
    let choices = Rc::new(choices);
    let selected = move || {
        choices
            .get(control.selected_index() as usize)
            .cloned()
            .ok_or_else(|| "Choose a registered property".to_owned())
    };
    let choose = selected.clone();
    scope.add(&toolbar, icons::ADD, "Add property transition", move || {
        let choice = choose()?;
        Ok(PropertyTransitionCommand::Add {
            property: choice.property,
            type_name: Some(choice.type_name.into()),
            timing: Default::default(),
        })
    });
    if let Some(entry) = entry.filter(|entry| entry.can_retarget) {
        let transition = entry.id;
        scope.add(&toolbar, RETARGET, "Retarget transition", move || {
            let choice = selected()?;
            Ok(PropertyTransitionCommand::Retarget {
                transition,
                property: choice.property,
                type_name: Some(choice.type_name.into()),
            })
        });
    }
    toolbar
}

fn add_timing_fields(
    panel: &StackPanel,
    scope: &TransitionScope,
    entry: &AuthoredPropertyTransition,
) {
    let duration = entry
        .duration_seconds
        .map(|seconds| field(panel, "Duration (seconds)", &seconds.to_string()));
    let delay = entry
        .delay_seconds
        .map(|seconds| field(panel, "Start delay (seconds)", &seconds.to_string()));
    let easing = entry.editable_timing.contains(&"Easing").then(|| {
        field(
            panel,
            "Easing",
            entry.easing.as_deref().unwrap_or("LinearEasing"),
        )
    });
    let authored = entry.easing.clone();
    let (id, duration_seconds, delay_seconds) =
        (entry.id, entry.duration_seconds, entry.delay_seconds);
    scope.add(
        &buttons(panel),
        icons::CHECK,
        "Apply transition timing",
        move || {
            let timing = PropertyTransitionTiming {
                duration: edited_seconds(duration.as_ref(), duration_seconds)?,
                delay: edited_seconds(delay.as_ref(), delay_seconds)?,
                easing: easing
                    .as_ref()
                    .map(|field| text(field))
                    .filter(|value| value.trim() != authored.as_deref().unwrap_or("LinearEasing"))
                    .map(|value| Easing::parse(&value).map_err(|error| error.to_string()))
                    .transpose()?,
            };
            Ok(PropertyTransitionCommand::Timing {
                transition: id,
                timing,
            })
        },
    );
}

fn edited_seconds(
    field: Option<&Ref<TextBox>>,
    initial: Option<f64>,
) -> Result<Option<f64>, String> {
    let Some(field) = field else { return Ok(None) };
    let seconds = text(field)
        .trim()
        .parse::<f64>()
        .map_err(|_| "Enter a nonnegative number of seconds")?;
    if !seconds.is_finite() || seconds < 0.0 {
        return Err("Enter a nonnegative number of seconds".into());
    }
    Ok((Some(seconds) != initial).then_some(seconds))
}

fn resource_target_field(panel: &StackPanel, scope: &TransitionScope, uri: String) {
    let target = field(panel, "Target type", scope.target.as_deref().unwrap_or(""));
    let scope = scope.clone();
    add_button(
        &buttons(panel),
        RETARGET,
        "Choose resource target type",
        move || {
            scope.run(|workspace| {
                let target = text(&target).trim().to_owned();
                if target.is_empty() {
                    workspace.authoring.borrow_mut().property_transition_target = None;
                    return Ok(());
                }
                let editor = workspace
                    .session
                    .borrow()
                    .property_transition_editor(scope.owner, Some(&target))?;
                if editor.choices.is_empty() {
                    return Err("Choose an animatable XAML type with registered properties".into());
                }
                workspace.authoring.borrow_mut().property_transition_target =
                    Some((uri.clone(), scope.owner, target));
                Ok(())
            });
        },
    );
}

#[derive(Clone)]
struct TransitionScope {
    workspace: Weak<DesignerWorkspace>,
    owner: NodeId,
    generation: u64,
    target: Option<String>,
    index: usize,
}

impl TransitionScope {
    fn run(&self, action: impl FnOnce(&DesignerWorkspace) -> Result<(), String>) {
        let Some(workspace) = self.workspace.upgrade() else {
            return;
        };
        let current = workspace.session.borrow().generation() == self.generation;
        let result = if current {
            action(&workspace)
        } else {
            Err("The source changed. Open the current transition again.".into())
        };
        if result.is_ok() {
            workspace.refresh(true);
        }
        workspace.report(result);
    }

    fn add(
        &self,
        toolbar: &StackPanel,
        icon: &str,
        label: &str,
        command: impl Fn() -> Result<PropertyTransitionCommand, String> + 'static,
    ) {
        let scope = self.clone();
        add_button(toolbar, icon, label, move || {
            scope.run(|workspace| {
                let command = command()?;
                let index = match &command {
                    PropertyTransitionCommand::Add { .. } => None,
                    PropertyTransitionCommand::Move { index, .. } => Some(*index),
                    _ => Some(scope.index),
                };
                let mut session = workspace.session.borrow_mut();
                session.edit_property_transition(scope.owner, scope.target.as_deref(), command)?;
                workspace.authoring.borrow_mut().property_transition = session
                    .property_transitions(scope.owner)
                    .ok()
                    .and_then(|list| {
                        let last = list.entries.len().checked_sub(1)?;
                        Some(list.entries[index.unwrap_or(last).min(last)].id)
                    });
                Ok(())
            });
        });
    }

    fn reveal(&self, toolbar: &StackPanel, source: SourceSpan) {
        let scope = self.clone();
        add_button(toolbar, SOURCE, "Reveal transition in source", move || {
            if let Some(workspace) = scope.workspace.upgrade() {
                if workspace.session.borrow().generation() == scope.generation {
                    workspace.select_source_span(source);
                }
            }
        });
    }
}
