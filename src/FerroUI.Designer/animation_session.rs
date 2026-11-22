//! UI-thread adapter for independently selected, source-backed animation programs.
use crate::DesignerSession;
use ferroui_base::styling::Style;
use ferroui_base::{BoxedValue, FerroProperty, StyledElement, TypeInfo};
use ferroui_designer_animation::{
    AnimationCodec, AnimationCodecRegistry, AnimationEffect, AnimationProgram, AnimationPrograms, AnimationValue,
    CompiledTimeline, CurveChannel, CurvePoint, CurveWindow, Easing, ImportOptions, Player, PreviewRange, Sample, Timeline, TimelineTimingEdit, TrackCurveChannel, ValueKind,
};
use ferroui_designer_core::{Document, Edit, NodeId};
use ferroui_markup::markup::parsers::SelectorParser;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

pub(crate) struct AnimationSession {
    pub(crate) native: crate::animation_runtime::NativeAnimations,
    pub(crate) timeline: Timeline,
    pub(crate) compiled: CompiledTimeline,
    pub(crate) player: Player,
    pub(crate) recording: bool,
    programs: Vec<AnimationProgram>,
    codecs: AnimationCodecRegistry,
    active: Option<NodeId>,
    warnings: Vec<String>,
    pub(crate) play_all: bool,
    pub(crate) elapsed_clock: bool,
    pub(crate) playback_completed: bool,
    pub(crate) preview_range: Option<PreviewRange>,
    pub(crate) pending_playback: Option<crate::TimelinePreviewPose>,
    compiled_programs: BTreeMap<NodeId, CompiledTimeline>,
    compiled_targets: BTreeMap<(NodeId, NodeId), CompiledTimeline>,
    pub(crate) preview_targets: BTreeMap<NodeId, Vec<NodeId>>,
    pub(crate) target_elapsed: BTreeMap<(NodeId, NodeId), f64>,
}
impl AnimationSession {
    pub(crate) fn clear_program_state(&mut self) {
        self.native.clear();
        self.active = None;
        self.preview_targets.clear();
        self.compiled_targets.clear();
        self.play_all = false;
        self.elapsed_clock = false;
        self.playback_completed = false;
        self.preview_range = None;
        self.pending_playback = None;
        self.target_elapsed.clear();
    }
    pub(crate) fn new() -> Result<Self, String> {
        let timeline = Timeline::new(3.0).map_err(|error| error.to_string())?;
        let compiled = timeline.compile().map_err(|error| error.to_string())?;
        Ok(Self {
            native: Default::default(),
            timeline,
            compiled,
            player: Player::new(),
            recording: false,
            programs: Vec::new(),
            codecs: AnimationCodecRegistry::default(),
            active: None,
            warnings: Vec::new(),
            play_all: false,
            elapsed_clock: false,
            playback_completed: false,
            preview_range: None,
            pending_playback: None,
            compiled_programs: BTreeMap::new(),
            compiled_targets: BTreeMap::new(),
            preview_targets: BTreeMap::new(),
            target_elapsed: BTreeMap::new(),
        })
    }
}
impl DesignerSession {
    pub fn timeline(&self) -> &Timeline {
        &self.animation.timeline
    }
    pub fn animation_programs(&self) -> &[AnimationProgram] {
        &self.animation.programs
    }
    pub fn active_animation_program(&self) -> Option<NodeId> {
        self.animation.active
    }
    pub fn animation_warnings(&self) -> &[String] {
        &self.animation.warnings
    }
    pub fn time(&self) -> f64 {
        self.animation.player.time()
    }
    pub fn is_playing(&self) -> bool {
        self.animation.player.is_playing()
    }
    pub fn recording(&self) -> bool {
        self.animation.recording
    }
    pub fn set_recording(&mut self, value: bool) {
        self.animation.recording = value && !self.is_read_only() && !self.uses_rust_source();
        self.generation += 1;
    }
    pub fn register_animation_codec(&mut self, codec: Box<dyn AnimationCodec>) {
        self.animation.codecs.register(codec);
        self.import_animations();
        self.generation += 1;
    }
    pub fn select_animation_program(&mut self, id: NodeId) -> Result<(), String> {
        self.import_animations();
        let program = self
            .animation
            .programs
            .iter()
            .find(|program| program.id == id)
            .ok_or("Unknown animation program")?;
        let timeline = self.animation_timing(program);
        self.animation.compiled = timeline.compile().map_err(|error| error.to_string())?;
        self.animation.timeline = timeline;
        self.animation.active = Some(id);
        self.animation.player.stop();
        self.animation.target_elapsed.clear();
        self.animation.play_all = false;
        self.animation.elapsed_clock = false;
        self.animation.playback_completed = false;
        self.animation.preview_range = None;
        self.preview.clear_animation();
        self.generation += 1;
        Ok(())
    }
    pub fn create_animation_program(
        &mut self,
        name: &str,
        owner: NodeId,
        selector: &str,
        duration: f64,
    ) -> Result<NodeId, String> {
        if self.uses_rust_source() {
            return self.create_rust_animation(name, owner, selector, duration);
        }
        let timeline = Timeline::new(duration).map_err(|error| error.to_string())?;
        let before = self
            .animation
            .programs
            .iter()
            .map(|program| program.id)
            .collect::<BTreeSet<_>>();
        let edits = AnimationPrograms::create(&self.document, owner, name, selector, &timeline)
            .map_err(|error| error.to_string())?;
        self.apply("Create animation program", edits)
            .map_err(|error| error.to_string())?;
        let id = self
            .animation
            .programs
            .iter()
            .find(|program| !before.contains(&program.id))
            .ok_or("Created animation was not found")?
            .id;
        self.select_animation_program(id)?;
        Ok(id)
    }
    pub fn remove_animation_program(&mut self, id: NodeId) -> Result<(), String> {
        if self.uses_rust_source() {
            return self.edit_rust_animation("Remove Rust animation", id, ferroui_designer_rsx::RsxAnimationEdit::Remove).map(|_| ());
        }
        let edits =
            AnimationPrograms::remove(&self.document, id).map_err(|error| error.to_string())?;
        self.apply("Remove animation program", edits)
            .map_err(|error| error.to_string())?;
        Ok(())
    }
    pub fn rename_animation_program(&mut self, id: NodeId, name: &str) -> Result<(), String> {
        if self.uses_rust_source() {
            return self.edit_rust_animation("Rename Rust animation", id, ferroui_designer_rsx::RsxAnimationEdit::Rename(name)).map(|_| ());
        }
        let edits = AnimationPrograms::rename(&self.document, id, name)
            .map_err(|error| error.to_string())?;
        self.apply("Rename animation program", edits)
            .map_err(|error| error.to_string())?;
        Ok(())
    }
    pub fn duplicate_animation_program(
        &mut self,
        id: NodeId,
        name: &str,
    ) -> Result<NodeId, String> {
        if self.uses_rust_source() {
            return self.edit_rust_animation("Duplicate Rust animation", id, ferroui_designer_rsx::RsxAnimationEdit::Duplicate(name))?
                .ok_or_else(|| "The duplicated animation has no selection identity".into());
        }
        let before = self
            .animation
            .programs
            .iter()
            .map(|program| program.id)
            .collect::<BTreeSet<_>>();
        let edits = AnimationPrograms::duplicate(&self.document, id, name)
            .map_err(|error| error.to_string())?;
        self.apply("Duplicate animation program", edits)
            .map_err(|error| error.to_string())?;
        let id = self
            .animation
            .programs
            .iter()
            .find(|program| !before.contains(&program.id))
            .ok_or("Duplicated animation was not found")?
            .id;
        self.select_animation_program(id)?;
        Ok(id)
    }
    pub fn set_animation_program_selector(
        &mut self,
        id: NodeId,
        selector: &str,
    ) -> Result<(), String> {
        let edits = AnimationPrograms::set_selector(&self.document, id, selector)
            .map_err(|error| error.to_string())?;
        self.apply("Change animation targets", edits)
            .map_err(|error| error.to_string())?;
        Ok(())
    }
    /// Assign preview-only targets for a resource program or a currently inactive selector.
    /// None restores live selector matching. Authored selectors and source remain unchanged.
    pub fn set_animation_program_preview_targets(
        &mut self,
        id: NodeId,
        targets: Option<Vec<NodeId>>,
    ) -> Result<(), String> {
        if !self
            .animation
            .programs
            .iter()
            .any(|program| program.id == id)
        {
            return Err("Unknown animation program".into());
        }
        if let Some(targets) = targets {
            let mut unique = BTreeSet::new();
            for target in &targets {
                if !unique.insert(*target) || self.preview.visual(*target).is_none() {
                    return Err("Preview targets must be distinct live source controls".into());
                }
            }
            self.animation.preview_targets.insert(id, targets);
        } else {
            self.animation.preview_targets.remove(&id);
        }
        self.preview.clear_animation();
        self.import_animations();
        self.animation.player.stop();
        self.animation.play_all = false;
        self.animation.elapsed_clock = false;
        self.animation.preview_range = None;
        self.animation.pending_playback = None;
        self.generation += 1;
        Ok(())
    }
    pub fn set_animation_program_source(&mut self, id: NodeId, source: &str) -> Result<(), String> {
        let edits = AnimationPrograms::replace_source(&self.document, id, source)
            .map_err(|error| error.to_string())?;
        self.apply("Edit animation source", edits)
            .map_err(|error| error.to_string())?;
        Ok(())
    }
    /// Replace a program's complete typed timing/track model through its registered codec.
    pub fn update_animation_program(
        &mut self,
        id: NodeId,
        timeline: Timeline,
    ) -> Result<(), String> {
        let active = self.animation.active == Some(id);
        let edits = self.program_edits(id, &timeline)?;
        self.apply("Edit animation program", edits)
            .map_err(|error| error.to_string())?;
        if active && !self.animation.elapsed_clock {
            self.seek(self.animation.player.time())?;
        }
        Ok(())
    }
    /// Edit a program's clock with the same source transaction used by typed track updates.
    pub fn edit_animation_timing(
        &mut self,
        id: NodeId,
        changes: TimelineTimingEdit,
    ) -> Result<(), String> {
        let timeline = self
            .animation
            .programs
            .iter()
            .find(|program| program.id == id)
            .ok_or("Unknown animation program")?
            .timeline
            .as_ref()
            .ok_or("Edit an opaque animation through its source or register a codec")?;
        let next = timeline.with_timing(changes).map_err(|error| error.to_string())?;
        if &next == timeline {
            return Ok(());
        }
        self.update_animation_program(id, next)
    }
    fn editable_program(&self) -> Result<(), String> {
        if let Some(id) = self.animation.active {
            if self
                .animation
                .programs
                .iter()
                .find(|program| program.id == id)
                .is_none_or(|program| program.timeline.is_none())
            {
                return Err("This program has an opaque payload. Edit its source or register an animation codec before editing typed tracks".into());
            }
        }
        Ok(())
    }
    pub(crate) fn recording_target(&self, target: NodeId) -> NodeId {
        self.animation
            .active
            .and_then(|id| {
                self.animation
                    .programs
                    .iter()
                    .find(|program| program.id == id)
            })
            .filter(|program| program.targets.contains(&target))
            .map_or(target, AnimationProgram::primary_target)
    }
    pub fn seek(&mut self, time: f64) -> Result<(), String> {
        if !time.is_finite() || time < 0.0 {
            return Err("Animation time must be finite and nonnegative".into());
        }
        if let Some(program) = self
            .animation
            .programs
            .iter()
            .find(|program| Some(program.id) == self.animation.active && self.requires_native_animation(program))
            .cloned()
        {
            let elapsed = if self.seeks_animation_elapsed(&program) {
                time
            } else {
                self.animation.compiled.elapsed_at_playhead(time).map_err(|error| error.to_string())?
            };
            self.preview.clear_animation();
            self.animation
                .native
                .seek(&[program], &self.preview, elapsed)?;
            self.animation
                .player
                .seek(time)
                .map_err(|error| error.to_string())?;
            self.preview.host.update_layout();
            self.animation.target_elapsed.clear();
            self.animation.elapsed_clock = false;
            self.animation.playback_completed = false;
            self.animation.preview_range = None;
            self.animation.play_all = false;
            self.generation += 1;
            return Ok(());
        }
        self.editable_program()?;
        self.animation.native.stop();
        self.animation
            .player
            .seek(time.min(self.animation.timeline.duration))
            .map_err(|error| error.to_string())?;
        let samples = self.sample_active_program(self.animation.player.time(), true)?;
        self.preview.preview_samples(&samples)?;
        self.animation.target_elapsed.clear();
        self.animation.elapsed_clock = false;
        self.animation.playback_completed = false;
        self.animation.preview_range = None;
        self.animation.play_all = false;
        self.generation += 1;
        Ok(())
    }
    pub fn play(&mut self) {
        self.animation.play_all = false;
        self.animation.preview_range = None;
        self.animation.player.play();
        self.generation += 1;
    }
    pub(crate) fn start_animation_playback(
        &mut self,
        id: NodeId,
        time: Option<f64>,
        from_playhead: bool,
    ) -> Result<(), String> {
        if self.animation.active != Some(id) {
            self.select_animation_program(id)?;
        }
        let program = self.animation.programs.iter().find(|program| program.id == id).ok_or("Unknown animation program")?;
        if !self.has_animation_targets(program) {
            return Err("Choose live preview targets for this animation program".into());
        }
        let native = self.requires_native_animation(program);
        if native && self.animation.native.timing(id).is_none() {
            return Err("The authored animation is unavailable in the current native preview".into());
        }
        if let Some(time) = time.or_else(|| self.animation.play_all.then_some(0.0)) {
            let elapsed = if from_playhead && !self.seeks_animation_elapsed(program) {
                self.animation.compiled.elapsed_at_playhead(time).map_err(|error| error.to_string())?
            } else {
                time
            };
            if self.animation.timeline.total_duration().is_some_and(|end| elapsed > end) {
                return Err("Choose a time within the total playback duration".into());
            }
            self.animation.player.seek(elapsed).map_err(|error| error.to_string())?;
            self.animation.target_elapsed.clear();
        }
        self.play();
        self.advance(0.0)
    }
    pub fn play_all_animation_programs(&mut self) {
        self.animation.play_all = true;
        self.animation.preview_range = None;
        self.animation.player.play();
        self.generation += 1;
    }

    pub(crate) fn start_combined_playback(&mut self, reference: Option<NodeId>, elapsed: Option<f64>) -> Result<(), String> {
        if !self.animation.programs.iter().any(|program| self.animation_preview_available(program)) {
            return Err("Choose live preview targets for an animation before playing all programs".into());
        }
        if let Some(id) = reference.filter(|id| self.active_animation_program() != Some(*id)) {
            self.select_animation_program(id)?;
        }
        if let Some(time) = elapsed.or_else(|| (!self.animation.play_all || !self.animation.elapsed_clock).then_some(0.0)) {
            self.animation.player.seek(time).map_err(|error| error.to_string())?;
            self.animation.target_elapsed.clear();
        }
        self.play_all_animation_programs();
        self.advance(0.0)
    }
    pub fn set_interactive(&mut self, value: bool) {
        self.animation.player.pause();
        self.animation.native.stop();
        self.animation.preview_range = None;
        self.preview.set_interactive(value);
        self.generation += 1;
    }
    pub fn pause(&mut self) {
        self.animation.player.pause();
        self.generation += 1;
    }
    pub fn stop(&mut self) {
        self.stop_animation_playback();
        self.clear_interaction_preview();
        self.generation += 1;
    }
    pub(crate) fn stop_animation_playback(&mut self) {
        self.animation.player.stop();
        self.animation.native.stop();
        self.animation.target_elapsed.clear();
        self.animation.elapsed_clock = false;
        self.animation.playback_completed = false;
        self.animation.preview_range = None;
        self.animation.play_all = false;
        self.animation.pending_playback = None;
        self.preview.clear_animation();
    }
    pub fn preview_animation_programs(
        &mut self,
        ids: &[NodeId],
        elapsed: f64,
    ) -> Result<(), String> {
        if !elapsed.is_finite() || elapsed < 0.0 {
            return Err("Preview time must be finite and nonnegative".into());
        }
        let selected = ids.iter().copied().collect::<BTreeSet<_>>();
        if !selected.iter().all(|id| {
            self.animation
                .programs
                .iter()
                .any(|program| program.id == *id)
        }) {
            return Err("Unknown animation program".into());
        }
        let native_programs = self
            .animation
            .programs
            .iter()
            .filter(|program| selected.contains(&program.id) && self.has_animation_targets(program))
            .cloned()
            .collect::<Vec<_>>();
        if native_programs
            .iter()
            .any(|program| self.requires_native_animation(program))
        {
            self.preview.clear_animation();
            self.animation
                .native
                .seek(&native_programs, &self.preview, elapsed)?;
            self.preview.host.update_layout();
            self.animation.player.pause();
            self.animation.elapsed_clock = false;
            self.generation += 1;
            return Ok(());
        }
        self.animation.native.stop();
        let mut values = BTreeMap::new();
        for program in &self.animation.programs {
            if !selected.contains(&program.id) || !self.has_animation_targets(program) {
                continue;
            }
            if !self.animation.compiled_programs.contains_key(&program.id) {
                return Err(format!(
                    "Program {} requires an extension codec",
                    program.name
                ));
            }
            for sample in self.sample_program(program.id, elapsed, false)? {
                values.insert((sample.target, sample.property.clone()), sample);
            }
        }
        self.preview
            .preview_samples(&values.into_values().collect::<Vec<_>>())?;
        self.animation.player.pause();
        self.animation.elapsed_clock = false;
        self.generation += 1;
        Ok(())
    }
    pub fn advance(&mut self, seconds: f64) -> Result<(), String> {
        if !self.animation.player.is_playing() {
            return Ok(());
        }
        if !seconds.is_finite() || seconds < 0.0 {
            return Err("Playback delta must be finite and nonnegative".into());
        }
        if let Some(range) = self.animation.preview_range {
            return self.advance_preview_range(range, seconds);
        }
        let native_programs = self
            .animation
            .programs
            .iter()
            .filter(|program| (self.animation.play_all || Some(program.id) == self.animation.active) && self.animation_preview_available(program))
            .cloned()
            .collect::<Vec<_>>();
        if native_programs
            .iter()
            .any(|program| self.requires_native_animation(program))
        {
            let elapsed = self.animation.player.time() + seconds;
            self.preview.clear_animation();
            let complete =
                self.animation
                    .native
                    .advance_to(&native_programs, &self.preview, elapsed)?;
            self.animation
                .player
                .seek(elapsed)
                .map_err(|error| error.to_string())?;
            if complete {
                self.animation.player.pause();
            }
            self.preview.host.update_layout();
            self.animation.elapsed_clock = true;
            self.animation.playback_completed = complete;
            self.generation += 1;
            return Ok(());
        }
        self.animation.native.stop();
        let selected = self
            .animation
            .programs
            .iter()
            .filter(|program| {
                program.timeline.is_some()
                    && (self.animation.play_all || Some(program.id) == self.animation.active)
            })
            .collect::<Vec<_>>();
        let mut clocks = self.animation.target_elapsed.clone();
        let mut values = BTreeMap::new();
        let mut position = 0.0_f64;
        let mut completed = true;
        let mut sampled = false;
        for program in selected {
            let timeline = program.timeline.as_ref().expect("selected typed program");
            for target in &program.targets {
                sampled = true;
                let compiled = self
                    .animation
                    .compiled_targets
                    .get(&(program.id, *target))
                    .ok_or_else(|| {
                        format!(
                            "Program {} could not be compiled for target {target}",
                            program.name
                        )
                    })?;
                let visible = self
                    .preview
                    .visual(*target)
                    .is_some_and(|visual| visual.is_effectively_visible());
                let previous = clocks
                    .get(&(program.id, *target))
                    .copied()
                    .unwrap_or(self.animation.player.time());
                let mut elapsed = previous
                    + if !visible && timeline.pauses_when_hidden(true) {
                        0.0
                    } else {
                        seconds
                    };
                if !elapsed.is_finite() {
                    return Err("Playback clock overflow".into());
                }
                if let Some(end) = compiled.total_duration() {
                    elapsed = elapsed.min(end);
                    completed &= elapsed >= end;
                } else {
                    completed = false;
                }
                for sample in compiled
                    .sample(elapsed)
                    .map_err(|error| error.to_string())?
                {
                    values.insert((sample.target, sample.property.clone()), sample);
                }
                clocks.insert((program.id, *target), elapsed);
                position = position.max(elapsed);
            }
        }
        if !sampled {
            self.editable_program()?;
            position = self.animation.player.time() + seconds;
            if !position.is_finite() {
                return Err("Playback clock overflow".into());
            }
            if let Some(end) = self.animation.timeline.total_duration() {
                position = position.min(end);
                completed = position >= end;
            } else {
                completed = false;
            }
            for sample in self.sample_active_program(position, false)? {
                values.insert((sample.target, sample.property.clone()), sample);
            }
        }
        self.preview
            .preview_samples(&values.into_values().collect::<Vec<_>>())?;
        self.animation.target_elapsed = clocks;
        self.animation
            .player
            .seek(position)
            .map_err(|error| error.to_string())?;
        if completed {
            self.animation.player.pause();
        }
        self.animation.elapsed_clock = true;
        self.animation.playback_completed = completed;
        self.generation += 1;
        Ok(())
    }
    fn sample_active_program(&self, time: f64, scrub: bool) -> Result<Vec<Sample>, String> {
        if let Some(id) = self.animation.active {
            self.sample_program(id, time, scrub)
        } else {
            (if scrub {
                self.animation.compiled.scrub(time)
            } else {
                self.animation.compiled.sample(time)
            })
            .map_err(|error| error.to_string())
        }
    }
    fn sample_program(&self, id: NodeId, time: f64, scrub: bool) -> Result<Vec<Sample>, String> {
        let program = self
            .animation
            .programs
            .iter()
            .find(|program| program.id == id)
            .ok_or("Unknown animation program")?;
        let mut result = Vec::new();
        for target in &program.targets {
            let compiled = self
                .animation
                .compiled_targets
                .get(&(id, *target))
                .ok_or_else(|| {
                    format!(
                        "Program {} could not be compiled for target {target}",
                        program.name
                    )
                })?;
            result.extend(
                (if scrub {
                    compiled.scrub(time)
                } else {
                    compiled.sample(time)
                })
                .map_err(|error| error.to_string())?,
            );
        }
        Ok(result)
    }

    /// Sample one source-backed scalar curve from the accepted artboard. Native
    /// clips use the same loaded animator as playback. Typed clips use their
    /// per-target compiled track, including the target's underlying base value.
    pub fn animation_curve(
        &self,
        id: NodeId,
        target: NodeId,
        property: &str,
        window: CurveWindow,
    ) -> Result<Vec<CurvePoint>, String> {
        self.animation_curve_channel(id, target, property, CurveChannel::Value, window)
    }

    pub fn animation_curve_channel(
        &self,
        id: NodeId,
        target: NodeId,
        property: &str,
        channel: impl Into<TrackCurveChannel>,
        window: CurveWindow,
    ) -> Result<Vec<CurvePoint>, String> {
        if !matches!(self.preview_status(), crate::PreviewStatus::Ready { revision } if *revision == self.document().revision()) {
            return Err("Open the current animation preview to sample its tracks".into());
        }
        let program = self.animation.programs.iter().find(|program| program.id == id)
            .ok_or("Unknown animation program")?;
        let channel = channel.into();
        if self.requires_native_animation(program) {
            return self.animation.native.curve(program, &self.preview, target, property, channel, window);
        }
        let compiled = self.animation.compiled_targets.get(&(id, target))
            .or_else(|| self.animation.compiled_programs.get(&id))
            .ok_or("Choose a compiled animation program")?;
        let TrackCurveChannel::Property(channel) = channel else {
            return Err("A registered effect curve requires the native animator".into());
        };
        compiled.curve(target, property, channel, window).map_err(|error| error.to_string())
    }
    pub fn set_duration(&mut self, duration: f64) -> Result<(), String> {
        self.editable_program()?;
        let candidate = self.animation.timeline.with_timing(TimelineTimingEdit {
            duration: Some(duration),
            ..Default::default()
        }).map_err(|error| error.to_string())?;
        self.write_timeline(candidate, "Change animation duration")
    }
    pub fn record_keyframe(
        &mut self,
        target: NodeId,
        property: &str,
        value: AnimationValue,
    ) -> Result<(), String> {
        self.editable_program()?;
        let target = self.recording_target(target);
        let mut candidate = self.animation.timeline.clone();
        if candidate.track(target, property).is_none() {
            candidate
                .record(target, property, 0.0, value.clone())
                .map_err(|error| error.to_string())?;
            candidate
                .record(target, property, candidate.duration, value.clone())
                .map_err(|error| error.to_string())?;
        }
        candidate
            .record(
                target,
                property,
                self.animation.player.time().min(candidate.duration),
                value,
            )
            .map_err(|error| error.to_string())?;
        self.write_timeline(candidate, "Add animation keyframe")
    }

    pub(crate) fn record_effective_keyframe(
        &mut self,
        target: NodeId,
        property: &str,
        time: f64,
    ) -> Result<(), String> {
        self.editable_program()?;
        let track_target = self.recording_target(target);
        let authored_effect = self.animation.timeline.track(track_target, property)
            .and_then(|track| track.keyframes.first())
            .is_some_and(|key| matches!(&key.value, AnimationValue::Effect(_)));
        let transform_component = self.active_animation_program().is_some_and(|id| {
            self.animation.native.transform_component(id, property).is_some()
        });
        if self.uses_rust_source() || authored_effect || transform_component {
            let program = self.active_animation_program().ok_or("Choose an authored animation")?;
            return self.record_sampled_keyframe(program, track_target, property, time);
        }
        self.seek(time)?;
        let visual = self
            .preview
            .visual(target)
            .ok_or("The animation target has no accepted preview")?;
        let info =
            ferroui_inspect::Inspector::property(&visual, property).map_err(|error| error.to_string())?;
        let value = self.animation_value_from_preview(target, property, &visual.get_value_untyped(info))?;
        self.record_keyframe(target, property, value)?;
        let mut key = ferroui_designer_animation::KeyframeLocation {
            target: self.recording_target(target), property: property.into(), time,
        };
        key.reconcile_time(self.timeline());
        self.seek(key.time)
    }

    pub(crate) fn record_sampled_keyframe(
        &mut self, program: NodeId, target: NodeId, property: &str, time: f64,
    ) -> Result<(), String> {
        let sample = self.sample_keyframe(program, target, property, time)?;
        let pose = self.animation_program_pose(program, sample.time());
        self.edit_program_keyframe(program, target, property, sample.edit())?;
        // The source transaction has been committed. A newly rebuilt preview can
        // lose its target, so restoring the pose is independent of edit success.
        if let Some(pose) = pose {
            let _ = self.restore_timeline_preview_pose(&pose);
        }
        Ok(())
    }

    /// Sample an authored iteration without changing selection or the preview pose.
    pub fn sample_keyframe(
        &self, id: NodeId, target: NodeId, property: &str, time: f64,
    ) -> Result<crate::SampledKeyframe, String> {
        use ferroui_designer_animation::native_key_time;
        if !matches!(self.preview_status(), crate::PreviewStatus::Ready { revision } if *revision == self.document().revision()) {
            return Err("Open the current animation preview before adding a keyframe".into());
        }
        let program = self.animation_programs().iter().find(|program| program.id == id)
            .ok_or("Unknown animation program")?;
        let timeline = program.timeline.as_ref().ok_or("Choose a typed animation program")?;
        if !(0.0..=timeline.duration).contains(&time) {
            return Err("Choose a time within the animation duration".into());
        }
        let time = native_key_time(time).map_err(|error| error.to_string())?;
        let track = timeline.track(target, property);
        let key = track.and_then(|track| track.keyframes.iter().find(|key| key.time == time));
        let value = if track.is_some() && self.requires_native_animation(program) {
            let value = self.animation.native.sample_keyframe(program, &self.preview, target, property, time)?;
            self.animation_value_from_preview(target, property, &value)?
        } else if track.is_some() {
            self.animation.compiled_targets.get(&(id, target))
                .ok_or("Choose a compiled animation target")?.scrub(time)
                .map_err(|error| error.to_string())?.into_iter()
                .find(|sample| sample.target == target && sample.property == property)
                .ok_or("The animation did not produce a value for this property")?.value
        } else {
            let visual = self.preview.visual(target).ok_or("Choose a live preview target for this property")?;
            let info = ferroui_inspect::Inspector::property(&visual, property).map_err(|error| error.to_string())?;
            self.animation_value_from_preview(target, property, &visual.get_value_untyped(info))?
        };
        let value = if let Some(track) = track { track.sample_in_authored_form(time, value) } else { value };
        Ok(crate::SampledKeyframe {
            previous: key.map(|key| key.time),
            time,
            value: value.to_xaml(),
            easing: key.map_or(Easing::Linear.to_xaml(), |key| key.easing.to_xaml()),
        })
    }

    pub fn move_keyframe(
        &mut self,
        target: NodeId,
        property: &str,
        from: f64,
        to: f64,
    ) -> Result<(), String> {
        self.editable_program()?;
        let target = self.recording_target(target);
        let mut candidate = self.animation.timeline.clone();
        candidate
            .track_mut(target, property)
            .ok_or("No animation track")?
            .move_keyframe(from, to)
            .map_err(|error| error.to_string())?;
        self.write_timeline(candidate, "Move animation keyframe")
    }
    pub fn remove_keyframe(
        &mut self,
        target: NodeId,
        property: &str,
        time: f64,
    ) -> Result<(), String> {
        self.editable_program()?;
        let target = self.recording_target(target);
        if self.uses_rust_source() {
            let program = self.active_animation_program().ok_or("Choose a mapped Rust animation")?;
            return self.edit_program_keyframe(
                program, target, property, ferroui_designer_animation::KeyframeEdit::Remove(time),
            );
        }
        let mut candidate = self.animation.timeline.clone();
        candidate
            .track_mut(target, property)
            .ok_or("No animation track")?
            .remove_keyframe(time)
            .map_err(|error| error.to_string())?;
        candidate.tracks.retain(|track| !track.keyframes.is_empty());
        self.write_timeline(candidate, "Remove animation keyframe")
    }
    pub fn set_keyframe_easing(
        &mut self,
        target: NodeId,
        property: &str,
        time: f64,
        easing: Easing,
    ) -> Result<(), String> {
        self.editable_program()?;
        let target = self.recording_target(target);
        let mut candidate = self.animation.timeline.clone();
        candidate
            .track_mut(target, property)
            .ok_or("No animation track")?
            .set_easing(time, easing)
            .map_err(|error| error.to_string())?;
        self.write_timeline(candidate, "Change keyframe easing")
    }
    pub fn commit_animation(&mut self) -> Result<(), String> {
        self.write_timeline(self.animation.timeline.clone(), "Write animation to XAML")
    }
    fn write_timeline(&mut self, candidate: Timeline, label: &str) -> Result<(), String> {
        self.write_timeline_edits(candidate, label, Vec::new())?;
        // Reload replaces the runtime graph. Keep the artboard at its edit position after
        // key/value/curve changes instead of displaying the unanimated base until the next drag.
        if !self.animation.elapsed_clock {
            self.seek(self.animation.player.time())?;
        }
        Ok(())
    }
    pub(crate) fn program_edits(&self, id: NodeId, timeline: &Timeline) -> Result<Vec<Edit>, String> {
        if self.uses_rust_source() {
            return Err("Edit the Rust program in source or choose mapped keyframes in Timeline".into());
        }
        let program = self
            .animation
            .programs
            .iter()
            .find(|program| program.id == id)
            .ok_or("Unknown animation program")?;
        let generated = self
            .animation
            .codecs
            .encode_for_document(&self.document, program, timeline)
            .map_err(|error| error.to_string())?;
        ferroui_designer_animation::compose_animation_edit(&self.document, id, &generated)
            .map_err(|error| error.to_string())
    }
    pub(crate) fn write_timeline_edits(
        &mut self,
        candidate: Timeline,
        label: &str,
        ordinary: Vec<Edit>,
    ) -> Result<(), String> {
        self.ensure_writable().map_err(|error| error.to_string())?;
        if self.uses_rust_source() {
            return Err("Edit the Rust program in source or choose mapped keyframes in Timeline".into());
        }
        self.editable_program()?;
        candidate.validate().map_err(|error| error.to_string())?;
        let active = self.animation.active;
        let primary = active
            .and_then(|id| {
                self.animation
                    .programs
                    .iter()
                    .find(|program| program.id == id)
            })
            .map(AnimationProgram::primary_target);
        let mut targets = candidate
            .tracks
            .iter()
            .map(|track| track.target)
            .collect::<BTreeSet<_>>();
        if let Some(primary) = primary {
            targets.insert(primary);
        }
        if targets.is_empty() {
            self.animation.compiled = candidate.compile().map_err(|error| error.to_string())?;
            self.animation.timeline = candidate;
            if !ordinary.is_empty() {
                self.apply(label, ordinary)
                    .map_err(|error| error.to_string())?;
            }
            self.generation += 1;
            return Ok(());
        }
        let mut edits = Vec::new();
        let mut existing_names = self
            .document
            .ast()
            .elements()
            .filter_map(|node| node.xaml_name())
            .map(str::to_owned)
            .collect::<BTreeSet<_>>();
        for target in targets {
            let mut timeline = candidate.clone();
            timeline.tracks.retain(|track| track.target == target);
            if Some(target) == primary {
                edits.extend(self.program_edits(active.expect("active"), &timeline)?);
            } else {
                let owner = self
                    .document
                    .ast()
                    .node(target)
                    .ok_or("Animation target was removed")?;
                let name = owner.xaml_name().map(str::to_owned).unwrap_or_else(|| {
                    let mut suffix = 0;
                    loop {
                        let name = format!("DesignerNode{target}_{suffix}");
                        if existing_names.insert(name.clone()) {
                            break name;
                        }
                        suffix += 1;
                    }
                });
                let mut planned = self.document.clone();
                if owner.xaml_name().is_none() {
                    let edit = Edit::SetAttribute {
                        node: target,
                        name: "Name".into(),
                        value: name.clone(),
                    };
                    planned
                        .apply("Name animation target", vec![edit.clone()])
                        .map_err(|error| error.to_string())?;
                    edits.push(edit);
                }
                edits.extend(
                    AnimationPrograms::create(
                        &planned,
                        target,
                        &format!("{name} animation"),
                        &format!("{}#{name}", owner.name.replace(':', "|")),
                        &timeline,
                    )
                    .map_err(|error| error.to_string())?,
                );
            }
        }
        // Source patches address the original snapshot. Apply them before structural/name edits.
        edits.sort_by_key(|edit| match edit {
            Edit::Text { span, .. } => (0, std::cmp::Reverse(span.start)),
            _ => (1, std::cmp::Reverse(0)),
        });
        edits.extend(ordinary);
        self.commit_document(label.into(), edits)
            .map_err(|error| error.to_string())?;
        self.changed(true);
        Ok(())
    }
    pub(crate) fn import_animations(&mut self) {
        self.animation.native.clear();
        if !self.document.is_valid() {
            return;
        }
        self.animation.target_elapsed.clear();
        let previous = self.animation.active;
        let previous_key = previous
            .and_then(|id| {
                self.animation
                    .programs
                    .iter()
                    .find(|program| program.id == id)
            })
            .and_then(|program| program.source_key.clone());
        let named_overrides = self
            .animation
            .programs
            .iter()
            .filter_map(|program| {
                Some((
                    program.source_key.clone()?,
                    self.animation.preview_targets.get(&program.id)?.clone(),
                ))
            })
            .collect::<BTreeMap<_, _>>();
        let duration = self.animation.timeline.duration;
        let mut programs = AnimationPrograms::scan(&self.document);
        for program in &programs {
            if let Some(targets) = program
                .source_key
                .as_ref()
                .and_then(|key| named_overrides.get(key))
            {
                self.animation.preview_targets.insert(
                    program.id,
                    targets
                        .iter()
                        .copied()
                        .filter(|target| self.preview.visual(*target).is_some())
                        .collect(),
                );
            }
        }
        for program in &mut programs {
            match self
                .animation
                .preview_targets
                .get(&program.id)
                .cloned()
                .map(Ok)
                .unwrap_or_else(|| self.resolve_program_targets(program))
            {
                Ok(targets) => program.targets = targets,
                Err(error) => program.diagnostics.push(error),
            }
        }
        self.animation
            .native
            .refresh(&self.document, &self.preview, &programs);
        for program in &mut programs {
            let options = self.animation_import_options(program);
            match ferroui_designer_animation::animation_fragment(&self.document, program.id) {
                Ok(source) => self.animation.codecs.decode(program, &source, &options),
                Err(error) => program.diagnostics.push(error.to_string()),
            }
        }
        let mut compiled_targets = BTreeMap::new();
        for program in &mut programs {
            let Ok(source) =
                ferroui_designer_animation::animation_fragment(&self.document, program.id)
            else {
                continue;
            };
            for target in program.targets.clone() {
                let mut instance = program.clone();
                instance.targets = vec![target];
                instance.diagnostics.clear();
                let options = self.animation_import_options(&instance);
                self.animation
                    .codecs
                    .decode(&mut instance, &source, &options);
                if let Some(compiled) = instance
                    .timeline
                    .as_ref()
                    .and_then(|timeline| timeline.compile().ok())
                {
                    compiled_targets.insert((program.id, target), compiled);
                } else {
                    program.diagnostics.extend(
                        instance
                            .diagnostics
                            .into_iter()
                            .map(|diagnostic| format!("Target {target}: {diagnostic}")),
                    );
                }
            }
        }
        self.animation.compiled_targets = compiled_targets;
        for program in &mut programs {
            if self.animation.native.uses_factory_target(program.id) {
                program.diagnostics.push("Native preview uses the resource factory's artboard root; this temporary target is not authored in the document".into());
            }
        }
        self.animation
            .preview_targets
            .retain(|id, _| programs.iter().any(|program| program.id == *id));
        self.animation.warnings = programs
            .iter()
            .flat_map(|program| {
                program
                    .diagnostics
                    .iter()
                    .map(move |warning| format!("{}: {warning}", program.name))
            })
            .collect();
        self.animation.compiled_programs = programs
            .iter()
            .filter_map(|program| {
                program
                    .timeline
                    .as_ref()
                    .and_then(|timeline| timeline.compile().ok())
                    .map(|timeline| (program.id, timeline))
            })
            .collect();
        self.animation.active = previous
            .filter(|id| programs.iter().any(|program| program.id == *id))
            .or_else(|| {
                previous_key
                    .as_ref()
                    .and_then(|key| {
                        programs
                            .iter()
                            .find(|program| program.source_key.as_ref() == Some(key))
                    })
                    .map(|program| program.id)
            })
            .or_else(|| programs.first().map(|program| program.id));
        self.animation.timeline = self
            .animation
            .active
            .and_then(|id| programs.iter().find(|program| program.id == id))
            .map(|program| self.animation_timing(program))
            .unwrap_or_else(|| Timeline::new(duration).expect("retained duration"));
        self.animation.compiled = self
            .animation
            .timeline
            .compile()
            .expect("validated timeline");
        self.animation.programs = programs;
    }

    fn animation_timing(&self, program: &AnimationProgram) -> Timeline {
        if self.seeks_animation_elapsed(program) {
            if let Some(clock) = self.animation.native.timing(program.id) {
                return clock;
            }
        }
        program.timeline.clone().unwrap_or_else(|| {
            Timeline::new(self.animation.native.duration(program.id).unwrap_or(3.0)).expect("native duration")
        })
    }
    pub(crate) fn animation_value_for_target(
        &self,
        target: NodeId,
        property: &str,
        value: &str,
    ) -> Result<AnimationValue, String> {
        if let Some(kind) = self.animation_kind_for_target(target, property) {
            if kind == ValueKind::Color {
                if let Some(color) = ferroui_base::media::Color::try_parse(value) {
                    return Ok(color.into());
                }
            }
            return AnimationValue::parse_native_literal(value, kind).map_err(|error| error.to_string());
        }
        animation_value(property, value)
    }

    fn animation_value_from_preview(&self, target: NodeId, property: &str, value: &BoxedValue) -> Result<AnimationValue, String> {
        if let Some(effect) = value.downcast_ref::<Option<Rc<dyn ferroui_base::media::IEffect>>>() {
            return effect.as_deref().and_then(AnimationEffect::from_native).map(AnimationValue::Effect)
                .ok_or_else(|| "Choose a native effect that the source can represent as a keyframe".into());
        }
        self.animation_value_for_target(target, property, &ferroui_inspect::display_value(Some(value)))
    }
    /// Whether the current authored node exposes a property supported by the
    /// native typed animation layer.
    pub fn can_record_property(&self, target: NodeId, property: &str) -> bool {
        property != "Name"
            && self.can_manipulate()
            && self
                .preview
                .visual(target)
                .and_then(|visual| ferroui_inspect::Inspector::property(&visual, property).ok())
                .is_some_and(|info| {
                    !info.is_direct()
                        && !info.is_read_only()
                        && (info.property_type_name() == "bool"
                            || animation_property_kind(info).is_some())
                })
    }
    fn animation_kind_for_target(&self, target: NodeId, property: &str) -> Option<ValueKind> {
        let visual = self.preview.visual(target)?;
        let property = ferroui_inspect::Inspector::property(&visual, property).ok()?;
        animation_property_kind(property)
    }
    fn animation_import_options(&self, program: &AnimationProgram) -> ImportOptions {
        let mut options = ImportOptions {
            native_effect_endpoints: true,
            effect_color: Some(AnimationEffect::resolve_color),
            effect_literal: Some(AnimationEffect::parse_native),
            ..ImportOptions::default()
        };
        let target = program.primary_target();
        for setter in subtree_nodes(&self.document, program.id)
            .into_iter()
            .filter_map(|id| self.document.ast().node(id))
            .filter(|node| node.local_name() == "Setter")
        {
            let Some(property) = setter.attribute("Property") else {
                continue;
            };
            if let Some(component) = self.animation.native.transform_component(program.id, property) {
                options.property_kinds.insert(property.into(), ValueKind::Number);
                if let Some(base) = self.animation.native.transform_base_value(
                    program, &self.preview, target, component,
                ) {
                    options.base_values.insert(property.into(), AnimationValue::Number(base));
                }
                continue;
            }
            if self.animation_kind_for_target(target, property).is_none() {
                if let Some(info) = self
                    .preview
                    .visual(target)
                    .and_then(|visual| ferroui_inspect::Inspector::property(&visual, property).ok())
                {
                    if info.property_type_name() != "bool" {
                        options.interpolation_errors.insert(property.into(), format!("{property} uses native {} interpolation; preview uses the loaded framework animation and typed editing requires a matching value codec", info.property_type_name()));
                    }
                } else {
                    options.interpolation_errors.insert(property.into(), format!("{property} is resolved by the native animation target; use its loaded runtime program or register a typed codec"));
                }
            }
            let literal = setter
                .attribute("Value")
                .filter(|value| !value.starts_with('{'));
            let resolved = literal.and_then(|value| {
                self.animation_value_for_target(target, property, value)
                    .ok()
            });
            let kind = self
                .animation_kind_for_target(target, property)
                .or_else(|| resolved.as_ref().map(AnimationValue::kind));
            if let (Some(text), Some(value)) = (literal, resolved) {
                options
                    .literal_values
                    .insert((property.into(), text.into()), value);
            }
            if let Some(kind) = kind {
                options.property_kinds.insert(property.into(), kind);
            }
            let base = self
                .document
                .ast()
                .node(target)
                .and_then(|node| node.attribute(property))
                .filter(|value| !value.starts_with('{'))
                .and_then(|value| {
                    self.animation_value_for_target(target, property, value)
                        .ok()
                })
                .or_else(|| {
                    let visual = self.preview.visual(target)?;
                    let property_info =
                        ferroui_inspect::Inspector::property(&visual, property).ok()?;
                    self.animation_value_from_preview(target, property, &visual.get_value_untyped(property_info)).ok()
                });
            if let Some(base) = base.filter(|base| kind.is_none_or(|kind| base.kind() == kind)) {
                options.base_values.insert(property.into(), base);
            }
        }
        options
    }
    fn resolve_program_targets(&self, program: &AnimationProgram) -> Result<Vec<NodeId>, String> {
        let Some(style_id) = program.style else {
            return Ok(Vec::new());
        };
        let mut ancestors = Vec::new();
        let mut current = Some(style_id);
        while let Some(id) = current {
            let node = self
                .document
                .ast()
                .node(id)
                .ok_or("Style source disappeared")?;
            if node.local_name() == "Style" {
                ancestors.push(id);
            }
            current = node.parent;
        }
        ancestors.reverse();
        let mut styles = Vec::<ferroui_base::Ref<Style>>::new();
        for id in ancestors {
            let node = self.document.ast().node(id).expect("style");
            let namespaces = self.document.ast().namespaces(id);
            let parser = SelectorParser::new(move |prefix, name| {
                let namespace = namespaces.get(prefix).map(String::as_str).unwrap_or("");
                let clr = namespace.strip_prefix("using:").or_else(|| {
                    namespace
                        .strip_prefix("clr-namespace:")
                        .map(|value| value.split(';').next().unwrap_or(value))
                });
                clr.and_then(|namespace| TypeInfo::find(namespace, name))
                    .or_else(|| TypeInfo::find_by_name(name))
            });
            let mut selector = node.attribute("Selector").unwrap_or("").to_owned();
            if self.preview.is_window_projection() {
                selector = crate::preview_source::replace_window_selector(
                    &selector,
                    &self.document.ast().namespaces(id),
                );
            }
            let parsed = parser.parse(&selector).map_err(|error| error.to_string())?;
            let style = Style::new();
            style.set_selector(parsed);
            if let Some(parent) = styles.last() {
                parent.children().add(style.clone());
            }
            styles.push(style);
        }
        let style = styles.last().ok_or("Animation has no selector style")?;
        let selector = style
            .selector()
            .ok_or("This animation style has an empty selector")?;
        let parent = style.parent();
        Ok(subtree_nodes(&self.document, program.owner)
            .into_iter()
            .filter_map(|id| self.document.ast().node(id))
            .filter_map(|node| {
                let visual = self.preview.visual(node.id)?;
                let styled = visual.cast::<StyledElement>()?;
                selector
                    .match_(&styled, parent.as_deref(), false)
                    .is_match()
                    .then_some(node.id)
            })
            .collect())
    }
}

fn animation_property_kind(property: &FerroProperty) -> Option<ValueKind> {
    if AnimationEffect::is_effect_property(property) {
        return Some(ValueKind::Effect);
    }
    let name = property.property_type_name();
    if name.contains("Color") || name.contains("Brush") {
        return Some(ValueKind::Color);
    }
    match name.rsplit([':', '.']).next().unwrap_or(name) {
        "f64" | "f32" => Some(ValueKind::Number),
        "Point" => Some(ValueKind::Point),
        "Vector" => Some(ValueKind::Vector),
        "Size" => Some(ValueKind::Size),
        "Rect" => Some(ValueKind::Rect),
        "CornerRadius" => Some(ValueKind::CornerRadius),
        "RelativePoint" => Some(ValueKind::RelativePoint),
        "RelativeScalar" => Some(ValueKind::RelativeScalar),
        "Thickness" => Some(ValueKind::Thickness),
        _ => None,
    }
}

pub(crate) fn animation_value(property: &str, value: &str) -> Result<AnimationValue, String> {
    if matches!(
        property.rsplit('.').next().unwrap_or(property),
        "Background"
            | "Foreground"
            | "BorderBrush"
            | "Fill"
            | "Stroke"
            | "Color"
            | "CaretBrush"
            | "SelectionBrush"
    ) {
        if let Some(color) = ferroui_base::media::Color::try_parse(value) {
            return Ok(color.into());
        }
    }

    ferroui_designer_animation::state_value(property, value).map_err(|error| error.to_string())
}


fn subtree_nodes(document: &Document, root: NodeId) -> Vec<NodeId> {
    let mut pending = vec![root];
    let mut nodes = Vec::new();
    while let Some(id) = pending.pop() {
        if let Some(node) = document.ast().node(id) {
            nodes.push(id);
            pending.extend(node.children.iter().rev());
        }
    }
    nodes
}
