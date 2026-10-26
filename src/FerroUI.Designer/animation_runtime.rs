//! Reuse the loaded framework animation objects for values owned by native animators.
use crate::runtime::RuntimePreview;
use crate::PreviewStatus;
use ferroui_base::animation::{Animatable, Animation, IAnimation, IClock, PlayState, TimeSpan};
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::reactive::{IDisposable, IObservable, IObserver, LightweightSubject};
use ferroui_base::styling::{IStyle, StyleBase, Styles};
use ferroui_base::{BoxedValue, FerroObject, Ref, StyledElement};
use ferroui_designer_animation::AnimationProgram;
use ferroui_designer_core::{Document, NodeId};
use ferroui_markup_xaml::diagnostics::XamlSourceInfo;
use ferroui_markup_xaml::FromXamlObject;
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

#[derive(Default)]
pub(crate) struct NativeAnimations {
    objects: BTreeMap<NodeId, Rc<dyn IAnimation>>,
    factory_programs: BTreeSet<NodeId>,
    active: Vec<NodeId>,
    runs: Vec<NativeRun>,
    time: f64,
}
struct NativeRun {
    clock: Rc<NativeClock>,
    subscription: Rc<dyn IDisposable>,
    complete: Rc<Cell<bool>>,
}
impl Drop for NativeRun {
    fn drop(&mut self) {
        self.clock.state.set(PlayState::Stop);
        self.subscription.dispose();
    }
}
impl Drop for NativeAnimations {
    fn drop(&mut self) {
        self.stop();
    }
}
impl NativeAnimations {
    pub(crate) fn stop(&mut self) {
        self.runs.clear();
        self.active.clear();
        self.time = 0.0;
    }
    pub(crate) fn clear(&mut self) {
        self.stop();
        self.objects.clear();
        self.factory_programs.clear();
    }
    pub(crate) fn uses_factory_target(&self, id: NodeId) -> bool {
        self.factory_programs.contains(&id)
    }
    pub(crate) fn duration(&self, id: NodeId) -> Option<f64> {
        let animation = self.objects.get(&id)?.as_object()?.cast::<Animation>()?;
        let value = animation.duration().total_seconds();
        (value.is_finite() && value > 0.0).then_some(value)
    }
    pub(crate) fn refresh(
        &mut self,
        document: &Document,
        preview: &RuntimePreview,
        programs: &[AnimationProgram],
    ) {
        self.clear();
        if !matches!(preview.status, PreviewStatus::Ready { revision } if revision == document.revision())
        {
            return;
        }
        let Some(root) = preview.root() else { return };
        let root_value: BoxedValue = Rc::new(root.clone());
        let source_uri = XamlSourceInfo::get_xaml_source_info(&root_value)
            .and_then(|provenance| provenance.source_uri().cloned())
            .or_else(|| ferroui_base::utilities::Uri::absolute(preview.uri()).ok());
        let mut positions = BTreeMap::<usize, Vec<(usize, NodeId)>>::new();
        let mut previous = 0;
        let (mut line, mut column) = (1, 1);
        let mut sorted = programs.iter().collect::<Vec<_>>();
        sorted.sort_by_key(|program| program.span.start);
        for program in sorted {
            for character in document.valid_source()[previous..program.span.start].chars() {
                if character == '\n' {
                    line += 1;
                    column = 1;
                } else {
                    column += 1;
                }
            }
            previous = program.span.start;
            positions
                .entry(line)
                .or_default()
                .push((column, program.id));
        }
        let resource_names = programs
            .iter()
            .filter_map(|program| {
                document
                    .ast()
                    .node(program.id)?
                    .attributes
                    .iter()
                    .find(|attribute| {
                        attribute.namespace_uri.as_deref()
                            == Some("http://schemas.microsoft.com/winfx/2006/xaml")
                            && attribute.name.rsplit(':').next() == Some("Key")
                    })
                    .map(|attribute| attribute.value.clone())
            })
            .collect::<BTreeSet<_>>();
        let mut styles = Vec::<Rc<dyn IStyle>>::new();
        let mut dictionaries = Vec::<Ref<ResourceDictionary>>::new();
        let mut controls = vec![root.upcast::<StyledElement>()];
        controls.extend(
            document
                .ast()
                .elements()
                .filter_map(|node| preview.visual(node.id)?.cast::<StyledElement>()),
        );
        let mut seen = BTreeSet::new();
        for control in controls {
            if seen.insert(&*control as *const StyledElement as usize) {
                styles.extend(control.styles().snapshot().iter().cloned());
                dictionaries.push(control.resources());
                dictionaries.push(control.styles().resources());
                if let Some(theme) = control.theme() {
                    styles.push(theme.into());
                }
            }
        }
        let mut animations = Vec::<Rc<dyn IAnimation>>::new();
        seen.clear();
        while let Some(style) = styles.pop() {
            if let Some(object) = style.as_object() {
                if !seen.insert(object as *const FerroObject as usize) {
                    continue;
                }
                if let Some(base) = object.downcast_ref::<StyleBase>() {
                    animations.extend(base.animations().snapshot().iter().cloned());
                    dictionaries.push(base.resources());
                } else if let Some(collection) = object.downcast_ref::<Styles>() {
                    dictionaries.push(collection.resources());
                }
            }
            styles.extend(style.children().iter().cloned());
        }
        seen.clear();
        while let Some(dictionary) = dictionaries.pop() {
            if !seen.insert(&*dictionary as *const ResourceDictionary as usize) {
                continue;
            }
            // Materialize only animation resource keys authored in this document.
            for key in dictionary.keys().into_iter().filter(|key| {
                key.as_str()
                    .is_some_and(|name| resource_names.contains(name))
            }) {
                let value =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| dictionary.get(&key)))
                        .ok()
                        .flatten();
                if let Some(value) = value {
                    if let Some(animation) = Ref::<Animation>::from_xaml_object(&value) {
                        animations.push(animation.into());
                    } else if let Some(animation) = value.downcast_ref::<Rc<dyn IAnimation>>() {
                        animations.push(animation.clone());
                    }
                }
            }
            for provider in dictionary.merged_dictionaries().snapshot().iter() {
                if let Some(dictionary) = provider
                    .as_object()
                    .and_then(|object| object.downcast_ref::<ResourceDictionary>())
                {
                    dictionaries.push(dictionary.to_ref());
                }
            }
            for (_, provider) in dictionary.theme_dictionaries_snapshot() {
                if let Some(dictionary) = provider
                    .as_object()
                    .and_then(|object| object.downcast_ref::<ResourceDictionary>())
                {
                    dictionaries.push(dictionary.to_ref());
                }
            }
        }
        for animation in animations {
            let Some(object) = animation.as_object() else {
                continue;
            };
            let value: BoxedValue = Rc::new(object);
            let Some(info) = XamlSourceInfo::get_xaml_source_info(&value) else {
                continue;
            };
            if info.source_uri() != source_uri.as_ref() || source_uri.is_none() {
                continue;
            }
            let Some(candidates) = positions.get(&(info.line_number().max(1) as usize)) else {
                continue;
            };
            let column = info.line_position().max(1) as usize;
            let Some((distance, id)) = candidates
                .iter()
                .map(|(expected, id)| (expected.abs_diff(column), *id))
                .min()
            else {
                continue;
            };
            // The parser may point at '<' or at the first name character. Never bind a
            // different source program merely because it is on the same line.
            if distance <= 2 {
                self.objects.entry(id).or_insert(animation);
            }
        }
        if document
            .ast()
            .node(document.ast().root())
            .is_some_and(|root| root.local_name() == "ResourceDictionary")
        {
            self.factory_programs.extend(
                programs
                    .iter()
                    .filter(|program| {
                        program.style.is_none()
                            && program.targets.is_empty()
                            && self.objects.contains_key(&program.id)
                    })
                    .map(|program| program.id),
            );
        }
    }
    pub(crate) fn seek(
        &mut self,
        programs: &[AnimationProgram],
        preview: &RuntimePreview,
        elapsed: f64,
    ) -> Result<(), String> {
        validate_time(elapsed)?;
        let mut pending = Vec::new();
        for program in programs {
            if program.targets.is_empty() && !self.factory_programs.contains(&program.id) {
                continue;
            }
            let animation = self.objects.get(&program.id).ok_or_else(|| format!("Native program '{}' is not available in the accepted preview; load its defining style/resource and select preview targets", program.name))?.as_object().and_then(|animation| animation.cast::<Animation>()).ok_or("The source program has no preview-capable native Animation object")?;
            if program.targets.is_empty() && self.factory_programs.contains(&program.id) {
                let target = preview
                    .root()
                    .ok_or("The resource preview factory has no live root")?
                    .upcast::<Animatable>();
                for keyframe in animation.children().snapshot().iter() {
                    for setter in keyframe.setters().snapshot().iter() {
                        let property = setter
                            .property()
                            .ok_or("Native animation Setter has no target property")?;
                        if !property.is_attached()
                            && !property.owner_type().is_assignable_from(target.get_type())
                            && !ferroui_base::media::Transform::TYPE
                                .is_assignable_from(property.owner_type())
                        {
                            return Err(format!("Resource program '{}' targets {}, which is incompatible with factory preview root {}; return a compatible control from the resource factory", program.name, property, target.get_type()));
                        }
                    }
                }
                pending.push((animation.clone(), target));
            }
            for target in &program.targets {
                let visual = preview
                    .visual(*target)
                    .ok_or("Native animation target was removed")?;
                let target = visual
                    .cast::<Animatable>()
                    .ok_or("Target is not animatable")?;
                pending.push((animation.clone(), target));
            }
        }
        if pending.is_empty() {
            return Err("Choose live preview targets for this animation program".into());
        }
        self.stop();
        let mut runs = Vec::new();
        for (animation, target) in pending {
            let clock = Rc::new(NativeClock::default());
            let complete = Rc::new(Cell::new(false));
            let completed = complete.clone();
            let applied = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                animation.apply_for_preview(
                    &target,
                    Some(clock.clone()),
                    Some(Rc::new(move || completed.set(true))),
                )
            }));
            let subscription = applied.map_err(|payload| {
                format!(
                    "Native animation rejected the program: {}",
                    panic_message(payload)
                )
            })?;
            runs.push(NativeRun {
                clock,
                subscription,
                complete,
            });
        }
        pulse(&runs, 0.0)?;
        pulse(&runs, elapsed)?;
        self.active = programs.iter().map(|program| program.id).collect();
        self.runs = runs;
        self.time = elapsed;
        Ok(())
    }
    pub(crate) fn advance_to(
        &mut self,
        programs: &[AnimationProgram],
        preview: &RuntimePreview,
        elapsed: f64,
    ) -> Result<bool, String> {
        validate_time(elapsed)?;
        let ids = programs
            .iter()
            .map(|program| program.id)
            .collect::<Vec<_>>();
        if self.active != ids || self.runs.is_empty() || elapsed < self.time {
            self.seek(programs, preview, elapsed)?;
        } else {
            if let Err(error) = pulse(&self.runs, elapsed) {
                self.stop();
                return Err(error);
            }
            self.time = elapsed;
        }
        Ok(self.runs.iter().all(|run| run.complete.get()))
    }
}
fn pulse(runs: &[NativeRun], seconds: f64) -> Result<(), String> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        for run in runs {
            run.clock.subject.on_next(TimeSpan::from_seconds(seconds));
        }
    }))
    .map_err(|payload| format!("Native animation frame failed: {}", panic_message(payload)))
}
fn validate_time(value: f64) -> Result<(), String> {
    if value.is_finite() && value >= 0.0 && value * 10_000_000.0 < i64::MAX as f64 {
        Ok(())
    } else {
        Err("Native animation time is outside the clock range".into())
    }
}
fn panic_message(value: Box<dyn std::any::Any + Send>) -> String {
    value
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| {
            value
                .downcast_ref::<&str>()
                .map(|value| (*value).to_owned())
        })
        .unwrap_or_else(|| "runtime callback failed".into())
}
struct NativeClock {
    subject: LightweightSubject<TimeSpan>,
    state: Cell<PlayState>,
}
impl Default for NativeClock {
    fn default() -> Self {
        Self {
            subject: LightweightSubject::new(),
            state: Cell::new(PlayState::Run),
        }
    }
}
impl IObservable<TimeSpan> for NativeClock {
    fn subscribe(&self, observer: Rc<dyn IObserver<TimeSpan>>) -> Rc<dyn IDisposable> {
        self.subject.subscribe(observer)
    }
}
impl IClock for NativeClock {
    fn play_state(&self) -> PlayState {
        self.state.get()
    }
    fn set_play_state(&self, value: PlayState) {
        self.state.set(value);
    }
}
