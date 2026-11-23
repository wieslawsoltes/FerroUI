//! Timeline commands run on the application UI thread, beside its registered animators.

#[path = "hosted_animation_curve.rs"]
mod curve;
#[path = "hosted_animation_playback.rs"]
mod playback;
#[path = "hosted_keyframe_sample.rs"]
mod keyframe_sample;
#[path = "hosted_property_transitions.rs"]
mod property_transitions;
#[path = "hosted_state_routes.rs"]
mod state_routes;

pub(super) use curve::curve;
pub(super) use property_transitions::request as property_transition_request;

use crate::design_window_loader::LoadedDesign;
use super::hosted_keyframes::{HostedKeyframeCapabilities, HostedKeyframes};
use state_routes::HostedStateRoutes;
use crate::remote::previewer_windowing_platform::PreviewerWindowingPlatform;
use ferroui_base::animation::Animatable;
use ferroui_base::{Ref, StyledElement, Visual};
use ferroui_controls::Window;
use ferroui_designer_animation::native::{
    animation_timing, NativePlayback, NativeProgramIndex,
    NativeSourceIndex, NativeStateTriggers,
};
use ferroui_designer_animation::{
    AnimationProgram, AnimationPrograms, StatePreviewGroup, native_key_time, xaml_clip_delay,
};
use ferroui_designer_core::{Document, NodeId};
use ferroui_remote_protocol::designer::{
    DesignAnimationAuthoringRequest, DesignAnimationAuthoringResult,
    DesignAnimationCatalogMessage, DesignAnimationClip, DesignAnimationProgram, DesignAnimationRequestMessage,
    DesignAnimationResultMessage, DesignAnimationTarget, UpdateDesignMessage,
    DesignKeyframeSelectionRequest, DesignKeyframeSelectionResult, DESIGN_KEYFRAME_SELECTION_QUERY,
    DESIGN_ANIMATION_AUTHORING_QUERY, DESIGN_ANIMATION_BOX_SHADOWS_QUERY, DESIGN_ANIMATION_BRUSHES_QUERY, DESIGN_ANIMATION_CLIPS_QUERY, DESIGN_ANIMATION_CURVE_CHANNELS_QUERY,
    DESIGN_ANIMATION_CURVE_QUERY, DESIGN_ANIMATION_EFFECTS_QUERY, DESIGN_ANIMATION_EFFECT_OBJECTS_QUERY,
    DESIGN_ANIMATION_QUERY, DESIGN_ANIMATION_REGISTERED_EFFECTS_QUERY, DESIGN_ANIMATION_TRANSFORM_LISTS_QUERY, DESIGN_ANIMATION_VERSION,
    DESIGN_CLIP_AUTHORING_QUERY, DESIGN_STATE_AUTHORING_QUERY, DESIGN_STATE_ROUTES_QUERY,
    DESIGN_KEYFRAME_SAMPLING_QUERY,
    DESIGN_PROPERTY_TRANSITIONS_QUERY,
};
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use url::Url;

ferroui_base::ferro_thread_local! {
    static ENABLED: Cell<bool> = const { Cell::new(false) };
    static AUTHORING: Cell<bool> = const { Cell::new(false) };
    static KEYFRAME_SELECTION: Cell<bool> = const { Cell::new(false) };
    static KEYFRAME_SAMPLING: Cell<bool> = const { Cell::new(false) };
    static CURVES: Cell<bool> = const { Cell::new(false) };
    static CURVE_CHANNELS: Cell<bool> = const { Cell::new(false) };
    static EFFECTS: Cell<bool> = const { Cell::new(false) };
    static EFFECT_OBJECTS: Cell<bool> = const { Cell::new(false) };
    static REGISTERED_EFFECTS: Cell<bool> = const { Cell::new(false) };
    static BOX_SHADOWS: Cell<bool> = const { Cell::new(false) };
    static TRANSFORM_LISTS: Cell<bool> = const { Cell::new(false) };
    static BRUSHES: Cell<bool> = const { Cell::new(false) };
    static CLIPS: Cell<bool> = const { Cell::new(false) };
    static CLIP_AUTHORING: Cell<bool> = const { Cell::new(false) };
    static STATE_ROUTES: Cell<bool> = const { Cell::new(false) };
    static STATE_AUTHORING: Cell<bool> = const { Cell::new(false) };
    static PROPERTY_TRANSITIONS: Cell<bool> = const { Cell::new(false) };
    static ACTIVE: RefCell<Option<HostedAnimations>> = const { RefCell::new(None) };
}

pub(super) fn configure(transport: Option<&Url>, remote: bool) {
    let uri = transport.filter(|uri| remote && uri.scheme() == "tcp-bson");
    let requested = |key: &str| uri.is_some_and(|uri| {
        uri.query_pairs().any(|(name, version)| name == key && version == DESIGN_ANIMATION_VERSION)
    });
    let enabled = requested(DESIGN_ANIMATION_QUERY);
    ENABLED.set(enabled);
    AUTHORING.set(enabled && requested(DESIGN_ANIMATION_AUTHORING_QUERY));
    KEYFRAME_SELECTION.set(AUTHORING.get() && requested(DESIGN_KEYFRAME_SELECTION_QUERY));
    KEYFRAME_SAMPLING.set(AUTHORING.get() && requested(DESIGN_KEYFRAME_SAMPLING_QUERY));
    CURVES.set(enabled && requested(DESIGN_ANIMATION_CURVE_QUERY));
    CURVE_CHANNELS.set(CURVES.get() && requested(DESIGN_ANIMATION_CURVE_CHANNELS_QUERY));
    EFFECTS.set(enabled && requested(DESIGN_ANIMATION_EFFECTS_QUERY));
    EFFECT_OBJECTS.set(EFFECTS.get() && requested(DESIGN_ANIMATION_EFFECT_OBJECTS_QUERY));
    REGISTERED_EFFECTS.set(EFFECTS.get() && requested(DESIGN_ANIMATION_REGISTERED_EFFECTS_QUERY));
    BOX_SHADOWS.set(enabled && requested(DESIGN_ANIMATION_BOX_SHADOWS_QUERY));
    TRANSFORM_LISTS.set(enabled && requested(DESIGN_ANIMATION_TRANSFORM_LISTS_QUERY));
    BRUSHES.set(enabled && requested(DESIGN_ANIMATION_BRUSHES_QUERY));
    CLIPS.set(enabled && requested(DESIGN_ANIMATION_CLIPS_QUERY));
    CLIP_AUTHORING.set(AUTHORING.get() && requested(DESIGN_CLIP_AUTHORING_QUERY));
    STATE_ROUTES.set(enabled && requested(DESIGN_STATE_ROUTES_QUERY));
    STATE_AUTHORING.set(STATE_ROUTES.get() && AUTHORING.get() && requested(DESIGN_STATE_AUTHORING_QUERY));
    PROPERTY_TRANSITIONS.set(AUTHORING.get() && requested(DESIGN_PROPERTY_TRANSITIONS_QUERY));
    ACTIVE.with(|active| active.borrow_mut().take());
}

pub(super) struct AnimationSource {
    pub(super) identity: String,
    document: Rc<Document>,
    programs: Vec<AnimationProgram>,
    states: Vec<StatePreviewGroup>,
}

impl AnimationSource {
    pub(super) fn freeze_clock(&self) -> bool {
        !self.programs.is_empty()
    }
}

pub(super) fn prepare(update: &UpdateDesignMessage) -> Option<AnimationSource> {
    if !ENABLED.get() {
        return None;
    }
    let document = Document::new(update.source.xaml.as_deref()?).ok()?;
    let full_states = STATE_AUTHORING.get();
    let mut programs = AnimationPrograms::scan_hosted(&document);
    if full_states {
        programs.retain(|program| !AnimationPrograms::is_state_managed(&document, program.id));
    }
    let states = STATE_ROUTES.get().then(|| StatePreviewGroup::scan(&document, full_states)).unwrap_or_default();
    (!programs.is_empty() || !states.is_empty() || CLIP_AUTHORING.get() || full_states || PROPERTY_TRANSITIONS.get()).then(|| AnimationSource {
        identity: format!("ferroui-design/{}", update.revision),
        document: Rc::new(document),
        programs,
        states,
    })
}

pub(super) fn accept(
    revision: Option<i64>,
    source: Option<AnimationSource>,
    design: &LoadedDesign,
) -> Option<DesignAnimationCatalogMessage> {
    let revision = revision.filter(|_| ENABLED.get());
    ACTIVE.with(|active| active.borrow_mut().take());
    let revision = revision?;
    let mut catalog = DesignAnimationCatalogMessage {
        revision,
        keyframe_selection: KEYFRAME_SELECTION.get().then_some(true),
        clip_authoring: CLIP_AUTHORING.get().then_some(true),
        state_authoring: STATE_AUTHORING.get().then_some(true),
        ..Default::default()
    };
    if let Some(source) = source {
        let session = HostedAnimations::mount(revision, source, design, &mut catalog);
        ACTIVE.with(|active| active.borrow_mut().replace(session));
    }
    Some(catalog)
}

pub(super) fn sample(
    request: &DesignAnimationRequestMessage,
    window: Option<&Window>,
) -> DesignAnimationResultMessage {
    let mut answer = DesignAnimationResultMessage {
        revision: request.revision,
        request: request.request,
        ..Default::default()
    };
    let sample = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let window = window.ok_or("Open an app-hosted animation preview first")?;
        let platform = PreviewerWindowingPlatform::last_window()
            .ok_or("The application preview window retired")?;
        let (completed, elapsed) = ACTIVE.with(|active| {
            active
                .borrow_mut()
                .as_mut()
                .ok_or("Open an app-hosted animation preview first")?
                .sample(request)
        })?;
        window.update_layout();
        window.invalidate_visual();
        Ok::<_, String>((completed, elapsed, platform.request_frame()))
    }));
    match sample {
        Ok(Ok((completed, elapsed, sequence))) => {
            answer.completed = completed;
            answer.elapsed = elapsed;
            answer.after_sequence = sequence;
        }
        Ok(Err(error)) => answer.error = Some(error),
        Err(_) => {
            answer.error =
                Some("The application could not render the requested animation frame".into())
        }
    }
    answer
}

pub(super) fn plan_keyframe(request: &DesignAnimationAuthoringRequest) -> DesignAnimationAuthoringResult {
    let plan = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        ACTIVE.with(|active| {
            let mut active = active.borrow_mut();
            let host = active.as_mut().ok_or("Open an app-hosted animation preview first")?;
            host.claim(request.revision, request.request)?;
            let sampled = (request.sample == Some(true)).then(|| host.sample_keyframe(request)).transpose()?;
            host.keyframes.as_ref().ok_or("The application preview has no editable animation tracks")?
                .plan(request, sampled)
        })
    }));
    let error = match plan {
        Ok(Ok(plan)) => return plan,
        Ok(Err(error)) => error,
        Err(_) => "The animation codec could not plan this edit".into(),
    };
    DesignAnimationAuthoringResult {
        revision: request.revision, request: request.request, error: Some(error), ..Default::default()
    }
}

pub(super) fn plan_keyframe_selection(request: &DesignKeyframeSelectionRequest) -> DesignKeyframeSelectionResult {
    let plan = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if !KEYFRAME_SELECTION.get() {
            return Err("The application preview did not negotiate keyframe selection editing".into());
        }
        ACTIVE.with(|active| {
            let mut active = active.borrow_mut();
            let host = active.as_mut().ok_or("Open an app-hosted animation preview first")?;
            host.claim(request.revision, request.request)?;
            host.keyframes.as_ref().ok_or("The application preview has no editable animation tracks")?
                .plan_selection(request)
        })
    }));
    let error = match plan {
        Ok(Ok(plan)) => return plan,
        Ok(Err(error)) => error,
        Err(_) => "The animation codec could not plan this keyframe selection".into(),
    };
    DesignKeyframeSelectionResult {
        revision: request.revision, request: request.request, error: Some(error), ..Default::default()
    }
}

struct HostedAnimations {
    revision: i64,
    last_request: i64,
    index: NativeProgramIndex,
    programs: BTreeMap<i64, DesignAnimationProgram>,
    targets: BTreeMap<i64, Ref<StyledElement>>,
    current: Vec<(i64, Vec<i64>)>,
    playback: NativePlayback,
    keyframes: Option<HostedKeyframes>,
    property_transitions: Option<property_transitions::HostedPropertyTransitions>,
    state_routes: HostedStateRoutes,
    _state_triggers: NativeStateTriggers,
}

impl HostedAnimations {
    fn mount(
        revision: i64,
        source: AnimationSource,
        design: &LoadedDesign,
        catalog: &mut DesignAnimationCatalogMessage,
    ) -> Self {
        let controls = live_controls(design.root.clone().upcast::<StyledElement>());
        let locations = NativeSourceIndex::new(&source.document, &design.root, &source.identity);
        let mut candidates = BTreeMap::new();
        for control in &controls {
            if let Some(id) = locations
                .resolve_object(control)
                .and_then(|id| i64::try_from(id).ok())
            {
                // Repeated instances of one template declaration have separate runtime
                // identities. The source catalog does not pick one instance implicitly.
                candidates
                    .entry(id)
                    .and_modify(|candidate| *candidate = None)
                    .or_insert_with(|| Some(control.clone()));
            }
        }
        let mut targets = candidates
            .into_iter()
            .filter_map(|(id, control)| Some((id, control?)))
            .collect::<BTreeMap<_, _>>();
        let property_transitions = PROPERTY_TRANSITIONS.get().then(|| {
            property_transitions::HostedPropertyTransitions::mount(source.document.clone(), &targets, catalog)
        });
        let index =
            NativeProgramIndex::discover(&source.document, &locations, &source.programs, controls);
        let mut previous = 0;
        let mut utf16 = 0;
        for program in &source.programs {
            utf16 += source.document.valid_source()[previous..program.span.start]
                .encode_utf16()
                .count();
            previous = program.span.start;
            if let Some(entry) = catalog_entry(&source.document, &index, program, &targets, utf16) {
                catalog.programs.push(entry);
            }
        }
        let state_routes =
            HostedStateRoutes::mount(&source.document, source.states, &locations, design, &mut targets, catalog);
        let state_triggers = NativeStateTriggers::bind(&source.document, |id| {
            targets.get(&i64::try_from(id).ok()?).cloned()
        });
        catalog.targets = targets
            .iter()
            .map(|(&id, control)| {
                let type_name = control.get_type().name().to_owned();
                DesignAnimationTarget {
                    id,
                    label: control.name().unwrap_or_else(|| type_name.clone()),
                    type_name,
                }
            })
            .collect();
        let curves = CURVES.get();
        let authoring = AUTHORING.get();
        let keyframes = (curves || authoring).then(|| {
            HostedKeyframes::mount(
                source.document,
                source.programs,
                &index,
                &targets,
                &mut catalog.programs,
                HostedKeyframeCapabilities {
                    authoring,
                    curves,
                    curve_channels: CURVE_CHANNELS.get(),
                    sampling: KEYFRAME_SAMPLING.get(),
                    effects: EFFECTS.get(),
                    effect_objects: EFFECT_OBJECTS.get(),
                    registered_effects: REGISTERED_EFFECTS.get(),
                    box_shadows: BOX_SHADOWS.get(),
                    transform_lists: TRANSFORM_LISTS.get(),
                    brushes: BRUSHES.get(),
                },
            )
        });
        Self {
            revision,
            last_request: 0,
            index,
            programs: catalog.programs.iter().map(|p| (p.id, p.clone())).collect(),
            targets,
            current: Vec::new(),
            playback: NativePlayback::default(),
            keyframes,
            property_transitions,
            state_routes,
            _state_triggers: state_triggers,
        }
    }

    fn claim(&mut self, revision: i64, request: i64) -> Result<(), String> {
        if revision != self.revision || request <= self.last_request {
            return Err("Read the current app-hosted preview frame".into());
        }
        self.last_request = request;
        Ok(())
    }
}

fn catalog_entry(
    document: &Document,
    index: &NativeProgramIndex,
    program: &AnimationProgram,
    targets: &BTreeMap<i64, Ref<StyledElement>>,
    utf16: usize,
) -> Option<DesignAnimationProgram> {
    let animation = index.animation(program.id)?;
    let timing = animation_timing(animation)?;
    let mut default_targets = Vec::new();
    let mut compatible_targets = Vec::new();
    for (&id, control) in targets {
        if !control.cast::<Animatable>().is_some_and(|target| {
            index.validate_target(program.id, &target, "the preview target").is_ok()
        }) {
            continue;
        }
        compatible_targets.push(id);
        if belongs_to(document, id as NodeId, program.owner)
            && index.style_matches(program.id, control)
        {
            default_targets.push(id);
        }
    }
    Some(DesignAnimationProgram {
        id: i64::try_from(program.id).ok()?,
        name: program.name.clone(),
        selector: program.selector.clone(),
        duration: timing.duration,
        total_duration: timing.total_duration(),
        default_targets,
        compatible_targets,
        source_utf16: i32::try_from(utf16).ok()?,
        custom_animator: index.uses_custom_animators(program.id),
        authoring: None,
        curve_tracks: None,
        clip: CLIPS.get().then(|| DesignAnimationClip {
            delay: timing.delay,
            speed_ratio: timing.speed_ratio,
            editable: AUTHORING.get() && xaml_clip_delay(document, program.id)
                .and_then(|delay| native_key_time(delay).ok())
                .is_some_and(|delay| Some(delay) == native_key_time(timing.delay).ok()),
        }),
    })
}

fn belongs_to(document: &Document, id: NodeId, owner: NodeId) -> bool {
    let mut node = Some(id);
    while let Some(id) = node {
        if id == owner {
            return true;
        }
        node = document.ast().node(id).and_then(|node| node.parent);
    }
    false
}

fn live_controls(root: Ref<StyledElement>) -> Vec<Ref<StyledElement>> {
    let mut seen = BTreeSet::new();
    let mut pending = vec![root];
    let mut controls = Vec::new();
    while let Some(control) = pending.pop() {
        if !seen.insert(&*control as *const StyledElement as usize) {
            continue;
        }
        pending.extend(control.logical_children().snapshot().iter().cloned());
        if let Some(visual) = control.cast::<Visual>() {
            pending.extend(
                visual
                    .visual_children()
                    .snapshot()
                    .iter()
                    .filter_map(|child| child.cast::<StyledElement>()),
            );
        }
        controls.push(control);
    }
    controls
}
