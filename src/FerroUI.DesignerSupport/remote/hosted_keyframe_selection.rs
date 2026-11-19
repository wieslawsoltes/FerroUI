use super::{HostedKeyframes, with_codecs};
use ferroui_designer_animation::{
    AnimationError, KeyframeSelection, KeyframeSelectionPlan, Timeline, native_key_time,
};
use ferroui_remote_protocol::designer::{
    DesignKeyframeSelectionRequest, DesignKeyframeSelectionResult,
};

impl HostedKeyframes {
    pub(in crate::remote) fn plan_selection(
        &self,
        request: &DesignKeyframeSelectionRequest,
    ) -> Result<DesignKeyframeSelectionResult, String> {
        let (program, options) = self
            .programs
            .get(&request.program)
            .ok_or("Choose an editable animation in the current application preview")?;
        let original = program
            .timeline
            .as_ref()
            .ok_or("Choose a typed animation")?;
        let command =
            KeyframeSelection::from_json(&request.command).map_err(|error| error.to_string())?;
        let mut answer = DesignKeyframeSelectionResult {
            revision: request.revision,
            request: request.request,
            ..Default::default()
        };
        if request.projected {
            let source = serde_json::to_string(original).map_err(|error| error.to_string())?;
            if source.len() > 4 * 1024 * 1024 {
                return Err("The projected animation exceeds the 4 MiB planning limit".into());
            }
            answer.timeline = Some(source);
        }
        match command.plan(original).map_err(|error| error.to_string())? {
            KeyframeSelectionPlan::Copy(clipboard) => {
                answer.clipboard = Some(clipboard.to_text().map_err(|error| error.to_string())?);
            }
            KeyframeSelectionPlan::Edit { timeline, mut keys } => {
                if !request.projected && &timeline != original {
                    let (source, accepted) = with_codecs(|codecs| {
                        let source = codecs.encode_keyframes_for_document(
                            &self.document,
                            program,
                            &timeline,
                        )?;
                        let mut accepted = program.clone();
                        accepted.timeline = None;
                        accepted.codec = None;
                        codecs.decode(&mut accepted, &source, options);
                        let accepted_timeline = accepted
                            .timeline
                            .filter(|_| accepted.codec == program.codec)
                            .ok_or_else(|| {
                                AnimationError(
                                    "The animation codec could not read its encoded program".into(),
                                )
                            })?;
                        Ok::<_, AnimationError>((source, accepted_timeline))
                    })
                    .map_err(|error| error.to_string())?;
                    if !same_animation(&timeline, &accepted) {
                        return Err("The animation codec did not preserve the planned keys and clip timing".into());
                    }
                    for key in &mut keys {
                        key.reconcile_time(&accepted);
                    }
                    answer.patch = self.source_patch(program.id, &source)?;
                }
                answer.keys =
                    Some(serde_json::to_string(&keys).map_err(|error| error.to_string())?);
            }
        }
        Ok(answer)
    }
}

fn same_animation(planned: &Timeline, accepted: &Timeline) -> bool {
    same_time(planned.duration, accepted.duration)
        && same_time(planned.delay, accepted.delay)
        && same_time(planned.delay_between_iterations, accepted.delay_between_iterations)
        && planned.speed_ratio == accepted.speed_ratio
        && planned.repeat == accepted.repeat
        && planned.playback_direction == accepted.playback_direction
        && planned.playback_behavior == accepted.playback_behavior
        && planned.fill == accepted.fill
        && planned.easing == accepted.easing
        && planned.tracks.len() == accepted.tracks.len()
        && planned.tracks.iter().all(|track| {
            accepted
                .track(track.target, &track.property)
                .is_some_and(|actual| {
                    track.keyframes.len() == actual.keyframes.len()
                        && track
                            .keyframes
                            .iter()
                            .zip(&actual.keyframes)
                            .all(|(left, right)| {
                                same_time(left.time, right.time)
                                    && left.value == right.value
                                    && left.easing == right.easing
                            })
                })
        })
}

fn same_time(planned: f64, accepted: f64) -> bool {
    native_key_time(planned)
        .ok()
        .is_some_and(|time| (time - accepted).abs() < 0.00000005)
}
