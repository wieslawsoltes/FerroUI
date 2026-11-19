use crate::animation_codecs::with_codecs;
use ferroui_designer_animation::{
    animation_fragment, compose_animation_edit, AnimationError, AnimationProgram, AnimationValue,
    ImportOptions, KeyframeEdit, KeyframeLocation,
};
use ferroui_designer_core::{Document, NodeId, TextPatch};
use ferroui_remote_protocol::designer::{
    DesignAnimationAuthoring, DesignAnimationAuthoringRequest, DesignAnimationAuthoringResult,
    DesignAnimationKeyframe, DesignAnimationProgram, DesignAnimationSourcePatch,
    DesignAnimationTrack,
};
use std::collections::BTreeMap;

pub(super) struct HostedKeyframes {
    document: Document,
    programs: BTreeMap<i64, AnimationProgram>,
}

impl HostedKeyframes {
    pub(super) fn mount(
        document: Document,
        programs: Vec<AnimationProgram>,
        catalog: &mut [DesignAnimationProgram],
        can_edit: bool,
        curves: bool,
    ) -> Self {
        let candidates = programs
            .into_iter()
            .map(|program| (program.id, program))
            .collect::<BTreeMap<_, _>>();
        let mut programs = BTreeMap::new();
        for entry in catalog {
            let Some(program) = candidates.get(&(entry.id as NodeId)) else {
                continue;
            };
            let mut program = program.clone();
            program.targets = entry
                .default_targets
                .iter()
                .map(|id| *id as NodeId)
                .collect();
            let Ok(source) = animation_fragment(&document, program.id) else {
                continue;
            };
            // Keep ordinary preview usable if a project codec fails during catalog construction.
            let decoded = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                with_codecs(|codecs| {
                    codecs.decode(&mut program, &source, &ImportOptions::default())
                });
                tracks(&program)
            }));
            let Ok(Some(tracks)) = decoded else { continue };
            if curves {
                let tracks: Vec<_> = tracks.iter().filter(|track| {
                    track.keys.first().is_some_and(|key| key.value_kind == "Number")
                }).cloned().collect();
                entry.curve_tracks = (!tracks.is_empty()).then_some(tracks);
            }
            if can_edit && (program.has_project_codec() || entry.curve_tracks.is_some()) {
                entry.authoring = Some(DesignAnimationAuthoring {
                    codec: program.codec.clone().unwrap_or_default(),
                    tracks,
                });
                programs.insert(entry.id, program);
            }
        }
        Self { document, programs }
    }

    pub(super) fn plan(
        &self,
        request: &DesignAnimationAuthoringRequest,
    ) -> Result<DesignAnimationAuthoringResult, String> {
        let program = self
            .programs
            .get(&request.program)
            .ok_or("Choose an editable animation in the current application preview")?;
        let target =
            NodeId::try_from(request.target).map_err(|_| "Choose an authored animation track")?;
        let property = &request.property;
        let edit = KeyframeEdit::Upsert {
            previous: request.previous,
            time: request.time,
            value: &request.value,
            easing: &request.easing,
        };
        let draft = program
            .plan_keyframe(target, property, edit, |text, kind| {
                AnimationValue::parse(
                    text,
                    kind.ok_or_else(|| {
                        AnimationError("Choose an existing authored animation track".into())
                    })?,
                )
            })
            .map_err(|error| error.to_string())?;
        let (source, accepted) = with_codecs(|codecs| {
            let source = codecs.encode_for_document(&self.document, program, &draft)?;
            let mut accepted = program.clone();
            codecs.decode(&mut accepted, &source, &ImportOptions::default());
            Ok::<_, AnimationError>((source, accepted))
        })
        .map_err(|error| error.to_string())?;
        let accepted_timeline = accepted
            .timeline
            .as_ref()
            .filter(|_| accepted.codec == program.codec)
            .ok_or("The animation codec could not read its encoded program")?;
        let mut location = KeyframeLocation {
            target,
            property: property.clone(),
            time: request.time,
        };
        location.reconcile_time(accepted_timeline);
        let expected = draft
            .track(target, property)
            .and_then(|track| track.keyframes.iter().find(|key| key.time == request.time));
        let round_trip = accepted_timeline
            .track(target, property)
            .and_then(|track| track.keyframes.iter().find(|key| key.time == location.time));
        if !matches!((expected, round_trip), (Some(expected), Some(actual)) if expected.value == actual.value && expected.easing == actual.easing)
        {
            return Err("The animation codec did not preserve the draft value and easing".into());
        }
        let preview = accepted_timeline
            .inspect_keyframe(target, property, location.time)
            .map_err(|error| error.to_string())?;
        let edits = compose_animation_edit(&self.document, program.id, &source)
            .map_err(|error| error.to_string())?;
        let mut candidate = self.document.clone();
        candidate
            .apply("Edit application animation keyframe", edits)
            .map_err(|error| error.to_string())?;
        let original = self.document.source();
        let patch = TextPatch::between(original, candidate.source())
            .map(|patch| editor_patch(original, patch))
            .transpose()?;
        Ok(DesignAnimationAuthoringResult {
            revision: request.revision,
            request: request.request,
            patch,
            samples: preview.samples,
            value_kind: format!("{:?}", preview.value_kind),
            error: None,
        })
    }
}

fn tracks(program: &AnimationProgram) -> Option<Vec<DesignAnimationTrack>> {
    let timeline = program.timeline.as_ref()?;
    timeline.validate().ok()?;
    let tracks = timeline
        .tracks
        .iter()
        .filter_map(|track| {
            Some(DesignAnimationTrack {
                target: i64::try_from(track.target).ok()?,
                property: track.property.clone(),
                keys: track
                    .keyframes
                    .iter()
                    .map(|key| DesignAnimationKeyframe {
                        time: key.time,
                        value: key.value.to_xaml(),
                        easing: key.easing.to_xaml(),
                        value_kind: format!("{:?}", key.value.kind()),
                    })
                    .collect(),
            })
        })
        .collect::<Vec<_>>();
    (!tracks.is_empty()).then_some(tracks)
}

fn utf16_offset(source: &str, offset: usize) -> Result<i32, String> {
    i32::try_from(source[..offset].encode_utf16().count())
        .map_err(|_| "The animation edit exceeds the editor's source range".into())
}

fn editor_patch(source: &str, mut patch: TextPatch) -> Result<DesignAnimationSourcePatch, String> {
    let bytes = source.as_bytes();
    let splits_line_ending =
        |offset: usize| offset > 0 && bytes.get(offset - 1..offset + 1) == Some(b"\r\n");
    // Editors represent CRLF as one line break and cannot address its interior.
    if splits_line_ending(patch.span.start) {
        patch.span.start -= 1;
        patch.replacement.insert(0, '\r');
    }
    if splits_line_ending(patch.span.end) {
        patch.span.end += 1;
        patch.replacement.push('\n');
    }
    Ok(DesignAnimationSourcePatch {
        start: utf16_offset(source, patch.span.start)?,
        end: utf16_offset(source, patch.span.end)?,
        expected: source[patch.span.start..patch.span.end].to_owned(),
        replacement: patch.replacement,
    })
}
