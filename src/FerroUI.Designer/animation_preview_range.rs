use crate::{DesignerSession, TimelinePreviewPose};
use ferroui_designer_animation::PreviewRange;
use ferroui_designer_core::NodeId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PreviewRangePose {
    pub(crate) range: PreviewRange,
    pub(crate) all: bool,
    pub(crate) playing: bool,
}

impl DesignerSession {
    pub(crate) fn preview_range_limit(
        &self,
        program: Option<NodeId>,
        all: bool,
    ) -> Result<Option<f64>, String> {
        if !all {
            let program = self
                .animation_programs()
                .iter()
                .find(|item| Some(item.id) == program && self.animation_preview_available(item))
                .ok_or("Choose an animation with live preview targets")?;
            return self.preview_seek_limit(program);
        }
        let mut clocks = self
            .animation_programs()
            .iter()
            .filter(|item| self.animation_preview_available(item))
            .filter_map(|item| {
                self.animation_clip_clock(item)
                    .map(|(clock, _)| clock.total_duration())
            })
            .peekable();
        if clocks.peek().is_none() {
            return Err("Choose preview targets for at least one animation clip".into());
        }
        Ok(clocks.try_fold(0.0_f64, |limit, end| end.map(|end| limit.max(end))))
    }

    pub(crate) fn start_preview_range(
        &mut self,
        program: Option<NodeId>,
        all: bool,
        range: PreviewRange,
    ) -> Result<(), String> {
        let program = program
            .or_else(|| self.active_animation_program())
            .ok_or("Choose an animation program")?;
        if !self
            .animation_programs()
            .iter()
            .any(|item| item.id == program)
        {
            return Err("Choose an animation program in the current preview".into());
        }
        let limit = self.preview_range_limit(Some(program), all)?;
        range.validate(limit).map_err(|error| error.to_string())?;
        let same_clock = self.active_animation_program() == Some(program)
            && self.animation.play_all == all
            && (all
                || self.animation.preview_range.is_some()
                || !self.animation.elapsed_clock
                || self.uses_native_timeline_clock());
        let time = range.start_at(if same_clock { self.time() } else { range.start });
        self.sample_preview_range(program, all, time, Some(range), true, true)?;
        self.set_recording(false);
        Ok(())
    }

    pub(crate) fn advance_preview_range(
        &mut self,
        range: PreviewRange,
        seconds: f64,
    ) -> Result<(), String> {
        let program = self
            .active_animation_program()
            .ok_or("Choose an animation program")?;
        let time = self.time();
        let (next, completed) = range
            .advance(time, seconds)
            .map_err(|error| error.to_string())?;
        let restart = range.looping && seconds >= range.end - time;
        self.sample_preview_range(
            program,
            self.animation.play_all,
            next,
            Some(range),
            !completed,
            restart,
        )
    }

    pub(crate) fn restore_preview_range(
        &mut self,
        program: NodeId,
        pose: &TimelinePreviewPose,
        presentation: PreviewRangePose,
    ) -> Result<Option<TimelinePreviewPose>, String> {
        let all = presentation.all;
        ferroui_designer_animation::native_key_time(pose.time)
            .map_err(|error| error.to_string())?;
        let limit = match self.preview_range_limit(Some(program), all) {
            Ok(limit) => limit,
            Err(_) => return Ok(None),
        };
        let range = presentation.range.fit(limit);
        let mut time = limit.map_or(pose.time, |limit| pose.time.min(limit));
        let mut playing = false;
        if let Some(range) = range {
            time = if presentation.playing && range.looping {
                range.start_at(time)
            } else {
                time.clamp(range.start, range.end)
            };
            playing = presentation.playing && time < range.end;
        }
        self.sample_preview_range(program, all, time, range, playing, true)?;
        Ok(self
            .timeline_playback_pose()
            .or_else(|| self.timeline_preview_pose()))
    }

    fn sample_preview_range(
        &mut self,
        program: NodeId,
        all: bool,
        time: f64,
        range: Option<PreviewRange>,
        playing: bool,
        restart: bool,
    ) -> Result<(), String> {
        self.animation.preview_range = None;
        if all {
            if restart {
                self.animation.native.stop();
            }
            self.scrub_animation_clips(Some(program), time)?;
        } else {
            self.scrub_animation_program(program, time)?;
        }
        self.animation.preview_range = range;
        if let Some(range) = range {
            self.animation.playback_completed = !range.looping && time >= range.end;
        }
        if playing {
            self.animation.player.play();
        }
        Ok(())
    }
}
