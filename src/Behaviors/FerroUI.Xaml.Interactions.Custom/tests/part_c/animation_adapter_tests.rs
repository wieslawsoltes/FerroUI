//! Ported from `tests/Xaml.Behaviors.Interactions.UnitTests/Custom/AnimationAdapterTests.cs` of
//! Xaml.Behaviors.
//!
//! Adapted: the platform of the reference ticks the global clock of the animations; here the test
//! pulses the manual clock of the test application where the reference waits or runs the jobs.
//! The parallax test reads a composition visual, so its window renders through the test
//! compositor ([`CompositionApp`](super::support::CompositionApp)).
//! The document of the run-time loader test names the types without prefixes, as the namespaces
//! of the crates are mapped into the default XML namespace of the framework.

use super::support::{add_behavior, as_bool, boxed, run_jobs, start_app, window_with, CompositionApp};
use crate::{
    AnimateOnAttachedBehavior, AnimationBuilderRef, AnimationCompletedTrigger, BeginAnimationAction, FadeInBehavior,
    FluidMoveBehavior, FluidMoveScope, ParallaxBehavior, PlayAnimationBehavior, RunAnimationTrigger,
    StartAnimationAction, StartBuiltAnimationAction, TransitionsBehavior, TransitionsChangedTrigger, ASSEMBLY,
};
use ferroui_base::animation::{Animation, Cue, FillMode, IAnimationSetter, KeyFrame, TimeSpan, Transitions};
use ferroui_base::media::TranslateTransform;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::numerics::Vector3;
use ferroui_base::rendering::composition::ElementComposition;
use ferroui_base::styling::Setter;
use ferroui_base::{ferro_class, instantiate, BoxedValue, FerroObject, FerroObjectImpl, Ref, Vector, Visual};
use ferroui_controls::{Border, Canvas, Control, Panel, ScrollViewer, StackPanel};
use ferroui_markup_xaml::{RuntimeXamlLoaderConfiguration, RuntimeXamlLoaderDocument};
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use ferroui_xaml_behaviors_animations::IAnimationBuilder;
use ferroui_xaml_interactivity::{IAction, Interaction};
use std::cell::Cell;
use std::rc::Rc;

/// `RecordingAnimationBuilder`: builds an animation of no duration and counts the builds.
#[derive(Default)]
struct RecordingAnimationBuilder {
    build_count: Cell<i32>,
}

impl RecordingAnimationBuilder {
    fn build_count(&self) -> i32 {
        self.build_count.get()
    }
}

impl IAnimationBuilder for RecordingAnimationBuilder {
    fn build(&self, _control: &Control) -> Option<Ref<Animation>> {
        self.build_count.set(self.build_count.get() + 1);
        let animation = Animation::new();
        animation.set_duration(TimeSpan::ZERO);
        Some(animation)
    }
}

/// `RecordingAction`: an action of the object model that counts its executions.
#[repr(C)]
pub(crate) struct RecordingAction {
    base: FerroObject,
    execution_count: Cell<i32>,
}

ferro_class!(RecordingAction: FerroObject);
ferroui_base::ferro_impl_classes!(RecordingAction: FerroObjectImpl);

impl RecordingAction {
    fn static_constructor() {
        <dyn IAction>::register::<RecordingAction>();
    }

    fn new() -> Ref<Self> {
        instantiate(Self { base: FerroObject::construct(), execution_count: Cell::new(0) })
    }

    fn execution_count(&self) -> i32 {
        self.execution_count.get()
    }
}

impl IAction for RecordingAction {
    fn execute(&self, _sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        self.execution_count.set(self.execution_count.get() + 1);
        Some(Rc::new(true))
    }
}

/// The builder as the handle the properties hold.
fn builder_ref(builder: &Rc<RecordingAnimationBuilder>) -> Option<AnimationBuilderRef> {
    let builder: Rc<dyn IAnimationBuilder> = builder.clone();
    Some(AnimationBuilderRef::new(builder))
}

/// `new Animation { Duration = TimeSpan.Zero }`.
fn zero_animation() -> Ref<Animation> {
    let animation = Animation::new();
    animation.set_duration(TimeSpan::ZERO);
    animation
}

#[test]
fn transitions_behavior_applies_and_restores_transitions() {
    let _app = start_app();
    let original = Transitions::new();
    let replacement = Transitions::new();
    let target = Border::new();
    target.set_transitions(Some(original.clone()));
    let panel = Panel::new();
    panel.children().add(target.clone().upcast::<Control>());
    let behavior = TransitionsBehavior::new();
    behavior.set_value(TransitionsBehavior::transitions_source_property(), Some(replacement.clone()));
    assert!(behavior.transitions_source() == Some(replacement.clone()));
    add_behavior(&target, &behavior);
    let window = window_with(&panel);

    window.show();

    assert!(target.transitions() == Some(replacement.clone()));

    let control: Ref<Control> = target.clone().upcast();
    panel.children().remove(&control);

    assert!(target.transitions() == Some(original.clone()));
    window.close();
}

#[test]
fn runtime_loader_resolves_standalone_animations_and_behavior_adapters() {
    let _app = start_app();
    FerroRuntimeXamlLoader::register();
    let xaml = r#"<StackPanel xmlns="https://github.com/ferroui">
  <Border EntranceAnimations.FadeIn="0" />
  <Border>
    <Interaction.Behaviors>
      <FadeInBehavior InitialDelay="0:0:0" Duration="0:0:0" />
    </Interaction.Behaviors>
  </Border>
</StackPanel>"#;
    let mut configuration = RuntimeXamlLoaderConfiguration::new();
    configuration.local_assembly = Some(&ASSEMBLY);

    let result = FerroRuntimeXamlLoader::load_document(RuntimeXamlLoaderDocument::new(xaml), Some(configuration))
        .unwrap_or_else(|error| match error.inner_exception() {
            Some(inner) => panic!("the document failed to load: {}\n ---> {inner}", error.message()),
            None => panic!("the document failed to load: {}", error.message()),
        });

    let panel = from_markup_value::<Ref<StackPanel>>(&Some(result)).expect("a StackPanel");
    assert_eq!(2, panel.children().count());
    let border = panel.children().get(1).cast::<Border>().expect("a Border");
    let behaviors = Interaction::get_behaviors(&border);
    assert_eq!(1, behaviors.count());
    assert!(behaviors.get(0).cast::<FadeInBehavior>().is_some());
}

#[test]
fn transitions_changed_trigger_executes_for_shared_transition_observation() {
    let _app = start_app();
    let target = Border::new();
    let action = RecordingAction::new();
    let trigger = TransitionsChangedTrigger::new();
    trigger.actions().expect("the actions of the trigger").add(action.clone().upcast());
    add_behavior(&target, &trigger);
    run_jobs();
    let initial_count = action.execution_count();

    target.set_transitions(Some(Transitions::new()));
    run_jobs();

    assert_eq!(initial_count + 1, action.execution_count());
}

#[test]
fn fluid_move_behavior_animates_changed_child_position_through_shared_primitive() {
    let _app = start_app();
    let child = Border::new();
    child.set_width(40.0);
    child.set_height(40.0);
    Canvas::set_left(&child, 0.0);
    let canvas = Canvas::new();
    canvas.set_width(200.0);
    canvas.set_height(100.0);
    canvas.children().add(child.clone().upcast::<Control>());
    let behavior = FluidMoveBehavior::new();
    behavior.set_applies_to(FluidMoveScope::Children);
    behavior.set_duration(TimeSpan::ZERO);
    add_behavior(&canvas, &behavior);
    let window = window_with(&canvas);
    window.show();
    run_jobs();

    Canvas::set_left(&child, 80.0);
    run_jobs();

    let render_transform = child.render_transform().expect("a render transform");
    assert!(render_transform
        .as_object()
        .is_some_and(|object| object.downcast_ref::<TranslateTransform>().is_some()));
    window.close();
}

#[test]
fn parallax_behavior_tracks_scroll_offset_through_shared_primitive() {
    let app = CompositionApp::start();
    let target = Border::new();
    target.set_width(100.0);
    target.set_height(100.0);
    Canvas::set_left(&target, 30.0);
    Canvas::set_top(&target, 40.0);
    let content = Canvas::new();
    content.set_width(500.0);
    content.set_height(1000.0);
    content.children().add(target.clone().upcast::<Control>());
    let scroll_viewer = ScrollViewer::new();
    scroll_viewer.set_width(300.0);
    scroll_viewer.set_height(300.0);
    scroll_viewer.set_content(Some(Control::boxed(content)));
    let behavior = ParallaxBehavior::new();
    behavior.set_source_scroll_viewer(&scroll_viewer);
    behavior.set_parallax_ratio(0.25);
    add_behavior(&target, &behavior);
    let window = app.window_with(&scroll_viewer);
    window.show();
    app.run_jobs();

    scroll_viewer.set_offset(Vector::new(20.0, 100.0));
    app.run_jobs();

    assert_eq!(
        Some(Vector3::new(35.0, 65.0, 0.0).into()),
        ElementComposition::get_element_visual(&target).map(|visual| visual.offset())
    );
    window.close();
}

#[test]
fn animation_actions_preserve_targets_results_and_builder_precedence() {
    let _app = start_app();
    let target = Border::new();
    let animation = zero_animation();

    let start = StartAnimationAction::new();
    start.set_animation(Some(animation.clone()));
    assert!(!as_bool(start.execute(None, None)));
    assert!(as_bool(start.execute(Some(&boxed(&target)), None)));

    let begin = BeginAnimationAction::new();
    begin.set_animation(Some(animation.clone()));
    begin.set_target_control(&target);
    assert!(as_bool(begin.execute(None, None)));
    begin.set_is_enabled(false);
    assert!(!as_bool(begin.execute(Some(&boxed(&target)), None)));

    let builder = Rc::new(RecordingAnimationBuilder::default());
    let built = StartBuiltAnimationAction::new();
    built.set_animation_builder(builder_ref(&builder));
    assert!(as_bool(built.execute(Some(&boxed(&target)), None)));
    assert_eq!(1, builder.build_count());

    built.set_animation(Some(animation.clone()));
    assert!(as_bool(built.execute(Some(&boxed(&target)), None)));
    assert_eq!(1, builder.build_count());
}

#[test]
fn animate_on_attached_behavior_uses_builder_when_animation_is_missing() {
    let _app = start_app();
    let target = Border::new();
    let builder = Rc::new(RecordingAnimationBuilder::default());
    let behavior = AnimateOnAttachedBehavior::new();
    behavior.set_animation_builder(builder_ref(&builder));
    add_behavior(&target, &behavior);
    let window = window_with(&target);

    window.show();
    run_jobs();

    assert_eq!(1, builder.build_count());
    window.close();
}

#[test]
fn play_animation_behavior_runs_configured_animation_on_associated_visual() {
    let app = start_app();
    let target = Border::new();
    let animation = zero_animation();
    animation.set_fill_mode(FillMode::Forward);
    let setter: Rc<dyn IAnimationSetter> = Setter::new(Visual::opacity_property(), 0.25);
    animation.children().add(KeyFrame::with_cue(Cue::new(1.0), [setter]));
    let behavior = PlayAnimationBehavior::new();
    behavior.set_animation(Some(animation));
    add_behavior(&target, &behavior);
    let window = window_with(&target);

    window.show();
    run_jobs();
    app.clock.pulse(TimeSpan::ZERO);
    run_jobs();

    assert_eq!(0.25, target.opacity());
    window.close();
}

#[test]
fn animation_completed_trigger_executes_immediately_without_animation() {
    let _app = start_app();
    let target = Border::new();
    let action = RecordingAction::new();
    let trigger = AnimationCompletedTrigger::new();
    trigger.actions().expect("the actions of the trigger").add(action.clone().upcast());
    add_behavior(&target, &trigger);
    let window = window_with(&target);

    window.show();
    run_jobs();

    assert_eq!(1, action.execution_count());
    window.close();
}

#[test]
fn run_animation_trigger_uses_builder_and_executes_after_completion() {
    let app = start_app();
    let target = Border::new();
    let builder = Rc::new(RecordingAnimationBuilder::default());
    let action = RecordingAction::new();
    let trigger = RunAnimationTrigger::new();
    trigger.set_animation_builder(builder_ref(&builder));
    trigger.actions().expect("the actions of the trigger").add(action.clone().upcast());
    add_behavior(&target, &trigger);
    let window = window_with(&target);

    window.show();
    run_jobs();
    // `await Task.Delay(20)`: the clock of the animations ticks.
    app.clock.pulse(TimeSpan::ZERO);
    run_jobs();
    app.clock.pulse(TimeSpan::from_milliseconds(20.0));
    run_jobs();

    assert_eq!(1, builder.build_count());
    assert_eq!(1, action.execution_count());
    window.close();
}

/// Not from the reference: the fade in behavior has the delays of the reference by default.
#[test]
fn fade_in_behavior_has_the_default_delays_of_the_reference() {
    let _app = start_app();
    let behavior = FadeInBehavior::new();

    assert!(behavior.initial_delay() == TimeSpan::from_milliseconds(500.0));
    assert!(behavior.duration() == TimeSpan::from_milliseconds(250.0));
}
