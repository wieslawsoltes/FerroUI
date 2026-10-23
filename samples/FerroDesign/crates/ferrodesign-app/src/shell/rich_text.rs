//! Native range selection, editable typography and the canvas TextBox share
//! FerroUI's public text-layout run styles, direction and font features.
use super::*;
use ferrodesign_core::{TextDirection, TextStyleChange, TextStylePatch, TextTypographyPatch};
use crate::renderer::typography;
use ferroui_base::interactivity::RoutingStrategies;
#[path = "rich_text/paragraphs.rs"]
mod paragraphs;

struct UpdatingRichText<'a>(&'a Cell<bool>, bool);
impl Drop for UpdatingRichText<'_> {
    fn drop(&mut self) { self.0.set(self.1); }
}

struct OverrideField {
    mode: Ref<ComboBox>,
    value: Ref<TextBox>,
}

impl OverrideField {
    fn read<T>(&self, parse: impl FnOnce(&str) -> Result<T, String>) -> Result<TextStyleChange<T>, String> {
        match self.mode.selected_index() {
            1 => parse(&self.value.text().unwrap_or_default()).map(TextStyleChange::Set),
            2 => Ok(TextStyleChange::Inherit),
            _ => Ok(TextStyleChange::Keep),
        }
    }
}

impl EditorShell {
    pub(super) fn rich_text_dialog(self: &Rc<Self>) {
        self.finish_inline_text(true);
        let node = {
            let editor = self.editor.borrow();
            if editor.selection.len() != 1 { None } else {
                editor.document.nodes.get(&editor.selection[0])
                    .filter(|node| node.kind == NodeKind::Text).cloned()
            }
        };
        let Some(node) = node else {
            self.toast("Select one text layer to style a character range.");
            return;
        };
        let _updating = UpdatingRichText(&self.updating, self.updating.replace(true));
        self.modal("Rich text", &format!(
            "{} · Select characters below, then set or inherit individual styles. Every Apply is undoable.", node.name
        ));
        let host = self.control::<StackPanel>("ModalContent");
        host.set_spacing(10.0);
        host.children().add(rich_label("Character selection", true));
        let selection = TextBox::new();
        selection.set_name(Some("RichTextSelection".into()));
        selection.set_text(Some(&node.text));
        typography::configure_text_box(&selection, &node);
        selection.set_is_read_only(true);
        selection.set_accepts_return(true);
        // FerroUI defaults this to true; range authoring needs the range to
        // survive moving focus from the selection surface to style controls.
        selection.set_clear_selection_on_lost_focus(false);
        selection.set_text_wrapping(TextWrapping::Wrap);
        selection.set_min_height(88.0);
        selection.set_max_height(150.0);
        host.children().add(selection.clone());
        let status = rich_label("", true);
        status.set_name(Some("RichTextSelectionStatus".into()));
        host.children().add(status.clone());
        let weak_selection = selection.downgrade();
        let weak_status = status.downgrade();
        selection.property_changed(move |event| {
            if event.property() == TextBox::selection_start_property().as_property()
                || event.property() == TextBox::selection_end_property().as_property()
            {
                if let (Some(field), Some(status)) = (weak_selection.upgrade(), weak_status.upgrade()) {
                    update_range_status(&field, &status);
                }
            }
        });
        let select_all = Button::new();
        select_all.set_name(Some("RichTextSelectAll".into()));
        select_all.set_content(Some(Rc::new("Select all characters".to_string())));
        let weak_selection = selection.downgrade();
        select_all.click(move |_, _| {
            if let Some(field) = weak_selection.upgrade() { field.select_all(); field.focus(); }
        });
        host.children().add(select_all);

        let preview = TextBox::new();
        preview.set_name(Some("RichTextPreview".into()));
        preview.set_is_read_only(true);
        preview.set_accepts_return(true);
        preview.set_focusable(false);
        preview.set_text_wrapping(TextWrapping::Wrap);
        preview.set_max_height(160.0);
        update_rich_preview(&preview, &node);
        host.children().add(rich_label("Style preview", true));
        host.children().add(preview.clone());
        self.rich_typography_controls(&host, &node, &selection, &preview);
        let paragraph_button = Button::new();
        paragraph_button.set_name(Some("RichTextParagraphs".into()));
        paragraph_button.set_content(Some(Rc::new("Style selected paragraphs…".to_string())));
        let weak_shell = Rc::downgrade(self);
        let weak_selection = selection.downgrade();
        let paragraph_target = node.id;
        paragraph_button.click(move |_, _| {
            if let (Some(shell), Some(selection)) = (weak_shell.upgrade(), weak_selection.upgrade()) {
                shell.paragraph_dialog(paragraph_target, selection.selection_start(), selection.selection_end());
            }
        });
        host.children().add(paragraph_button);

        let (family_row, family) = override_field("Font family", "RichTextFamily", &node.font_family);
        let (size_row, size) = override_field("Font size", "RichTextSize", &node.font_size.to_string());
        let (weight_row, weight) = override_field("Font weight", "RichTextWeight", &node.font_weight.to_string());
        let color = node.fills.iter().rev().find_map(|fill| match fill.paint {
            Paint::Solid(color) if fill.visible => Some(color), _ => None,
        }).unwrap_or(Color::BLACK);
        let (color_row, color) = override_field("Text color", "RichTextColor", &color.hex());
        for row in [family_row, size_row, weight_row, color_row] { host.children().add(row); }
        let italic = rich_choice("RichTextItalic", &["Keep current", "Regular", "Italic", "Inherit layer"], 0);
        let decoration = rich_choice("RichTextDecoration", &[
            "Keep current", "No decoration", "Underline", "Strikethrough", "Inherit layer",
        ], 0);
        let emphasis = rich_grid("*,*");
        emphasis.children().add(labeled_control("Font style", italic.clone()));
        let decoration_panel = labeled_control("Decoration", decoration.clone());
        Grid::set_column(&decoration_panel, 1);
        emphasis.children().add(decoration_panel);
        host.children().add(emphasis);

        let error = rich_label("", false);
        error.set_name(Some("RichTextError".into()));
        error.set_foreground(Some(icons::brush("#C8372B")));
        error.set_is_visible(false);
        host.children().add(error.clone());
        let source = Rc::new(node.text);
        let target = node.id;
        let apply = Button::new();
        apply.set_name(Some("RichTextApply".into()));
        apply.set_content(Some(Rc::new("Apply range style".to_string())));
        apply.classes().add("primary");
        let weak_shell = Rc::downgrade(self);
        let weak_selection = selection.downgrade();
        let weak_preview = preview.downgrade();
        let weak_error = error.downgrade();
        let source_apply = source.clone();
        apply.click(move |_, _| {
            let (Some(shell), Some(selection), Some(preview), Some(error)) = (
                weak_shell.upgrade(), weak_selection.upgrade(), weak_preview.upgrade(), weak_error.upgrade(),
            ) else { return; };
            let result = (|| {
                let patch = TextStylePatch {
                    font_family: family.read(|value| {
                        let family = value.trim();
                        if family.is_empty() { Err("Enter a font family.".into()) } else { Ok(family.to_string()) }
                    })?,
                    font_size: size.read(|value| positive_font_size(value))?,
                    font_weight: weight.read(|value| font_weight_value(value))?,
                    color: color.read(|value| Color::from_hex(value)
                        .ok_or_else(|| "Enter a hexadecimal color, such as #7B61FF or #7B61FF80.".to_string()))?,
                    italic: match italic.selected_index() {
                        1 => TextStyleChange::Set(false), 2 => TextStyleChange::Set(true),
                        3 => TextStyleChange::Inherit, _ => TextStyleChange::Keep,
                    },
                    decoration: match decoration.selected_index() {
                        1 => TextStyleChange::Set(core::TextDecoration::None),
                        2 => TextStyleChange::Set(core::TextDecoration::Underline),
                        3 => TextStyleChange::Set(core::TextDecoration::Strikethrough),
                        4 => TextStyleChange::Inherit, _ => TextStyleChange::Keep,
                    },
                };
                if patch.is_empty() { return Err("Choose Set or Inherit for at least one property.".into()); }
                shell.apply_rich_range(target, &source_apply, &selection, &preview, &patch)
            })();
            show_rich_result(&error, result);
        });
        let reset = Button::new();
        reset.set_name(Some("RichTextInheritAll".into()));
        reset.set_content(Some(Rc::new("Inherit all styles".to_string())));
        let weak_shell = Rc::downgrade(self);
        let weak_selection = selection.downgrade();
        let weak_preview = preview.downgrade();
        let weak_error = error.downgrade();
        reset.click(move |_, _| {
            if let (Some(shell), Some(selection), Some(preview), Some(error)) = (
                weak_shell.upgrade(), weak_selection.upgrade(), weak_preview.upgrade(), weak_error.upgrade(),
            ) {
                let result = shell.apply_rich_range(target, &source, &selection, &preview, &TextStylePatch::inherit_all());
                show_rich_result(&error, result);
            }
        });
        let done = Button::new();
        done.set_name(Some("RichTextDone".into()));
        done.set_content(Some(Rc::new("Done".to_string())));
        let weak_shell = Rc::downgrade(self);
        done.click(move |_, _| { if let Some(shell) = weak_shell.upgrade() { shell.close_modal(); } });
        let actions = self.control::<StackPanel>("ModalActions");
        actions.children().add(reset);
        actions.children().add(apply);
        actions.children().add(done);
        selection.select_all();
        update_range_status(&selection, &status);
        selection.focus();
    }

    fn apply_rich_range(
        self: &Rc<Self>, id: NodeId, original_text: &str, selection: &Ref<TextBox>,
        preview: &Ref<TextBox>, patch: &TextStylePatch,
    ) -> Result<(), String> {
        if selection.text().as_deref() != Some(original_text) {
            return Err("The selection surface changed. Reopen Rich text to refresh it.".into());
        }
        let range = core::text_range_from_utf16(original_text, selection.selection_start(), selection.selection_end())
            .map_err(|error| error.to_string())?;
        if range.is_empty() { return Err("Select one or more characters before applying a style.".into()); }
        let mut editor = self.editor.borrow_mut();
        if editor.selection.as_slice() != [id]
            || editor.document.nodes.get(&id).is_none_or(|node| node.text != original_text)
        {
            return Err("The layer or its text changed. Reopen Rich text to refresh the selection.".into());
        }
        let changed = editor.style_text_range(id, range, patch).map_err(|error| error.to_string())?;
        let node = editor.document.nodes[&id].clone();
        drop(editor);
        typography::configure_text_box(selection, &node);
        update_rich_preview(preview, &node);
        if changed { self.changed(); }
        Ok(())
    }

    fn rich_typography_controls(self: &Rc<Self>, host: &StackPanel, node: &Node,
        selection: &Ref<TextBox>, preview: &Ref<TextBox>) {
        host.children().add(rich_label("Default paragraph direction", true));
        let direction = rich_choice("RichTextDirection", &["Left to right", "Right to left", "Auto · first strong character"], match node.text_direction {
            Some(TextDirection::RightToLeft) => 1, Some(TextDirection::Auto) => 2, _ => 0,
        });
        host.children().add(direction.clone());
        host.children().add(rich_label("OpenType features · kern=1, liga=0, ss01=1", true));
        let row = rich_grid("*,72");
        let features = TextBox::new();
        features.set_name(Some("RichTextFeatures".into()));
        features.set_text(Some(&core::format_font_features(&node.font_features)));
        features.set_placeholder_text(Some("Font defaults"));
        row.children().add(features.clone());
        let apply = Button::new();
        apply.set_name(Some("RichTextFeaturesApply".into()));
        apply.set_content(Some(Rc::new("Apply".to_string())));
        Grid::set_column(&apply, 1);
        row.children().add(apply.clone());
        host.children().add(row);
        let error = rich_label("", false);
        error.set_name(Some("RichTextTypographyError".into()));
        error.set_foreground(Some(icons::brush("#C8372B")));
        error.set_is_visible(false);
        host.children().add(error.clone());

        let target = node.id;
        let weak_shell = Rc::downgrade(self);
        let weak_direction = direction.downgrade();
        let weak_selection = selection.downgrade();
        let weak_preview = preview.downgrade();
        let weak_error = error.downgrade();
        direction.selection_changed(move |_, _| {
            let (Some(shell), Some(direction), Some(selection), Some(preview), Some(error)) =
                (weak_shell.upgrade(), weak_direction.upgrade(), weak_selection.upgrade(), weak_preview.upgrade(), weak_error.upgrade()) else { return; };
            if shell.updating.get() { return; }
            let direction = match direction.selected_index() {
                1 => TextStyleChange::Set(TextDirection::RightToLeft),
                2 => TextStyleChange::Set(TextDirection::Auto),
                _ => TextStyleChange::Inherit,
            };
            show_rich_result(&error, shell.apply_rich_typography(target, &selection, &preview,
                &TextTypographyPatch { direction, ..Default::default() }));
        });

        let committed = Rc::new(RefCell::new(features.text().unwrap_or_default()));
        let weak_shell = Rc::downgrade(self);
        let weak_features = features.downgrade();
        let weak_selection = selection.downgrade();
        let weak_preview = preview.downgrade();
        let weak_error = error.downgrade();
        let committed_value = committed.clone();
        let commit: Rc<dyn Fn()> = Rc::new(move || {
            let (Some(shell), Some(field), Some(selection), Some(preview), Some(error)) =
                (weak_shell.upgrade(), weak_features.upgrade(), weak_selection.upgrade(), weak_preview.upgrade(), weak_error.upgrade()) else { return; };
            if shell.updating.get() || field.is_composing_text() { return; }
            let text = field.text().unwrap_or_default();
            if text == *committed_value.borrow() { return; }
            let result = (|| {
                let features = core::parse_font_features(&text).map_err(|error| error.to_string())?;
                let canonical = core::format_font_features(&features);
                shell.apply_rich_typography(target, &selection, &preview,
                    &TextTypographyPatch { font_features: TextStyleChange::Set(features), ..Default::default() })?;
                *committed_value.borrow_mut() = canonical.clone();
                field.set_text(Some(&canonical));
                Ok(())
            })();
            show_rich_result(&error, result);
        });
        let blur = commit.clone();
        features.lost_focus(move |_, _| blur());
        let enter = commit.clone();
        let weak_features = features.downgrade();
        features.add_handler_with(InputElement::key_down_event(), move |_, event| {
            if event.key == Key::Enter && weak_features.upgrade().is_some_and(|field| !field.is_composing_text()) {
                enter(); event.set_handled(true);
            }
        }, RoutingStrategies::TUNNEL, false);
        let clicked = commit.clone();
        apply.click(move |_, _| clicked());
        self.bind_inspector_draft(&features, committed, commit, Some(&error));
    }

    fn apply_rich_typography(self: &Rc<Self>, id: NodeId, selection: &TextBox,
        preview: &TextBox, patch: &TextTypographyPatch) -> Result<(), String> {
        let mut editor = self.editor.borrow_mut();
        if editor.selection.as_slice() != [id] {
            return Err("The text layer changed. Reopen Rich text to refresh the controls.".into());
        }
        let changed = editor.edit_text_typography(&[id], patch).map_err(|error| error.to_string())?;
        let node = editor.document.nodes[&id].clone();
        drop(editor);
        typography::configure_text_box(selection, &node);
        update_rich_preview(preview, &node);
        if changed > 0 { self.changed(); }
        Ok(())
    }
}

fn show_rich_result(error: &TextBlock, result: Result<(), String>) {
    if let Err(message) = result { error.set_text(Some(&message)); error.set_is_visible(true); error.bring_into_view(); }
    else { error.set_is_visible(false); }
}

fn positive_font_size(value: &str) -> Result<f64, String> {
    let size = value.trim().parse::<f64>().map_err(|_| "Enter a positive font size.".to_string())?;
    if size.is_finite() && size > 0.0 { Ok(size) } else { Err("Enter a positive finite font size.".into()) }
}

fn font_weight_value(value: &str) -> Result<u16, String> {
    value.trim().parse::<u16>().ok().filter(|weight| (1..=1000).contains(weight))
        .ok_or_else(|| "Enter a font weight between 1 and 1000.".into())
}

fn rich_label(value: &str, secondary: bool) -> Ref<TextBlock> {
    let text = TextBlock::new();
    text.set_text(Some(value));
    text.set_font_size(12.0);
    text.set_text_wrapping(TextWrapping::Wrap);
    if secondary { text.set_foreground(Some(icons::brush("#777777"))); }
    text
}

fn rich_grid(columns: &str) -> Ref<Grid> {
    let grid = Grid::new();
    grid.set_column_definitions(ColumnDefinitions::parse(columns).unwrap());
    grid.set_column_spacing(8.0);
    grid
}

fn rich_choice(name: &str, labels: &[&str], selected: i32) -> Ref<ComboBox> {
    let choice = ComboBox::new();
    choice.set_name(Some(name.into()));
    for label in labels { choice.items().add(Some(Rc::new(label.to_string()))); }
    choice.set_selected_index(selected);
    choice
}

fn labeled_control(label: &str, control: Ref<ComboBox>) -> Ref<StackPanel> {
    let panel = StackPanel::new();
    panel.set_spacing(4.0);
    panel.children().add(rich_label(label, true));
    panel.children().add(control);
    panel
}

fn override_field(label: &str, name: &str, value: &str) -> (Ref<Grid>, OverrideField) {
    let row = rich_grid("115,110,*");
    let caption = rich_label(label, true);
    caption.set_vertical_alignment(VerticalAlignment::Center);
    row.children().add(caption);
    let mode = rich_choice(&format!("{name}Mode"), &["Keep", "Set", "Inherit"], 0);
    Grid::set_column(&mode, 1);
    row.children().add(mode.clone());
    let field = TextBox::new();
    field.set_name(Some(name.into()));
    field.set_text(Some(value));
    field.set_is_enabled(false);
    Grid::set_column(&field, 2);
    row.children().add(field.clone());
    let weak_mode = mode.downgrade();
    let weak_field = field.downgrade();
    mode.selection_changed(move |_, _| {
        if let (Some(mode), Some(field)) = (weak_mode.upgrade(), weak_field.upgrade()) {
            field.set_is_enabled(mode.selected_index() == 1);
        }
    });
    (row, OverrideField { mode, value: field })
}

fn update_range_status(field: &TextBox, status: &TextBlock) {
    let text = field.text().unwrap_or_default();
    let value = match core::text_range_from_utf16(&text, field.selection_start(), field.selection_end()) {
        Ok(range) if !range.is_empty() => format!("{} characters selected · selection stays active while you change styles", text[range].chars().count()),
        Ok(_) => "Select characters with the mouse or Shift + arrow keys.".into(),
        Err(error) => error.to_string(),
    };
    status.set_text(Some(&value));
}

fn update_rich_preview(preview: &TextBox, node: &Node) {
    preview.set_text(Some(&node.text));
    typography::configure_text_box(preview, node);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rich_text_fields_reject_nonfinite_sizes_and_out_of_range_weights() {
        assert_eq!(positive_font_size(" 18.5 "), Ok(18.5));
        for invalid in ["NaN", "inf", "-1", "0", ""] { assert!(positive_font_size(invalid).is_err()); }
        assert_eq!(font_weight_value("650"), Ok(650));
        for invalid in ["0", "1001", "-1", "400.5", ""] { assert!(font_weight_value(invalid).is_err()); }
    }
}
