#[path = "channel_picker.rs"]
mod channel_picker;

use super::*;
use ferroui_controls::{MenuFlyout, MenuItem, MenuItemToggleType};
use ferroui_designer_animation::{CurvePoint, KeyframeComponentEdit, KeyframeEdit};

impl DesignerWorkspace {
    pub(in crate::workspace) fn connect_timeline_curves(self: &Rc<Self>) {
        let button = self.root.get_control::<Button>("TimelineCurves");
        let menu = MenuFlyout::new();
        for (label, metric) in [
            ("Keyframes", None),
            ("Value curves", Some(CurveMetric::Value)),
            ("Velocity curves", Some(CurveMetric::Velocity)),
        ] {
            let item = MenuItem::new();
            item.set_header(Some(Rc::new(label.to_owned())));
            item.set_toggle_type(MenuItemToggleType::Radio);
            item.set_is_checked(metric.is_none());
            let weak = Rc::downgrade(self);
            item.click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.set_timeline_curves(metric);
                }
            });
            menu.items().add(Some(Control::boxed(item)));
        }
        button.set_flyout(Some(menu.upcast()));
        crate::chrome::apply_designer_menu_resources(&button, &self.root);
    }

    fn set_timeline_curves(self: &Rc<Self>, metric: Option<CurveMetric>) {
        self.timeline_ui.borrow_mut().curves = metric;
        let button = self.root.get_control::<Button>("TimelineCurves");
        button.classes().set("designer-motion", metric.is_some());
        let label = match metric {
            None => "Animation graph · Keyframes",
            Some(CurveMetric::Value) => "Animation graph · Value",
            Some(CurveMetric::Velocity) => "Animation graph · Velocity",
        };
        AutomationProperties::set_name(&button, Some(label));
        ToolTip::set_tip(&button, Some(Rc::new(label.to_owned())));
        self.rebuild_tracks();
    }

    pub(super) fn fit_timeline_selection(&self) {
        let range = {
            let ui = self.timeline_ui.borrow();
            (ui.curves.is_some() && ui.selection_explicit)
                .then(|| {
                    ui.selection
                        .iter()
                        .map(|key| key.time)
                        .fold(None, |range, time| {
                            Some(range.map_or((time, time), |(first, last): (f64, f64)| {
                                (first.min(time), last.max(time))
                            }))
                        })
                })
                .flatten()
        };
        let view = range.map(|(first, last)| {
            let duration = self.session.borrow().timeline().duration;
            let padding = ((last - first) * 0.1).max(duration * 0.03);
            ((first - padding).max(0.0), (last + padding).min(duration))
        });
        self.set_timeline_view(view);
    }

    pub(super) fn attach_timeline_curve(
        self: &Rc<Self>,
        lane: &TimelineLane,
        track: &Track,
        context: &TimelineContext,
        metric: CurveMetric,
        channel: TrackCurveChannel,
    ) {
        let Some(program) = context.program else {
            return;
        };
        let keys: Vec<_> = track
            .keyframes
            .iter()
            .filter_map(|key| channel.authored_key_value(&key.value)
                .map(|value| CurvePoint::new(key.time, value)))
            .collect();
        let target = track.target;
        let query = Rc::downgrade(self);
        let query_context = context.clone();
        let query_property = track.property.clone();
        let owner = Rc::downgrade(self);
        let context = context.clone();
        let property = track.property.clone();
        let edit_channel = channel.clone();
        lane.configure_curve(
            metric,
            channel.clone(),
            keys,
            move |window| {
                let workspace = query.upgrade().ok_or("The Timeline closed")?;
                query_context.check(&workspace)?;
                let session = workspace.session.borrow();
                let points = session.animation_curve_channel(
                    program,
                    target,
                    &query_property,
                    channel.clone(),
                    window,
                )?;
                metric.measure(points).map_err(|error| error.to_string())
            },
            move |from, time, value| {
                if let Some(workspace) = owner.upgrade() {
                    let selection = SelectedKey {
                        target,
                        property: property.clone(),
                        time: from,
                    };
                    let result = workspace
                        .apply_curve_drag(&context, &selection, time, &KeyframeComponentEdit {
                            channel: edit_channel.clone(), value,
                        })
                        .and_then(|time| workspace.session.borrow_mut().seek(time));
                    if result.is_ok() || context.check(&workspace).is_err() {
                        workspace.refresh(true);
                    }
                    workspace.report(result);
                }
            },
        );
    }

    fn apply_curve_drag(
        &self,
        context: &TimelineContext,
        selected: &SelectedKey,
        time: f64,
        edit: &KeyframeComponentEdit,
    ) -> Result<f64, String> {
        context.check(self)?;
        let session = self.session.borrow();
        if !session.can_manipulate() {
            return Err("Wait for the current editable animation preview".into());
        }
        let program = context.program.ok_or("Choose an animation")?;
        let easing = session
            .timeline()
            .track(selected.target, &selected.property)
            .and_then(|track| track.keyframes.iter().find(|key| key.time == selected.time))
            .map(|key| key.easing.to_xaml())
            .ok_or("Choose an authored curve key")?;
        let value = session.program_keyframe_component(program, selected.target, &selected.property, selected.time, edit)?;
        drop(session);
        self.apply_timeline_keyframe(context, selected, time, &value, &easing)
    }

    pub(super) fn apply_timeline_keyframe(
        &self,
        context: &TimelineContext,
        selected: &SelectedKey,
        time: f64,
        value: &str,
        easing: &str,
    ) -> Result<f64, String> {
        context.check(self)?;
        let id = context.program.ok_or("Choose an animation")?;
        self.session.borrow_mut().edit_program_keyframe(
            id,
            selected.target,
            &selected.property,
            KeyframeEdit::Upsert {
                previous: Some(selected.time),
                time,
                value,
                easing,
            },
        )?;
        self.remember_keyframe(SelectedKey {
            time,
            ..selected.clone()
        });
        Ok(self
            .timeline_ui
            .borrow()
            .selected
            .as_ref()
            .map_or(time, |key| key.time))
    }
}
