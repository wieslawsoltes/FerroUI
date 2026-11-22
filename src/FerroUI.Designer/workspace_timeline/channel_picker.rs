use super::*;
use ferroui_base::layout::VerticalAlignment;

impl DesignerWorkspace {
    pub(in crate::workspace::timeline) fn timeline_curve_channel(
        &self,
        track: &Track,
        choices: &[TrackCurveChannel],
    ) -> Option<TrackCurveChannel> {
        self.timeline_ui
            .borrow()
            .curve_channels
            .get(&(track.target, track.property.clone()))
            .filter(|channel| choices.contains(channel))
            .or_else(|| choices.first())
            .cloned()
    }

    pub(in crate::workspace::timeline) fn timeline_curve_title(
        self: &Rc<Self>,
        title: Ref<Button>,
        lane: &Ref<TimelineLane>,
        track: &Track,
        context: &TimelineContext,
        (metric, channel): &(CurveMetric, TrackCurveChannel),
        choices: Vec<TrackCurveChannel>,
    ) -> Ref<Control> {
        let picker = ComboBox::new();
        picker.set_width(match channel {
            TrackCurveChannel::Transform(_) | TrackCurveChannel::BoxShadow { .. } | TrackCurveChannel::Brush(_) => 164.0,
            TrackCurveChannel::EffectParameter { .. } => 124.0,
            _ => 82.0,
        });
        picker.set_font_size(10.0);
        picker.set_vertical_alignment(VerticalAlignment::Center);
        picker.classes().add("designer-field");
        let label = format!("{} curve channel", track.property);
        let metric = *metric;
        AutomationProperties::set_name(&picker, Some(&label));
        ToolTip::set_tip(&picker, Some(Rc::new(label)));
        for choice in &choices {
            picker.items().add(Some(Rc::new(choice.label().into_owned())));
        }
        if let Some(index) = choices.iter().position(|choice| choice == channel) {
            picker.set_selected_index(index as i32);
        }
        let workspace = Rc::downgrade(self);
        let lane = lane.downgrade();
        let weak_picker = picker.downgrade();
        let track = track.clone();
        let context = context.clone();
        picker.selection_changed(move |_, _| {
            let (Some(workspace), Some(lane), Some(picker)) =
                (workspace.upgrade(), lane.upgrade(), weak_picker.upgrade())
            else {
                return;
            };
            let Some(channel) = choices.get(picker.selected_index() as usize) else {
                return;
            };
            if context.check(&workspace).is_ok() {
                workspace
                    .timeline_ui
                    .borrow_mut()
                    .curve_channels
                    .insert((track.target, track.property.clone()), channel.clone());
                workspace.attach_timeline_curve(&lane, &track, &context, metric, channel.clone());
            }
        });
        let header = Grid::new();
        header.set_column_definitions(ColumnDefinitions::parse("*,Auto").unwrap());
        Grid::set_column(&picker, 1);
        header.children().add(title.upcast::<Control>());
        header.children().add(picker.upcast::<Control>());
        header.upcast()
    }
}
