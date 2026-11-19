use crate::{DesignerSession, PreviewStatus};
use ferroui_designer_animation::{KeyframeClipboard, KeyframeLocation, KeyframePaste, Timeline};
use ferroui_designer_core::{NodeId, TextPatch};

impl DesignerSession {
    pub(crate) fn copy_program_keyframes(
        &self,
        program: NodeId,
        locations: &[KeyframeLocation],
    ) -> Result<KeyframeClipboard, String> {
        self.transfer_timeline(program)?
            .copy_keyframes(locations)
            .map_err(|error| error.to_string())
    }

    pub(crate) fn mirror_program_keyframes(
        &mut self,
        program: NodeId,
        locations: &[KeyframeLocation],
        pivot: f64,
    ) -> Result<Vec<KeyframeLocation>, String> {
        let timeline = self.transfer_timeline(program)?;
        let inserted = if let Some(rust) = &self.rust {
            let (patches, inserted) =
                rust.mirror_keys(&self.document, program, timeline, locations, pivot)?;
            self.commit_keyframe_transfer("Mirror Rust keyframes", patches)?;
            inserted
        } else {
            let (candidate, inserted) = timeline
                .with_mirrored_keyframes(locations, pivot)
                .map_err(|error| error.to_string())?;
            if &candidate != timeline {
                self.update_animation_program(program, candidate)?;
            }
            inserted
        };
        Ok(self.accepted_keyframe_locations(program, inserted))
    }

    pub(crate) fn paste_program_keyframes(
        &mut self,
        program: NodeId,
        paste: &KeyframePaste,
    ) -> Result<Vec<KeyframeLocation>, String> {
        let timeline = self.transfer_timeline(program)?;
        let inserted = if let Some(rust) = &self.rust {
            let (patches, inserted) = rust.paste_keys(&self.document, program, timeline, paste)?;
            self.commit_keyframe_transfer("Paste Rust keyframes", patches)?;
            inserted
        } else {
            let (candidate, inserted) = timeline
                .with_pasted_keyframes(paste)
                .map_err(|error| error.to_string())?;
            let authored = self
                .animation_programs()
                .iter()
                .find(|entry| entry.id == program)
                .ok_or("Choose an animation program")?;
            if !authored.has_project_codec() {
                for destination in &inserted {
                    if !self.can_record_property(destination.target, &destination.property) {
                        continue;
                    }
                    let track = candidate
                        .track(destination.target, &destination.property)
                        .expect("planned track");
                    let index = track
                        .keyframes
                        .binary_search_by(|key| key.time.total_cmp(&destination.time))
                        .expect("planned key time");
                    self.preview.prepare_animation_value(
                        destination.target,
                        &destination.property,
                        &track.keyframes[index].value,
                    )?;
                }
            }
            self.update_animation_program(program, candidate)?;
            inserted
        };
        Ok(self.accepted_keyframe_locations(program, inserted))
    }

    fn transfer_timeline(&self, program: NodeId) -> Result<&Timeline, String> {
        if !matches!(self.preview_status(), PreviewStatus::Ready { revision } if *revision == self.document().revision())
        {
            return Err("Wait for the current animation preview".into());
        }
        self.animation_programs()
            .iter()
            .find(|entry| entry.id == program)
            .and_then(|entry| entry.timeline.as_ref())
            .ok_or_else(|| "Choose a typed animation program".into())
    }

    fn commit_keyframe_transfer(
        &mut self,
        label: &str,
        patches: Vec<TextPatch>,
    ) -> Result<(), String> {
        if !patches.is_empty() {
            self.commit_rust_patches(label, patches)
                .map_err(|error| error.to_string())?;
            self.changed(true);
        }
        Ok(())
    }

    pub(crate) fn accepted_keyframe_locations(
        &self,
        program: NodeId,
        mut keys: Vec<KeyframeLocation>,
    ) -> Vec<KeyframeLocation> {
        if let Some(timeline) = self
            .animation_programs()
            .iter()
            .find(|entry| entry.id == program)
            .and_then(|entry| entry.timeline.as_ref())
        {
            for key in &mut keys {
                key.reconcile_time(timeline);
            }
        }
        keys
    }
}
