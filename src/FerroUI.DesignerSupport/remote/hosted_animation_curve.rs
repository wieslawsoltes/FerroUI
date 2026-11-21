use super::{ACTIVE, CURVES};
use ferroui_base::animation::Animatable;
use ferroui_designer_animation::native::sample_curve_track;
use ferroui_designer_animation::{CurveChannel, CurveWindow};
use ferroui_designer_core::NodeId;
use ferroui_remote_protocol::designer::{
    DesignAnimationCurvePoint, DesignAnimationCurveRequest, DesignAnimationCurveResult,
};

pub(in crate::remote) fn curve(
    request: &DesignAnimationCurveRequest,
) -> DesignAnimationCurveResult {
    let mut answer = DesignAnimationCurveResult {
        revision: request.revision,
        request: request.request,
        ..Default::default()
    };
    if !CURVES.get() {
        answer.error = Some("This application preview did not negotiate curve sampling".into());
        return answer;
    }
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        ACTIVE.with(|slot| {
            let mut current = slot.borrow_mut();
            let host = current
                .as_mut()
                .ok_or("Open the current application preview")?;
            host.claim(request.revision, request.request)?;
            let program = host
                .programs
                .get(&request.program)
                .ok_or("Choose a current app-hosted animation")?;
            let channel = request.channel.as_deref().map(str::parse::<CurveChannel>)
                .transpose().map_err(|error| error.to_string())?.unwrap_or_default();
            if !program.curve_tracks.as_ref().is_some_and(|tracks| {
                tracks
                    .iter()
                    .any(|track| {
                        track.property == request.property && match &track.curve_channels {
                            Some(names) => request.channel.as_ref().is_some_and(|name| names.contains(name)),
                            None => track.keys.first().is_some_and(|key| {
                                CurveChannel::for_authored_kind(&key.value_kind).contains(&channel)
                            }),
                        }
                    })
            }) {
                return Err("Choose an authored animation track and one of its curve channels".into());
            }
            let target = host
                .targets
                .get(&request.target)
                .filter(|_| program.compatible_targets.contains(&request.target))
                .and_then(|control| control.cast::<Animatable>())
                .ok_or("Choose a compatible live preview target")?;
            let id =
                NodeId::try_from(request.program).map_err(|_| "The native animation retired")?;
            let animation = host
                .index
                .animation(id)
                .ok_or("The native animation retired")?;
            let window = CurveWindow {
                start: request.start,
                end: request.end,
            };
            sample_curve_track(animation, &target, &request.property, channel, window)
        })
    }));
    match result {
        Ok(Ok(points)) => {
            answer.points = points
                .into_iter()
                .map(|point| DesignAnimationCurvePoint {
                    time: point.time,
                    value: point.value,
                    break_before: point.break_before.then_some(true),
                })
                .collect();
        }
        Ok(Err(error)) => answer.error = Some(error),
        Err(_) => {
            answer.error = Some("The application could not sample this animation track".into())
        }
    }
    answer
}
