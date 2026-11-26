use super::AuthoredRsxFrame;
use crate::hosted_animation::checked_source_patch;
use ferroui_designer_animation::{KeyframeLocation, KeyframeSelection, Timeline};
use ferroui_designer_core::{NodeId, TextPatch};
use ferroui_remote_protocol::designer::DesignAnimationSourcePatch;

impl AuthoredRsxFrame {
    pub(crate) fn hosted_keyframe_selection(
        &self,
        program: i64,
        command: &KeyframeSelection,
        timeline: &Timeline,
    ) -> Result<
        (
            Option<DesignAnimationSourcePatch>,
            Option<Vec<KeyframeLocation>>,
        ),
        String,
    > {
        let clip =
            self.clip(NodeId::try_from(program).map_err(|_| "Choose a mapped Rust clip")?)?;
        let mut locations = command.keyframes().to_vec();
        for location in &mut locations {
            location.reconcile_time(timeline);
            let value = timeline
                .track(location.target, &location.property)
                .and_then(|track| track.keyframes.iter().find(|key| key.time == location.time))
                .ok_or("Choose an authored Rust keyframe")?;
            if !clip.maps_key(&location.property, location.time, value.value.kind()) {
                return Err("Choose a keyframe with a mapped Rust declaration".into());
            }
        }
        let (patches, keys) = match command {
            KeyframeSelection::CopyKeyframes { .. } => return Ok((None, None)),
            KeyframeSelection::PasteKeyframes { paste } => {
                clip.paste_keyframes(&self.source, &self.catalog, timeline, paste)?
            }
            KeyframeSelection::MirrorKeyframes { pivot, .. } => {
                clip.mirror_keyframes(&self.source, &self.catalog, timeline, &locations, *pivot)?
            }
            _ => clip.author_keyframes(
                &self.source,
                &self.catalog,
                timeline,
                &locations,
                command.group_edit().expect("group edit"),
            )?,
        };
        let updated =
            TextPatch::apply_all(&self.source, patches).map_err(|error| error.to_string())?;
        Ok((checked_source_patch(&self.source, &updated)?, Some(keys)))
    }
}
