use super::{HostedAnimations, KEYFRAME_SAMPLING};
use ferroui_base::animation::Animatable;
use ferroui_designer_animation::native::{sample_keyframe_value, sample_typed_keyframe};
use ferroui_designer_animation::{AnimationBrush, Easing, native_key_time};
use ferroui_designer_core::NodeId;
use ferroui_remote_protocol::designer::{DesignAnimationAuthoringRequest, DesignAnimationKeyframe};

impl HostedAnimations {
    pub(super) fn sample_keyframe(
        &self,
        request: &DesignAnimationAuthoringRequest,
    ) -> Result<DesignAnimationKeyframe, String> {
        if !KEYFRAME_SAMPLING.get() {
            return Err("The application preview did not negotiate keyframe sampling".into());
        }
        if request.previous.is_some() || !request.value.is_empty() || !request.easing.is_empty() {
            return Err(
                "Choose a new sampled keyframe or supply an explicit keyframe draft".into(),
            );
        }
        let program = self
            .programs
            .get(&request.program)
            .ok_or("Choose a current app-hosted animation")?;
        let track = program
            .authoring
            .as_ref()
            .and_then(|authored| {
                authored.tracks.iter().find(|track| {
                    track.can_sample == Some(true)
                        && track.target == request.target
                        && track.property == request.property
                })
            })
            .ok_or("Choose a sampleable authored animation track")?;
        if !(0.0..=program.duration).contains(&request.time) {
            return Err("Choose a key time within the animation duration".into());
        }
        let time = native_key_time(request.time).map_err(|error| error.to_string())?;
        if track
            .keys
            .iter()
            .any(|key| native_key_time(key.time).ok() == Some(time))
        {
            return Err("Another keyframe occupies the destination time".into());
        }
        let id = NodeId::try_from(request.program).map_err(|_| "The native animation retired")?;
        let animation = self
            .index
            .animation(id)
            .ok_or("The native animation retired")?;
        let target = request.sample_target.unwrap_or(track.target);
        let target = self
            .targets
            .get(&target)
            .filter(|_| program.compatible_targets.contains(&target))
            .and_then(|control| control.cast::<Animatable>())
            .ok_or("Choose a compatible live preview target")?;
        let value = if track.keys.first().is_some_and(|key| key.value_kind == "Brush") {
            let sample = sample_keyframe_value(animation, &target, &track.property, time)?;
            AnimationBrush::sampled_value(&sample)?
        } else {
            sample_typed_keyframe(animation, &target, &track.property, time)?
        };
        let authored = self.keyframes.as_ref().ok_or("The accepted authoring track retired")?;
        let value = authored.sample_in_authored_form(request.program, track.target, &track.property, time, value);
        let value_kind = format!("{:?}", value.kind());
        if !track.keys.iter().all(|key| key.value_kind == value_kind) {
            return Err("The native sample differs from the authored property type".into());
        }
        Ok(DesignAnimationKeyframe {
            time,
            value: value.to_xaml(),
            easing: Easing::Linear.to_xaml(),
            value_kind,
        })
    }
}
