//! Arrange clips on their shared elapsed clock while leaving iteration keys untouched.
use crate::xaml::{native_namespace, native_ticks, parse_time, time_string};
use crate::{require, AnimationError, Result, Timeline};
use ferroui_designer_core::{Document, Edit, NodeId};
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct ClipSpan {
    pub start: f64,
    pub end: Option<f64>,
}

impl Timeline {
    /// Elapsed clock positions, including speed, initial delay and finite repeats.
    pub fn clip_span(&self) -> ClipSpan {
        ClipSpan {
            start: self.delay / self.speed_ratio,
            end: self.total_duration(),
        }
    }
}

/// Plan ordered offsets in the animation's authored time units. With no explicit start,
/// retain the first clip's delay. A positive gap separates clips and a negative gap overlaps them.
pub fn clip_delays(clips: &[&Timeline], start: Option<f64>, gap: f64) -> Result<Vec<f64>> {
    require(!clips.is_empty(), "Choose at least one clip")?;
    require(gap.is_finite(), "Clip spacing must be finite")?;
    let mut next = start;
    let mut delays = Vec::with_capacity(clips.len());
    for (index, &clip) in clips.iter().enumerate() {
        clip.validate()?;
        let span = clip.clip_span();
        let delay = match next {
            None => clip.delay,
            Some(time) => {
                native_ticks(time)?;
                // Both the native playback clock and the RSX ms() builder use milliseconds.
                // Round subsequent starts up so quantization cannot eat space between clips.
                let millis = native_ticks(time * clip.speed_ratio)? as f64 / 10_000.0;
                if index == 0 {
                    millis.round_ties_even() / 1000.0
                } else {
                    millis.ceil() / 1000.0
                }
            }
        };
        native_ticks(delay)?;
        let actual_start = delay / clip.speed_ratio;
        native_ticks(actual_start)?;
        let end = span.end.map(|end| actual_start + (end - span.start));
        if let Some(end) = end {
            native_ticks(end)?;
        }
        delays.push(delay);
        if index + 1 < clips.len() {
            let time = end.ok_or_else(|| {
                AnimationError("An infinite clip must be last in the sequence".into())
            })? + gap;
            // Adding and subtracting the same authored span can straddle the
            // boundary by a few bits. Preserve equal starts at that boundary.
            let rounding = f64::EPSILON * actual_start.abs().max(gap.abs()) * 4.0;
            require(
                time >= actual_start - rounding,
                "The overlap cannot exceed the preceding clip's length",
            )?;
            next = Some(time.max(actual_start));
        }
    }
    Ok(delays)
}

/// The delay of a native clip with a directly editable literal attribute.
pub fn xaml_clip_delay(document: &Document, id: NodeId) -> Option<f64> {
    let ast = document.ast();
    let node = ast.node(id)?;
    if node.local_name() != "Animation"
        || !native_namespace(node.namespace_uri.as_deref())
        || crate::states::state_program_role(document, id).is_some()
        || node
            .children
            .iter()
            .filter_map(|&id| ast.node(id))
            .any(|child| child.local_name() == "Animation.Delay")
    {
        return None;
    }
    node.attribute("Delay")
        .map_or(Some(0.0), |value| parse_time(value).ok())
}

pub fn xaml_clip_delay_edit(document: &Document, id: NodeId, delay: f64) -> Result<Option<Edit>> {
    native_ticks(delay)?;
    let original = xaml_clip_delay(document, id).ok_or_else(|| {
        AnimationError("Choose a native clip with a literal animation delay".into())
    })?;
    Ok((original != delay).then(|| Edit::SetAttribute {
        node: id,
        name: "Delay".into(),
        value: time_string(delay),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Repeat;

    #[test]
    fn placement_uses_elapsed_length_and_candidate_speed() {
        let mut first = Timeline::new(0.75).unwrap();
        first.delay = 0.5;
        first.repeat = Repeat::Count(3);
        first.delay_between_iterations = 0.125;
        first.speed_ratio = 2.0;
        let mut second = Timeline::new(1.0).unwrap();
        second.speed_ratio = 1.25;
        assert_eq!(
            first.clip_span(),
            ClipSpan {
                start: 0.25,
                end: Some(1.5)
            }
        );
        assert_eq!(
            clip_delays(&[&first, &second], None, 0.1).unwrap(),
            [0.5, 2.0]
        );
        assert_eq!(
            clip_delays(&[&first, &second], Some(1.0), -0.5).unwrap(),
            [2.0, 2.188]
        );
        assert!(clip_delays(&[&first, &second], None, -1.3).is_err());
        let coincident = Timeline::new(0.8).unwrap();
        assert_eq!(
            clip_delays(&[&coincident, &coincident], Some(0.1), -0.8).unwrap(),
            [0.1, 0.1]
        );
        assert!(clip_delays(&[&first], Some(-1.0), 0.0).is_err());
        assert!(clip_delays(&[&first], Some(f64::MAX), 0.0).is_err());
        second.repeat = Repeat::Infinite;
        assert!(clip_delays(&[&first, &second], None, 0.0).is_ok());
        assert!(clip_delays(&[&second, &first], None, 0.0).is_err());
    }

    #[test]
    fn xaml_offset_replaces_only_the_literal_and_keeps_other_forms_read_only() {
        let source = "<Animation Duration='0:0:2' Delay = '0:0:1'><!-- 🌿 --><KeyFrame Cue='0%'/></Animation>";
        let mut document = Document::new(source).unwrap();
        let root = document.ast().root();
        let edit = xaml_clip_delay_edit(&document, root, 1.25)
            .unwrap()
            .unwrap();
        document.apply("Place clip", vec![edit]).unwrap();
        assert_eq!(
            document.source(),
            source.replace("Delay = '0:0:1'", "Delay = '0:0:1.2500000'")
        );
        for source in [
            "<Animation Delay='{Binding Time}'/>",
            "<Animation><Animation.Delay>0:0:1</Animation.Delay></Animation>",
            "<Animation xmlns='using:Other' />",
        ] {
            let document = Document::new(source).unwrap();
            assert!(xaml_clip_delay_edit(&document, document.ast().root(), 0.5).is_err());
        }
    }
}
