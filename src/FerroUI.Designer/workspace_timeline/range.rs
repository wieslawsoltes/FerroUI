use super::*;
use ferroui_base::layout::VerticalAlignment;
use ferroui_controls::CheckBox;
use ferroui_designer_animation::PreviewRange;

#[derive(Default)]
pub(super) struct WorkAreas {
    program: Option<WorkArea>,
    clips: Option<WorkArea>,
}

struct WorkArea {
    uri: String,
    scope: (Option<usize>, Option<NodeId>),
    range: Option<PreviewRange>,
}

fn scope(session: &DesignerSession, all: bool) -> (Option<usize>, Option<NodeId>) {
    (
        session.rust_map().map(|map| map.invocation.start),
        (!all)
            .then_some(session.active_animation_program())
            .flatten(),
    )
}

impl DesignerWorkspace {
    pub(in crate::workspace) fn preview_range_for_view(
        &self,
        session: &DesignerSession,
    ) -> Option<PreviewRange> {
        let ui = self.timeline_ui.borrow();
        let all = ui.compare_clips;
        let native = session
            .animation
            .preview_range
            .filter(|_| session.animation.play_all == all);
        if session.is_playing() {
            return native;
        }
        let saved = if all {
            &ui.ranges.clips
        } else {
            &ui.ranges.program
        };
        let range = saved
            .as_ref()
            .filter(|saved| {
                saved.uri == session.document_uri() && saved.scope == scope(session, all)
            })
            .map_or(native, |saved| saved.range)?;
        range.fit(
            session
                .preview_range_limit(session.active_animation_program(), all)
                .ok()?,
        )
    }

    fn set_preview_range(&self, range: Option<PreviewRange>) -> Result<(), String> {
        let session = self.session.borrow();
        if session.is_playing() {
            return Err("Pause playback before changing the preview range".into());
        }
        let all = self.comparing_animation_clips();
        if let Some(range) = range {
            let limit = session.preview_range_limit(session.active_animation_program(), all)?;
            range.validate(limit).map_err(|error| error.to_string())?;
        }
        let setting = Some(WorkArea {
            uri: session.document_uri().into(),
            scope: scope(&session, all),
            range,
        });
        let mut ui = self.timeline_ui.borrow_mut();
        if all {
            ui.ranges.clips = setting;
        } else {
            ui.ranges.program = setting;
        }
        Ok(())
    }

    pub(in crate::workspace) fn show_preview_range(self: &Rc<Self>) -> Result<(), String> {
        let host = self
            .root
            .get_control::<ContentControl>("TimelineInspectorHost");
        if host
            .content()
            .as_ref()
            .and_then(Control::from_boxed)
            .is_some_and(|control| control.name().as_deref() == Some("TimelineRangeEditor"))
        {
            self.close_preview_range();
            return Ok(());
        }
        let session = self.session.borrow();
        let all = self.comparing_animation_clips();
        let limit = session.preview_range_limit(session.active_animation_program(), all)?;
        let context = TimelineContext::new(&session);
        let current = self
            .preview_range_for_view(&session)
            .and_then(|range| range.fit(limit));
        let default = PreviewRange {
            start: 0.0,
            end: limit.unwrap_or_else(|| self.timeline_view_duration()),
            looping: true,
        };
        drop(session);
        let range = current.unwrap_or(default);
        let body = StackPanel::new();
        body.set_name(Some("TimelineRangeEditor".into()));
        body.set_margin(Thickness::new(4.0, 3.0, 4.0, 5.0));
        body.set_spacing(6.0);
        let fields = StackPanel::new();
        fields.set_spacing(8.0);
        fields.set_orientation(Orientation::Horizontal);
        let enabled = range_check(
            &fields,
            "Limit playback to preview range",
            current.is_some(),
        );
        let start = range_field(&fields, "In (s)", range.start);
        let end = range_field(&fields, "Out (s)", range.end);
        let looping = range_check(&fields, "Loop range", range.looping);
        let error = TextBlock::new();
        error.set_text_wrapping(ferroui_base::media::TextWrapping::Wrap);
        error.set_is_visible(false);
        let apply = super::clips::clip_tool(crate::icons::CHECK, "Apply range");
        apply.set_vertical_alignment(VerticalAlignment::Center);
        let weak = Rc::downgrade(self);
        let feedback = error.downgrade();
        let range_context = context.clone();
        apply.click(move |_, _| {
            let Some(workspace) = weak.upgrade() else {
                return;
            };
            let result = range_context.check(&workspace).and_then(|()| {
                if workspace.comparing_animation_clips() != all {
                    return Err("The timeline clock changed while editing its range".into());
                }
                let range = (enabled.is_checked() == Some(true))
                    .then(|| {
                        let parse = |field: &TextBox| {
                            field
                                .text()
                                .unwrap_or_default()
                                .trim()
                                .parse::<f64>()
                                .map_err(|_| "Enter In and Out in seconds".to_owned())
                        };
                        Ok::<_, String>(PreviewRange {
                            start: parse(&start)?,
                            end: parse(&end)?,
                            looping: looping.is_checked() == Some(true),
                        })
                    })
                    .transpose()?;
                workspace.set_preview_range(range)
            });
            match result {
                Ok(()) => workspace.close_preview_range(),
                Err(message) => {
                    if let Some(feedback) = feedback.upgrade() {
                        feedback.set_text(Some(&message));
                        feedback.set_is_visible(true);
                    }
                }
            }
        });
        fields.children().add(apply.upcast::<Control>());
        let cancel = super::clips::clip_tool(crate::icons::CLOSE, "Close range settings");
        cancel.set_vertical_alignment(VerticalAlignment::Center);
        let weak = Rc::downgrade(self);
        cancel.click(move |_, _| {
            if let Some(workspace) = weak.upgrade() {
                workspace.close_preview_range();
            }
        });
        fields.children().add(cancel.upcast::<Control>());
        body.children().add(fields.upcast::<Control>());
        body.children().add(error.upcast::<Control>());
        self.append_preview_markers(&body, context, all);
        let weak = Rc::downgrade(self);
        body.add_handler(InputElement::key_down_event(), move |_, event| {
            if event.key == Key::Escape {
                if let Some(workspace) = weak.upgrade() {
                    workspace.close_preview_range();
                }
                event.set_handled(true);
            }
        });
        host.set_content(Some(Control::boxed(&body)));
        host.invalidate_measure();
        Ok(())
    }

    fn close_preview_range(self: &Rc<Self>) {
        self.refresh(true);
        self.root.get_control::<Button>("TimelineRange").focus();
    }

    pub(super) fn connect_preview_range(
        self: &Rc<Self>,
        ruler: &TimelineLane,
        context: &TimelineContext,
    ) {
        let weak = Rc::downgrade(self);
        let context = context.clone();
        ruler.add_handler(InputElement::key_down_event(), move |_, args| {
            if !matches!(args.key, Key::I | Key::O) || !args.key_modifiers.is_empty() {
                return;
            }
            let Some(workspace) = weak.upgrade() else {
                return;
            };
            let result = context.check(&workspace).and_then(|()| {
                let session = workspace.session.borrow();
                let limit = session.preview_range_limit(
                    session.active_animation_program(),
                    workspace.comparing_animation_clips(),
                )?;
                let end = limit.unwrap_or_else(|| workspace.timeline_view_duration());
                let mut range =
                    workspace
                        .preview_range_for_view(&session)
                        .unwrap_or(PreviewRange {
                            start: 0.0,
                            end,
                            looping: true,
                        });
                let time = session.time();
                if args.key == Key::I {
                    range.start = time;
                    if range.end <= time {
                        range.end = end;
                    }
                } else {
                    range.end = time;
                    if range.start >= time {
                        range.start = 0.0;
                    }
                }
                drop(session);
                workspace.set_preview_range(Some(range))
            });
            args.set_handled(true);
            if result.is_ok() {
                workspace.refresh(true);
            }
            workspace.report(result);
        });
    }
}

fn range_check(parent: &StackPanel, title: &str, checked: bool) -> Ref<CheckBox> {
    let control = CheckBox::new();
    control.set_vertical_alignment(VerticalAlignment::Center);
    control.set_content(Some(Rc::new(title.to_owned())));
    control.set_is_checked(Some(checked));
    AutomationProperties::set_name(&control, Some(title));
    parent.children().add(control.clone().upcast::<Control>());
    control
}

fn range_field(parent: &StackPanel, label: &str, time: f64) -> Ref<TextBox> {
    let caption = TextBlock::new();
    caption.set_text(Some(label));
    caption.set_vertical_alignment(VerticalAlignment::Center);
    parent.children().add(caption.upcast::<Control>());
    let input = TextBox::new();
    input.classes().add("designer-field");
    input.set_vertical_alignment(VerticalAlignment::Center);
    input.set_width(72.0);
    input.set_text(Some(&time.to_string()));
    AutomationProperties::set_name(&input, Some(label));
    parent.children().add(input.clone().upcast::<Control>());
    input
}
