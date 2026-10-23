//! Native effect-stack controls hosted by the compiled XAML inspector shell.
use super::*;
use ferrodesign_core::{EffectKind, EffectPropertyEdit, EffectStackEdit, MAX_EFFECT_DISTANCE, MAX_EFFECT_RADIUS};
use ferroui_base::input::KeyModifiers;
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::IBrush;
use ferroui_base::CornerRadius;

const EFFECT_LABELS: [&str; 4] = ["Drop shadow", "Inner shadow", "Layer blur", "Background blur"];
const EFFECT_KINDS: [EffectKind; 4] = [EffectKind::DropShadow, EffectKind::InnerShadow, EffectKind::LayerBlur, EffectKind::BackgroundBlur];

/// Tree replacement can raise LostFocus from a discarded field. Such events
/// must not commit the old row into the new stack after a remove or reorder.
struct UpdatingEffects<'a>(&'a Cell<bool>, bool);
impl Drop for UpdatingEffects<'_> {
    fn drop(&mut self) { self.0.set(self.1); }
}

struct EffectsContext {
    selection: Vec<NodeId>,
    source: NodeId,
    error: WeakRef<TextBlock>,
}

#[derive(Clone, Copy)]
struct EffectNumber {
    min: f64,
    max: f64,
    percentage: bool,
}

impl EffectNumber {
    fn parse(self, text: &str) -> Result<f64, String> {
        let text = text.trim();
        let text = if self.percentage { text.strip_suffix('%').unwrap_or(text).trim() } else { text };
        let value = text.parse::<f64>().map_err(|_| self.error())?;
        if !value.is_finite() || !(self.min..=self.max).contains(&value) { return Err(self.error()); }
        Ok(value)
    }
    fn error(self) -> String { format!("Enter a finite value from {} to {}{}.", self.min, self.max, if self.percentage { "%" } else { "" }) }
}

impl EditorShell {
    pub(super) fn effects_dialog(self: &Rc<Self>) {
        let snapshot = {
            let editor = self.editor.borrow();
            let editable: Vec<_> = editor.selection.iter().copied()
                .filter(|id| editor.document.nodes.contains_key(id) && !editor.document.effective_locked(*id)).collect();
            editable.first().or(editor.selection.first()).and_then(|id| editor.document.nodes.get(id)).map(|node| {
                let mixed = editable.iter().any(|id| editor.document.nodes[id].effects != node.effects);
                (node.id, node.name.clone(), node.effects.clone(), editor.selection.clone(), editable.len(), mixed,
                    editable.iter().any(|id| !editor.document.nodes[id].effects.is_empty()))
            })
        };
        let Some((source, name, effects, selection, editable, mixed, any_effects)) = snapshot else {
            self.close_modal();
            self.toast("Select a layer to edit its effects.");
            return;
        };
        let _updating = UpdatingEffects(&self.updating, self.updating.replace(true));
        let description = if editable == 0 {
            format!("{name} · Unlock the selected layers to edit their effects.")
        } else if selection.len() > 1 {
            format!("Showing {name}. Matching rows update {editable} unlocked selected layers. Changes support undo.")
        } else {
            format!("{name} · Enter or Tab applies a value. Arrow keys adjust numbers; Shift uses a larger step.")
        };
        self.modal("Effects", &description);
        let host = self.control::<StackPanel>("ModalContent");
        host.set_spacing(12.0);
        let error = effect_text("", true);
        error.set_name(Some("EffectStackError".into()));
        error.set_foreground(Some(icons::brush("#C8372B")));
        error.set_is_visible(false);
        host.children().add(error.clone());
        let context = Rc::new(EffectsContext { selection, source, error: error.downgrade() });
        if mixed {
            host.children().add(effect_text("Mixed effect stacks · values show the first unlocked layer. Each property changes only that property on matching rows; missing rows stay unchanged.", true));
        }
        if effects.is_empty() {
            host.children().add(effect_text("This layer has no effects. Add a shadow or blur below.", true));
        }
        for (index, effect) in effects.iter().enumerate() {
            let card = self.effect_card(&context, index, effect, effects.len());
            card.set_is_enabled(editable != 0);
            host.children().add(card);
        }

        let actions = self.control::<StackPanel>("ModalActions");
        let add_type = ComboBox::new();
        add_type.set_name(Some("EffectAddType".into()));
        add_type.set_width(144.0);
        add_type.set_height(30.0);
        add_type.set_is_enabled(editable != 0);
        for label in EFFECT_LABELS { add_type.items().add(Some(Rc::new(label.to_string()))); }
        add_type.set_selected_index(0);
        let weak_type = add_type.downgrade();
        actions.children().add(add_type);
        let add_context = context.clone();
        actions.children().add(self.effect_button("Add effect", "EffectAdd", editable != 0, move |shell| {
            let Some(choice) = weak_type.upgrade() else { return; };
            let Some(kind) = usize::try_from(choice.selected_index()).ok().and_then(|index| EFFECT_KINDS.get(index)) else { return; };
            shell.effect_action(&add_context, EffectStackEdit::Add(*kind), true);
        }));
        let clear_context = context.clone();
        actions.children().add(self.effect_button("Clear all", "EffectClear", editable != 0 && any_effects, move |shell| {
            shell.effect_action(&clear_context, EffectStackEdit::Clear, true);
        }));
        let done = self.effect_button("Done", "EffectDone", true, |shell| shell.close_modal());
        done.classes().add("primary");
        actions.children().add(done);
    }

    /// Both the inspector + button and the panel use the same validated core
    /// command, including multi-selection, inherited locks and style detachment.
    pub(super) fn add_effect(self: &Rc<Self>) {
        let ids = self.editor.borrow().selection.clone();
        if ids.is_empty() { self.toast("Select a layer to add an effect."); return; }
        let result = self.editor.borrow_mut().edit_effects(&ids, EffectStackEdit::Add(EffectKind::DropShadow));
        match result {
            Ok(count) => {
                if count != 0 { self.changed(); }
                self.effects_dialog();
            }
            Err(error) => self.toast(&error.to_string()),
        }
    }

    fn effect_card(self: &Rc<Self>, context: &Rc<EffectsContext>, index: usize, effect: &Effect, count: usize) -> Ref<Border> {
        let card = Border::new();
        card.set_name(Some(format!("EffectCard{index}")));
        card.set_padding(Thickness::uniform(12.0));
        card.set_corner_radius(CornerRadius::uniform(8.0));
        card.set_background(Some(icons::brush("#F7F7F7")));
        card.set_border_brush(Some(icons::brush("#E5E5E5")));
        card.set_border_thickness(Thickness::uniform(1.0));
        let content = StackPanel::new();
        content.set_spacing(9.0);
        card.set_child(content.clone());

        let header = effect_grid("*,44,54,78,72", 5.0);
        let visible = CheckBox::new();
        visible.set_name(Some(format!("EffectVisible{index}")));
        visible.set_content(Some(Rc::new(format!("Effect {}", index + 1))));
        visible.set_is_checked(Some(effect.visible));
        let weak_shell = Rc::downgrade(self);
        let weak_visible = visible.downgrade();
        let visible_context = context.clone();
        visible.is_checked_changed(move |_, _| {
            if let (Some(shell), Some(check)) = (weak_shell.upgrade(), weak_visible.upgrade()) {
                shell.effect_action(&visible_context, effect_property(index, EffectPropertyEdit::Visible(check.is_checked() == Some(true))), false);
            }
        });
        header.children().add(visible);
        for (column, caption, suffix, enabled, edit) in [
            (1, "Up", "Up", index > 0, EffectStackEdit::Move { index, to: index.saturating_sub(1) }),
            (2, "Down", "Down", index + 1 < count, EffectStackEdit::Move { index, to: index + 1 }),
            (3, "Duplicate", "Duplicate", true, EffectStackEdit::Duplicate(index)),
            (4, "Remove", "Remove", true, EffectStackEdit::Remove(index)),
        ] {
            let context = context.clone();
            let button = self.effect_button(caption, &format!("Effect{suffix}{index}"), enabled, move |shell| {
                shell.effect_action(&context, edit.clone(), true);
            });
            Grid::set_column(&button, column);
            header.children().add(button);
        }
        content.children().add(header);

        let properties = effect_grid("*,140", 10.0);
        let type_panel = StackPanel::new();
        type_panel.set_spacing(4.0);
        type_panel.children().add(effect_text("Type", true));
        let choice = ComboBox::new();
        choice.set_name(Some(format!("EffectType{index}")));
        for label in EFFECT_LABELS { choice.items().add(Some(Rc::new(label.to_string()))); }
        choice.set_selected_index(EFFECT_KINDS.iter().position(|kind| *kind == effect.kind).unwrap_or_default() as i32);
        let weak_shell = Rc::downgrade(self);
        let weak_choice = choice.downgrade();
        let type_context = context.clone();
        choice.selection_changed(move |_, _| {
            let (Some(shell), Some(choice)) = (weak_shell.upgrade(), weak_choice.upgrade()) else { return; };
            let Some(kind) = usize::try_from(choice.selected_index()).ok().and_then(|index| EFFECT_KINDS.get(index)) else { return; };
            shell.effect_action(&type_context, effect_property(index, EffectPropertyEdit::Kind(*kind)), true);
        });
        type_panel.children().add(choice);
        properties.children().add(type_panel);
        let radius = self.effect_numeric(context, index, "Blur", "Radius", effect.radius,
            EffectNumber { min: 0.0, max: MAX_EFFECT_RADIUS, percentage: false }, EffectPropertyEdit::Radius);
        Grid::set_column(&radius, 1);
        properties.children().add(radius);
        content.children().add(properties);

        if matches!(effect.kind, EffectKind::DropShadow | EffectKind::InnerShadow) {
            let geometry = effect_grid("*,*,*", 10.0);
            let limits = EffectNumber { min: -MAX_EFFECT_DISTANCE, max: MAX_EFFECT_DISTANCE, percentage: false };
            for (column, label, suffix, value, property) in [
                (0, "X", "OffsetX", effect.offset.x, EffectPropertyEdit::OffsetX as fn(f64) -> EffectPropertyEdit),
                (1, "Y", "OffsetY", effect.offset.y, EffectPropertyEdit::OffsetY as fn(f64) -> EffectPropertyEdit),
                (2, "Spread", "Spread", effect.spread, EffectPropertyEdit::Spread as fn(f64) -> EffectPropertyEdit),
            ] {
                let field = self.effect_numeric(context, index, label, suffix, value, limits, property);
                Grid::set_column(&field, column);
                geometry.children().add(field);
            }
            content.children().add(geometry);

            let color_row = effect_grid("*,120,34", 10.0);
            let swatch = Border::new();
            swatch.set_name(Some(format!("EffectSwatch{index}")));
            swatch.set_height(32.0);
            swatch.set_corner_radius(CornerRadius::uniform(4.0));
            swatch.set_border_brush(Some(icons::brush("#D5D5D5")));
            swatch.set_border_thickness(Thickness::uniform(1.0));
            swatch.set_background(Some(effect_color_brush(effect.color)));
            swatch.set_vertical_alignment(VerticalAlignment::Bottom);
            Grid::set_column(&swatch, 2);
            let color_context = context.clone();
            let weak_swatch = swatch.downgrade();
            let color = self.effect_field("Color · RGB hex", &format!("EffectColor{index}"),
                &Color::rgb(effect.color.r, effect.color.g, effect.color.b).hex(), None, move |shell, text| {
                    let color = effect_rgb(text)?;
                    shell.apply_effects(&color_context, effect_property(index, EffectPropertyEdit::Rgb { r: color.r, g: color.g, b: color.b }), false)?;
                    shell.refresh_effect_swatch(&color_context, index, &weak_swatch);
                    Ok(())
                });
            color_row.children().add(color);
            let opacity_context = context.clone();
            let weak_swatch = swatch.downgrade();
            let limits = EffectNumber { min: 0.0, max: 100.0, percentage: true };
            // Preserve imported alpha until the user actually edits this field.
            let opacity = format!("{:.2}", effect.color.a as f64 * 100.0 / 255.0);
            let opacity = self.effect_field("Opacity %", &format!("EffectOpacity{index}"), &opacity, Some(limits), move |shell, text| {
                let opacity = limits.parse(text)? / 100.0;
                shell.apply_effects(&opacity_context, effect_property(index, EffectPropertyEdit::Opacity(opacity)), false)?;
                shell.refresh_effect_swatch(&opacity_context, index, &weak_swatch);
                Ok(())
            });
            Grid::set_column(&opacity, 1);
            color_row.children().add(opacity);
            color_row.children().add(swatch);
            content.children().add(color_row);
        } else {
            content.children().add(effect_text(if effect.kind == EffectKind::LayerBlur {
                "Blurs the layer and its rendered contents. Shadow color, offset and spread are retained when changing types."
            } else {
                "Blurs content behind this layer. Use a translucent fill to reveal the background through the layer."
            }, true));
        }
        card
    }

    fn effect_numeric(self: &Rc<Self>, context: &Rc<EffectsContext>, index: usize, label: &str, suffix: &str,
        value: f64, limits: EffectNumber, property: fn(f64) -> EffectPropertyEdit) -> Ref<StackPanel> {
        let context = context.clone();
        self.effect_field(label, &format!("Effect{suffix}{index}"), &value.to_string(), Some(limits), move |shell, text| {
            shell.apply_effects(&context, effect_property(index, property(limits.parse(text)?)), false)
        })
    }

    fn effect_field(self: &Rc<Self>, label: &str, name: &str, value: &str, numeric: Option<EffectNumber>,
        apply: impl Fn(&Rc<Self>, &str) -> Result<(), String> + 'static) -> Ref<StackPanel> {
        let panel = StackPanel::new();
        panel.set_spacing(4.0);
        panel.children().add(effect_text(label, true));
        let field = TextBox::new();
        field.set_name(Some(name.into()));
        field.set_text(Some(value));
        panel.children().add(field.clone());
        let error = effect_text("", true);
        error.set_name(Some(format!("{name}Error")));
        error.set_foreground(Some(icons::brush("#C8372B")));
        error.set_is_visible(false);
        panel.children().add(error.clone());
        let weak_shell = Rc::downgrade(self);
        let weak_field = field.downgrade();
        let weak_error = error.downgrade();
        let committed = Rc::new(RefCell::new(value.to_string()));
        let committed_value = committed.clone();
        let commit = Rc::new(move || {
            let (Some(shell), Some(field), Some(error)) = (weak_shell.upgrade(), weak_field.upgrade(), weak_error.upgrade()) else { return; };
            if !shell.effects_open() || field.is_composing_text() { return; }
            let text = field.text().unwrap_or_default();
            if *committed_value.borrow() == text { return; }
            match apply(&shell, &text) {
                Ok(()) => {
                    *committed_value.borrow_mut() = text;
                    error.set_is_visible(false);
                    field.set_border_brush(Some(icons::brush("#E5E5E5")));
                }
                Err(message) => {
                    error.set_text(Some(&message));
                    error.set_is_visible(true);
                    field.set_border_brush(Some(icons::brush("#C8372B")));
                }
            }
        });
        let blur = commit.clone();
        field.add_handler(InputElement::lost_focus_event(), move |_, _| blur());
        let draft_commit = commit.clone();
        let draft_committed = committed.clone();
        let weak_field = field.downgrade();
        let weak_error = error.downgrade();
        let weak_shell = Rc::downgrade(self);
        // Numeric arrows belong to the effect editor. Capture them before the
        // TextBox's caret-navigation handler can consume the bubble event.
        field.add_handler_with(InputElement::key_down_event(), move |_, event| {
            let (Some(shell), Some(field), Some(error)) = (weak_shell.upgrade(), weak_field.upgrade(), weak_error.upgrade()) else { return; };
            if !shell.effects_open() || field.is_composing_text() { return; }
            match event.key {
                Key::Enter => { commit(); event.set_handled(true); }
                Key::Escape => {
                    let previous = committed.borrow().clone();
                    field.set_text(Some(&previous));
                    error.set_is_visible(false);
                    field.set_border_brush(Some(icons::brush("#E5E5E5")));
                    event.set_handled(true);
                }
                Key::Up | Key::Down if numeric.is_some() => {
                    let limits = numeric.unwrap();
                    if let Ok(value) = limits.parse(&field.text().unwrap_or_default()) {
                        let step = if event.key_modifiers.contains(KeyModifiers::SHIFT) { 10.0 }
                            else if event.key_modifiers.contains(KeyModifiers::ALT) { 0.1 } else { 1.0 };
                        let delta = if event.key == Key::Up { step } else { -step };
                        let value = (value + delta).clamp(limits.min, limits.max);
                        field.set_text(Some(&value.to_string()));
                    }
                    commit();
                    event.set_handled(true);
                }
                _ => {},
            }
        }, RoutingStrategies::TUNNEL, false);
        self.bind_inspector_draft(&field, draft_committed, draft_commit, Some(&error));
        panel
    }

    fn effects_open(&self) -> bool {
        !self.updating.get() && self.control::<Control>("ModalBackdrop").is_visible()
            && self.control::<TextBlock>("ModalTitle").text().as_deref() == Some("Effects")
    }

    fn apply_effects(self: &Rc<Self>, context: &EffectsContext, edit: EffectStackEdit, rebuild: bool) -> Result<(), String> {
        if !self.effects_open() { return Ok(()); }
        if self.editor.borrow().selection != context.selection {
            self.effects_dialog();
            return Err("Selection changed. Edit the refreshed effect rows.".into());
        }
        let changed = self.editor.borrow_mut().edit_effects(&context.selection, edit).map_err(|error| error.to_string())?;
        if let Some(error) = context.error.upgrade() { error.set_is_visible(false); }
        if changed != 0 { self.changed(); }
        if rebuild { self.effects_dialog(); }
        Ok(())
    }

    fn effect_action(self: &Rc<Self>, context: &EffectsContext, edit: EffectStackEdit, rebuild: bool) {
        if let Err(message) = self.apply_effects(context, edit, rebuild) {
            if let Some(error) = context.error.upgrade() {
                error.set_text(Some(&message));
                error.set_is_visible(true);
            }
            self.toast(&message);
        }
    }

    fn refresh_effect_swatch(&self, context: &EffectsContext, index: usize, swatch: &WeakRef<Border>) {
        let Some(swatch) = swatch.upgrade() else { return; };
        if let Some(effect) = self.editor.borrow().document.nodes.get(&context.source).and_then(|node| node.effects.get(index)) {
            swatch.set_background(Some(effect_color_brush(effect.color)));
        }
    }

    fn effect_button(self: &Rc<Self>, caption: &str, name: &str, enabled: bool,
        operation: impl Fn(&Rc<Self>) + 'static) -> Ref<Button> {
        let button = Button::new();
        button.set_name(Some(name.into()));
        button.set_content(Some(Rc::new(caption.to_string())));
        button.set_height(30.0);
        button.set_is_enabled(enabled);
        let weak_shell = Rc::downgrade(self);
        button.click(move |_, _| {
            if let Some(shell) = weak_shell.upgrade() { if shell.effects_open() { operation(&shell); } }
        });
        button
    }
}

fn effect_property(index: usize, edit: EffectPropertyEdit) -> EffectStackEdit { EffectStackEdit::Property { index, edit } }

fn effect_color_brush(color: Color) -> Rc<dyn IBrush> {
    Rc::new(ImmutableSolidColorBrush::new(ferroui_base::media::Color::from_argb(color.a, color.r, color.g, color.b)))
}

fn effect_rgb(value: &str) -> Result<Color, String> {
    let value = value.trim().strip_prefix('#').unwrap_or(value.trim());
    if !matches!(value.len(), 3 | 6) { return Err("Use 3 or 6 RGB hexadecimal digits. Set transparency with Opacity.".into()); }
    Color::from_hex(value).ok_or_else(|| "Use 3 or 6 RGB hexadecimal digits.".into())
}

fn effect_text(value: &str, muted: bool) -> Ref<TextBlock> {
    let text = TextBlock::new();
    text.set_text(Some(value));
    text.set_font_size(if muted { 11.0 } else { 12.0 });
    text.set_foreground(Some(icons::brush(if muted { "#737373" } else { "#242424" })));
    text.set_text_wrapping(TextWrapping::Wrap);
    text
}

fn effect_grid(columns: &str, spacing: f64) -> Ref<Grid> {
    let grid = Grid::new();
    grid.set_column_definitions(ColumnDefinitions::parse(columns).expect("effect columns"));
    grid.set_column_spacing(spacing);
    grid
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_effect_fields_reject_nonfinite_units_and_out_of_range_values() {
        let radius = EffectNumber { min: 0.0, max: MAX_EFFECT_RADIUS, percentage: false };
        for text in ["NaN", "inf", "-inf", "-1", "10001", "2%", "", "1e999"] { assert!(radius.parse(text).is_err(), "{text}"); }
        assert_eq!(radius.parse(" 12.125 ").unwrap(), 12.125);
        let opacity = EffectNumber { min: 0.0, max: 100.0, percentage: true };
        assert_eq!(opacity.parse(" 25.5 % ").unwrap(), 25.5);
        for text in ["NaN%", "100.001%", "-0.01%", "25%%"] { assert!(opacity.parse(text).is_err()); }
    }

    #[test]
    fn rgb_entry_does_not_accept_hidden_alpha_or_non_hex_input() {
        assert_eq!(effect_rgb(" #1aF ").unwrap(), Color::rgb(17, 170, 255));
        assert_eq!(effect_rgb("123456").unwrap(), Color::rgb(18, 52, 86));
        for text in ["#1234", "#11223344", "#fffffg", "#", "ééé", "##123"] { assert!(effect_rgb(text).is_err(), "{text}"); }
    }

    #[test]
    fn displayed_opacity_round_trips_every_document_alpha_without_rounding_loss() {
        let opacity = EffectNumber { min: 0.0, max: 100.0, percentage: true };
        for alpha in 0..=255_u8 {
            let text = format!("{:.2}", alpha as f64 * 100.0 / 255.0);
            let parsed = opacity.parse(&text).unwrap() / 100.0;
            assert_eq!((parsed * 255.0).round() as u8, alpha, "displayed opacity {text}");
        }
        for (text, expected) in [("0%", 0), ("100%", 255), ("37.5%", 96), ("50%", 128)] {
            assert_eq!((opacity.parse(text).unwrap() * 255.0 / 100.0).round() as u8, expected);
        }
    }
}
