//! Capture and restore the native clocks that produced an accepted preview frame.
use ferroui_designer_animation::AnimationProgramLocator as ProgramLocator;
use crate::{DesignerSession, PreviewSourceAnchor, PreviewStatus, TimelinePreviewPose};
use ferroui_designer_core::NodeId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PlaybackPose {
    pub(crate) elapsed: f64,
    pub(crate) all: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    playing: bool,
    #[serde(default)]
    pub(crate) completed: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    clocks: Vec<ProgramClocks>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProgramClocks {
    source_key: Option<String>,
    anchor: Option<ProgramLocator>,
    targets: Vec<TargetClock>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct TargetClock {
    target: PreviewSourceAnchor,
    elapsed: f64,
}

impl DesignerSession {
    pub(crate) fn timeline_playback_pose(&self) -> Option<TimelinePreviewPose> {
        if !self.animation.elapsed_clock {
            return None;
        }
        let elapsed = self.time();
        let time = if self.uses_native_timeline_clock() {
            elapsed
        } else {
            self.animation.compiled.playback_position(elapsed).ok()?
        };
        let mut pose = self.timeline_pose_at(time)?;
        let all = self.animation.play_all;
        let mut clocks = BTreeMap::<NodeId, Vec<TargetClock>>::new();
        // Visible peers usually share one clock. Persist only clocks with a distinct
        // position, such as a hidden target or a shorter clip in Play All.
        for (&(program, target), &time) in &self.animation.target_elapsed {
            if time == elapsed || (!all && program != pose.program) {
                continue;
            }
            if let Ok(target) = PreviewSourceAnchor::capture(self.document(), target) {
                clocks.entry(program).or_default().push(TargetClock {
                    target,
                    elapsed: time,
                });
            }
        }
        let clocks = self
            .animation_programs()
            .iter()
            .filter_map(|program| {
                let targets = clocks.remove(&program.id)?;
                let anchor = ProgramLocator::capture(self.document(), program);
                (program.source_key.is_some() || anchor.is_some()).then(|| ProgramClocks {
                    source_key: program.source_key.clone(),
                    anchor,
                    targets,
                })
            })
            .collect();
        pose.playback = Some(PlaybackPose {
            elapsed,
            all,
            playing: self.uses_rust_source() && self.is_playing(),
            completed: self.animation.playback_completed,
            clocks,
        });
        Some(pose)
    }

    pub(crate) fn restore_playback_pose(
        &mut self,
        program: NodeId,
        pose: &PlaybackPose,
    ) -> Result<Option<TimelinePreviewPose>, String> {
        validate_clock(pose.elapsed)?;
        if !self.animation_programs().iter().any(|candidate| {
            (pose.all || candidate.id == program) && self.animation_preview_available(candidate)
        }) {
            return Ok(None);
        }
        let mut clocks = BTreeMap::new();
        for saved in &pose.clocks {
            let Some(program) =
                self.resolve_preview_program(saved.source_key.as_deref(), saved.anchor.as_ref())
            else {
                continue;
            };
            for clock in &saved.targets {
                validate_clock(clock.elapsed)?;
                if let Some(target) = clock
                    .target
                    .resolve(self.document())
                    .ok()
                    .filter(|target| program.targets.contains(target))
                {
                    clocks.insert((program.id, target), clock.elapsed);
                }
            }
        }
        self.select_animation_program(program)?;
        self.animation.play_all = pose.all;
        self.animation.target_elapsed = clocks;
        self.animation
            .player
            .seek(pose.elapsed)
            .map_err(|error| error.to_string())?;
        self.animation.player.play();
        let result = self.advance(0.0);
        if !pose.playing {
            self.animation.player.pause();
        }
        result?;
        Ok(self.timeline_playback_pose())
    }

    pub(crate) fn remember_playback_for_reload(&mut self) {
        if self.animation.pending_playback.is_none() {
            self.animation.pending_playback = self.timeline_playback_pose().or_else(|| {
                (self.uses_rust_source() && (self.preview.has_animation_layer() || self.animation.native.has_pose()))
                    .then(|| self.timeline_preview_pose()).flatten()
            });
        }
    }

    pub(crate) fn restore_playback_after_reload(&mut self) {
        if !matches!(self.preview_status(), PreviewStatus::Ready { .. }) {
            return;
        }
        let Some(pose) = self.animation.pending_playback.take() else {
            return;
        };
        self.animation.elapsed_clock = false;
        self.animation.play_all = false;
        self.animation.player.stop();
        if let Err(error) = self.restore_timeline_preview_pose(&pose) {
            self.preview.clear_animation();
            self.animation.native.stop();
            self.preview
                .notices
                .push(format!("Animation preview: {error}"));
        }
    }
}

fn validate_clock(elapsed: f64) -> Result<(), String> {
    if elapsed.is_finite() && elapsed >= 0.0 && elapsed * 10_000_000.0 < i64::MAX as f64 {
        Ok(())
    } else {
        Err("Animation preview time is outside the native clock range".into())
    }
}
