//! Bind Designer Studio's authored nodes to the shared native animation preview.
use crate::{runtime::RuntimePreview, PreviewStatus};
use ferroui_base::animation::Animatable;
use ferroui_base::animation::animators::TransformAnimator;
use ferroui_base::StyledElement;
use ferroui_designer_animation::native::{
    animation_timing, sample_curve_track, sample_keyframe_value, validate_animation_target, NativePlayback, NativeProgramIndex,
    NativeSourceIndex,
};
use ferroui_designer_animation::{AnimationProgram, CurvePoint, CurveWindow, Timeline, TrackCurveChannel};
use ferroui_designer_core::{Document, NodeId};
use std::collections::BTreeSet;

#[derive(Default)]
pub(crate) struct NativeAnimations {
    index: NativeProgramIndex,
    factory_programs: BTreeSet<NodeId>,
    active: Vec<NodeId>,
    playback: NativePlayback,
}

impl NativeAnimations {
    pub(crate) fn has_pose(&self) -> bool {
        !self.active.is_empty()
    }

    pub(crate) fn stop(&mut self) {
        self.playback.stop();
        self.active.clear();
    }

    pub(crate) fn clear(&mut self) {
        self.stop();
        self.index = NativeProgramIndex::default();
        self.factory_programs.clear();
    }

    pub(crate) fn uses_factory_target(&self, id: NodeId) -> bool {
        self.factory_programs.contains(&id)
    }

    pub(crate) fn uses_custom_animators(&self, id: NodeId) -> bool {
        self.index.uses_custom_animators(id)
    }

    pub(crate) fn uses_transform_components(&self, id: NodeId) -> bool {
        self.index.uses_transform_components(id)
    }

    pub(crate) fn transform_component(
        &self,
        id: NodeId,
        name: &str,
    ) -> Option<&'static ferroui_base::FerroProperty> {
        self.index.transform_component(id, name)
    }

    pub(crate) fn transform_base_value(
        &self,
        program: &AnimationProgram,
        preview: &RuntimePreview,
        node: NodeId,
        property: &'static ferroui_base::FerroProperty,
    ) -> Option<f64> {
        let target = self.track_target(program, preview, node).ok()?;
        TransformAnimator::preview_base_value(&target, property)
    }

    pub(crate) fn duration(&self, id: NodeId) -> Option<f64> {
        let value = self.index.animation(id)?.duration().total_seconds();
        (value.is_finite() && value > 0.0).then_some(value)
    }

    pub(crate) fn timing(&self, id: NodeId) -> Option<Timeline> {
        animation_timing(self.index.animation(id)?)
    }

    pub(crate) fn curve(
        &self,
        program: &AnimationProgram,
        preview: &RuntimePreview,
        target: NodeId,
        property: &str,
        channel: TrackCurveChannel,
        window: CurveWindow,
    ) -> Result<Vec<CurvePoint>, String> {
        let animation = self.index.animation(program.id).ok_or("The native animation is unavailable")?;
        let timeline = program.timeline.as_ref().ok_or("Choose a typed animation program")?;
        let track = timeline.track(target, property).ok_or("Choose an authored animation track")?;
        if !TrackCurveChannel::for_track(track, timeline.duration, timeline.fill).contains(&channel) {
            return Err("Choose an authored animation track and one of its curve channels".into());
        }
        let object = self.track_target(program, preview, target)?;
        sample_curve_track(animation, &object, property, channel, window)
    }

    pub(crate) fn sample_keyframe(
        &self,
        program: &AnimationProgram,
        preview: &RuntimePreview,
        target: NodeId,
        property: &str,
        time: f64,
    ) -> Result<ferroui_base::BoxedValue, String> {
        let animation = self.index.animation(program.id).ok_or("The native animation is unavailable")?;
        let object = self.track_target(program, preview, target)?;
        sample_keyframe_value(animation, &object, property, time)
    }

    fn track_target(
        &self, program: &AnimationProgram, preview: &RuntimePreview, target: NodeId,
    ) -> Result<ferroui_base::Ref<Animatable>, String> {
        if program.targets.contains(&target) {
            preview.visual(target).and_then(|visual| visual.cast::<Animatable>())
        } else if self.uses_factory_target(program.id) && target == program.primary_target() {
            preview.root().map(|root| root.upcast::<Animatable>())
        } else {
            None
        }.ok_or_else(|| "Choose a live preview target for this track".into())
    }

    pub(crate) fn refresh(
        &mut self,
        document: &Document,
        preview: &RuntimePreview,
        programs: &[AnimationProgram],
    ) {
        self.clear();
        if programs.is_empty()
            || !matches!(preview.status, PreviewStatus::Ready { revision } if revision == document.revision())
        {
            return;
        }
        let Some(root) = preview.root() else { return };
        let source = NativeSourceIndex::new(document, &root, preview.uri());
        let controls = std::iter::once(root.upcast::<StyledElement>()).chain(
            document
                .ast()
                .elements()
                .filter_map(|node| preview.visual(node.id)?.cast::<StyledElement>()),
        );
        self.index = NativeProgramIndex::discover(document, &source, programs, controls);
        if document
            .ast()
            .node(document.ast().root())
            .is_some_and(|root| root.local_name() == "ResourceDictionary")
        {
            self.factory_programs.extend(
                programs
                    .iter()
                    .filter(|program| {
                        program.style.is_none()
                            && program.targets.is_empty()
                            && self.index.animation(program.id).is_some()
                    })
                    .map(|program| program.id),
            );
        }
    }

    pub(crate) fn seek(
        &mut self,
        programs: &[AnimationProgram],
        preview: &RuntimePreview,
        elapsed: f64,
    ) -> Result<(), String> {
        self.restart(programs, preview, elapsed).map(|_| ())
    }

    pub(crate) fn advance_to(
        &mut self,
        programs: &[AnimationProgram],
        preview: &RuntimePreview,
        elapsed: f64,
    ) -> Result<bool, String> {
        if !self
            .active
            .iter()
            .copied()
            .eq(programs.iter().map(|program| program.id))
            || !self.playback.is_active()
            || elapsed < self.playback.elapsed()
        {
            return self.restart(programs, preview, elapsed);
        }
        self.playback.advance_to(elapsed)
    }

    fn restart(
        &mut self,
        programs: &[AnimationProgram],
        preview: &RuntimePreview,
        elapsed: f64,
    ) -> Result<bool, String> {
        let mut bindings = Vec::new();
        for program in programs {
            let uses_root = program.targets.is_empty() && self.uses_factory_target(program.id);
            if program.targets.is_empty() && !uses_root {
                continue;
            }
            let animation = self.index.animation(program.id).ok_or_else(|| {
                format!(
                    "Native program '{}' is not available in the accepted preview",
                    program.name
                )
            })?;
            if uses_root {
                let target = preview
                    .root()
                    .ok_or("The resource preview factory has no live root")?
                    .upcast::<Animatable>();
                validate_animation_target(animation, &target, "factory preview root")
                    .map_err(|error| format!("Resource program '{}': {error}", program.name))?;
                bindings.push((animation.clone(), target));
            }
            for id in &program.targets {
                let target = preview
                    .visual(*id)
                    .and_then(|visual| visual.cast::<Animatable>())
                    .ok_or("The native animation target is no longer available")?;
                bindings.push((animation.clone(), target));
            }
        }
        let complete = self.playback.seek(bindings, elapsed)?;
        self.active = programs.iter().map(|program| program.id).collect();
        Ok(complete)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::animation::{Animation, FillMode, KeyFrame, SignedDuration};
    use ferroui_base::styling::Setter;
    use ferroui_base::threading::Dispatcher;
    use ferroui_base::Visual;
    use std::rc::Rc;

    #[test]
    fn rejected_native_animation_tick_removes_the_preview_values() {
        let target = Visual::new();
        let opacity = Visual::opacity_property();
        target.add_property_changed_handler(Rc::new(move |event| {
            if event.property() == opacity.as_property() && event.new_value().downcast_ref::<f64>() == Some(&0.5) {
                Dispatcher::current_dispatcher().raise_unhandled_exception(std::io::Error::other("Rejected preview opacity"));
            }
        }));
        let animation = Animation::new();
        animation.set_duration(SignedDuration::from_seconds(1.0));
        animation.set_fill_mode(FillMode::Both);
        for (time, value) in [(0.0, 1.0), (1.0, 0.0)] {
            animation.children().add(KeyFrame::with_key_time(
                SignedDuration::from_seconds(time),
                [Setter::new(opacity, value) as _],
            ));
        }
        let mut playback = NativePlayback::default();
        playback.seek(vec![(animation, target.clone().upcast())], 0.0).unwrap();
        let error = playback.advance_to(0.5).unwrap_err();
        assert!(error.contains("Rejected preview opacity"));
        assert!(!playback.is_active());
        assert_eq!(target.opacity(), 1.0);
    }
}
