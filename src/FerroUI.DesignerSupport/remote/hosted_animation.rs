//! Timeline commands run on the application UI thread, beside its registered animators.

#[path = "hosted_animation_curve.rs"]
mod curve;
#[path = "hosted_state_routes.rs"]
mod state_routes;

pub(super) use curve::curve;

use crate::design_window_loader::LoadedDesign;
use super::hosted_keyframes::HostedKeyframes;
use state_routes::HostedStateRoutes;
use crate::remote::previewer_windowing_platform::PreviewerWindowingPlatform;
use ferroui_base::animation::Animatable;
use ferroui_base::{Ref, StyledElement, Visual};
use ferroui_controls::Window;
use ferroui_designer_animation::native::{
    animation_timing, validate_animation_target, NativePlayback, NativeProgramIndex,
    NativeSourceIndex,
};
use ferroui_designer_animation::{AnimationProgram, AnimationPrograms, ProjectedStateGroup};
use ferroui_designer_core::{Document, NodeId};
use ferroui_remote_protocol::designer::{
    DesignAnimationAuthoringRequest, DesignAnimationAuthoringResult,
    DesignAnimationCatalogMessage, DesignAnimationProgram, DesignAnimationRequestMessage,
    DesignAnimationResultMessage, DesignAnimationTarget, UpdateDesignMessage,
    DESIGN_ANIMATION_AUTHORING_QUERY, DESIGN_ANIMATION_CURVE_QUERY, DESIGN_ANIMATION_QUERY,
    DESIGN_ANIMATION_VERSION, DESIGN_STATE_ROUTES_QUERY,
};
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use url::Url;

ferroui_base::ferro_thread_local! {
    static ENABLED: Cell<bool> = const { Cell::new(false) };
    static AUTHORING: Cell<bool> = const { Cell::new(false) };
    static CURVES: Cell<bool> = const { Cell::new(false) };
    static STATE_ROUTES: Cell<bool> = const { Cell::new(false) };
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
    CURVES.set(enabled && requested(DESIGN_ANIMATION_CURVE_QUERY));
    STATE_ROUTES.set(enabled && requested(DESIGN_STATE_ROUTES_QUERY));
    ACTIVE.with(|active| active.borrow_mut().take());
}

pub(super) struct AnimationSource {
    pub(super) identity: String,
    document: Document,
    programs: Vec<AnimationProgram>,
    states: Vec<ProjectedStateGroup>,
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
    let programs = AnimationPrograms::scan_hosted(&document);
    let states = STATE_ROUTES.get().then(|| ProjectedStateGroup::scan(&document)).unwrap_or_default();
    (!programs.is_empty() || !states.is_empty()).then(|| AnimationSource {
        identity: format!("ferroui-design/{}", update.revision),
        document,
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
            host.keyframes.as_ref().ok_or("The application preview has no editable animation tracks")?.plan(request)
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

struct HostedAnimations {
    revision: i64,
    last_request: i64,
    index: NativeProgramIndex,
    programs: BTreeMap<i64, DesignAnimationProgram>,
    targets: BTreeMap<i64, Ref<StyledElement>>,
    current: Option<(i64, Vec<i64>)>,
    playback: NativePlayback,
    keyframes: Option<HostedKeyframes>,
    state_routes: HostedStateRoutes,
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
            HostedStateRoutes::mount(&source.document, source.states, design, &mut targets, catalog);
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
            HostedKeyframes::mount(source.document, source.programs, &index, &mut catalog.programs, authoring, curves)
        });
        Self {
            revision,
            last_request: 0,
            index,
            programs: catalog.programs.iter().map(|p| (p.id, p.clone())).collect(),
            targets,
            current: None,
            playback: NativePlayback::default(),
            keyframes,
            state_routes,
        }
    }

    fn claim(&mut self, revision: i64, request: i64) -> Result<(), String> {
        if revision != self.revision || request <= self.last_request {
            return Err("Read the current app-hosted preview frame".into());
        }
        self.last_request = request;
        Ok(())
    }

    fn sample(&mut self, command: &DesignAnimationRequestMessage) -> Result<(bool, f64), String> {
        self.claim(command.revision, command.request)?;
        let Some(id) = command.program else {
            self.current = None;
            self.playback.stop();
            return Ok((false, 0.0));
        };
        let entry = self
            .programs
            .get(&id)
            .ok_or("Choose a current native animation program")?;
        let targets = command
            .targets
            .clone()
            .unwrap_or_else(|| entry.default_targets.clone());
        let clip = (id, targets);
        if let Some(route) = self.state_routes.get(id) {
            if clip.1 != entry.default_targets {
                return Err("A state route uses its authored targets".into());
            }
            if command.elapsed < 0.0 || !command.elapsed.is_finite()
                || (command.restart && command.elapsed > entry.duration)
            {
                return Err("Choose a time within the state route duration".into());
            }
            let elapsed = command.elapsed.min(entry.duration);
            let complete = if !command.restart && self.current.as_ref() == Some(&clip) && self.playback.is_active() {
                self.playback.advance_to(elapsed)?
            } else {
                self.playback.seek(route.bindings(), elapsed)?
            };
            self.current = Some(clip);
            return Ok((complete || elapsed == entry.duration, elapsed));
        }
        if !command.restart && self.current.as_ref() == Some(&clip) && self.playback.is_active() {
            return Ok((self.playback.advance_to(command.elapsed)?, command.elapsed));
        }
        if command.elapsed > entry.total_duration.unwrap_or(f64::INFINITY) {
            return Err("Choose a time within the animation's elapsed duration".into());
        }
        if clip.1.is_empty() || clip.1.len() != clip.1.iter().collect::<BTreeSet<_>>().len() {
            return Err("Choose distinct live preview targets for the animation".into());
        }
        let animation = self
            .index
            .animation(id as NodeId)
            .ok_or("The native animation was retired")?;
        let bindings = clip
            .1
            .iter()
            .map(|target| {
                let target = self
                    .targets
                    .get(target)
                    .and_then(|target| target.cast::<Animatable>())
                    .ok_or("Choose current application preview targets")?;
                validate_animation_target(animation, &target, "the preview target")?;
                Ok::<_, String>((animation.clone(), target))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let complete = self.playback.seek(bindings, command.elapsed)?;
        self.current = Some(clip);
        Ok((complete, command.elapsed))
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
            validate_animation_target(animation, &target, "the preview target").is_ok()
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
