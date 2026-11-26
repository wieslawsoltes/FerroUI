#[path = "hosted_keyframe_selection.rs"]
mod selection;

use crate::animation_codecs::with_codecs;
use ferroui_base::animation::Animatable;
use ferroui_base::media::CustomEffectRegistry;
use ferroui_base::{Ref, StyledElement};
use ferroui_designer_animation::native::{native_keyframe_options_for_target, NativeProgramIndex};
use ferroui_designer_animation::{
    animation_fragment, compose_animation_edit, native_keyframe_patches, AnimationError, AnimationProgram, AnimationValue,
    CurveChannel, ImportOptions, KeyframeComponentEdit, KeyframeEdit, KeyframeLocation, Timeline,
    TrackCurveChannel, ValueKind,
};
use ferroui_designer_core::{Document, NodeId, TextPatch};
use ferroui_remote_protocol::designer::{
    DesignAnimationAuthoring, DesignAnimationAuthoringRequest, DesignAnimationAuthoringResult,
    DesignAnimationKeyframe, DesignAnimationProgram, DesignAnimationSourcePatch,
    DesignAnimationTrack, DesignKeyframeComponent,
};
use std::collections::BTreeMap;
use std::rc::Rc;

pub(super) struct HostedKeyframes {
    document: Rc<Document>,
    programs: BTreeMap<i64, (AnimationProgram, ImportOptions)>,
    component_batch: bool,
}

pub(super) struct HostedKeyframeCapabilities {
    pub(super) authoring: bool,
    pub(super) curves: bool,
    pub(super) curve_channels: bool,
    pub(super) sampling: bool,
    pub(super) components: bool,
    pub(super) effects: bool,
    pub(super) effect_objects: bool,
    pub(super) registered_effects: bool,
    pub(super) box_shadows: bool,
    pub(super) transform_lists: bool,
    pub(super) brushes: bool,
}

impl HostedKeyframes {
    pub(super) fn mount(
        document: Rc<Document>,
        programs: Vec<AnimationProgram>,
        native: &NativeProgramIndex,
        targets: &BTreeMap<i64, Ref<StyledElement>>,
        catalog: &mut [DesignAnimationProgram],
        capabilities: HostedKeyframeCapabilities,
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
            let Some(animation) = native.animation(program.id) else { continue };
            program.targets = entry
                .default_targets
                .iter()
                .map(|id| *id as NodeId)
                .collect();
            let target = entry.default_targets.first()
                .and_then(|id| targets.get(id))
                .and_then(|target| target.cast::<Animatable>());
            let options = native_keyframe_options_for_target(&document, &program, animation, target.as_deref());
            let Ok(source) = animation_fragment(&document, program.id) else {
                continue;
            };
            // Keep ordinary preview usable if a project codec fails during catalog construction.
            let decoded = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                with_codecs(|codecs| {
                    codecs.decode(&mut program, &source, &options)
                });
                tracks(&program, capabilities.components)
            }));
            let Ok(Some(mut tracks)) = decoded else { continue };
            tracks.retain(|track| track.keys.iter().all(|key| {
                match key.value_kind.as_str() {
                    "Effect" => capabilities.effects
                        && (capabilities.effect_objects || !key.value.starts_with('<'))
                        && (capabilities.registered_effects || CustomEffectRegistry::invocation_name(&key.value).is_none()),
                    "BoxShadows" => capabilities.box_shadows,
                    "TransformList" => capabilities.transform_lists,
                    "Brush" => capabilities.brushes,
                    _ => true,
                }
            }));
            if tracks.is_empty() {
                continue;
            }
            if capabilities.curves {
                let tracks: Vec<_> = tracks
                    .iter()
                    .filter(|track| {
                        track.keys.first().is_some_and(|key| {
                            if let Some(channels) = &track.curve_channels {
                                return capabilities.curve_channels && !channels.is_empty()
                                    && matches!((key.value_kind.as_str(), options.property_kinds.get(&track.property)),
                                        ("RelativePoint", Some(ValueKind::RelativePoint))
                                        | ("RelativeScalar", Some(ValueKind::RelativeScalar))
                                        | ("Effect", Some(ValueKind::Effect))
                                        | ("BoxShadows", Some(ValueKind::BoxShadows))
                                        | ("TransformList", Some(ValueKind::TransformList))
                                        | ("Brush", Some(ValueKind::Brush)));
                            }
                            if key.value_kind == "Color"
                                && options.property_kinds.get(&track.property) != Some(&ValueKind::Color)
                            {
                                return false;
                            }
                            key.value_kind == "Number"
                                || (capabilities.curve_channels
                                    && !CurveChannel::for_authored_kind(&key.value_kind).is_empty())
                        })
                    })
                    .cloned()
                    .collect();
                entry.curve_tracks = (!tracks.is_empty()).then_some(tracks);
            }
            let typed = program.timeline.as_ref().is_some_and(|timeline| timeline.tracks.iter().all(|track| {
                options.property_kinds.get(&track.property).is_some_and(|kind| *kind != ValueKind::Discrete
                    && track.keyframes.iter().all(|key| key.value.kind() == *kind))
            }));
            if capabilities.authoring && (program.has_project_codec() || typed) {
                if capabilities.sampling {
                    for track in &mut tracks {
                        let supported = options.property_kinds.get(&track.property).is_some_and(|kind| {
                            *kind != ValueKind::Discrete && track.keys.iter().all(|key| key.value_kind == format!("{kind:?}"))
                        });
                        track.can_sample = (supported && !entry.compatible_targets.is_empty()).then_some(true);
                    }
                }
                entry.authoring = Some(DesignAnimationAuthoring {
                    codec: program.codec.clone().unwrap_or_default(),
                    tracks,
                    component_batch: capabilities.components.then_some(true),
                });
                programs.insert(entry.id, (program, options));
            }
        }
        Self { document, programs, component_batch: capabilities.components }
    }

    pub(super) fn sample_in_authored_form(
        &self, program: i64, target: i64, property: &str, time: f64, value: AnimationValue,
    ) -> AnimationValue {
        let track = u64::try_from(target).ok().and_then(|target| {
            self.programs.get(&program)?.0.timeline.as_ref()?.track(target, property)
        });
        if let Some(track) = track { track.sample_in_authored_form(time, value) } else { value }
    }

    pub(super) fn plan(
        &self,
        request: &DesignAnimationAuthoringRequest,
        sampled: Option<DesignAnimationKeyframe>,
    ) -> Result<DesignAnimationAuthoringResult, String> {
        let (program, options) = self
            .programs
            .get(&request.program)
            .ok_or("Choose an editable animation in the current application preview")?;
        let target =
            NodeId::try_from(request.target).map_err(|_| "Choose an authored animation track")?;
        let property = &request.property;
        let components = match (&request.component, &request.components) {
            (Some(component), None) => Some(std::slice::from_ref(component)),
            (None, Some(components)) if self.component_batch => Some(components.as_slice()),
            (None, Some(_)) => return Err("This application preview does not support batch keyframe components".into()),
            (None, None) => None,
            (Some(_), Some(_)) => return Err("Supply one component or a components array".into()),
        };
        let component_value = components.map(|components| {
            if request.sample == Some(true) || sampled.is_some() || !request.value.is_empty() {
                return Err("Supply a sampled key, a complete keyframe value, or component values".into());
            }
            let previous = request.previous.ok_or("Choose an existing authored keyframe")?;
            let edits = components.iter().map(|component| Ok(KeyframeComponentEdit {
                channel: component.channel.parse::<TrackCurveChannel>().map_err(|error| error.to_string())?,
                value: component.value,
            })).collect::<Result<Vec<_>, String>>()?;
            program.keyframe_components(target, property, previous, &edits)
                .map(|value| value.to_xaml()).map_err(|error| error.to_string())
        }).transpose()?;
        let (time, value, easing) = sampled.as_ref().map_or(
            (request.time, component_value.as_deref().unwrap_or(&request.value), request.easing.as_str()),
            |key| (key.time, key.value.as_str(), key.easing.as_str()),
        );
        let edit = KeyframeEdit::Upsert {
            previous: request.previous,
            time,
            value,
            easing,
        };
        let draft = program
            .plan_keyframe(target, property, edit, |text, kind| {
                AnimationValue::parse_native_literal(
                    text,
                    kind.ok_or_else(|| {
                        AnimationError("Choose an existing authored animation track".into())
                    })?,
                )
            })
            .map_err(|error| error.to_string())?;
        let retimed = edit.retiming(target, property);
        let (accepted_timeline, patch) = self.edit_source(
            program, &draft, retimed.as_slice(), options, sampled.is_some(),
        )?;
        let mut location = KeyframeLocation {
            target,
            property: property.clone(),
            time,
        };
        location.reconcile_time(&accepted_timeline);
        let expected = draft
            .track(target, property)
            .and_then(|track| track.keyframes.iter().find(|key| key.time == time));
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
        let resolved_key = round_trip.filter(|_| component_value.is_some()).map(|key| DesignAnimationKeyframe {
            time: key.time, value: key.value.to_xaml(), easing: key.easing.to_xaml(),
            value_kind: format!("{:?}", key.value.kind()), curve_values: None,
        });
        Ok(DesignAnimationAuthoringResult {
            revision: request.revision,
            request: request.request,
            patch,
            samples: preview.samples,
            value_kind: format!("{:?}", preview.value_kind),
            sampled_key: sampled.map(|mut key| {
                key.time = location.time;
                key
            }),
            resolved_key,
            error: None,
        })
    }

    fn edit_source(
        &self,
        program: &AnimationProgram,
        draft: &Timeline,
        retimed: &[(KeyframeLocation, KeyframeLocation)],
        options: &ImportOptions,
        authored_keys: bool,
    ) -> Result<(Timeline, Option<DesignAnimationSourcePatch>), String> {
        let precise = native_keyframe_patches(&self.document, program, draft, retimed, || options)
            .map_err(|error| error.to_string())?;
        with_codecs(|codecs| {
            let (source, patch) = if let Some(patches) = precise {
                let mut candidate = self.document.as_ref().clone();
                let change = candidate.apply_text_patches(candidate.revision(), "Stage a native keyframe edit", patches)
                    .map_err(|error| error.to_string())?;
                let source = animation_fragment(&candidate, program.id).map_err(|error| error.to_string())?;
                let patch = change.patches.into_iter().next().map(|patch| {
                    DesignAnimationSourcePatch::from_utf8(self.document.source(), patch.span.range(), patch.replacement)
                }).transpose()?;
                (source, patch)
            } else {
                let source = if authored_keys {
                    codecs.encode_keyframes_for_document(&self.document, program, draft)
                } else {
                    codecs.encode_for_document(&self.document, program, draft)
                }.map_err(|error| error.to_string())?;
                let patch = self.source_patch(program.id, &source)?;
                (source, patch)
            };
            let mut accepted = program.clone();
            codecs.decode(&mut accepted, &source, options);
            let timeline = accepted.timeline.filter(|_| accepted.codec == program.codec)
                .ok_or("The animation codec could not read its encoded program")?;
            Ok((timeline, patch))
        })
    }

    fn source_patch(&self, program: NodeId, source: &str) -> Result<Option<DesignAnimationSourcePatch>, String> {
        let edits = compose_animation_edit(&self.document, program, source)
            .map_err(|error| error.to_string())?;
        let mut candidate = self.document.as_ref().clone();
        candidate
            .apply("Edit application animation keys", edits)
            .map_err(|error| error.to_string())?;
        let original = self.document.source();
        TextPatch::between(original, candidate.source())
            .map(|patch| DesignAnimationSourcePatch::from_utf8(original, patch.span.range(), patch.replacement))
            .transpose()
    }
}

fn tracks(program: &AnimationProgram, components: bool) -> Option<Vec<DesignAnimationTrack>> {
    let timeline = program.timeline.as_ref()?;
    timeline.validate().ok()?;
    let tracks = timeline
        .tracks
        .iter()
        .filter_map(|track| {
            let kind = track.keyframes.first()?.value.kind();
            let specialized = matches!(kind,
                ValueKind::RelativePoint | ValueKind::RelativeScalar | ValueKind::Effect | ValueKind::BoxShadows | ValueKind::TransformList | ValueKind::Brush)
                ;
            let channels = if specialized || components {
                TrackCurveChannel::for_track(track, timeline.duration, timeline.fill)
            } else {
                Vec::new()
            };
            let curve_channels = specialized.then(|| {
                serde_json::to_value(&channels).ok().and_then(|channels| serde_json::from_value(channels).ok())
            }).flatten();
            Some(DesignAnimationTrack {
                target: i64::try_from(track.target).ok()?,
                property: track.property.clone(),
                curve_channels,
                can_sample: None,
                keys: track
                    .keyframes
                    .iter()
                    .map(|key| DesignAnimationKeyframe {
                        time: key.time,
                        value: key.value.to_xaml(),
                        easing: key.easing.to_xaml(),
                        value_kind: format!("{:?}", key.value.kind()),
                        curve_values: components.then(|| {
                            TrackCurveChannel::key_components(&channels, &key.value).into_iter().filter_map(|value| {
                                let channel = serde_json::to_value(value.channel).ok()?;
                                Some(DesignKeyframeComponent {
                                    channel: channel.as_str()?.to_owned(),
                                    value: value.value,
                                    step: value.step.map(|step| step.as_str().into()),
                                })
                            }).collect()
                        }),
                    })
                    .collect(),
            })
        })
        .collect::<Vec<_>>();
    (!tracks.is_empty()).then_some(tracks)
}
