use super::*;
use ferrodesign_core::{ParagraphStylePatch, TextAlign};

impl EditorShell {
    pub(super) fn paragraph_dialog(self: &Rc<Self>, id: NodeId, start: i32, end: i32) {
        let node = self.editor.borrow().document.nodes.get(&id).filter(|node| node.kind == NodeKind::Text).cloned();
        let Some(node) = node else { self.toast("Select a text layer to style paragraphs."); return; };
        let _updating = UpdatingRichText(&self.updating, self.updating.replace(true));
        self.modal("Paragraphs", "Select text or place the caret in a paragraph. Apply changes whole paragraphs and creates one undo step.");
        let host = self.control::<StackPanel>("ModalContent");
        host.set_spacing(10.0);
        let selection = TextBox::new();
        selection.set_name(Some("ParagraphSelection".into()));
        selection.set_text(Some(&node.text));
        typography::configure_text_box(&selection, &node);
        selection.set_accepts_return(true);
        selection.set_is_read_only(true);
        selection.set_clear_selection_on_lost_focus(false);
        selection.set_min_height(120.0);
        selection.set_max_height(220.0);
        selection.set_text_wrapping(TextWrapping::Wrap);
        host.children().add(selection.clone());
        let status = rich_label("", true);
        status.set_name(Some("ParagraphSelectionStatus".into()));
        host.children().add(status.clone());
        let weak_selection = selection.downgrade();
        let weak_status = status.downgrade();
        selection.property_changed(move |event| {
            if event.property() == TextBox::selection_start_property().as_property()
                || event.property() == TextBox::selection_end_property().as_property() {
                if let (Some(field), Some(status)) = (weak_selection.upgrade(), weak_status.upgrade()) {
                    paragraph_status(&field, &status);
                }
            }
        });

        let alignment = rich_choice("ParagraphAlignment", &["Keep", "Left", "Center", "Right", "Justify", "Inherit"], 0);
        let direction = rich_choice("ParagraphDirection", &["Keep", "Left to right", "Right to left", "Auto · per paragraph", "Inherit"], 0);
        let row = rich_grid("*,*");
        row.children().add(labeled_control("Alignment", alignment.clone()));
        let direction_label = labeled_control("Direction", direction.clone());
        Grid::set_column(&direction_label, 1);
        row.children().add(direction_label);
        host.children().add(row);
        let (line_row, line) = override_field("Line height ×", "ParagraphLineHeight", &node.line_height.to_string());
        let (letter_row, letter) = override_field("Letter spacing", "ParagraphLetterSpacing", &node.letter_spacing.to_string());
        let (before_row, before) = override_field("Space before", "ParagraphBefore", "0");
        let (after_row, after) = override_field("Space after", "ParagraphAfter", "0");
        let (indent_row, indent) = override_field("First-line indent", "ParagraphIndent", "0");
        for row in [line_row, letter_row, before_row, after_row, indent_row] { host.children().add(row); }
        host.children().add(rich_label("Spacing and indentation use document units. Indentation starts at the left edge for LTR paragraphs and the right edge for RTL paragraphs.", true));
        let error = rich_label("", false);
        error.set_name(Some("ParagraphError".into()));
        error.set_foreground(Some(icons::brush("#C8372B")));
        error.set_is_visible(false);
        host.children().add(error.clone());

        let source = Rc::new(node.text);
        let apply = Button::new();
        apply.set_name(Some("ParagraphApply".into()));
        apply.set_content(Some(Rc::new("Apply paragraphs".to_string())));
        apply.classes().add("primary");
        let weak_shell = Rc::downgrade(self);
        let weak_selection = selection.downgrade();
        let weak_error = error.downgrade();
        let source_apply = source.clone();
        apply.click(move |_, _| {
            let (Some(shell), Some(selection), Some(error)) = (weak_shell.upgrade(), weak_selection.upgrade(), weak_error.upgrade()) else { return; };
            let result = (|| {
                let patch = ParagraphStylePatch {
                    alignment: match alignment.selected_index() {
                        1 => TextStyleChange::Set(TextAlign::Left), 2 => TextStyleChange::Set(TextAlign::Center),
                        3 => TextStyleChange::Set(TextAlign::Right), 4 => TextStyleChange::Set(TextAlign::Justify),
                        5 => TextStyleChange::Inherit, _ => TextStyleChange::Keep,
                    },
                    direction: match direction.selected_index() {
                        1 => TextStyleChange::Set(TextDirection::LeftToRight), 2 => TextStyleChange::Set(TextDirection::RightToLeft),
                        3 => TextStyleChange::Set(TextDirection::Auto), 4 => TextStyleChange::Inherit, _ => TextStyleChange::Keep,
                    },
                    line_height: line.read(number)?, letter_spacing: letter.read(number)?,
                    space_before: before.read(number)?, space_after: after.read(number)?, indent: indent.read(number)?,
                };
                if patch.is_empty() { return Err("Choose Set or Inherit for at least one paragraph property.".into()); }
                shell.apply_paragraph_range(id, &source_apply, &selection, &patch)
            })();
            show_rich_result(&error, result);
        });
        let inherit = Button::new();
        inherit.set_name(Some("ParagraphInheritAll".into()));
        inherit.set_content(Some(Rc::new("Inherit paragraph styles".to_string())));
        let weak_shell = Rc::downgrade(self);
        let weak_selection = selection.downgrade();
        let weak_error = error.downgrade();
        inherit.click(move |_, _| {
            if let (Some(shell), Some(selection), Some(error)) = (weak_shell.upgrade(), weak_selection.upgrade(), weak_error.upgrade()) {
                show_rich_result(&error, shell.apply_paragraph_range(id, &source, &selection, &ParagraphStylePatch::inherit_all()));
            }
        });
        let done = Button::new();
        done.set_name(Some("ParagraphDone".into()));
        done.set_content(Some(Rc::new("Done".to_string())));
        let weak_shell = Rc::downgrade(self);
        done.click(move |_, _| { if let Some(shell) = weak_shell.upgrade() { shell.close_modal(); } });
        let actions = self.control::<StackPanel>("ModalActions");
        actions.children().add(inherit);
        actions.children().add(apply);
        actions.children().add(done);
        selection.set_selection_start(start);
        selection.set_selection_end(end);
        paragraph_status(&selection, &status);
        selection.focus();
    }

    fn apply_paragraph_range(self: &Rc<Self>, id: NodeId, original_text: &str, field: &TextBox,
        patch: &ParagraphStylePatch) -> Result<(), String> {
        if field.text().as_deref() != Some(original_text) { return Err("Reopen Paragraphs after changing the layer text.".into()); }
        let range = core::text_range_from_utf16(original_text, field.selection_start(), field.selection_end())
            .map_err(|error| error.to_string())?;
        let mut editor = self.editor.borrow_mut();
        if editor.selection.as_slice() != [id] || editor.document.nodes.get(&id).is_none_or(|node| node.text != original_text) {
            return Err("The text layer changed. Reopen Paragraphs to refresh the selection.".into());
        }
        let changed = editor.style_text_paragraphs(id, range, patch).map_err(|error| error.to_string())?;
        let node = editor.document.nodes[&id].clone();
        drop(editor);
        typography::configure_text_box(field, &node);
        if changed > 0 { self.changed(); }
        Ok(())
    }
}

fn number(value: &str) -> Result<f64, String> {
    value.trim().parse::<f64>().ok().filter(|value| value.is_finite())
        .ok_or_else(|| "Enter a finite number.".to_string())
}

fn paragraph_status(field: &TextBox, status: &TextBlock) {
    let text = field.text().unwrap_or_default();
    let message = match core::text_range_from_utf16(&text, field.selection_start(), field.selection_end()) {
        Ok(range) => {
            let paragraphs = core::text_paragraph_ranges(&text);
            let caret = paragraphs.partition_point(|paragraph| paragraph.start <= range.start).saturating_sub(1);
            let count = if range.is_empty() { 1 } else {
                paragraphs.iter().filter(|paragraph| paragraph.start < range.end && paragraph.end > range.start).count()
            };
            if range.is_empty() { format!("Paragraph {} · caret selection", caret + 1) }
            else { format!("{count} paragraphs selected · character styling remains independent") }
        }
        Err(error) => error.to_string(),
    };
    status.set_text(Some(&message));
}
