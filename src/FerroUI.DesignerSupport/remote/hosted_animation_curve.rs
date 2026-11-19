use super::{ACTIVE, CURVES};
use ferroui_base::animation::Animatable;
use ferroui_designer_animation::native::sample_scalar_track;
use ferroui_designer_animation::CurveWindow;
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
            if !program.curve_tracks.as_ref().is_some_and(|tracks| {
                tracks
                    .iter()
                    .any(|track| track.property == request.property)
            }) {
                return Err("Choose an authored scalar animation track".into());
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
            sample_scalar_track(animation, &target, &request.property, window)
        })
    }));
    match result {
        Ok(Ok(points)) => {
            answer.points = points
                .into_iter()
                .map(|point| DesignAnimationCurvePoint {
                    time: point.time,
                    value: point.value,
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
