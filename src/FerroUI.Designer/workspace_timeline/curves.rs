use super::*;
use ferroui_designer_animation::{AnimationValue, CurvePoint, KeyframeEdit};

impl DesignerWorkspace {
    pub(in crate::workspace) fn connect_timeline_curves(self: &Rc<Self>) {
        let button = self.root.get_control::<Button>("TimelineCurves");
        let weak = Rc::downgrade(self);
        button.click(move |_, _| {
            if let Some(this) = weak.upgrade() {
                let enabled = !this.timeline_ui.borrow().curves;
                this.set_timeline_curves(enabled);
            }
        });
    }

    fn set_timeline_curves(self: &Rc<Self>, enabled: bool) {
        self.timeline_ui.borrow_mut().curves = enabled;
        let button = self.root.get_control::<Button>("TimelineCurves");
        button.classes().set("designer-motion", enabled);
        let label = if enabled {
            "Show keyframe lanes"
        } else {
            "Show value curves"
        };
        AutomationProperties::set_name(&button, Some(label));
        ToolTip::set_tip(&button, Some(Rc::new(label.to_owned())));
        self.rebuild_tracks();
    }

    pub(super) fn fit_timeline_selection(&self) {
        let range = {
            let ui = self.timeline_ui.borrow();
            (ui.curves && ui.selection_explicit)
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
    ) -> bool {
        let Some(program) = context.program else {
            return false;
        };
        let keys: Vec<_> = track
            .keyframes
            .iter()
            .filter_map(|key| {
                if let AnimationValue::Number(value) = key.value {
                    Some(CurvePoint {
                        time: key.time,
                        value,
                    })
                } else {
                    None
                }
            })
            .collect();
        if keys.is_empty() {
            return false;
        }

        let target = track.target;
        let query = Rc::downgrade(self);
        let query_context = context.clone();
        let query_property = track.property.clone();
        let owner = Rc::downgrade(self);
        let context = context.clone();
        let property = track.property.clone();
        lane.configure_curve(
            keys,
            move |window| {
                let workspace = query.upgrade().ok_or("The Timeline closed")?;
                query_context.check(&workspace)?;
                let session = workspace.session.borrow();
                session.animation_curve(program, target, &query_property, window)
            },
            move |from, time, value| {
                if let Some(workspace) = owner.upgrade() {
                    let selection = SelectedKey {
                        target,
                        property: property.clone(),
                        time: from,
                    };
                    let result = workspace
                        .apply_curve_drag(&context, &selection, time, value)
                        .and_then(|time| workspace.session.borrow_mut().seek(time));
                    if result.is_ok() || context.check(&workspace).is_err() {
                        workspace.refresh(true);
                    }
                    workspace.report(result);
                }
            },
        );
        true
    }

    fn apply_curve_drag(
        &self,
        context: &TimelineContext,
        selected: &SelectedKey,
        time: f64,
        value: f64,
    ) -> Result<f64, String> {
        context.check(self)?;
        let session = self.session.borrow();
        if !session.can_manipulate() {
            return Err("Wait for the current editable animation preview".into());
        }
        let easing = session
            .timeline()
            .track(selected.target, &selected.property)
            .and_then(|track| track.keyframes.iter().find(|key| key.time == selected.time))
            .map(|key| key.easing.to_xaml())
            .ok_or("Choose an authored curve key")?;
        drop(session);
        self.apply_timeline_keyframe(context, selected, time, &value.to_string(), &easing)
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
