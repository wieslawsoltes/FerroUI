use super::HostedAnimations;
use ferroui_base::Ref;
use ferroui_base::animation::{Animatable, Animation};
use ferroui_designer_animation::native::validate_animation_target;
use ferroui_designer_core::NodeId;
use ferroui_remote_protocol::designer::DesignAnimationRequestMessage;
use std::collections::BTreeSet;

type TargetedClip = (i64, Vec<i64>);

impl HostedAnimations {
    pub(super) fn sample(
        &mut self,
        command: &DesignAnimationRequestMessage,
    ) -> Result<(bool, f64), String> {
        self.claim(command.revision, command.request)?;
        let mut clips = self.requested_clips(command)?;
        if clips.is_empty() {
            self.current.clear();
            self.playback.stop();
            return Ok((false, 0.0));
        }
        // Mount overlapping animation-priority values in authored style order.
        clips.sort_by_key(|(program, _)| self.programs[program].source_utf16);
        let advance = !command.restart && self.current == clips && self.playback.is_active();
        let elapsed = command.elapsed;
        if let Some(route) = command.program.and_then(|id| self.state_routes.get(id)) {
            let (program, targets) = &clips[0];
            let entry = &self.programs[program];
            if *targets != entry.default_targets {
                return Err("A state route uses its authored targets".into());
            }
            if !elapsed.is_finite()
                || elapsed < 0.0
                || (command.restart && elapsed > entry.duration)
            {
                return Err("Choose a time within the state route duration".into());
            }
            let elapsed = elapsed.min(entry.duration);
            let complete = if advance {
                self.playback.advance_to(elapsed)?
            } else {
                self.playback.seek(route.bindings(), elapsed)?
            };
            self.current = clips;
            return Ok((complete || elapsed == entry.duration, elapsed));
        }
        if advance {
            return Ok((self.playback.advance_to(elapsed)?, elapsed));
        }
        let end = clips.iter().fold(0.0_f64, |end, (program, _)| {
            end.max(
                self.programs[program]
                    .total_duration
                    .unwrap_or(f64::INFINITY),
            )
        });
        if elapsed > end {
            return Err("Choose a time within the animation's elapsed duration".into());
        }
        let mut bindings = Vec::new();
        for (program, targets) in &clips {
            let animation = self
                .index
                .animation(*program as NodeId)
                .ok_or("The native animation was retired")?;
            self.bind_targets(animation, targets, &mut bindings)?;
        }
        let complete = self.playback.seek(bindings, elapsed)?;
        self.current = clips;
        Ok((complete, elapsed))
    }

    fn requested_clips(
        &self,
        command: &DesignAnimationRequestMessage,
    ) -> Result<Vec<TargetedClip>, String> {
        let resolve = |id, explicit: Option<&[i64]>| {
            let program = self
                .programs
                .get(&id)
                .ok_or("Choose a current native animation program")?;
            Ok::<_, String>((id, explicit.unwrap_or(&program.default_targets).to_vec()))
        };
        if let Some(clips) = &command.clips {
            if command.program.is_some() || command.targets.is_some() || clips.is_empty() {
                return Err("Choose a single animation or a nonempty set of elapsed clips".into());
            }
            let mut seen = BTreeSet::new();
            return clips
                .iter()
                .map(|clip| {
                    if !seen.insert(clip.program)
                        || !self
                            .programs
                            .get(&clip.program)
                            .is_some_and(|program| program.clip.is_some())
                    {
                        return Err(
                            "Choose distinct current native clips for combined playback".into()
                        );
                    }
                    resolve(clip.program, clip.targets.as_deref())
                })
                .collect();
        }
        command
            .program
            .into_iter()
            .map(|id| resolve(id, command.targets.as_deref()))
            .collect()
    }

    fn bind_targets(
        &self,
        animation: &Ref<Animation>,
        targets: &[i64],
        bindings: &mut Vec<(Ref<Animation>, Ref<Animatable>)>,
    ) -> Result<(), String> {
        if targets.is_empty() || targets.len() != targets.iter().collect::<BTreeSet<_>>().len() {
            return Err("Choose distinct live preview targets for the animation".into());
        }
        for id in targets {
            let target = self
                .targets
                .get(id)
                .and_then(|target| target.cast::<Animatable>())
                .ok_or("Choose current application preview targets")?;
            validate_animation_target(animation, &target, "the preview target")?;
            bindings.push((animation.clone(), target));
        }
        Ok(())
    }
}
