//! Clip placement reuses the loaded animation clock and the document's source transaction.
use crate::DesignerSession;
use ferroui_designer_animation::{
    clip_delays, native_key_time, xaml_clip_delay, xaml_clip_delay_edit, AnimationProgram, Timeline,
};
use ferroui_designer_core::NodeId;
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

impl DesignerSession {
    pub(crate) fn animation_clip_clock<'a>(
        &self,
        program: &'a AnimationProgram,
    ) -> Option<(Cow<'a, Timeline>, bool)> {
        let native = self
            .requires_native_animation(program)
            .then(|| self.animation.native.timing(program.id))
            .flatten();
        let clock = native
            .map(Cow::Owned)
            .or_else(|| program.timeline.as_ref().map(Cow::Borrowed))?;
        let editable = self.can_manipulate()
            && xaml_clip_delay(self.document(), program.id).is_some_and(|delay| {
                native_key_time(clock.delay).is_ok_and(|authored| authored == delay)
            })
            && self
                .rust
                .as_ref()
                .is_none_or(|rust| rust.animation(self.document(), program.id).is_ok());
        Some((clock, editable))
    }

    /// Resolve the placements against the current frame before planning source edits.
    pub fn plan_animation_clip_placement(
        &self,
        ids: &[NodeId],
        start: Option<f64>,
        gap: f64,
    ) -> Result<Vec<(NodeId, f64)>, String> {
        if ids.iter().copied().collect::<BTreeSet<_>>().len() != ids.len() {
            return Err("Select each clip once".into());
        }
        let programs = self
            .animation_programs()
            .iter()
            .map(|program| (program.id, program))
            .collect::<BTreeMap<_, _>>();
        let clocks = ids
            .iter()
            .map(|id| {
                let program = programs
                    .get(id)
                    .ok_or("Choose a clip in the current Timeline")?;
                self.animation_clip_clock(program)
                    .filter(|(_, editable)| *editable)
                    .map(|(clock, _)| clock)
                    .ok_or_else(|| format!("{} has no editable literal delay", program.name))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let clocks = clocks.iter().map(Cow::as_ref).collect::<Vec<_>>();
        let delays = clip_delays(&clocks, start, gap).map_err(|error| error.to_string())?;
        Ok(ids.iter().copied().zip(delays).collect())
    }

    pub(crate) fn place_animation_clips(
        &mut self,
        ids: &[NodeId],
        start: Option<f64>,
        gap: f64,
    ) -> Result<(), String> {
        let placements = self.plan_animation_clip_placement(ids, start, gap)?;
        let label = if ids.len() == 1 {
            "Place animation clip"
        } else {
            "Sequence animation clips"
        };
        if let Some(rust) = &self.rust {
            let patches = rust.place_animation_clips(self.document(), &placements)?;
            if !patches.is_empty() {
                self.commit_rust_patches(label, patches)
                    .map_err(|error| error.to_string())?;
                self.changed(true);
            }
        } else {
            let mut edits = Vec::new();
            for (id, delay) in placements {
                edits.extend(
                    xaml_clip_delay_edit(self.document(), id, delay)
                        .map_err(|error| error.to_string())?,
                );
            }
            if !edits.is_empty() {
                self.apply(label, edits)
                    .map_err(|error| error.to_string())?;
            }
        }
        Ok(())
    }

    pub(crate) fn scrub_animation_clips(
        &mut self,
        reference: Option<NodeId>,
        elapsed: f64,
    ) -> Result<(), String> {
        native_key_time(elapsed).map_err(|error| error.to_string())?;
        self.start_combined_playback(reference, Some(elapsed))?;
        self.pause();
        Ok(())
    }
}
