use super::*;

impl DesignerWorkspace {
    pub(super) fn apply_clip_placement(
        self: &Rc<Self>,
        context: &TimelineContext,
        clips: &[NodeId],
        start: Option<f64>,
        gap: f64,
    ) {
        let result = context.check(self).and_then(|()| {
            let mut session = self.session.borrow_mut();
            if session.is_playing() {
                return Err("Pause the clips before placing them".into());
            }
            session.place_animation_clips(clips, start, gap)
        });
        if result.is_ok() {
            self.refresh(true);
        }
        self.report(result);
    }

    pub(super) fn clip_placement_fields(
        self: &Rc<Self>,
        context: &TimelineContext,
        ids: Vec<NodeId>,
        initial: f64,
    ) {
        let fields = StackPanel::new();
        fields.set_orientation(Orientation::Horizontal);
        fields.set_spacing(5.0);
        let start = clip_field(&fields, "Start (s)", "ClipStart", initial);
        let gap = (ids.len() > 1).then(|| clip_field(&fields, "Gap / overlap (s)", "ClipGap", 0.0));
        let title = if ids.len() == 1 {
            "Set selected clip start"
        } else {
            "Sequence selected clips"
        };
        let apply = clip_tool("F1 M1 8L3 6L6 9L13 2L15 4L6 13Z", title);
        apply.set_name(Some("ApplyClipPlacement".into()));
        let weak = Rc::downgrade(self);
        let context = context.clone();
        apply.click(move |_, _| {
            if let Some(this) = weak.upgrade() {
                let parse =
                    |field: &TextBox| field.text().unwrap_or_default().trim().parse::<f64>();
                match (
                    parse(&start),
                    gap.as_ref().map(|field| parse(field)).transpose(),
                ) {
                    (Ok(start), Ok(gap)) => this.apply_clip_placement(
                        &context,
                        &ids,
                        (start != initial).then_some(start),
                        gap.unwrap_or(0.0),
                    ),
                    _ => this.report(Err("Enter clip timing in seconds".into())),
                }
            }
        });
        fields.children().add(apply.upcast::<Control>());
        self.root
            .get_control::<ContentControl>("TimelineInspectorHost")
            .set_content(Some(Control::boxed(&fields)));
    }
}

fn clip_field(fields: &StackPanel, label: &str, name: &str, initial: f64) -> Ref<TextBox> {
    let caption = TextBlock::new();
    caption.set_text(Some(label));
    caption.set_vertical_alignment(ferroui_base::layout::VerticalAlignment::Center);
    fields.children().add(caption.upcast::<Control>());
    let input = TextBox::new();
    input.set_name(Some(name.into()));
    input.set_text(Some(&initial.to_string()));
    input.set_width(80.0);
    input.classes().add("designer-field");
    AutomationProperties::set_name(&input, Some(label));
    fields.children().add(input.clone().upcast::<Control>());
    input
}
