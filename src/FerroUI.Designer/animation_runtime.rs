//! Bind Designer Studio's authored nodes to the shared native animation preview.
use crate::{runtime::RuntimePreview, PreviewStatus};
use ferroui_base::animation::Animatable;
use ferroui_base::StyledElement;
use ferroui_designer_animation::native::{
    animation_timing, validate_animation_target, NativePlayback, NativeProgramIndex,
    NativeSourceIndex,
};
use ferroui_designer_animation::{AnimationProgram, CurvePoint, CurveWindow, Timeline};
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

    pub(crate) fn duration(&self, id: NodeId) -> Option<f64> {
        let value = self.index.animation(id)?.duration().total_seconds();
        (value.is_finite() && value > 0.0).then_some(value)
    }

    pub(crate) fn timing(&self, id: NodeId) -> Option<Timeline> {
        animation_timing(self.index.animation(id)?)
    }

    pub(crate) fn scalar_curve(
        &self,
        program: &AnimationProgram,
        preview: &RuntimePreview,
        target: NodeId,
        property: &str,
        window: CurveWindow,
    ) -> Result<Vec<CurvePoint>, String> {
        let animation = self.index.animation(program.id).ok_or("The native animation is unavailable")?;
        let track = program.timeline.as_ref().and_then(|timeline| timeline.track(target, property))
            .ok_or("Choose an authored animation track")?;
        let object = if program.targets.contains(&target) {
            preview.visual(target).and_then(|visual| visual.cast::<Animatable>())
        } else if self.uses_factory_target(program.id) && target == program.primary_target() {
            preview.root().map(|root| root.upcast::<Animatable>())
        } else {
            None
        }.ok_or("Choose a live preview target for this curve")?;
        let registered = ferroui_inspect::Inspector::property(&object, property)
            .map_err(|error| error.to_string())?;
        let duration = animation.duration().total_seconds();
        let times = window.sample_times(duration, track.keyframes.iter().map(|key| key.time))
            .map_err(|error| error.to_string())?;
        let positions = times.iter().map(|time| time / duration).collect::<Vec<_>>();
        let values = animation.sample_track_for_preview(&object, registered, &positions)
            .map_err(|error| error.to_string())?;
        times.into_iter().zip(values).map(|(time, value)| {
            let value = value.as_ref().downcast_ref::<f64>().copied()
                .or_else(|| value.as_ref().downcast_ref::<f32>().map(|value| f64::from(*value)))
                .filter(|value| value.is_finite()).ok_or("The native animation does not produce a finite scalar")?;
            Ok(CurvePoint { time, value })
        }).collect::<Result<_, &str>>().map_err(str::to_owned)
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
