#[path = "rust_states/routes.rs"]
mod routes;

use super::*;
use ferroui_designer_core::Ast;
use std::sync::Arc;

const REVEAL: &str = "F1 M5,3 L0,8 5,13 6,12 2,8 6,4 Z M11,3 L16,8 11,13 10,12 14,8 10,4 Z";
const APPLY: &str = "F1 M2,8 L4,6 7,9 13,3 15,5 7,13 Z";
const SEEK: &str = "F1 M3,2 L12,8 3,14 Z M13,2 L15,2 15,14 13,14 Z";
const CLEAR: &str =
    "F1 M3,6 C4,2 10,1 13,4 L11,6 C8,3 5,5 5,8 C5,11 8,13 11,11 L13,13 C9,16 3,14 2,10 L0,10 3,6 Z";

#[derive(Clone)]
struct AcceptedSource {
    document: String,
    revision: u64,
    syntax: Arc<Ast>,
}

impl AcceptedSource {
    fn new(session: &DesignerSession) -> Self {
        Self {
            document: session.document_uri().to_owned(),
            revision: session.document().revision(),
            syntax: session.document().ast_snapshot(),
        }
    }

    fn check(&self, workspace: &DesignerWorkspace) -> Result<(), String> {
        let session = workspace.session.borrow();
        if workspace.pending_source.get()
            || !session.authored_is_valid()
            || session.document_uri() != self.document
            || session.document().revision() != self.revision
            || !std::ptr::eq(session.document().ast(), self.syntax.as_ref())
        {
            return Err("The Rust source changed. Reopen its state editor.".into());
        }
        Ok(())
    }
}

impl DesignerWorkspace {
    pub(super) fn build_rust_states(self: &Rc<Self>, panel: &StackPanel, owner: NodeId) {
        let session = self.session.borrow();
        let groups = session
            .state_groups()
            .into_iter()
            .filter(|group| group.owner == owner)
            .collect::<Vec<_>>();
        let accepted = AcceptedSource::new(&session);
        drop(session);
        let active_name = self.authoring.borrow().group.clone();
        let Some(group) = groups
            .iter()
            .find(|group| group.name == active_name)
            .or_else(|| groups.first())
            .cloned()
        else {
            description(panel, "Select the owner of a mapped Rust state group.");
            return;
        };
        if groups.len() > 1 {
            for candidate in groups {
                let name = candidate.name;
                action(panel, &name.clone(), self, move |this| {
                    this.authoring.borrow_mut().group = name.clone();
                    Ok(())
                });
            }
        }
        section(panel, &group.name);
        let group = Rc::new(group);
        let group_tools = toolbar(panel);
        let key = group.name.clone();
        let revision = accepted.revision;
        icon_action(
            &group_tools,
            "Reveal Rust state group",
            REVEAL,
            self,
            accepted.clone(),
            move |this| this.reveal_rust_state_source(revision, owner, &key, None, None),
        );
        icon_action(
            &group_tools,
            "Clear state preview",
            CLEAR,
            self,
            accepted.clone(),
            |this| {
                this.session.borrow_mut().clear_interaction_preview();
                Ok(())
            },
        );
        self.build_rust_state_values(panel, &group, &accepted);
        self.build_rust_state_routes(panel, group, accepted);
    }

    fn build_rust_state_values(
        self: &Rc<Self>,
        panel: &StackPanel,
        group: &Rc<VisualStateGroup>,
        accepted: &AcceptedSource,
    ) {
        section(panel, "States");
        for state in &group.states {
            let name = state.name.clone();
            let group = group.clone();
            let accepted = accepted.clone();
            action(panel, &name.clone(), self, move |this| {
                accepted.check(this)?;
                this.authoring.borrow_mut().state = name.clone();
                this.session
                    .borrow_mut()
                    .preview_state(group.owner, &group.name, Some(&name))
            });
        }
        let current = self.authoring.borrow().state.clone();
        let Some(index) = group.states.iter().position(|state| state.name == current) else {
            return;
        };
        let state = &group.states[index];
        if state.setters.is_empty() {
            return;
        }
        let inputs = state
            .setters
            .iter()
            .map(|setter| {
                let name = if setter.target.is_empty() {
                    setter.property.clone()
                } else {
                    format!("{}.{}", setter.target, setter.property)
                };
                field(panel, &name, &setter.value)
            })
            .collect::<Vec<_>>();
        let group = group.clone();
        icon_action(
            &toolbar(panel),
            "Apply state values",
            APPLY,
            self,
            accepted.clone(),
            move |this| {
                let mut model = (*group).clone();
                for (setter, input) in model.states[index].setters.iter_mut().zip(&inputs) {
                    setter.value = text(input);
                }
                this.session.borrow_mut().upsert_state_group(model)
            },
        );
    }

    pub(crate) fn reveal_rust_state_source(
        &self,
        revision: u64,
        owner: NodeId,
        group: &str,
        from: Option<&str>,
        to: Option<&str>,
    ) -> Result<(), String> {
        let span = {
            let session = self.session.borrow();
            if self.pending_source.get() || session.document().revision() != revision {
                return Err("Read the current Rust state source before revealing it".into());
            }
            let source = session
                .rust_state_group(owner, group)
                .ok_or("Choose a mapped Rust state group")?;
            if let Some(to) = to {
                source
                    .transition_span(from, to)
                    .ok_or("Choose a mapped Rust state route")?
            } else {
                source.declaration
            }
        };
        self.select_source_span(span);
        Ok(())
    }
}

fn toolbar(panel: &StackPanel) -> Ref<StackPanel> {
    let row = StackPanel::new();
    row.set_orientation(Orientation::Horizontal);
    row.set_spacing(4.0);
    panel.children().add(row.clone().upcast::<Control>());
    row
}

fn icon_action(
    row: &StackPanel,
    title: &str,
    path: &str,
    workspace: &Rc<DesignerWorkspace>,
    accepted: AcceptedSource,
    run: impl Fn(&Rc<DesignerWorkspace>) -> Result<(), String> + 'static,
) {
    let button = Button::new();
    button.classes().add("icon");
    button.set_width(27.0);
    button.set_height(26.0);
    AutomationProperties::set_name(&button, Some(title));
    ToolTip::set_tip(&button, Some(Rc::new(title.to_owned())));
    let icon = crate::icons::vector_icon(path, 14.0);
    icon.classes().add("designer-icon");
    button.set_content(Some(Control::boxed(&icon)));
    let weak = Rc::downgrade(workspace);
    button.click(move |_, event| {
        if let Some(this) = weak.upgrade() {
            let revision = this.session.borrow().document().revision();
            let result = accepted.check(&this).and_then(|()| run(&this));
            let authored = this.session.borrow().document().revision() != revision;
            this.refresh(authored);
            this.report(result);
            event.set_handled(true);
        }
    });
    row.children().add(button.upcast::<Control>());
}
