//! Native inspector for one source-owned behavior and its ordered actions.
use super::{
    action, behavior_drafts, choice, choice_field, code_field, description, field, optional,
    section, text, AuthoringUi, DesignerWorkspace,
};
use crate::icons;
use crate::workspace::inspector_actions::{add_button, picker, RESET, SOURCE};
use ferroui_base::layout::{HorizontalAlignment, Orientation};
use ferroui_base::{Ref, Thickness};
use ferroui_controls::{Control, Expander, StackPanel};
use ferroui_designer_animation::{
    BehaviorAction, BehaviorEdit, BehaviorModel, BehaviorSource, BehaviorTrigger,
};
use ferroui_designer_core::NodeId;
use std::rc::{Rc, Weak};

const PLAY: &str = "F1 M4 1.5L14 8L4 14.5Z";

impl DesignerWorkspace {
    pub(super) fn build_behavior_controls(self: &Rc<Self>, panel: &StackPanel, owner: NodeId) {
        let session = self.session.borrow();
        let Ok(behaviors) = session.behavior_sources(owner) else {
            return;
        };
        let writable = session.can_manipulate();
        let can_attach = writable && BehaviorSource::can_attach(session.document(), owner);
        drop(session);

        if behaviors.is_empty() {
            if can_attach {
                self.attach_behavior_fields(panel, owner);
            } else {
                description(panel, "Edit the behavior collection in source.");
            }
            return;
        }
        let current = self.authoring.borrow().behavior;
        let index = behaviors
            .iter()
            .position(|behavior| Some(behavior.node) == current)
            .unwrap_or(0);
        let behavior = &behaviors[index];
        add_picker(
            panel,
            self,
            "Behavior",
            &behaviors,
            index,
            |source| (source.node, trigger_label(&source.model.trigger)),
            |ui, node| {
                ui.behavior = Some(node);
                ui.behavior_action = None;
            },
        );
        self.behavior_details(panel, behavior, index, writable);
        if can_attach {
            let draft = expander(panel, "Attach behavior");
            self.attach_behavior_fields(&draft, owner);
        }
    }

    fn attach_behavior_fields(self: &Rc<Self>, panel: &StackPanel, owner: NodeId) {
        let trigger = behavior_drafts::trigger(panel);
        section(panel, "Action");
        let first = behavior_drafts::action(panel, true, None);
        let generation = self.session.borrow().generation();
        action(panel, "Attach behavior", self, move |this| {
            let mut session = this.session.borrow_mut();
            if session.generation() != generation || !session.can_manipulate() {
                return Err("The behavior owner changed. Reopen the behavior editor.".into());
            }
            let model = BehaviorModel {
                id: None,
                owner,
                trigger: trigger()?,
                actions: vec![first()?],
            };
            let behavior = session.upsert_behavior(model)?;
            this.authoring.borrow_mut().behavior = Some(behavior);
            this.authoring.borrow_mut().behavior_action = None;
            Ok(())
        });
    }

    fn behavior_details(
        self: &Rc<Self>,
        panel: &StackPanel,
        behavior: &BehaviorSource,
        index: usize,
        writable: bool,
    ) {
        let scope = BehaviorScope::new(self, behavior.node);
        let id = behavior.node;
        let session = self.session.borrow();
        let previewable = session.can_preview_behavior(behavior.model.owner, index);
        let preview = session.active_behavior_preview();
        let edit_trigger_fields = behavior.can_edit_trigger(session.document());
        let source = session
            .document()
            .ast()
            .node(id)
            .map(|node| {
                session.document().valid_source()[node.span.start..node.span.end].to_owned()
            })
            .unwrap_or_default();
        drop(session);
        let toolbar = buttons(panel);
        if previewable {
            scope.add(&toolbar, PLAY, "Preview actions", move |this| {
                this.session.borrow_mut().preview_behavior(id)
            });
        }
        if preview.is_some() {
            scope.add(&toolbar, RESET, "Reset behavior preview", |this| {
                this.session.borrow_mut().clear_interaction_preview();
                Ok(())
            });
        }
        scope.reveal(&toolbar, id, "Reveal behavior in source");
        if writable {
            scope.add(&toolbar, icons::REMOVE, "Remove behavior", move |this| {
                this.session.borrow_mut().remove_behavior(id)
            });
            if edit_trigger_fields {
                edit_trigger(panel, &scope, &behavior.model.trigger);
            }
        }

        if !behavior.actions.is_empty() {
            self.behavior_action_details(panel, behavior, &scope, writable);
        }
        if writable && behavior.can_edit_actions() {
            let draft = expander(panel, "Add action");
            let item = behavior_drafts::action(&draft, false, None);
            scope.add(&buttons(&draft), icons::ADD, "Append action", move |this| {
                this.session
                    .borrow_mut()
                    .edit_behavior(id, BehaviorEdit::AppendAction(item()?))
            });
        }
        if writable {
            let advanced = expander(panel, "Behavior XAML");
            let editor = code_field(&advanced, "Source", &source);
            scope.add(
                &buttons(&advanced),
                icons::CHECK,
                "Apply behavior XAML",
                move |this| {
                    this.session
                        .borrow_mut()
                        .set_behavior_source(id, &text(&editor))
                },
            );
        }
    }

    fn behavior_action_details(
        self: &Rc<Self>,
        panel: &StackPanel,
        behavior: &BehaviorSource,
        scope: &BehaviorScope,
        writable: bool,
    ) {
        let current = self.authoring.borrow().behavior_action;
        let index = behavior
            .actions
            .iter()
            .position(|action| Some(action.node) == current)
            .unwrap_or(0);
        let action = &behavior.actions[index];
        let id = behavior.node;
        let node = action.node;
        add_picker(
            panel,
            self,
            "Action",
            &behavior.actions,
            index,
            |source| (source.node, action_label(&source.value)),
            |ui, node| ui.behavior_action = Some(node),
        );
        let tools = buttons(panel);
        scope.reveal(&tools, node, "Reveal action in source");
        if !writable || !behavior.can_edit_actions() {
            return;
        }
        for (destination, icon, label) in [
            (index.checked_sub(1), icons::MOVE_UP, "Move action earlier"),
            (
                (index + 1 < behavior.actions.len()).then_some(index + 1),
                icons::MOVE_DOWN,
                "Move action later",
            ),
        ] {
            if let Some(index) = destination {
                scope.add(&tools, icon, label, move |this| {
                    this.authoring.borrow_mut().behavior_action = Some(node);
                    this.session
                        .borrow_mut()
                        .edit_behavior(id, BehaviorEdit::MoveAction { node, index })
                });
            }
        }
        scope.add(&tools, icons::REMOVE, "Remove action", move |this| {
            this.session
                .borrow_mut()
                .edit_behavior(id, BehaviorEdit::RemoveAction(node))
        });
        if !matches!(
            action.value,
            BehaviorAction::Custom { .. } | BehaviorAction::Markup { .. }
        ) {
            let fields = expander(panel, "Edit action");
            let draft = behavior_drafts::action(&fields, false, Some(&action.value));
            scope.add(
                &buttons(&fields),
                icons::CHECK,
                "Apply action",
                move |this| {
                    this.session.borrow_mut().edit_behavior(
                        id,
                        BehaviorEdit::SetAction {
                            node,
                            value: draft()?,
                        },
                    )
                },
            );
        }
    }
}

struct BehaviorScope {
    workspace: Weak<DesignerWorkspace>,
    generation: u64,
    id: NodeId,
}

impl BehaviorScope {
    fn new(workspace: &Rc<DesignerWorkspace>, id: NodeId) -> Self {
        Self {
            workspace: Rc::downgrade(workspace),
            generation: workspace.session.borrow().generation(),
            id,
        }
    }

    fn add(
        &self,
        panel: &StackPanel,
        icon: &str,
        label: &str,
        run: impl Fn(&Rc<DesignerWorkspace>) -> Result<(), String> + 'static,
    ) {
        let weak = self.workspace.clone();
        let generation = self.generation;
        add_button(panel, icon, label, move || {
            if let Some(workspace) = weak.upgrade() {
                let current = workspace.session.borrow().generation();
                let result = if current == generation {
                    run(&workspace)
                } else {
                    Err("The behavior or selection changed. Reopen the editor.".into())
                };
                workspace.refresh(true);
                workspace.report(result);
            }
        });
    }

    fn reveal(&self, panel: &StackPanel, node: NodeId, label: &str) {
        self.add(panel, SOURCE, label, move |workspace| {
            workspace.reveal_source(node);
            Ok(())
        });
    }
}

fn edit_trigger(panel: &StackPanel, scope: &BehaviorScope, trigger: &BehaviorTrigger) {
    let id = scope.id;
    match trigger {
        BehaviorTrigger::Event { event, source } => {
            let fields = expander(panel, "Edit trigger");
            let event = field(&fields, "Event", event);
            let source = field(&fields, "Source", source.as_deref().unwrap_or(""));
            scope.add(
                &buttons(&fields),
                icons::CHECK,
                "Apply trigger",
                move |workspace| {
                    workspace.session.borrow_mut().edit_behavior(
                        id,
                        BehaviorEdit::SetTrigger(BehaviorTrigger::Event {
                            event: text(&event),
                            source: optional(&source),
                        }),
                    )
                },
            );
        }
        BehaviorTrigger::Data {
            binding,
            comparison,
            value,
            revert_on_false,
        } => {
            let fields = expander(panel, "Edit trigger");
            let binding = field(&fields, "Binding", binding);
            let comparison = choice_field(
                &fields,
                None,
                "Comparison",
                comparison,
                &[
                    "Equal",
                    "NotEqual",
                    "LessThan",
                    "LessThanOrEqual",
                    "GreaterThan",
                    "GreaterThanOrEqual",
                ],
            );
            let value = field(&fields, "Value", value);
            let revert = choice_field(
                &fields,
                None,
                "Revert on false",
                if *revert_on_false { "Yes" } else { "No" },
                &["No", "Yes"],
            );
            scope.add(
                &buttons(&fields),
                icons::CHECK,
                "Apply trigger",
                move |workspace| {
                    workspace.session.borrow_mut().edit_behavior(
                        id,
                        BehaviorEdit::SetTrigger(BehaviorTrigger::Data {
                            binding: text(&binding),
                            comparison: choice(&comparison),
                            value: text(&value),
                            revert_on_false: choice(&revert) == "Yes",
                        }),
                    )
                },
            );
        }
        BehaviorTrigger::Custom { .. } => {}
    }
}

fn add_picker<T>(
    panel: &StackPanel,
    workspace: &Rc<DesignerWorkspace>,
    label: &str,
    rows: &[T],
    index: usize,
    describe: impl Fn(&T) -> (NodeId, String),
    select: impl Fn(&mut AuthoringUi, NodeId) + 'static,
) {
    let (ids, labels): (Vec<_>, Vec<_>) = rows.iter().map(describe).unzip();
    let control = picker(label, labels.iter().map(String::as_str), index);
    let weak_control = control.downgrade();
    let weak = Rc::downgrade(workspace);
    control.selection_changed(move |_, _| {
        if let (Some(workspace), Some(control)) = (weak.upgrade(), weak_control.upgrade()) {
            if let Some(&node) = ids.get(control.selected_index() as usize) {
                select(&mut workspace.authoring.borrow_mut(), node);
                workspace.rebuild_authoring();
            }
        }
    });
    panel.children().add(control.upcast::<Control>());
}

fn buttons(panel: &StackPanel) -> Ref<StackPanel> {
    let row = StackPanel::new();
    row.set_orientation(Orientation::Horizontal);
    row.set_spacing(3.0);
    panel.children().add(row.clone().upcast::<Control>());
    row
}

fn expander(panel: &StackPanel, heading: &str) -> Ref<StackPanel> {
    let fields = StackPanel::new();
    fields.set_margin(Thickness::new(0.0, 5.0, 0.0, 5.0));
    fields.set_spacing(4.0);
    let expander = Expander::new();
    expander.set_header(Some(Rc::new(heading.to_owned())));
    expander.set_content(Some(Control::boxed(fields.clone())));
    expander.set_horizontal_alignment(HorizontalAlignment::Stretch);
    expander.set_horizontal_content_alignment(HorizontalAlignment::Stretch);
    panel.children().add(expander.upcast::<Control>());
    fields
}

fn trigger_label(trigger: &BehaviorTrigger) -> String {
    match trigger {
        BehaviorTrigger::Event { event, .. } => format!("On {event}"),
        BehaviorTrigger::Data { binding, .. } => format!("When {binding}"),
        BehaviorTrigger::Custom { type_name, .. } => type_name.clone(),
    }
}

fn action_label(action: &BehaviorAction) -> String {
    match action {
        BehaviorAction::ChangeProperty { property, .. } => format!("Change {property}"),
        BehaviorAction::InvokeCommand { command, .. } => format!("Invoke {command}"),
        BehaviorAction::CallMethod { method, .. } => format!("Call {method}"),
        BehaviorAction::AddClass { class, .. } => format!("Add class {class}"),
        BehaviorAction::RemoveClass { class, .. } => format!("Remove class {class}"),
        BehaviorAction::Custom { type_name, .. } => type_name.clone(),
        BehaviorAction::Markup { .. } => "XAML".into(),
    }
}
