//! Native layer commands share one presentation-state path with source rows and hosts.
use super::*;
use crate::{LayerCommand, LayerEditSnapshot};
use ferroui_controls::{MenuItem, ToolTip};

impl DesignerWorkspace {
    /// Runs a command against the current authored selection. Presentation commands do not
    /// flush a pending source buffer or create document/undo/collaboration operations.
    pub fn edit_layers(self: &Rc<Self>, command: LayerCommand) -> Result<(), String> {
        let result = self.session.borrow_mut().apply_layer_command(command);
        if result.is_ok() {
            self.runtime_selection.set(None);
            self.surface.invalidate_layout_helpers();
            self.refresh(true);
        }
        result
    }

    /// Source rows retain guarded identities rather than reusing local NodeIds after a tab,
    /// source, scope or layer-view change. An obsolete row cannot hide or lock a new object.
    pub fn edit_layer_snapshot(
        self: &Rc<Self>,
        snapshot: &LayerEditSnapshot,
        command: LayerCommand,
    ) -> Result<(), String> {
        let result = self
            .session
            .borrow_mut()
            .apply_layer_edit(snapshot, command);
        if result.is_ok() {
            self.surface.invalidate_layout_helpers();
            self.refresh(true);
        }
        result
    }

    pub(super) fn connect_layer_commands(self: &Rc<Self>) {
        for (name, command) in layer_commands() {
            let weak = Rc::downgrade(self);
            self.root.get_control::<MenuItem>(name).click(move |_, _| {
                if let Some(workspace) = weak.upgrade() {
                    workspace.report(workspace.edit_layers(command));
                }
            });
        }
        let weak = Rc::downgrade(self);
        self.root
            .get_control::<Button>("LayerIsolationExit")
            .click(move |_, _| {
                if let Some(workspace) = weak.upgrade() {
                    workspace.report(workspace.edit_layers(LayerCommand::ExitIsolation));
                }
            });
        let weak = Rc::downgrade(self);
        self.root
            .add_handler(InputElement::key_down_event(), move |_, event| {
                let Some(workspace) = weak.upgrade() else {
                    return;
                };
                if workspace.text_input_focused()
                    || !event.key_modifiers.contains(KeyModifiers::SHIFT)
                    || !event
                        .key_modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::META)
                    || event.key_modifiers.contains(KeyModifiers::ALT)
                {
                    return;
                }
                let command = match event.key {
                    Key::L => Some(LayerCommand::ToggleLock),
                    Key::H => Some(LayerCommand::ToggleHidden),
                    _ => None,
                };
                if let Some(command) = command {
                    workspace.report(workspace.edit_layers(command));
                    event.set_handled(true);
                }
            });
        ToolTip::set_tip(
            &self.root.get_control::<Button>("LayerIsolationExit"),
            Some(Rc::new(
                "Return to the full canvas. Escape also exits isolation.".to_owned(),
            )),
        );
    }

    pub(super) fn refresh_layer_commands(&self) {
        let session = self.session.borrow();
        let selected = session.selection();
        let valid = session.document().is_valid();
        let authored = valid
            && !selected.is_empty()
            && selected.iter().all(|node| {
                session
                    .document()
                    .ast()
                    .node(*node)
                    .is_some_and(|node| node.kind == ferroui_designer_core::NodeKind::Element)
            });
        for (name, command) in layer_commands() {
            let enabled = match command {
                LayerCommand::ShowAll | LayerCommand::UnlockAll => true,
                LayerCommand::ExitIsolation => !session.layer_isolation().is_empty(),
                LayerCommand::Isolate => {
                    authored
                        && selected
                            .iter()
                            .all(|node| !session.layer_state(*node).is_hidden())
                }
                _ => authored,
            };
            self.root
                .get_control::<MenuItem>(name)
                .set_is_enabled(enabled);
        }
        for (name, on, off, active) in [
            (
                "LayerToggleLock",
                "Unlock selected layers",
                "Lock selected layers",
                authored
                    && selected
                        .iter()
                        .all(|node| session.layer_state(*node).locked),
            ),
            (
                "LayerToggleHidden",
                "Show selected layers",
                "Hide selected layers in designer",
                authored
                    && selected
                        .iter()
                        .all(|node| session.layer_state(*node).hidden),
            ),
        ] {
            self.root
                .get_control::<MenuItem>(name)
                .set_header(Some(Rc::new(if active { on } else { off }.to_owned())));
        }
        let count = session.layer_isolation().len();
        self.root
            .get_control::<Control>("LayerIsolationBanner")
            .set_is_visible(count > 0);
        self.root
            .get_control::<TextBlock>("LayerIsolationText")
            .set_text(Some(&format!(
                "Isolating {count} {} · surrounding layout is preserved",
                if count == 1 { "layer" } else { "layers" }
            )));
    }
}

fn layer_commands() -> [(&'static str, LayerCommand); 10] {
    [
        ("LayerToggleLock", LayerCommand::ToggleLock),
        ("LayerToggleHidden", LayerCommand::ToggleHidden),
        ("LayerIsolate", LayerCommand::Isolate),
        ("LayerExitIsolation", LayerCommand::ExitIsolation),
        ("LayerUnlockAll", LayerCommand::UnlockAll),
        ("LayerShowAll", LayerCommand::ShowAll),
        ("LayerSelectParent", LayerCommand::SelectParent),
        ("LayerSelectChildren", LayerCommand::SelectChildren),
        ("LayerSelectNext", LayerCommand::SelectNextSibling),
        ("LayerSelectPrevious", LayerCommand::SelectPreviousSibling),
    ]
}
