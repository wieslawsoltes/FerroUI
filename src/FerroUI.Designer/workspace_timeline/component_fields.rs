use super::*;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_controls::{CheckBox, ScrollViewer, WrapPanel};
use ferroui_designer_animation::{
    BrushCurveChannel, BrushField, CurveChannel, CurveStep, KeyframeComponentEdit,
    KeyframeCurveValue, TransformCurveChannel,
};
use std::collections::HashMap;

#[derive(Clone, Default)]
pub(super) struct ViewState {
    pub as_text: bool,
    pub group: String,
}

pub(super) struct ComponentDraft {
    groups: Vec<(String, Vec<KeyframeCurveValue>)>,
    values: RefCell<HashMap<TrackCurveChannel, String>>,
    pub view: RefCell<ViewState>,
}

impl ComponentDraft {
    pub(super) fn new(
        track: &Track,
        key: &Keyframe,
        duration: f64,
        fill: ferroui_designer_animation::FillMode,
        previous: ViewState,
    ) -> Option<Rc<Self>> {
        let channels = TrackCurveChannel::for_track(track, duration, fill);
        let mut groups: Vec<(String, Vec<KeyframeCurveValue>)> = Vec::new();
        for field in TrackCurveChannel::key_components(&channels, &key.value) {
            if field.channel.is_scalar() {
                continue;
            }
            let (group, _) = field_labels(&field.channel);
            if let Some((_, fields)) = groups.iter_mut().find(|(name, _)| *name == group) {
                fields.push(field);
            } else {
                groups.push((group, vec![field]));
            }
        }
        let first = groups
            .iter()
            .find(|(name, _)| name == "Brush")
            .or_else(|| groups.first())?;
        let group = if groups.iter().any(|(name, _)| *name == previous.group) {
            previous.group
        } else {
            first.0.clone()
        };
        Some(Rc::new(Self {
            groups,
            values: RefCell::new(HashMap::new()),
            view: RefCell::new(ViewState {
                group,
                as_text: previous.as_text,
            }),
        }))
    }

    pub(super) fn edits(&self) -> Result<Vec<KeyframeComponentEdit>, String> {
        let values = self.values.borrow();
        self.groups
            .iter()
            .flat_map(|(_, fields)| fields)
            .filter_map(|field| {
                let text = values.get(&field.channel)?;
                Some(
                    text.parse::<f64>()
                        .ok()
                        .filter(|value| value.is_finite())
                        .ok_or_else(|| format!("Enter a number for {}", field.channel.label()))
                        .map(|value| (field, value)),
                )
            })
            .filter_map(|result| match result {
                Ok((field, value)) if value == field.value => None,
                Ok((field, value)) => Some(Ok(KeyframeComponentEdit {
                    channel: field.channel.clone(),
                    value,
                })),
                Err(error) => Some(Err(error)),
            })
            .collect()
    }

    pub(super) fn mount(
        self: &Rc<Self>,
        row: &StackPanel,
        value: &Ref<StackPanel>,
        writable: bool,
        commit: &Rc<dyn Fn()>,
    ) -> Ref<ScrollViewer> {
        let fields = WrapPanel::new();
        fields.set_item_spacing(10.0);
        fields.set_line_spacing(4.0);
        fields.set_margin(Thickness::new(4.0, 0.0, 0.0, 6.0));
        fields.set_is_enabled(writable);
        let shelf = ScrollViewer::new();
        shelf.set_max_height(112.0);
        shelf.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled);
        shelf.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Auto);
        shelf.set_content(Some(Control::boxed(&fields)));
        let mode = component_mode();
        row.children().add(mode.clone().upcast::<Control>());
        let picker = ComboBox::new();
        picker.classes().add("designer-field");
        picker.set_name(Some("KeyframeValueGroup".into()));
        picker.set_width(140.0);
        picker.set_min_height(26.0);
        picker.set_font_size(11.0);
        AutomationProperties::set_name(&picker, Some("Keyframe value group"));
        ToolTip::set_tip(&picker, Some(Rc::new("Keyframe value group".to_owned())));
        for (name, _) in &self.groups {
            picker.items().add(Some(Rc::new(name.clone())));
        }
        let view = self.view.borrow().clone();
        picker.set_selected_index(
            self.groups
                .iter()
                .position(|(name, _)| *name == view.group)
                .unwrap_or(0) as i32,
        );
        row.children().add(picker.clone().upcast::<Control>());

        let draft = self.clone();
        let weak_fields = fields.downgrade();
        let weak_picker = picker.downgrade();
        let apply = commit.clone();
        picker.selection_changed(move |_, _| {
            if let (Some(fields), Some(picker)) = (weak_fields.upgrade(), weak_picker.upgrade()) {
                if let Some((name, _)) = usize::try_from(picker.selected_index())
                    .ok()
                    .and_then(|index| draft.groups.get(index))
                {
                    draft.view.borrow_mut().group.clone_from(name);
                    draft.render(&fields, &apply);
                }
            }
        });
        let draft = self.clone();
        let weak_mode = mode.downgrade();
        let weak_value = value.downgrade();
        let weak_picker = picker.downgrade();
        let weak_shelf = shelf.downgrade();
        let show_mode = move |as_text: bool| {
            let (Some(mode), Some(value), Some(picker), Some(shelf)) = (
                weak_mode.upgrade(),
                weak_value.upgrade(),
                weak_picker.upgrade(),
                weak_shelf.upgrade(),
            ) else {
                return;
            };
            let label = if as_text {
                "Edit keyframe components"
            } else {
                "Edit keyframe value as text"
            };
            AutomationProperties::set_name(&mode, Some(label));
            ToolTip::set_tip(&mode.upcast::<Control>(), Some(Rc::new(label.to_owned())));
            value.set_is_visible(as_text);
            shelf.set_is_visible(!as_text);
            picker.set_is_visible(!as_text && draft.groups.len() > 1);
        };
        show_mode(view.as_text);
        let draft = self.clone();
        mode.click(move |_, _| {
            let as_text = {
                let mut view = draft.view.borrow_mut();
                view.as_text = !view.as_text;
                view.as_text
            };
            show_mode(as_text);
        });
        self.render(&fields, commit);
        shelf
    }

    fn render(self: &Rc<Self>, panel: &WrapPanel, commit: &Rc<dyn Fn()>) {
        panel.children().clear();
        let view = self.view.borrow();
        let Some((_, fields)) = self.groups.iter().find(|(name, _)| *name == view.group) else {
            return;
        };
        for (index, field) in fields.iter().enumerate() {
            let (_, mut label) = field_labels(&field.channel);
            if field.channel.uses_normalized_values() {
                label.push_str(" (0–1)");
            }
            let initial = self
                .values
                .borrow()
                .get(&field.channel)
                .cloned()
                .unwrap_or_else(|| {
                    let value = field.value;
                    if field.channel.component().is_color() {
                        format!("{value:.4}")
                            .trim_end_matches('0')
                            .trim_end_matches('.')
                            .to_owned()
                    } else {
                        value.to_string()
                    }
                });
            let row = StackPanel::new();
            row.set_orientation(Orientation::Horizontal);
            row.set_spacing(4.0);
            let name = format!("KeyframeComponent{index}");
            let draft = self.clone();
            let channel = field.channel.clone();
            if field.step == Some(CurveStep::Boolean) {
                let input = CheckBox::new();
                input.set_name(Some(name.into()));
                input.set_content(Some(Rc::new(label.clone())));
                input.set_font_size(11.0);
                input.set_is_checked(Some(initial == "1"));
                AutomationProperties::set_name(&input, Some(&label));
                let weak = input.downgrade();
                input.is_checked_changed(move |_, _| {
                    if let Some(input) = weak.upgrade() {
                        draft.values.borrow_mut().insert(
                            channel.clone(),
                            if input.is_checked() == Some(true) {
                                "1"
                            } else {
                                "0"
                            }
                            .into(),
                        );
                    }
                });
                commit_on_enter(&input, commit);
                row.children().add(input.upcast::<Control>());
            } else {
                let input = key_field(&row, &label, &name, &initial, 66.0);
                let weak = input.downgrade();
                input.text_changing(move |_, _| {
                    if let Some(input) = weak.upgrade() {
                        draft.values.borrow_mut().insert(
                            channel.clone(),
                            input.text().unwrap_or_default().to_string(),
                        );
                    }
                });
                commit_on_enter(&input, commit);
            }
            panel.children().add(row.upcast::<Control>());
        }
    }
}

pub(super) fn commit_on_enter(input: &Control, commit: &Rc<dyn Fn()>) {
    let commit = commit.clone();
    input.add_handler(InputElement::key_down_event(), move |_, event| {
        if event.key == Key::Enter {
            event.set_handled(true);
            commit();
        }
    });
}

fn component_mode() -> Ref<Button> {
    let mode = button("Edit keyframe value as text");
    mode.set_name(Some("KeyframeTextMode".into()));
    mode.classes().add("nav");
    mode.classes().add("designer-timeline-tool");
    mode.set_width(28.0);
    mode.set_min_height(26.0);
    mode.set_content(Some(Control::boxed(timeline_icon(
        "M6,4 L2,8 6,12 M10,4 L14,8 10,12",
        0xffa9adba,
    ))));
    mode
}

fn field_labels(channel: &TrackCurveChannel) -> (String, String) {
    let component = channel.component().label();
    let group = match channel {
        TrackCurveChannel::BoxShadow { layer, .. } => format!("Shadow {layer}"),
        TrackCurveChannel::Brush(BrushCurveChannel::Stop { index, .. }) => format!("Stop {index}"),
        TrackCurveChannel::Brush(BrushCurveChannel::Field {
            field: BrushField::TransformOrigin,
            ..
        }) => "Transform origin".into(),
        TrackCurveChannel::Brush(BrushCurveChannel::Field { .. }) => {
            return ("Brush".into(), channel.label().into_owned())
        }
        TrackCurveChannel::Transform(TransformCurveChannel::Operation { .. }) => channel
            .label()
            .strip_suffix(component)
            .unwrap_or("Operation")
            .trim_end()
            .to_owned(),
        TrackCurveChannel::EffectParameter {
            component: CurveChannel::Value,
            ..
        } => return ("Parameters".into(), channel.label().into_owned()),
        TrackCurveChannel::EffectParameter { .. } => channel
            .label()
            .strip_suffix(component)
            .unwrap_or("Parameters")
            .trim_end()
            .to_owned(),
        _ => return ("Value".into(), channel.label().into_owned()),
    };
    (group, component.into())
}
