//! `AnimatedVisualPlayer` and `AnimatedIcon`: controls that show an animated visual. Each frame is
//! evaluated and drawn through the drawing context, on the GPU of the backend.

use crate::playback::{AnimatedIconAnimationQueueBehavior, IconMove, IconStates, PlayerClock};
use crate::{AnimatedVisual, AnimatedVisualError, AnimatedVisualSource, CompositionScene, IAnimatedVisualSource};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::media::{Color, FlowDirection, ISolidColorBrush, MediaExtensions, Stretch, StretchDirection};
use ferroui_base::rendering::composition::{CompositionVisual, ElementComposition};
use ferroui_base::threading::{Dispatcher, DispatcherPriority, DispatcherTimer};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_properties, instantiate, AttachedProperty, FerroObject, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Rect, Ref, Size, StyledElementImpl,
    StyledProperty, StyledPropertyOptions, Vector, Vector3D, Visual, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::automation::peers::{AutomationPeer, ImageAutomationPeer};
use ferroui_controls::primitives::{TemplatedControl, TemplatedControlImpl, ToggleButton};
use ferroui_controls::{
    Button, ComboBox, Control, ControlImpl, ControlImplExt, IconElement, IconSource, IconSourceImpl, IconSourceImplExt,
    NavigationViewItem,
};
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::pin::Pin;
use std::rc::{Rc, Weak};
use std::task::{Context, Poll, Waker};
use std::time::Duration;

/// A segment shorter than this is not animated (`AnimatedIcon::PlaySegment`).
const SHORTEST_SEGMENT_SECONDS: f64 = 0.020;

ferroui_base::ferro_thread_local! {
    /// The scenes whose first commit is on its way to the compositor.
    static PENDING_REFRESH: RefCell<Vec<Weak<RefCell<CompositionScene>>>> = RefCell::new(Vec::new());
}

/// Sends the progress of the scenes now on the compositor.
fn refresh_pending() {
    let scenes = PENDING_REFRESH.with(|pending| std::mem::take(&mut *pending.borrow_mut()));
    for scene in scenes.iter().filter_map(Weak::upgrade) {
        scene.borrow().refresh();
    }
}

/// What a player and an icon keep to show their visual: the composition scene, built once when
/// the control is on screen, in a holder of the size of the control.
#[derive(Default)]
struct Presenter {
    visual: Option<Rc<AnimatedVisual>>,
    scene: Option<Rc<RefCell<CompositionScene>>>,
    holder: Option<Rc<CompositionVisual>>,
    colors: Vec<(String, Color)>,
    progress: f64,
}

impl Presenter {
    fn measure(&self, stretch: Stretch, available: Size) -> Size {
        match &self.visual {
            Some(visual) => MediaExtensions::calculate_size(
                stretch,
                available,
                Size::new(visual.width, visual.height),
                StretchDirection::Both,
            ),
            None => Size::default(),
        }
    }

    /// Takes `visual` and the colours of its source. Whether the visual is another one.
    fn take(&mut self, visual: Option<Rc<AnimatedVisual>>, source: Option<&Rc<dyn IAnimatedVisualSource>>) -> bool {
        let same = match (&self.visual, &visual) {
            (Some(old), Some(new)) => Rc::ptr_eq(old, new),
            (None, None) => true,
            _ => false,
        };
        self.visual = visual;
        if !same {
            self.scene = None;
        }
        for (name, color) in source.map_or_else(Vec::new, |source| source.color_properties()) {
            self.set_color(&name, Some(color));
        }
        !same
    }

    fn set_color(&mut self, name: &str, color: Option<Color>) {
        self.colors.retain(|(existing, _)| existing != name);
        if let Some(color) = color {
            self.colors.push((name.to_owned(), color));
        }
        if let Some(scene) = &self.scene {
            scene.borrow_mut().set_color(name, color);
        }
    }

    fn set_progress(&mut self, progress: f64) {
        self.progress = progress;
        if let Some(scene) = &self.scene {
            scene.borrow_mut().set_progress(progress);
        }
    }

    /// Builds the scene of the visual if the element is in a composition, and shows it.
    fn show(&mut self, element: &Visual) {
        let Some(visual) = self.visual.clone() else {
            self.hide(element);
            return;
        };
        if self.scene.is_some() {
            return;
        }
        let Some(own) = ElementComposition::get_element_visual(element) else {
            return;
        };
        let compositor = own.compositor().clone();
        let mut scene = CompositionScene::build(&compositor, &visual);
        for (name, color) in &self.colors {
            scene.set_color(name, Some(*color));
        }
        scene.set_progress(self.progress);
        let holder = compositor.create_container_visual();
        holder.children().insert_at_top(scene.root().clone());
        ElementComposition::set_element_child_visual(element, Some(holder.clone()));
        let scene = Rc::new(RefCell::new(scene));
        // A paused animation takes the progress once it runs on the compositor.
        let weak = Rc::downgrade(&scene);
        PENDING_REFRESH.with(|pending| pending.borrow_mut().push(weak));
        compositor.request_commit_async().processed().on_completed(|| {
            Dispatcher::ui_thread().post(refresh_pending, DispatcherPriority::RENDER);
        });
        self.scene = Some(scene);
        self.holder = Some(holder);
    }

    fn hide(&mut self, element: &Visual) {
        if self.holder.take().is_some() {
            ElementComposition::set_element_child_visual(element, None);
        }
        self.scene = None;
    }

    /// Fits the scene into `bounds`.
    fn arrange(&self, stretch: Stretch, mirrored: bool, bounds: Size) {
        let (Some(visual), Some(scene), Some(holder)) = (&self.visual, &self.scene, &self.holder) else {
            return;
        };
        holder.set_size(Vector::new(bounds.width, bounds.height));
        if bounds.width <= 0.0 || bounds.height <= 0.0 || visual.width <= 0.0 || visual.height <= 0.0 {
            return;
        }
        let source = Size::new(visual.width, visual.height);
        let scale = MediaExtensions::calculate_scaling(stretch, bounds, source, StretchDirection::Both);
        let (width, height) = (source.width * scale.x, source.height * scale.y);
        let (x, y) = ((bounds.width - width) * 0.5, (bounds.height - height) * 0.5);
        let scene = scene.borrow();
        let root = scene.root();
        if mirrored {
            root.set_scale(Vector3D::new(-scale.x, scale.y, 1.0));
            root.set_offset(Vector3D::new(bounds.width - x, y, 0.0));
        } else {
            root.set_scale(Vector3D::new(scale.x, scale.y, 1.0));
            root.set_offset(Vector3D::new(x, y, 0.0));
        }
    }
}

fn frame_timer<T>(owner: &Ref<T>, tick: fn(&T)) -> Rc<DispatcherTimer>
where
    T: ferroui_base::ObjectType + 'static,
{
    let weak = owner.downgrade();
    DispatcherTimer::with_callback(Duration::from_millis(16), DispatcherPriority::RENDER, move |_| {
        if let Some(owner) = weak.upgrade() {
            tick(&owner);
        }
    })
}

#[derive(Default)]
struct CompletionState {
    done: Cell<bool>,
    waker: RefCell<Option<Waker>>,
}

impl CompletionState {
    fn complete(&self) {
        self.done.set(true);
        if let Some(waker) = self.waker.borrow_mut().take() {
            waker.wake();
        }
    }
}

/// Completes when a play ends or is stopped (the task of `PlayAsync`).
pub struct PlayCompletion {
    state: Rc<CompletionState>,
}

impl PlayCompletion {
    /// Whether the play has ended.
    pub fn is_completed(&self) -> bool {
        self.state.done.get()
    }
}

impl Future for PlayCompletion {
    type Output = ();

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<()> {
        if self.state.done.get() {
            return Poll::Ready(());
        }
        *self.state.waker.borrow_mut() = Some(context.waker().clone());
        Poll::Pending
    }
}

/// Plays an animated visual.
#[repr(C)]
pub struct AnimatedVisualPlayer {
    base: Control,
    source_generation: Rc<Cell<u64>>,
    presenter: RefCell<Presenter>,
    clock: RefCell<PlayerClock>,
    timer: RefCell<Option<Rc<DispatcherTimer>>>,
    completion: RefCell<Option<Rc<CompletionState>>>,
    load_error: RefCell<Option<AnimatedVisualError>>,
    fallback: RefCell<Option<Ref<Control>>>,
    attached: Cell<bool>,
    loaded: Cell<bool>,
}

ferro_class!(AnimatedVisualPlayer: Control);
ferroui_base::ferro_class_info!(AnimatedVisualPlayer {
    new: AnimatedVisualPlayer::new,
    markup: { namespace: "FerroUI.AnimatedVisuals", content: Source },
});
ferro_impl_classes!(AnimatedVisualPlayer: StyledElementImpl, InteractiveImpl, InputElementImpl);

ferro_properties! {
    impl AnimatedVisualPlayer {
        pub fn playback_rate_property() -> StyledProperty<f64> {
            FerroProperty::register::<Self, _>("PlaybackRate", 1.0)
        }
        pub fn auto_play_property() -> StyledProperty<bool> {
            FerroProperty::register::<Self, _>("AutoPlay", true)
        }
        pub fn stretch_property() -> StyledProperty<Stretch> {
            FerroProperty::register::<Self, _>("Stretch", Stretch::Uniform)
        }
        pub fn source_property() -> StyledProperty<Option<Ref<AnimatedVisualSource>>> {
            FerroProperty::register::<Self, _>("Source", None)
        }
        pub fn fallback_content_property() -> StyledProperty<Option<Ref<Control>>> {
            FerroProperty::register::<Self, _>("FallbackContent", None)
        }
    }
}

impl FerroObjectImpl for AnimatedVisualPlayer {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        let property = change.property();
        if property == Self::source_property().as_property() {
            this.watch_source();
            if this.attached.get() {
                this.load();
            }
        } else if property == Self::fallback_content_property().as_property() {
            this.change_fallback(this.get_value(Self::fallback_content_property()));
        } else if property == Self::stretch_property().as_property() {
            this.invalidate_measure();
            this.invalidate_arrange();
        } else if property == Self::auto_play_property().as_property()
            && this.auto_play()
            && this.is_animated_visual_loaded()
            && !this.is_playing()
        {
            drop(this.play_async(0.0, 1.0, true));
        }
    }
}

impl LayoutableImpl for AnimatedVisualPlayer {
    fn measure_override(this: &Self, available: Size) -> Size {
        if let Some(fallback) = this.shown_fallback() {
            fallback.measure(available);
            return fallback.desired_size();
        }
        this.presenter.borrow().measure(this.stretch(), available)
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        if let Some(fallback) = this.shown_fallback() {
            fallback.arrange(Rect::from_size(final_size));
        }
        this.presenter.borrow().arrange(this.stretch(), false, final_size);
        final_size
    }
}

impl VisualImpl for AnimatedVisualPlayer {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        this.attached.set(true);
        this.load();
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        this.attached.set(false);
        this.stop_timer();
        Self::parent_on_detached_from_visual_tree(this, e);
    }
}

impl ControlImpl for AnimatedVisualPlayer {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        ImageAutomationPeer::new(this).upcast()
    }

    fn on_loaded(this: &Self, e: &RoutedEventArgs) {
        Self::parent_on_loaded(this, e);
        this.loaded.set(true);
        this.present();
    }

    fn on_unloaded(this: &Self, e: &RoutedEventArgs) {
        this.loaded.set(false);
        this.presenter.borrow_mut().hide(this);
        Self::parent_on_unloaded(this, e);
    }
}

impl AnimatedVisualPlayer {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: Control::construct(),
            source_generation: Rc::new(Cell::new(0)),
            presenter: RefCell::new(Presenter::default()),
            clock: RefCell::new(PlayerClock::default()),
            timer: RefCell::new(None),
            completion: RefCell::new(None),
            load_error: RefCell::new(None),
            fallback: RefCell::new(None),
            attached: Cell::new(false),
            loaded: Cell::new(false),
        })
    }

    /// Shows the progress in the scene, built when the player is on screen.
    fn present(&self) {
        let progress = self.clock.borrow().progress;
        let mut presenter = self.presenter.borrow_mut();
        presenter.set_progress(progress);
        if self.loaded.get() && presenter.scene.is_none() {
            presenter.show(self);
            presenter.arrange(self.stretch(), false, self.bounds().size());
        }
    }

    pub fn playback_rate(&self) -> f64 {
        self.get_value(Self::playback_rate_property())
    }
    pub fn set_playback_rate(&self, value: f64) {
        self.set_value(Self::playback_rate_property(), value);
    }
    pub fn auto_play(&self) -> bool {
        self.get_value(Self::auto_play_property())
    }
    pub fn set_auto_play(&self, value: bool) {
        self.set_value(Self::auto_play_property(), value);
    }
    pub fn stretch(&self) -> Stretch {
        self.get_value(Self::stretch_property())
    }
    pub fn set_stretch(&self, value: Stretch) {
        self.set_value(Self::stretch_property(), value);
    }

    /// The source of the visual. A new source is loaded at once when the player is in a tree.
    pub fn source(&self) -> Option<Ref<AnimatedVisualSource>> {
        self.get_value(Self::source_property())
    }
    pub fn set_source(&self, source: Option<Ref<AnimatedVisualSource>>) {
        self.set_value(Self::source_property(), source);
    }

    /// What is shown when the source gives no visual.
    pub fn fallback_content(&self) -> Option<Ref<Control>> {
        self.get_value(Self::fallback_content_property())
    }
    pub fn set_fallback_content(&self, content: Option<Ref<Control>>) {
        self.set_value(Self::fallback_content_property(), content);
    }

    fn change_fallback(&self, content: Option<Ref<Control>>) {
        let old = std::mem::replace(&mut *self.fallback.borrow_mut(), content);
        if let Some(old) = old {
            self.visual_children().remove(&old.upcast::<Visual>());
        }
        self.update_fallback();
    }

    /// Follows a source whose visual can change. An earlier source stops counting.
    fn watch_source(&self) {
        let generation = self.source_generation.get() + 1;
        self.source_generation.set(generation);
        let Some(source) = self.source().and_then(|source| source.source()) else {
            return;
        };
        if let Some(dynamic) = source.as_dynamic() {
            let weak = self.to_ref().downgrade();
            let current = self.source_generation.clone();
            dynamic.add_animated_visual_invalidated(Rc::new(move || {
                if current.get() == generation {
                    if let Some(player) = weak.upgrade() {
                        if player.attached.get() {
                            player.load();
                        }
                    }
                }
            }));
        }
    }

    pub fn is_animated_visual_loaded(&self) -> bool {
        self.presenter.borrow().visual.is_some()
    }
    pub fn is_playing(&self) -> bool {
        self.clock.borrow().is_playing()
    }
    /// The progress shown, from 0 to 1.
    pub fn progress(&self) -> f64 {
        self.clock.borrow().progress
    }
    /// The length of the visual.
    pub fn duration(&self) -> Duration {
        self.presenter
            .borrow()
            .visual
            .as_ref()
            .map_or(Duration::ZERO, |visual| Duration::from_secs_f64(visual.duration_seconds().max(0.0)))
    }
    /// Why the source gave no visual.
    pub fn load_error(&self) -> Option<AnimatedVisualError> {
        self.load_error.borrow().clone()
    }

    fn shown_fallback(&self) -> Option<Ref<Control>> {
        if self.is_animated_visual_loaded() {
            None
        } else {
            self.fallback.borrow().clone()
        }
    }

    fn update_fallback(&self) {
        let fallback = self.fallback.borrow().clone();
        if let Some(fallback) = fallback {
            let visual = fallback.upcast::<Visual>();
            let shown = self.visual_children().contains(&visual);
            if self.is_animated_visual_loaded() {
                if shown {
                    self.visual_children().remove(&visual);
                }
            } else if !shown {
                self.visual_children().add(visual);
            }
        }
        self.invalidate_measure();
    }

    fn load(&self) {
        let source = self.source().and_then(|source| source.source());
        let loaded = source.as_ref().map(|source| source.try_create_animated_visual());
        let visual = match loaded {
            Some(Ok(visual)) => {
                self.load_error.borrow_mut().take();
                Some(visual)
            }
            Some(Err(error)) => {
                *self.load_error.borrow_mut() = Some(error);
                None
            }
            None => {
                self.load_error.borrow_mut().take();
                None
            }
        };
        let has_visual = visual.is_some();
        let changed = self.presenter.borrow_mut().take(visual, source.as_ref());
        self.present();
        if !changed {
            // The same visual, with new colours: the play goes on.
            return;
        }
        self.update_fallback();
        if !has_visual {
            self.complete_play();
            self.clock.borrow_mut().set_progress(0.0);
            self.stop_timer();
        } else if self.is_playing() {
            self.start_timer();
        } else if self.auto_play() {
            drop(self.play_async(0.0, 1.0, true));
        }
    }

    /// Plays from `from_progress` to `to_progress`, again and again when `looped`. A segment that
    /// ends before it starts goes through the end. The result completes when the play ends, is
    /// stopped or is replaced.
    pub fn play_async(&self, from_progress: f64, to_progress: f64, looped: bool) -> PlayCompletion {
        self.complete_play();
        self.clock.borrow_mut().play(from_progress, to_progress, looped);
        let state = Rc::new(CompletionState::default());
        *self.completion.borrow_mut() = Some(state.clone());
        if self.is_animated_visual_loaded() {
            self.start_timer();
        }
        self.present();
        PlayCompletion { state }
    }

    pub fn pause(&self) {
        self.clock.borrow_mut().pause();
        self.stop_timer();
    }

    pub fn resume(&self) {
        self.clock.borrow_mut().resume();
        if self.clock.borrow().is_running() {
            self.start_timer();
        }
    }

    /// Goes to a progress and ends the play.
    pub fn set_progress(&self, progress: f64) {
        self.clock.borrow_mut().set_progress(progress);
        self.complete_play();
        self.stop_timer();
        self.present();
    }

    /// Ends the play, back at where it began.
    pub fn stop(&self) {
        if self.is_playing() {
            self.clock.borrow_mut().stop();
            self.complete_play();
            self.stop_timer();
            self.present();
        }
    }

    fn complete_play(&self) {
        let state = self.completion.borrow_mut().take();
        if let Some(state) = state {
            state.complete();
        }
    }

    fn start_timer(&self) {
        if !self.attached.get() {
            return;
        }
        let existing = self.timer.borrow().clone();
        let timer = existing.unwrap_or_else(|| {
            let timer = frame_timer(&self.to_ref(), Self::tick);
            *self.timer.borrow_mut() = Some(timer.clone());
            timer
        });
        timer.start();
    }

    fn stop_timer(&self) {
        let timer = self.timer.borrow().clone();
        if let Some(timer) = timer {
            timer.stop();
        }
    }

    fn tick(&self) {
        let now = Dispatcher::ui_thread().now() as f64;
        let duration = self.presenter.borrow().visual.as_ref().map_or(0.0, |visual| visual.duration_seconds());
        let rate = self.playback_rate();
        let finished = self.clock.borrow_mut().advance(now, duration, rate);
        self.present();
        if finished {
            self.stop_timer();
            self.complete_play();
        }
    }
}

impl Drop for AnimatedVisualPlayer {
    fn drop(&mut self) {
        if let Some(timer) = self.timer.get_mut().take() {
            timer.stop();
        }
    }
}

/// The segment an icon is playing.
#[derive(Clone, Copy)]
struct IconSegment {
    from: f64,
    to: f64,
    start_ms: f64,
    duration_ms: f64,
}

/// Shows an animated visual whose markers are the transitions between named states.
#[repr(C)]
pub struct AnimatedIcon {
    base: IconElement,
    source_generation: Rc<Cell<u64>>,
    pending_state: Cell<bool>,
    state_owner: RefCell<Option<(Ref<Control>, u64)>>,
    derived_state: RefCell<Option<String>>,
    presenter: RefCell<Presenter>,
    markers: RefCell<Vec<(String, f64)>>,
    states: RefCell<IconStates>,
    progress: Cell<f64>,
    segment: Cell<Option<IconSegment>>,
    previous_length: Cell<f64>,
    timer: RefCell<Option<Rc<DispatcherTimer>>>,
    fallback: RefCell<Option<Ref<Control>>>,
    attached: Cell<bool>,
    loaded: Cell<bool>,
}

ferro_class!(AnimatedIcon: IconElement);
ferroui_base::ferro_class_info!(AnimatedIcon {
    new: AnimatedIcon::new,
    markup: { namespace: "FerroUI.AnimatedVisuals", content: Source },
});
ferro_impl_classes!(AnimatedIcon: StyledElementImpl, InteractiveImpl, InputElementImpl, TemplatedControlImpl);

ferro_properties! {
    impl AnimatedIcon {
        /// The state: set on the icon or on an ancestor, which it inherits from.
        pub fn state_property() -> AttachedProperty<Option<String>> {
            FerroProperty::register_attached_with::<AnimatedIcon, Visual, _>(
                "State",
                StyledPropertyOptions::new(None).inherits(true),
            )
        }
        pub fn source_property() -> StyledProperty<Option<Ref<AnimatedVisualSource>>> {
            FerroProperty::register::<Self, _>("Source", None)
        }
        pub fn fallback_icon_source_property() -> StyledProperty<Option<Ref<IconSource>>> {
            FerroProperty::register::<Self, _>("FallbackIconSource", None)
        }
        pub fn mirrored_when_right_to_left_property() -> StyledProperty<bool> {
            FerroProperty::register::<Self, _>("MirroredWhenRightToLeft", false)
        }
    }
}

impl FerroObjectImpl for AnimatedIcon {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        let property = change.property();
        if property == Self::state_property().as_property() {
            this.on_state_changed();
        } else if property == Self::source_property().as_property() {
            this.watch_source();
            if this.attached.get() {
                this.load();
            }
        } else if property == Self::fallback_icon_source_property().as_property() {
            let element = this
                .get_value(Self::fallback_icon_source_property())
                .and_then(|source| source.create_icon_element())
                .map(|element| element.upcast::<Control>());
            this.set_fallback_icon(element);
        } else if property == TemplatedControl::foreground_property().as_property() {
            this.apply_foreground();
        } else if property == Self::mirrored_when_right_to_left_property().as_property()
            || property == Visual::flow_direction_property().as_property()
        {
            this.invalidate_arrange();
        }
    }
}

impl LayoutableImpl for AnimatedIcon {
    fn measure_override(this: &Self, available: Size) -> Size {
        if let Some(fallback) = this.shown_fallback() {
            fallback.measure(available);
            return fallback.desired_size();
        }
        this.presenter.borrow().measure(Stretch::Uniform, available)
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        if let Some(fallback) = this.shown_fallback() {
            fallback.arrange(Rect::from_size(final_size));
        }
        // A state waits for the layout it changed in (`OnLayoutUpdatedAfterStateChanged`).
        if this.pending_state.replace(false) {
            this.take_state();
        }
        this.presenter.borrow().arrange(Stretch::Uniform, this.is_mirrored(), final_size);
        final_size
    }
}

impl VisualImpl for AnimatedIcon {
    /// An icon turns only when `MirroredWhenRightToLeft` says so.
    fn bypass_flow_direction_policies(_this: &Self) -> bool {
        true
    }

    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        this.attached.set(true);
        this.load();
        this.apply_foreground();
        this.follow_state_owner();
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        this.attached.set(false);
        this.stop_timer();
        this.leave_state_owner();
        Self::parent_on_detached_from_visual_tree(this, e);
    }
}

impl ControlImpl for AnimatedIcon {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        ImageAutomationPeer::new(this).upcast()
    }

    fn on_loaded(this: &Self, e: &RoutedEventArgs) {
        Self::parent_on_loaded(this, e);
        this.loaded.set(true);
        this.present();
    }

    fn on_unloaded(this: &Self, e: &RoutedEventArgs) {
        this.loaded.set(false);
        this.presenter.borrow_mut().hide(this);
        Self::parent_on_unloaded(this, e);
    }
}

/// The state of an icon inside `owner`, as the templates of WinUI set it: the interaction, and for
/// a toggle whether it is on.
fn state_of(owner: &Control) -> String {
    let classes = owner.classes();
    let pressed = owner.downcast_ref::<Button>().map_or_else(|| classes.contains(":pressed"), Button::is_pressed);
    let toggle = owner.downcast_ref::<ToggleButton>().map(ToggleButton::is_checked);
    state_name(pressed, owner.is_pointer_over(), toggle)
}

/// The name of a state: the interaction, then `On`, `Off` or `Indeterminate` for a toggle.
fn state_name(pressed: bool, pointer_over: bool, toggle: Option<Option<bool>>) -> String {
    let interaction = if pressed {
        "Pressed"
    } else if pointer_over {
        "PointerOver"
    } else {
        "Normal"
    };
    match toggle {
        Some(Some(true)) => format!("{interaction}On"),
        Some(Some(false)) => format!("{interaction}Off"),
        Some(None) => format!("{interaction}Indeterminate"),
        None => interaction.to_owned(),
    }
}

impl AnimatedIcon {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: IconElement::construct(),
            source_generation: Rc::new(Cell::new(0)),
            pending_state: Cell::new(false),
            state_owner: RefCell::new(None),
            derived_state: RefCell::new(None),
            presenter: RefCell::new(Presenter::default()),
            markers: RefCell::new(Vec::new()),
            states: RefCell::new(IconStates::default()),
            progress: Cell::new(0.0),
            segment: Cell::new(None),
            previous_length: Cell::new(0.0),
            timer: RefCell::new(None),
            fallback: RefCell::new(None),
            attached: Cell::new(false),
            loaded: Cell::new(false),
        })
    }

    fn is_mirrored(&self) -> bool {
        self.mirrored_when_right_to_left() && self.flow_direction() == FlowDirection::RightToLeft
    }

    /// Shows the progress in the scene, built when the icon is on screen.
    fn present(&self) {
        let mut presenter = self.presenter.borrow_mut();
        presenter.set_progress(self.progress.get());
        if self.loaded.get() && presenter.scene.is_none() {
            presenter.show(self);
            presenter.arrange(Stretch::Uniform, self.is_mirrored(), self.bounds().size());
        }
    }

    /// The state set on `element` (`AnimatedIcon.GetState`).
    pub fn get_state(element: &FerroObject) -> Option<String> {
        element.get_value(Self::state_property())
    }

    /// Sets the state on `element` (`AnimatedIcon.SetState`).
    pub fn set_state(element: &FerroObject, state: Option<String>) {
        element.set_value(Self::state_property(), state);
    }

    pub fn source(&self) -> Option<Ref<AnimatedVisualSource>> {
        self.get_value(Self::source_property())
    }
    pub fn set_source(&self, source: Option<Ref<AnimatedVisualSource>>) {
        self.set_value(Self::source_property(), source);
    }

    pub fn fallback_icon_source(&self) -> Option<Ref<IconSource>> {
        self.get_value(Self::fallback_icon_source_property())
    }
    pub fn set_fallback_icon_source(&self, source: Option<Ref<IconSource>>) {
        self.set_value(Self::fallback_icon_source_property(), source);
    }

    pub fn mirrored_when_right_to_left(&self) -> bool {
        self.get_value(Self::mirrored_when_right_to_left_property())
    }
    pub fn set_mirrored_when_right_to_left(&self, value: bool) {
        self.set_value(Self::mirrored_when_right_to_left_property(), value);
    }

    /// The colour the paints bound to `Foreground` take: that of a solid foreground. A brush of
    /// another kind leaves the colour of the document.
    fn apply_foreground(&self) {
        let color: Option<Color> =
            self.foreground().and_then(|brush| brush.as_solid_color_brush().map(ISolidColorBrush::color));
        self.presenter.borrow_mut().set_color("Foreground", color);
        self.present();
    }

    /// Follows a source whose visual can change. An earlier source stops counting.
    fn watch_source(&self) {
        let generation = self.source_generation.get() + 1;
        self.source_generation.set(generation);
        let Some(source) = self.source().and_then(|source| source.source()) else {
            return;
        };
        if let Some(dynamic) = source.as_dynamic() {
            let weak = self.to_ref().downgrade();
            let current = self.source_generation.clone();
            dynamic.add_animated_visual_invalidated(Rc::new(move || {
                if current.get() == generation {
                    if let Some(icon) = weak.upgrade() {
                        if icon.attached.get() {
                            icon.load();
                        }
                    }
                }
            }));
        }
    }

    /// Takes its state from the nearest button, combo box or navigation item around it, unless
    /// something else sets the state.
    fn follow_state_owner(&self) {
        self.leave_state_owner();
        let owner = self.get_visual_ancestors().filter_map(|ancestor| ancestor.cast::<Control>()).find(|control| {
            control.downcast_ref::<Button>().is_some()
                || control.downcast_ref::<ComboBox>().is_some()
                || control.downcast_ref::<NavigationViewItem>().is_some()
        });
        let Some(owner) = owner else {
            return;
        };
        let weak = self.to_ref().downgrade();
        let token = owner.add_property_changed_handler(Rc::new(move |change| {
            if matches!(change.property().name(), "IsPointerOver" | "IsPressed" | "IsChecked") {
                if let Some(icon) = weak.upgrade() {
                    icon.update_from_owner();
                }
            }
        }));
        *self.state_owner.borrow_mut() = Some((owner, token));
        self.update_from_owner();
    }

    fn leave_state_owner(&self) {
        if let Some((owner, token)) = self.state_owner.borrow_mut().take() {
            owner.remove_property_changed_handler(token);
        }
        if self.derived_state.borrow_mut().take().is_some() {
            self.clear_value(Self::state_property());
        }
    }

    fn update_from_owner(&self) {
        let owner = self.state_owner.borrow().as_ref().map(|(owner, _)| owner.clone());
        let Some(owner) = owner else {
            return;
        };
        // A state set by the application or inherited from it stays.
        let current: Option<String> = self.get_value(Self::state_property());
        if current != *self.derived_state.borrow() {
            return;
        }
        let state = state_of(&owner);
        *self.derived_state.borrow_mut() = Some(state.clone());
        self.set_value(Self::state_property(), Some(state));
    }

    /// What is shown when the source gives no visual (`FallbackIconSource`).
    pub(crate) fn set_fallback_icon(&self, icon: Option<Ref<Control>>) {
        let old = std::mem::replace(&mut *self.fallback.borrow_mut(), icon);
        if let Some(old) = old {
            self.visual_children().remove(&old.upcast::<Visual>());
        }
        self.update_fallback();
    }

    /// How a state that arrives during a play is taken.
    pub fn set_animation_queue_behavior(&self, behavior: AnimatedIconAnimationQueueBehavior) {
        self.states.borrow_mut().behavior = behavior;
    }

    /// The progress shown.
    pub fn progress(&self) -> f64 {
        self.progress.get()
    }

    fn shown_fallback(&self) -> Option<Ref<Control>> {
        if self.presenter.borrow().visual.is_some() {
            None
        } else {
            self.fallback.borrow().clone()
        }
    }

    fn update_fallback(&self) {
        let fallback = self.fallback.borrow().clone();
        if let Some(fallback) = fallback {
            let visual = fallback.upcast::<Visual>();
            let shown = self.visual_children().contains(&visual);
            if self.presenter.borrow().visual.is_some() {
                if shown {
                    self.visual_children().remove(&visual);
                }
            } else if !shown {
                self.visual_children().add(visual);
            }
        }
        self.invalidate_measure();
    }

    fn load(&self) {
        let source = self.source().and_then(|source| source.source());
        let visual = source.as_ref().and_then(|source| source.try_create_animated_visual().ok());
        *self.markers.borrow_mut() = visual.as_ref().map_or_else(Vec::new, |visual| visual.progress_markers());
        self.presenter.borrow_mut().take(visual, source.as_ref());
        // The colours of the source do not replace the foreground of the icon.
        if self.foreground().is_some() {
            self.apply_foreground();
        }
        self.update_fallback();
        self.present();
    }

    /// In a tree a state is taken after the next layout; outside one, at once.
    fn on_state_changed(&self) {
        if self.attached.get() {
            self.pending_state.set(true);
            self.invalidate_arrange();
        } else {
            self.take_state();
        }
    }

    fn take_state(&self) {
        let state = self.get_value(Self::state_property()).unwrap_or_default();
        let step = {
            let markers = self.markers.borrow();
            self.states.borrow_mut().request(&markers, &state)
        };
        if let Some(step) = step {
            self.apply(step);
        }
    }

    fn apply(&self, step: IconMove) {
        match step {
            IconMove::Cut(progress) => {
                self.progress.set(progress);
                self.present();
            }
            IconMove::Play { from, to, multiplier } => self.play_segment(from, to, multiplier),
        }
    }

    /// Plays to `to`, from `from` or from where the icon is (`AnimatedIcon::PlaySegment`).
    fn play_segment(&self, from: Option<f64>, to: f64, multiplier: f64) {
        let length = from.map_or(self.previous_length.get(), |from| (to - from).abs());
        self.previous_length.set(length);
        let visual_seconds = self.presenter.borrow().visual.as_ref().map_or(0.0, |visual| visual.duration_seconds());
        let seconds = visual_seconds * length / multiplier;
        if seconds < SHORTEST_SEGMENT_SECONDS || !self.attached.get() {
            self.progress.set(to);
            self.present();
            self.segment_finished();
            return;
        }
        let start = from.unwrap_or(self.progress.get());
        self.progress.set(start);
        self.segment.set(Some(IconSegment {
            from: start,
            to,
            start_ms: Dispatcher::ui_thread().now() as f64,
            duration_ms: seconds * 1000.0,
        }));
        let existing = self.timer.borrow().clone();
        let timer = existing.unwrap_or_else(|| {
            let timer = frame_timer(&self.to_ref(), Self::tick);
            *self.timer.borrow_mut() = Some(timer.clone());
            timer
        });
        timer.start();
    }

    fn tick(&self) {
        let Some(segment) = self.segment.get() else {
            self.stop_timer();
            return;
        };
        let now = Dispatcher::ui_thread().now() as f64;
        let t = ((now - segment.start_ms) / segment.duration_ms).clamp(0.0, 1.0);
        self.progress.set(segment.from + (segment.to - segment.from) * t);
        self.present();
        if t >= 1.0 {
            self.segment.set(None);
            self.stop_timer();
            self.segment_finished();
        }
    }

    fn segment_finished(&self) {
        let step = {
            let markers = self.markers.borrow();
            self.states.borrow_mut().completed(&markers)
        };
        if let Some(step) = step {
            self.apply(step);
        }
    }

    fn stop_timer(&self) {
        let timer = self.timer.borrow().clone();
        if let Some(timer) = timer {
            timer.stop();
        }
    }
}

/// The description of an [`AnimatedIcon`] (`AnimatedIconSource`), for what takes an icon source.
#[repr(C)]
pub struct AnimatedIconSource {
    base: IconSource,
}

ferro_class!(AnimatedIconSource: IconSource);
ferroui_base::ferro_class_info!(AnimatedIconSource {
    new: AnimatedIconSource::new,
    markup: { namespace: "FerroUI.AnimatedVisuals", content: Source },
});
ferro_impl_classes!(AnimatedIconSource: FerroObjectImpl);

ferro_properties! {
    impl AnimatedIconSource {
        pub fn source_property() -> StyledProperty<Option<Ref<AnimatedVisualSource>>> {
            FerroProperty::register::<Self, _>("Source", None)
        }
        pub fn fallback_icon_source_property() -> StyledProperty<Option<Ref<IconSource>>> {
            FerroProperty::register::<Self, _>("FallbackIconSource", None)
        }
        pub fn mirrored_when_right_to_left_property() -> StyledProperty<bool> {
            FerroProperty::register::<Self, _>("MirroredWhenRightToLeft", false)
        }
    }
}

impl IconSourceImpl for AnimatedIconSource {
    fn create_icon_element_core(this: &Self) -> Option<Ref<IconElement>> {
        let icon = AnimatedIcon::new();
        icon.set_source(this.source());
        icon.set_fallback_icon_source(this.fallback_icon_source());
        icon.set_mirrored_when_right_to_left(this.mirrored_when_right_to_left());
        Some(icon.upcast())
    }

    fn get_icon_element_property_core(this: &Self, property: &'static FerroProperty) -> Option<&'static FerroProperty> {
        if property == Self::source_property().as_property() {
            Some(AnimatedIcon::source_property().as_property())
        } else if property == Self::fallback_icon_source_property().as_property() {
            Some(AnimatedIcon::fallback_icon_source_property().as_property())
        } else if property == Self::mirrored_when_right_to_left_property().as_property() {
            Some(AnimatedIcon::mirrored_when_right_to_left_property().as_property())
        } else {
            Self::parent_get_icon_element_property_core(this, property)
        }
    }
}

impl AnimatedIconSource {
    pub fn construct() -> Self {
        Self { base: IconSource::construct() }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn source(&self) -> Option<Ref<AnimatedVisualSource>> {
        self.get_value(Self::source_property())
    }
    pub fn set_source(&self, value: Option<Ref<AnimatedVisualSource>>) {
        self.set_value(Self::source_property(), value);
    }
    pub fn fallback_icon_source(&self) -> Option<Ref<IconSource>> {
        self.get_value(Self::fallback_icon_source_property())
    }
    pub fn set_fallback_icon_source(&self, value: Option<Ref<IconSource>>) {
        self.set_value(Self::fallback_icon_source_property(), value);
    }
    pub fn mirrored_when_right_to_left(&self) -> bool {
        self.get_value(Self::mirrored_when_right_to_left_property())
    }
    pub fn set_mirrored_when_right_to_left(&self, value: bool) {
        self.set_value(Self::mirrored_when_right_to_left_property(), value);
    }
}

impl Drop for AnimatedIcon {
    fn drop(&mut self) {
        if let Some(timer) = self.timer.get_mut().take() {
            timer.stop();
        }
    }
}

#[cfg(test)]
#[path = "player_tests.rs"]
mod tests;
