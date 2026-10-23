//! Four native drafts share one validated padding commit and one discard.
use super::*;

const NAMES: [&str; 4] = [
    "LayoutPaddingTop",
    "LayoutPaddingRight",
    "LayoutPaddingBottom",
    "LayoutPaddingLeft",
];
const LABELS: [&str; 4] = ["Top", "Right", "Bottom", "Left"];

fn values(padding: core::Padding) -> [f64; 4] {
    [padding.top, padding.right, padding.bottom, padding.left]
}
fn padding(values: [f64; 4]) -> core::Padding {
    core::Padding {
        top: values[0],
        right: values[1],
        bottom: values[2],
        left: values[3],
    }
}

impl EditorShell {
    pub(super) fn layout_padding_controls(
        self: &Rc<Self>,
        snapshot: &LayoutSnapshot,
        error: &Ref<TextBlock>,
        expected: Rc<RefCell<LayoutEditGuard>>,
    ) {
        let host = self.control::<StackPanel>("ModalContent");
        host.children().add(layout_text("Frame padding", false));
        host.children().add(layout_text("Unlinked sides keep their own values. Linking changes no padding until the next field edit; that value then applies to all four sides. Enter, Tab, Apply, or Save validates all sides together. Escape discards the padding drafts.", true));
        let initial = values(snapshot.layout.padding);
        let linked = Rc::new(Cell::new(initial.iter().all(|value| *value == initial[0])));
        let link = CheckBox::new();
        link.set_name(Some("LayoutPaddingLinked".into()));
        link.set_content(Some(Rc::new(
            "Link all four sides for the next edit".to_string(),
        )));
        link.set_is_checked(Some(linked.get()));
        host.children().add(link.clone());

        let fields: [Ref<TextBox>; 4] =
            std::array::from_fn(|index| layout_input(NAMES[index], &number(initial[index])));
        host.children()
            .add(layout_pair("Top", &fields[0], "Right", &fields[1]));
        host.children()
            .add(layout_pair("Bottom", &fields[2], "Left", &fields[3]));
        let weak_fields = fields.each_ref().map(|field| field.downgrade());
        let committed: [Rc<RefCell<String>>; 4] = fields
            .each_ref()
            .map(|field| Rc::new(RefCell::new(field.text().unwrap_or_default())));
        // Unedited sides retain their exact numbers even when the inspector's
        // two-decimal presentation is shorter than the stored value.
        let accepted = Rc::new(Cell::new(initial));
        let last_edited = Rc::new(Cell::new(0usize));
        for (index, field) in fields.iter().enumerate() {
            let weak_shell = Rc::downgrade(self);
            let weak_field = field.downgrade();
            let last = last_edited.clone();
            field.text_changing(move |_, _| {
                if let (Some(shell), Some(field)) = (weak_shell.upgrade(), weak_field.upgrade()) {
                    if !shell.updating.get() && !field.is_composing_text() {
                        last.set(index);
                    }
                }
            });
        }
        let weak_shell = Rc::downgrade(self);
        let weak_error = error.downgrade();
        let previous = committed.clone();
        let inputs = weak_fields.clone();
        let link_mode = linked.clone();
        let parent = snapshot.parent;
        let commit: Rc<dyn Fn()> = Rc::new(move || {
            let Some(shell) = weak_shell.upgrade() else {
                return;
            };
            let Some(fields) = inputs
                .iter()
                .map(WeakRef::upgrade)
                .collect::<Option<Vec<_>>>()
            else {
                return;
            };
            if shell.updating.get()
                || fields.iter().any(|field| {
                    !field.is_attached_to_visual_tree()
                        || !field.is_effectively_visible()
                        || field.is_composing_text()
                })
            {
                return;
            }
            let text: [String; 4] =
                std::array::from_fn(|index| fields[index].text().unwrap_or_default());
            let dirty: [bool; 4] =
                std::array::from_fn(|index| text[index] != *previous[index].borrow());
            if !dirty.iter().any(|dirty| *dirty) {
                return;
            }
            let result = (|| {
                let mut parsed = accepted.get();
                for index in 0..4 {
                    let value = layout_number(&text[index], &format!("{} padding", LABELS[index]))
                        .map_err(core::DocumentError::Invalid)?;
                    if dirty[index] {
                        parsed[index] = value;
                    }
                }
                if link_mode.get() {
                    let source = if dirty[last_edited.get()] {
                        last_edited.get()
                    } else {
                        dirty.iter().position(|dirty| *dirty).unwrap()
                    };
                    parsed = [parsed[source]; 4];
                }
                let mut editor = shell.editor.borrow_mut();
                if !expected.borrow().matches(&editor) {
                    return Err(core::DocumentError::Invalid(
                        "The frame changed. Reopen layout authoring before applying this padding."
                            .into(),
                    ));
                }
                let revision = editor.revision;
                let result = editor.set_layout_padding(parent, padding(parsed))?;
                expected.borrow_mut().accept(&editor);
                Ok((result, editor.revision != revision, parsed))
            })();
            match result {
                Ok((result, changed, parsed)) => {
                    accepted.set(parsed);
                    {
                        let _updating =
                            UpdatingLayout(&shell.updating, shell.updating.replace(true));
                        for index in 0..4 {
                            let text = number(parsed[index]);
                            *previous[index].borrow_mut() = text.clone();
                            fields[index].set_text(Some(&text));
                        }
                    }
                    if let Some(error) = weak_error.upgrade() {
                        error.set_is_visible(false);
                    }
                    if changed {
                        shell.changed();
                    }
                    if !result.warnings.is_empty() {
                        shell.toast(&result.warnings.join("\n"));
                    }
                }
                Err(error) => layout_show_error(&weak_error, &error.to_string()),
            }
            shell.sync_document_draft_dirty();
        });

        let weak_shell = Rc::downgrade(self);
        let inputs = weak_fields.clone();
        let previous = committed.clone();
        let discard: Rc<dyn Fn()> = Rc::new(move || {
            let Some(shell) = weak_shell.upgrade() else { return; };
            let _updating = UpdatingLayout(&shell.updating, shell.updating.replace(true));
            // Read accepted strings before raising any text notification.
            let values = previous.each_ref().map(|value| value.borrow().clone());
            for (field, value) in inputs.iter().zip(values.iter()) {
                if let Some(field) = field.upgrade() {
                    field.set_text(Some(value));
                    field.set_border_brush(Some(icons::brush("#E5E5E5")));
                }
            }
        });
        for (field, value) in fields.iter().zip(committed.iter()) {
            let blur = commit.clone();
            field.add_handler(InputElement::lost_focus_event(), move |_, _| blur());
            let enter = commit.clone();
            let weak_field = field.downgrade();
            field.add_handler(InputElement::key_down_event(), move |_, event| {
                if event.key == Key::Enter
                    && weak_field
                        .upgrade()
                        .is_some_and(|field| !field.is_composing_text())
                {
                    enter();
                    event.set_handled(true);
                }
            });
            self.bind_inspector_draft_with_discard(field, value.clone(), commit.clone(),
                Some(error), Some(discard.clone()));
        }
        let weak_shell = Rc::downgrade(self);
        let weak_link = link.downgrade();
        link.is_checked_changed(move |_, _| {
            let (Some(shell), Some(link)) = (weak_shell.upgrade(), weak_link.upgrade()) else {
                return;
            };
            if shell.updating.get() {
                return;
            }
            let requested = link.is_checked() == Some(true);
            if shell.flush_inspector_drafts() {
                linked.set(requested);
            } else {
                let _updating = UpdatingLayout(&shell.updating, shell.updating.replace(true));
                link.set_is_checked(Some(linked.get()));
            }
        });
        host.children().add(self.layout_button(
            "Apply padding",
            "LayoutApplyPadding",
            true,
            move |_| commit(),
        ));
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "layout_padding_tests.rs"]
mod tests;
