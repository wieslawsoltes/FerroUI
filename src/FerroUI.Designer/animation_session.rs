//! UI-thread adapter for independently selected, source-backed animation programs.
use crate::DesignerSession;
use ferroui_base::styling::Style;
use ferroui_base::{StyledElement, TypeInfo};
use ferroui_designer_animation::{
    AnimationCodec, AnimationCodecRegistry, AnimationProgram, AnimationPrograms, AnimationValue,
    CompiledTimeline, Easing, ImportOptions, Player, Sample, Timeline, ValueKind,
};
use ferroui_designer_core::{Document, Edit, NodeId};
use ferroui_markup::markup::parsers::SelectorParser;
use std::collections::{BTreeMap, BTreeSet};

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
    play_all: bool,
    compiled_programs: BTreeMap<NodeId, CompiledTimeline>,
    compiled_targets: BTreeMap<(NodeId, NodeId), CompiledTimeline>,
    preview_targets: BTreeMap<NodeId, Vec<NodeId>>,
    target_elapsed: BTreeMap<(NodeId, NodeId), f64>,
}
impl AnimationSession {
    pub(crate) fn clear_program_state(&mut self) {
        self.native.clear();
        self.active = None;
        self.preview_targets.clear();
        self.compiled_targets.clear();
        self.play_all = false;
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
        self.animation.recording = value && !self.is_read_only();
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
        let timeline = program.timeline.clone().unwrap_or_else(|| {
            Timeline::new(self.animation.native.duration(id).unwrap_or(3.0))
                .expect("native duration")
        });
        self.animation.compiled = timeline.compile().map_err(|error| error.to_string())?;
        self.animation.timeline = timeline;
        self.animation.active = Some(id);
        self.animation.player.stop();
        self.animation.target_elapsed.clear();
        self.animation.play_all = false;
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
        let edits =
            AnimationPrograms::remove(&self.document, id).map_err(|error| error.to_string())?;
        self.apply("Remove animation program", edits)
            .map_err(|error| error.to_string())?;
        Ok(())
    }
    pub fn rename_animation_program(&mut self, id: NodeId, name: &str) -> Result<(), String> {
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
        if active {
            self.seek(self.animation.player.time())?;
        }
        Ok(())
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
            .find(|program| Some(program.id) == self.animation.active && program.timeline.is_none())
            .cloned()
        {
            self.preview.clear_animation();
            self.animation
                .native
                .seek(&[program], &self.preview, time)?;
            self.animation
                .player
                .seek(time)
                .map_err(|error| error.to_string())?;
            self.preview.host.update_layout();
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
        self.generation += 1;
        Ok(())
    }
    pub fn play(&mut self) {
        self.animation.play_all = false;
        self.animation.player.play();
        self.generation += 1;
    }
    pub fn play_all_animation_programs(&mut self) {
        self.animation.play_all = true;
        self.animation.player.play();
        self.generation += 1;
    }
    pub fn set_interactive(&mut self, value: bool) {
        self.animation.player.pause();
        self.animation.native.stop();
        self.preview.set_interactive(value);
        self.generation += 1;
    }
    pub fn pause(&mut self) {
        self.animation.player.pause();
        self.generation += 1;
    }
    pub fn stop(&mut self) {
        self.animation.player.stop();
        self.animation.native.stop();
        self.animation.target_elapsed.clear();
        self.preview.clear_animation();
        self.clear_interaction_preview();
        self.generation += 1;
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
            .filter(|program| selected.contains(&program.id))
            .cloned()
            .collect::<Vec<_>>();
        if native_programs
            .iter()
            .any(|program| program.timeline.is_none())
        {
            self.preview.clear_animation();
            self.animation
                .native
                .seek(&native_programs, &self.preview, elapsed)?;
            self.preview.host.update_layout();
            self.generation += 1;
            return Ok(());
        }
        self.animation.native.stop();
        let mut values = BTreeMap::new();
        for program in &self.animation.programs {
            if !selected.contains(&program.id) {
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
        let native_programs = self
            .animation
            .programs
            .iter()
            .filter(|program| self.animation.play_all || Some(program.id) == self.animation.active)
            .cloned()
            .collect::<Vec<_>>();
        if native_programs
            .iter()
            .any(|program| program.timeline.is_none())
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
    pub fn set_duration(&mut self, duration: f64) -> Result<(), String> {
        self.editable_program()?;
        if !duration.is_finite() || duration <= 0.0 {
            return Err("Duration must be finite and positive".into());
        }
        let mut candidate = self.animation.timeline.clone();
        let scale = duration / candidate.duration;
        for track in &mut candidate.tracks {
            for key in &mut track.keyframes {
                key.time *= scale;
            }
        }
        candidate.duration = duration;
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
        self.seek(self.animation.player.time())
    }
    fn program_edits(&self, id: NodeId, timeline: &Timeline) -> Result<Vec<Edit>, String> {
        let program = self
            .animation
            .programs
            .iter()
            .find(|program| program.id == id)
            .ok_or("Unknown animation program")?;
        let original = ferroui_designer_animation::animation_fragment(&self.document, id)
            .map_err(|error| error.to_string())?;
        let generated = self
            .animation
            .codecs
            .encode(program, &original, timeline)
            .map_err(|error| error.to_string())?;
        let generated =
            if program.codec.as_deref() == Some("ferroui.native") && !timeline.tracks.is_empty() {
                let mut generated_document =
                    Document::new(generated).map_err(|error| error.to_string())?;
                let root = generated_document.ast().root();
                let existing = generated_document
                    .ast()
                    .node(root)
                    .expect("root")
                    .attributes
                    .iter()
                    .any(|attribute| {
                        attribute.namespace_uri.as_deref()
                            == Some("http://schemas.microsoft.com/winfx/2006/xaml")
                            && attribute.name.rsplit(':').next() == Some("SetterTargetType")
                    });
                if !existing {
                    let target = self
                        .document
                        .ast()
                        .node(program.primary_target())
                        .ok_or("Animation target source disappeared")?;
                    let mut namespaces = generated_document.ast().namespaces(root);
                    let prefix = namespaces
                        .iter()
                        .find(|(prefix, uri)| {
                            !prefix.is_empty()
                                && uri.as_str() == "http://schemas.microsoft.com/winfx/2006/xaml"
                        })
                        .map(|(prefix, _)| prefix.clone())
                        .unwrap_or_else(|| {
                            let mut prefix = "x".to_owned();
                            while namespaces.contains_key(&prefix) {
                                prefix.push('_');
                            }
                            prefix
                        });
                    namespaces.insert(
                        prefix.clone(),
                        "http://schemas.microsoft.com/winfx/2006/xaml".into(),
                    );
                    let mut edits = vec![Edit::SetAttribute {
                        node: root,
                        name: format!("xmlns:{prefix}"),
                        value: "http://schemas.microsoft.com/winfx/2006/xaml".into(),
                    }];
                    let (target_prefix, target_local) = target
                        .name
                        .split_once(':')
                        .unwrap_or(("", target.name.as_str()));
                    let mut target_type = target.name.clone();
                    if let Some(namespace) =
                        self.document.ast().namespaces(target.id).get(target_prefix)
                    {
                        if namespaces.get(target_prefix) != Some(namespace) {
                            let mut alias = "designertarget".to_owned();
                            while namespaces.contains_key(&alias) {
                                alias.push('_');
                            }
                            edits.push(Edit::SetAttribute {
                                node: root,
                                name: format!("xmlns:{alias}"),
                                value: namespace.clone(),
                            });
                            target_type = format!("{alias}:{target_local}");
                        }
                    }
                    edits.push(Edit::SetAttribute {
                        node: root,
                        name: format!("{prefix}:SetterTargetType"),
                        value: target_type,
                    });
                    generated_document
                        .apply("Retain animation setter target type", edits)
                        .map_err(|error| error.to_string())?;
                }
                generated_document.source().to_owned()
            } else {
                generated
            };
        let source = ferroui_designer_animation::native_fragment(&self.document, id, &generated)
            .map_err(|error| error.to_string())?;
        let mut edits = AnimationPrograms::replace_source(&self.document, id, &source)
            .map_err(|error| error.to_string())?;
        let mut staged = self.document.clone();
        staged
            .apply("Stage animation namespace composition", edits.clone())
            .map_err(|error| error.to_string())?;
        let original_start = self
            .document
            .ast()
            .node(id)
            .ok_or("Program source disappeared")?
            .span
            .start;
        let rewritten = staged
            .ast()
            .elements()
            .find(|node| node.span.start == original_start && node.local_name() == "Animation")
            .ok_or("Rewritten animation disappeared")?;
        let parent_scope = rewritten
            .parent
            .map(|parent| staged.ast().namespaces(parent))
            .unwrap_or_default();
        let root = staged.ast().root();
        let mut root_scope = staged.ast().namespaces(root);
        for attribute in rewritten
            .attributes
            .iter()
            .filter(|attribute| attribute.is_namespace())
        {
            let prefix = attribute.name.strip_prefix("xmlns:").unwrap_or("");
            if parent_scope.get(prefix) == Some(&attribute.value) {
                edits.push(Edit::RemoveAttribute {
                    node: rewritten.id,
                    name: attribute.name.clone(),
                });
            } else if !prefix.is_empty() && !root_scope.contains_key(prefix) {
                edits.push(Edit::SetAttribute {
                    node: root,
                    name: attribute.name.clone(),
                    value: attribute.value.clone(),
                });
                edits.push(Edit::RemoveAttribute {
                    node: rewritten.id,
                    name: attribute.name.clone(),
                });
                root_scope.insert(prefix.to_owned(), attribute.value.clone());
            }
        }
        Ok(edits)
    }
    pub(crate) fn write_timeline_edits(
        &mut self,
        candidate: Timeline,
        label: &str,
        ordinary: Vec<Edit>,
    ) -> Result<(), String> {
        self.ensure_writable().map_err(|error| error.to_string())?;
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
        self.animation
            .native
            .refresh(&self.document, &self.preview, &programs);
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
            .and_then(|program| program.timeline.clone())
            .unwrap_or_else(|| {
                Timeline::new(
                    self.animation
                        .active
                        .and_then(|id| self.animation.native.duration(id))
                        .unwrap_or(duration),
                )
                .expect("retained duration")
            });
        self.animation.compiled = self
            .animation
            .timeline
            .compile()
            .expect("validated timeline");
        self.animation.programs = programs;
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
                    return Ok(color_value(color));
                }
            }
            return AnimationValue::parse(value, kind).map_err(|error| error.to_string());
        }
        animation_value(property, value)
    }
    fn animation_kind_for_target(&self, target: NodeId, property: &str) -> Option<ValueKind> {
        let visual = self.preview.visual(target)?;
        let property = ferroui_inspect::Inspector::property(&visual, property).ok()?;
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
    fn animation_import_options(&self, program: &AnimationProgram) -> ImportOptions {
        let mut options = ImportOptions::default();
        let target = program.primary_target();
        for setter in subtree_nodes(&self.document, program.id)
            .into_iter()
            .filter_map(|id| self.document.ast().node(id))
            .filter(|node| node.local_name() == "Setter")
        {
            let Some(property) = setter.attribute("Property") else {
                continue;
            };
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
                    self.animation_value_for_target(
                        target,
                        property,
                        &ferroui_inspect::display_value(Some(
                            &visual.get_value_untyped(property_info),
                        )),
                    )
                    .ok()
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
            return Ok(color_value(color));
        }
    }

    ferroui_designer_animation::state_value(property, value).map_err(|error| error.to_string())
}

fn color_value(color: ferroui_base::media::Color) -> AnimationValue {
    AnimationValue::Color {
        a: color.a as f64 / 255.0,
        r: color.r as f64 / 255.0,
        g: color.g as f64 / 255.0,
        b: color.b as f64 / 255.0,
    }
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
