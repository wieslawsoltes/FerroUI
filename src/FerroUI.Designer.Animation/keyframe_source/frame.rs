use super::{children, native_element, KeyPairs, Patches};
use crate::xaml::{key_time, native_key_time, time_string};
use crate::{AnimationError, Easing, Result};
use ferroui_designer_core::{Document, Node};

pub(super) fn plan(
    document: &Document,
    node: &Node,
    duration: f64,
    keys: &mut KeyPairs<'_>,
    output: &mut Patches,
) -> Result<bool> {
    let time = key_time(node.attribute("Cue"), node.attribute("KeyTime"), duration)?;
    let mut timing = None;
    for setter in children(document, node.id).filter(|node| native_element(node, "Setter")) {
        let property = setter.attribute("Property").ok_or_else(stale)?;
        let (old, next) = keys
            .get_mut(property)
            .and_then(|track| track.remove(&time.to_bits()))
            .ok_or_else(stale)?;
        let next_timing = (next.time, &next.easing, &old.easing);
        if timing.as_ref().is_some_and(|timing| *timing != next_timing) {
            // A cue and incoming spline belong to the complete frame. The codec
            // can split a shared frame if only one setter changes its timing.
            return Ok(false);
        }
        timing = Some(next_timing);
        if old.value != next.value {
            super::value::plan(document, setter, &old.value, &next.value, output)?;
        }
    }
    let Some((next, easing, old_easing)) = timing else {
        return Ok(true);
    };
    if next != time {
        let next = native_key_time(next)?;
        if let Some(cue) = node.attribute("Cue") {
            let fraction = next / duration;
            let text = if cue.trim_end().ends_with('%') {
                format!("{}%", fraction * 100.0)
            } else {
                fraction.to_string()
            };
            output.attribute(document, node, "Cue", Some(text))?;
        } else {
            output.attribute(document, node, "KeyTime", Some(time_string(next)))?;
        }
    }
    if easing != old_easing {
        let spline = match easing {
            Easing::Linear => None,
            Easing::CubicBezier { .. } | Easing::Spring { .. } => Some(easing.to_xaml()),
            Easing::Named(_) => return Ok(false),
        };
        output.attribute(document, node, "KeySpline", spline)?;
    }
    Ok(true)
}

fn stale() -> AnimationError {
    AnimationError("The authored keyframe setters changed; wait for the current preview".into())
}
