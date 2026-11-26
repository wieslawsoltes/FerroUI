use super::*;
use ferroui_base::layout::VerticalAlignment;
use ferroui_controls::ScrollViewer;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_designer_animation::{PreviewMarker, PreviewMarkerEdit};

pub(super) struct MarkerEditor {
    context: TimelineContext,
    all: bool,
    rows: WeakRef<StackPanel>,
    list: WeakRef<ScrollViewer>,
    input: WeakRef<TextBox>,
    add: WeakRef<Button>,
    presented: Option<(Vec<PreviewMarker>, bool)>,
}

impl DesignerWorkspace {
    pub(super) fn append_preview_markers(
        self: &Rc<Self>,
        body: &StackPanel,
        context: TimelineContext,
        all: bool,
    ) {
        let form = StackPanel::new();
        form.set_orientation(Orientation::Horizontal);
        form.set_spacing(4.0);
        let input = TextBox::new();
        input.set_name(Some("TimelineMarkerName".into()));
        input.set_placeholder_text(Some("Marker name"));
        input.set_max_length(48);
        input.set_width(176.0);
        input.classes().add("designer-field");
        AutomationProperties::set_name(&input, Some("Marker name"));
        let add = super::clips::clip_tool(crate::icons::ADD, "Add or update marker at playhead");
        add.set_name(Some("TimelineMarkerAdd".into()));
        add.set_vertical_alignment(VerticalAlignment::Center);
        let rows = StackPanel::new();
        rows.set_name(Some("TimelineMarkerList".into()));
        rows.set_orientation(Orientation::Horizontal);
        rows.set_spacing(10.0);
        let list = ScrollViewer::new();
        list.set_margin(Thickness::new(10.0, 0.0, 0.0, 0.0));
        list.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Auto);
        list.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Disabled);
        list.set_content(Some(Control::boxed(&rows)));
        let weak = Rc::downgrade(self);
        let name = input.downgrade();
        let marker_context = context.clone();
        let submit = Rc::new(move || {
            let (Some(workspace), Some(input)) = (weak.upgrade(), name.upgrade()) else {
                return;
            };
            let name = input.text().unwrap_or_default();
            if name.trim().is_empty() {
                return;
            }
            let result = workspace
                .check_marker_context(&marker_context, all)
                .and_then(|()| {
                    let mut session = workspace.session.borrow_mut();
                    let time = if all && !session.animation.play_all {
                        0.0
                    } else {
                        session.time()
                    };
                    session.edit_preview_marker(None, all, PreviewMarkerEdit::Set { name, time })
                });
            if result.is_ok() {
                input.set_text(None);
            }
            workspace.report(result);
            workspace.refresh(false);
            input.focus();
        });
        let click = submit.clone();
        add.click(move |_, _| click());
        input.add_handler(InputElement::key_down_event(), move |_, args| {
            if args.key == Key::Enter {
                submit();
                args.set_handled(true);
            }
        });
        form.children().add(input.clone().upcast::<Control>());
        form.children().add(add.clone().upcast::<Control>());
        let marker_bar = Grid::new();
        marker_bar.set_column_definitions(
            ColumnDefinitions::parse("Auto,*").expect("valid marker row columns"),
        );
        marker_bar.children().add(form.upcast::<Control>());
        Grid::set_column(&list, 1);
        marker_bar.children().add(list.clone().upcast::<Control>());
        body.children().add(marker_bar.upcast::<Control>());
        self.timeline_ui.borrow_mut().marker_editor = Some(MarkerEditor {
            context,
            all,
            rows: rows.downgrade(),
            list: list.downgrade(),
            input: input.downgrade(),
            add: add.downgrade(),
            presented: None,
        });
        self.refresh_preview_markers();
    }

    pub(super) fn connect_preview_markers(
        self: &Rc<Self>,
        ruler: &TimelineLane,
        context: &TimelineContext,
        all: bool,
    ) {
        let workspace = Rc::downgrade(self);
        let context = context.clone();
        ruler.on_marker(move |name| {
            if let Some(workspace) = workspace.upgrade() {
                workspace.jump_to_marker(&context, all, name);
            }
        });
    }

    pub(in crate::workspace) fn refresh_preview_markers(self: &Rc<Self>) {
        let all = self.comparing_animation_clips();
        let session = self.session.borrow();
        let markers = session.preview_markers(None, all);
        let enabled = markers.is_ok() && !session.is_playing();
        let markers = markers.unwrap_or_default();
        for ruler in self
            .timeline_lanes
            .borrow()
            .iter()
            .filter_map(WeakRef::upgrade)
            .filter(|lane| lane.is_ruler())
        {
            ruler.set_preview_markers(&markers);
        }
        drop(session);
        let mut ui = self.timeline_ui.borrow_mut();
        let Some(editor) = &mut ui.marker_editor else {
            return;
        };
        if editor.context.check(self).is_ok() && editor.all == all {
            editor.show(self, markers, enabled);
        }
    }

    fn check_marker_context(&self, context: &TimelineContext, all: bool) -> Result<(), String> {
        context.check(self)?;
        if self.comparing_animation_clips() != all {
            return Err("The preview clock changed while using a marker".into());
        }
        Ok(())
    }

    fn jump_to_marker(self: &Rc<Self>, context: &TimelineContext, all: bool, name: &str) {
        let result = self.check_marker_context(context, all).and_then(|()| {
            self.session
                .borrow_mut()
                .seek_preview_marker(None, all, name)
        });
        self.report(result);
        self.refresh(false);
    }
}

impl MarkerEditor {
    fn show(
        &mut self,
        workspace: &Rc<DesignerWorkspace>,
        markers: Vec<PreviewMarker>,
        enabled: bool,
    ) {
        let Some(rows) = self.rows.upgrade() else {
            return;
        };
        if self
            .presented
            .as_ref()
            .is_some_and(|(previous, active)| *previous == markers && *active == enabled)
        {
            return;
        }
        if let Some(add) = self.add.upgrade() {
            add.set_is_enabled(enabled);
        }
        if let Some(input) = self.input.upgrade() {
            input.set_is_enabled(enabled);
        }
        if let Some(list) = self.list.upgrade() {
            list.set_is_visible(!markers.is_empty());
        }
        rows.children().clear();
        for marker in &markers {
            rows.children()
                .add(self.row(workspace, marker, enabled).upcast::<Control>());
        }
        self.presented = Some((markers, enabled));
    }

    fn row(
        &self,
        workspace: &Rc<DesignerWorkspace>,
        marker: &PreviewMarker,
        enabled: bool,
    ) -> Ref<StackPanel> {
        let row = StackPanel::new();
        row.set_orientation(Orientation::Horizontal);
        row.set_spacing(4.0);
        let jump = button(&marker.name);
        jump.classes().add("designer-timeline-tool");
        jump.set_max_width(200.0);
        jump.set_is_enabled(enabled);
        let title = format!("Seek to {} at {} seconds", marker.name, marker.time);
        AutomationProperties::set_name(&jump, Some(&title));
        ToolTip::set_tip(&jump, Some(Rc::new(title)));
        let weak = Rc::downgrade(workspace);
        let context = self.context.clone();
        let all = self.all;
        let name = marker.name.clone();
        jump.click(move |_, _| {
            if let Some(workspace) = weak.upgrade() {
                workspace.jump_to_marker(&context, all, &name);
            }
        });
        let seconds = TextBlock::new();
        seconds.set_text(Some(&format!("{:.3} s", marker.time)));
        seconds.set_vertical_alignment(VerticalAlignment::Center);
        let remove = super::clips::clip_tool(
            crate::icons::REMOVE,
            &format!("Remove marker {}", marker.name),
        );
        remove.set_is_enabled(enabled);
        let weak = Rc::downgrade(workspace);
        let context = self.context.clone();
        let name = marker.name.clone();
        let input = self.input.clone();
        remove.click(move |_, _| {
            let Some(workspace) = weak.upgrade() else {
                return;
            };
            let result = workspace
                .check_marker_context(&context, all)
                .and_then(|()| {
                    workspace.session.borrow_mut().edit_preview_marker(
                        None,
                        all,
                        PreviewMarkerEdit::Remove { name: name.clone() },
                    )
                });
            workspace.report(result);
            workspace.refresh(false);
            if let Some(input) = input.upgrade() {
                input.focus();
            }
        });
        row.children().add(jump.upcast::<Control>());
        row.children().add(seconds.upcast::<Control>());
        row.children().add(remove.upcast::<Control>());
        row
    }
}
