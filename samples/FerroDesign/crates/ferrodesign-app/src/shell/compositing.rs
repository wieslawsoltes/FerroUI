//! Native layer masks and group compositing, hosted by the XAML inspector.
use super::*;
use ferrodesign_core::{CompositingEdit, MaskType};

const MASK_TYPES: [MaskType; 3] = [MaskType::Alpha, MaskType::Vector, MaskType::Luminance];
const MASK_LABELS: [&str; 3] = ["Alpha", "Vector", "Luminance"];

struct UpdatingCompositing<'a>(&'a Cell<bool>, bool);
impl Drop for UpdatingCompositing<'_> {
    fn drop(&mut self) {
        self.0.set(self.1);
    }
}

impl EditorShell {
    pub(super) fn compositing_dialog(self: &Rc<Self>) {
        let snapshot = {
            let editor = self.editor.borrow();
            let editable: Vec<_> = editor
                .selection
                .iter()
                .copied()
                .filter(|id| {
                    editor.document.nodes.contains_key(id) && !editor.document.effective_locked(*id)
                })
                .collect();
            editable
                .first()
                .or(editor.selection.first())
                .and_then(|id| editor.document.nodes.get(id))
                .map(|node| {
                    let mixed = editable.iter().any(|id| {
                        let other = &editor.document.nodes[id];
                        (other.is_mask, other.mask_type, other.blend_mode)
                            != (node.is_mask, node.mask_type, node.blend_mode)
                    });
                    (
                        node.name.clone(),
                        node.is_mask,
                        node.mask_type,
                        node.blend_mode,
                        editor.selection.clone(),
                        editable.len(),
                        mixed,
                    )
                })
        };
        let Some((name, is_mask, mask_type, blend, selection, editable, mixed)) = snapshot else {
            self.close_modal();
            self.toast("Select a layer to edit masks and compositing.");
            return;
        };
        let _updating = UpdatingCompositing(&self.updating, self.updating.replace(true));
        self.modal(
            "Masks and compositing",
            &format!(
                "{name} · Changes apply to {editable} unlocked selected layers and support undo."
            ),
        );
        let content = self.control::<StackPanel>("ModalContent");
        content.set_spacing(12.0);
        let error = compositing_text("");
        error.set_name(Some("CompositingError".into()));
        error.set_foreground(Some(icons::brush("#C8372B")));
        error.set_is_visible(false);
        content.children().add(error.clone());
        if mixed {
            content.children().add(compositing_text("Mixed values · showing the first unlocked layer. Each control changes only its own property."));
        }
        let enabled = CheckBox::new();
        enabled.set_name(Some("CompositingMaskEnabled".into()));
        enabled.set_content(Some(Rc::new("Use as mask".to_string())));
        enabled.set_is_checked(Some(is_mask));
        enabled.set_is_enabled(editable != 0);
        let weak_shell = Rc::downgrade(self);
        let weak_enabled = enabled.downgrade();
        let ids = selection.clone();
        let weak_error = error.downgrade();
        enabled.is_checked_changed(move |_, _| {
            if let (Some(shell), Some(enabled)) = (weak_shell.upgrade(), weak_enabled.upgrade()) {
                shell.compositing_action(
                    &ids,
                    CompositingEdit::MaskEnabled(enabled.is_checked() == Some(true)),
                    &weak_error,
                );
            }
        });
        content.children().add(enabled);
        content.children().add(compositing_text("A mask controls layers above it in the same parent. Its source stays independently editable. Create a mask group to contain the affected layers."));

        content.children().add(compositing_text("Mask type"));
        let choice = ComboBox::new();
        choice.set_name(Some("CompositingMaskType".into()));
        choice.set_is_enabled(editable != 0);
        for label in MASK_LABELS {
            choice.items().add(Some(Rc::new(label.to_string())));
        }
        choice.set_selected_index(
            MASK_TYPES
                .iter()
                .position(|kind| *kind == mask_type)
                .unwrap_or_default() as i32,
        );
        let weak_shell = Rc::downgrade(self);
        let weak_choice = choice.downgrade();
        let ids = selection.clone();
        let weak_error = error.downgrade();
        choice.selection_changed(move |_, _| {
            let (Some(shell), Some(choice)) = (weak_shell.upgrade(), weak_choice.upgrade()) else {
                return;
            };
            let Some(kind) = usize::try_from(choice.selected_index())
                .ok()
                .and_then(|index| MASK_TYPES.get(index))
            else {
                return;
            };
            shell.compositing_action(&ids, CompositingEdit::MaskType(*kind), &weak_error);
        });
        content.children().add(choice);
        content.children().add(compositing_text("Alpha uses transparency, including strokes and effects. Vector uses visible fill and stroke regions. Luminance uses brightness and transparency; white reveals and black hides."));

        content.children().add(compositing_text("Layer blend"));
        let blend_choice = ComboBox::new();
        blend_choice.set_name(Some("CompositingBlend".into()));
        blend_choice.set_is_enabled(editable != 0);
        for (label, _) in BLEND_OPTIONS {
            blend_choice.items().add(Some(Rc::new(label.to_string())));
        }
        blend_choice.set_selected_index(
            BLEND_OPTIONS
                .iter()
                .position(|(_, mode)| *mode == blend)
                .unwrap_or_default() as i32,
        );
        let weak_shell = Rc::downgrade(self);
        let weak_choice = blend_choice.downgrade();
        let weak_error = error.downgrade();
        blend_choice.selection_changed(move |_, _| {
            let (Some(shell), Some(choice)) = (weak_shell.upgrade(), weak_choice.upgrade()) else {
                return;
            };
            let Some((_, mode)) = usize::try_from(choice.selected_index())
                .ok()
                .and_then(|index| BLEND_OPTIONS.get(index))
            else {
                return;
            };
            shell.compositing_action(&selection, CompositingEdit::BlendMode(*mode), &weak_error);
        });
        content.children().add(blend_choice);
        content.children().add(compositing_text("Pass through lets child blends interact with artwork outside the group. Normal combines the group's contents before blending. Group opacity applies to the completed result once."));

        let actions = self.control::<StackPanel>("ModalActions");
        let group = Button::new();
        group.set_name(Some("CompositingCreateGroup".into()));
        group.set_content(Some(Rc::new("Create mask group".to_string())));
        group.set_is_enabled(editable >= 2);
        let weak_shell = Rc::downgrade(self);
        group.click(move |_, _| {
            if let Some(shell) = weak_shell.upgrade() {
                shell.group_mask_selection();
            }
        });
        actions.children().add(group);
        let done = Button::new();
        done.set_name(Some("CompositingDone".into()));
        done.set_content(Some(Rc::new("Done".to_string())));
        done.classes().add("primary");
        let weak_shell = Rc::downgrade(self);
        done.click(move |_, _| {
            if let Some(shell) = weak_shell.upgrade() {
                shell.close_modal();
            }
        });
        actions.children().add(done);
    }

    fn compositing_action(
        self: &Rc<Self>,
        selection: &[NodeId],
        edit: CompositingEdit,
        error: &WeakRef<TextBlock>,
    ) {
        if self.updating.get()
            || !self.control::<Control>("ModalBackdrop").is_visible()
            || self.control::<TextBlock>("ModalTitle").text().as_deref()
                != Some("Masks and compositing")
        {
            return;
        }
        if self.editor.borrow().selection != selection {
            self.compositing_dialog();
            self.toast("Selection changed. Edit the refreshed compositing controls.");
            return;
        }
        let result = self.editor.borrow_mut().edit_compositing(selection, edit);
        match result {
            Ok(changed) => {
                if let Some(error) = error.upgrade() {
                    error.set_is_visible(false);
                }
                if changed != 0 {
                    self.changed();
                }
            }
            Err(message) => {
                if let Some(error) = error.upgrade() {
                    error.set_text(Some(&message.to_string()));
                    error.set_is_visible(true);
                }
                self.toast(&message.to_string());
            }
        }
    }

    pub(super) fn mask_selection(self: &Rc<Self>, enabled: bool) {
        let selection = self.editor.borrow().selection.clone();
        if selection.is_empty() {
            self.toast("Select a mask source layer.");
            return;
        }
        let result = self
            .editor
            .borrow_mut()
            .edit_compositing(&selection, CompositingEdit::MaskEnabled(enabled));
        match result {
            Ok(changed) => {
                if changed != 0 {
                    self.changed();
                }
                self.compositing_dialog();
            }
            Err(error) => self.toast(&error.to_string()),
        }
    }

    pub(super) fn group_mask_selection(self: &Rc<Self>) {
        let result = self.editor.borrow_mut().create_mask_group();
        match result {
            Ok(_) => {
                self.changed();
                self.close_modal();
            }
            Err(error) => self.toast(&error.to_string()),
        }
    }
}

fn compositing_text(value: &str) -> Ref<TextBlock> {
    let text = TextBlock::new();
    text.set_text(Some(value));
    text.set_font_size(12.0);
    text.set_foreground(Some(icons::brush("#505050")));
    text.set_text_wrapping(TextWrapping::Wrap);
    text
}
