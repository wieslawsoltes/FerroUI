use super::*;

pub(super) fn easing_fields(
    row: &StackPanel,
    easing: &Easing,
    writable: bool,
) -> (Ref<TextBox>, Ref<EasingPreview>) {
    let curve = key_field(row, "Curve", "KeyframeCurve", &easing.to_xaml(), 112.0);
    let preview = EasingPreview::new();
    preview.set_name(Some("KeyframeEasingPreview".into()));
    preview.set_easing(Some(easing.clone()));
    row.children().add(preview.clone().upcast::<Control>());
    if writable {
        let weak_curve = curve.downgrade();
        preview.edit_handles(move |easing| {
            if let Some(curve) = weak_curve.upgrade() {
                curve.set_text(Some(&easing.to_xaml()));
            }
        });
    }
    let preset = ComboBox::new();
    preset.set_name(Some("KeyframeEasingPreset".into()));
    preset.set_width(108.0);
    preset.classes().add("designer-field");
    preset.set_min_height(26.0);
    preset.set_font_size(11.0);
    let presets = keyframe_easing_presets();
    for option in presets {
        preset.items().add(Some(Rc::new(option.label.to_owned())));
    }
    preset.items().add(Some(Rc::new("Custom".to_owned())));
    let index = presets
        .iter()
        .position(|option| Easing::parse(option.value).ok().as_ref() == Some(easing))
        .unwrap_or(presets.len());
    preset.set_selected_index(index as i32);
    AutomationProperties::set_name(&preset, Some("Incoming keyframe easing preset"));
    ToolTip::set_tip(
        &preset,
        Some(Rc::new(
            "Choose an easing preset, then Apply keyframe. Custom curves accept x1,y1,x2,y2."
                .to_owned(),
        )),
    );
    let updating_preset = Rc::new(Cell::new(false));
    let syncing = updating_preset.clone();
    let weak_preset = preset.downgrade();
    let weak_curve = curve.downgrade();
    preset.selection_changed(move |_, _| {
        if syncing.get() {
            return;
        }
        if let (Some(preset), Some(curve)) = (weak_preset.upgrade(), weak_curve.upgrade()) {
            if let Some(option) = presets.get(preset.selected_index().max(0) as usize) {
                curve.set_text(Some(option.value));
            } else {
                curve.focus();
            }
        }
    });
    let weak_preview = preview.downgrade();
    let weak_preset = preset.downgrade();
    let weak_curve = curve.downgrade();
    curve.text_changing(move |_, _| {
        let (Some(preview), Some(preset), Some(curve)) = (
            weak_preview.upgrade(),
            weak_preset.upgrade(),
            weak_curve.upgrade(),
        ) else {
            return;
        };
        let draft = Easing::parse(&curve.text().unwrap_or_default()).ok();
        if preview.easing() != draft {
            preview.set_easing(draft.clone());
        }
        let index = presets
            .iter()
            .position(|option| Easing::parse(option.value).ok() == draft)
            .unwrap_or(presets.len()) as i32;
        if preset.selected_index() != index {
            updating_preset.set(true);
            preset.set_selected_index(index);
            updating_preset.set(false);
        }
    });
    row.children().add(preset.upcast::<Control>());
    ToolTip::set_tip(
        &curve,
        Some(Rc::new(
            "LinearEasing or cubic Bézier x1,y1,x2,y2".to_owned(),
        )),
    );
    (curve, preview)
}
