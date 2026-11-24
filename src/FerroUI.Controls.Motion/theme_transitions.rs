//! The theme transitions of WinUI: `EntranceThemeTransition`, `RepositionThemeTransition`,
//! `ContentThemeTransition`, `AddDeleteThemeTransition` and `PopupThemeTransition`, in a
//! `TransitionCollection`.
//!
//! A collection is set through an attached property of [`ThemeTransitions`], with markup aliases
//! on the WinUI names `ChildrenTransitions`, `ItemContainerTransitions` and `ChildTransitions`.
//! `UIElement.Transitions` of WinUI is written `ThemeTransitions.Transitions`, because the
//! `Transitions` of an element is the collection of property transitions
//! (docs/porting/DEVIATIONS.md, "Theme transitions").
//!
//! The motion is a set of composition animations built once per collection and compositor:
//! elements share them and start them with their own final values. Nothing here runs per frame,
//! and an element without a collection has no cost.

use ferroui_base::animation::easings::Easing;
use ferroui_base::collections::{FerroList, NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs};
use ferroui_base::numerics::Vector2;
use ferroui_base::reactive::IDisposable;
use ferroui_base::rendering::composition::animations::{
    AnimationDelayBehavior, CompositionAnimationGroup, CompositionEasingFunction, ICompositionAnimation,
    ICompositionAnimationBase,
};
use ferroui_base::rendering::composition::expressions::ExpressionVariant;
use ferroui_base::rendering::composition::{
    CompositionVisual, Compositor, ElementComposition, ICompositionObjectAnimations,
};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_markup_type, ferro_properties, ferro_static_type, ferro_thread_local,
    instantiate, AttachedProperty, FerroObject, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Rect, Ref, StyledProperty, Visual, WeakRef,
};
use ferroui_controls::primitives::Popup;
use ferroui_controls::{Control, ItemsChangedEventArgs, ItemsControl, Panel};
use std::cell::{Cell, RefCell};
use std::ops::Deref;
use std::rc::{Rc, Weak};
use std::time::Duration;

// The timings and curves of WinUI come from the system theme (`GetThemeAnimationProperty`) and
// are not in its sources; these are this port's values (DEVIATIONS.md, "Theme transitions").
const SHOW_DURATION: Duration = Duration::from_millis(300);
const HIDE_DURATION: Duration = Duration::from_millis(150);
const REPOSITION_DURATION: Duration = Duration::from_millis(300);
const STAGGER_STEP: Duration = Duration::from_millis(50);
/// The elements after this many start together with the last delayed one.
const STAGGER_SLOTS: usize = 8;

const OFFSET: &str = "Offset";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Entrance,
    Reposition,
    Content,
    AddDelete,
    Popup,
}

/// `Microsoft.UI.Xaml.Media.Animation.Transition`: the base of the theme transitions.
#[repr(C)]
pub struct ThemeTransition {
    base: FerroObject,
    kind: Kind,
    hosts: RefCell<Vec<Weak<Host>>>,
}

ferro_class!(ThemeTransition: FerroObject);
ferro_class_info!(ThemeTransition { markup: { namespace: "FerroUI.Controls" } });

impl FerroObjectImpl for ThemeTransition {}

impl ThemeTransition {
    fn construct(kind: Kind) -> Self {
        Self { base: FerroObject::construct(), kind, hosts: RefCell::new(Vec::new()) }
    }

    /// The hosts of the transition build their animations again.
    fn changed(&self) {
        let hosts: Vec<_> = self.hosts.borrow().iter().filter_map(Weak::upgrade).collect();
        for host in hosts {
            host.refresh();
        }
    }

    fn join(&self, host: &Rc<Host>) {
        let mut hosts = self.hosts.borrow_mut();
        hosts.retain(|known| known.strong_count() > 0);
        if !hosts.iter().any(|known| std::ptr::eq(known.as_ptr(), Rc::as_ptr(host))) {
            hosts.push(Rc::downgrade(host));
        }
    }

    fn leave(&self, host: &Host) {
        self.hosts.borrow_mut().retain(|known| known.strong_count() > 0 && !std::ptr::eq(known.as_ptr(), host));
    }
}

/// `EntranceThemeTransition`: elements fade in from an offset when they first appear.
#[repr(C)]
pub struct EntranceThemeTransition {
    base: ThemeTransition,
}

ferro_class!(EntranceThemeTransition: ThemeTransition);
ferro_class_info!(EntranceThemeTransition { new: EntranceThemeTransition::new, markup: { namespace: "FerroUI.Controls" } });

ferro_properties! {
    impl EntranceThemeTransition {
        /// Defines the `FromHorizontalOffset` property: 0 by default, as in WinUI.
        pub fn from_horizontal_offset_property() -> StyledProperty<f64> {
            FerroProperty::register::<EntranceThemeTransition, _>("FromHorizontalOffset", 0.0)
        }
        /// Defines the `FromVerticalOffset` property: 28 by default, as in WinUI.
        pub fn from_vertical_offset_property() -> StyledProperty<f64> {
            FerroProperty::register::<EntranceThemeTransition, _>("FromVerticalOffset", 28.0)
        }
        /// Defines the `IsStaggeringEnabled` property: `false` by default, as in WinUI.
        pub fn is_staggering_enabled_property() -> StyledProperty<bool> {
            FerroProperty::register::<EntranceThemeTransition, _>("IsStaggeringEnabled", false)
        }
    }
}

impl FerroObjectImpl for EntranceThemeTransition {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        this.base.changed();
    }
}

impl EntranceThemeTransition {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: ThemeTransition::construct(Kind::Entrance) }
    }

    /// Creates a transition with the default values of WinUI.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The horizontal distance elements start from.
    pub fn from_horizontal_offset(&self) -> f64 {
        self.get_value(Self::from_horizontal_offset_property())
    }

    /// Sets [`from_horizontal_offset`](Self::from_horizontal_offset).
    pub fn set_from_horizontal_offset(&self, value: f64) {
        self.set_value(Self::from_horizontal_offset_property(), value);
    }

    /// The vertical distance elements start from.
    pub fn from_vertical_offset(&self) -> f64 {
        self.get_value(Self::from_vertical_offset_property())
    }

    /// Sets [`from_vertical_offset`](Self::from_vertical_offset).
    pub fn set_from_vertical_offset(&self, value: f64) {
        self.set_value(Self::from_vertical_offset_property(), value);
    }

    /// Whether elements start one after the other.
    pub fn is_staggering_enabled(&self) -> bool {
        self.get_value(Self::is_staggering_enabled_property())
    }

    /// Sets [`is_staggering_enabled`](Self::is_staggering_enabled).
    pub fn set_is_staggering_enabled(&self, value: bool) {
        self.set_value(Self::is_staggering_enabled_property(), value);
    }
}

/// `RepositionThemeTransition`: an element slides to the place layout gives it.
#[repr(C)]
pub struct RepositionThemeTransition {
    base: ThemeTransition,
}

ferro_class!(RepositionThemeTransition: ThemeTransition);
ferro_class_info!(RepositionThemeTransition { new: RepositionThemeTransition::new, markup: { namespace: "FerroUI.Controls" } });

ferro_properties! {
    impl RepositionThemeTransition {
        /// Defines the `IsStaggeringEnabled` property: `true` by default, as in WinUI.
        pub fn is_staggering_enabled_property() -> StyledProperty<bool> {
            FerroProperty::register::<RepositionThemeTransition, _>("IsStaggeringEnabled", true)
        }
    }
}

impl FerroObjectImpl for RepositionThemeTransition {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        this.base.changed();
    }
}

impl RepositionThemeTransition {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: ThemeTransition::construct(Kind::Reposition) }
    }

    /// Creates a transition with the default values of WinUI.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Whether elements start one after the other.
    pub fn is_staggering_enabled(&self) -> bool {
        self.get_value(Self::is_staggering_enabled_property())
    }

    /// Sets [`is_staggering_enabled`](Self::is_staggering_enabled).
    pub fn set_is_staggering_enabled(&self, value: bool) {
        self.set_value(Self::is_staggering_enabled_property(), value);
    }
}

/// `ContentThemeTransition`: content fades in from an offset and fades out.
#[repr(C)]
pub struct ContentThemeTransition {
    base: ThemeTransition,
}

ferro_class!(ContentThemeTransition: ThemeTransition);
ferro_class_info!(ContentThemeTransition { new: ContentThemeTransition::new, markup: { namespace: "FerroUI.Controls" } });

ferro_properties! {
    impl ContentThemeTransition {
        /// Defines the `HorizontalOffset` property: 0 by default, as in WinUI.
        pub fn horizontal_offset_property() -> StyledProperty<f64> {
            FerroProperty::register::<ContentThemeTransition, _>("HorizontalOffset", 0.0)
        }
        /// Defines the `VerticalOffset` property: 28 by default, as in WinUI.
        pub fn vertical_offset_property() -> StyledProperty<f64> {
            FerroProperty::register::<ContentThemeTransition, _>("VerticalOffset", 28.0)
        }
    }
}

impl FerroObjectImpl for ContentThemeTransition {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        this.base.changed();
    }
}

impl ContentThemeTransition {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: ThemeTransition::construct(Kind::Content) }
    }

    /// Creates a transition with the default values of WinUI.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The horizontal distance content starts from.
    pub fn horizontal_offset(&self) -> f64 {
        self.get_value(Self::horizontal_offset_property())
    }

    /// Sets [`horizontal_offset`](Self::horizontal_offset).
    pub fn set_horizontal_offset(&self, value: f64) {
        self.set_value(Self::horizontal_offset_property(), value);
    }

    /// The vertical distance content starts from.
    pub fn vertical_offset(&self) -> f64 {
        self.get_value(Self::vertical_offset_property())
    }

    /// Sets [`vertical_offset`](Self::vertical_offset).
    pub fn set_vertical_offset(&self, value: f64) {
        self.set_value(Self::vertical_offset_property(), value);
    }
}

/// `AddDeleteThemeTransition`: added elements fade in, removed ones fade out and the others slide
/// to their new places.
#[repr(C)]
pub struct AddDeleteThemeTransition {
    base: ThemeTransition,
}

ferro_class!(AddDeleteThemeTransition: ThemeTransition);
ferro_class_info!(AddDeleteThemeTransition { new: AddDeleteThemeTransition::new, markup: { namespace: "FerroUI.Controls" } });

impl FerroObjectImpl for AddDeleteThemeTransition {}

impl AddDeleteThemeTransition {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: ThemeTransition::construct(Kind::AddDelete) }
    }

    /// Creates a transition.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}

/// `PopupThemeTransition`: the child of a popup fades in from an offset when the popup opens.
#[repr(C)]
pub struct PopupThemeTransition {
    base: ThemeTransition,
}

ferro_class!(PopupThemeTransition: ThemeTransition);
ferro_class_info!(PopupThemeTransition { new: PopupThemeTransition::new, markup: { namespace: "FerroUI.Controls" } });

ferro_properties! {
    impl PopupThemeTransition {
        /// Defines the `FromHorizontalOffset` property: 0 by default, as in WinUI.
        pub fn from_horizontal_offset_property() -> StyledProperty<f64> {
            FerroProperty::register::<PopupThemeTransition, _>("FromHorizontalOffset", 0.0)
        }
        /// Defines the `FromVerticalOffset` property: 40 by default, as in WinUI.
        pub fn from_vertical_offset_property() -> StyledProperty<f64> {
            FerroProperty::register::<PopupThemeTransition, _>("FromVerticalOffset", 40.0)
        }
    }
}

impl FerroObjectImpl for PopupThemeTransition {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        this.base.changed();
    }
}

impl PopupThemeTransition {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: ThemeTransition::construct(Kind::Popup) }
    }

    /// Creates a transition with the default values of WinUI.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The horizontal distance the child starts from.
    pub fn from_horizontal_offset(&self) -> f64 {
        self.get_value(Self::from_horizontal_offset_property())
    }

    /// Sets [`from_horizontal_offset`](Self::from_horizontal_offset).
    pub fn set_from_horizontal_offset(&self, value: f64) {
        self.set_value(Self::from_horizontal_offset_property(), value);
    }

    /// The vertical distance the child starts from.
    pub fn from_vertical_offset(&self) -> f64 {
        self.get_value(Self::from_vertical_offset_property())
    }

    /// Sets [`from_vertical_offset`](Self::from_vertical_offset).
    pub fn set_from_vertical_offset(&self, value: f64) {
        self.set_value(Self::from_vertical_offset_property(), value);
    }
}

/// `TransitionCollection`: the theme transitions of an element. Handles are shared.
#[derive(Clone, Debug, PartialEq)]
pub struct TransitionCollection(FerroList<Ref<ThemeTransition>>);

ferroui_base::styled_value!(TransitionCollection);

impl TransitionCollection {
    /// Creates an empty collection.
    pub fn new() -> Self {
        Self(FerroList::new())
    }

    /// Creates a collection holding `items`.
    pub fn from_items(items: impl IntoIterator<Item = Ref<ThemeTransition>>) -> Self {
        Self(FerroList::from_items(items))
    }
}

impl Default for TransitionCollection {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for TransitionCollection {
    type Target = FerroList<Ref<ThemeTransition>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

ferro_markup_type!(class TransitionCollection {
    namespace: "FerroUI.Controls",
    handles: [TransitionCollection, Option<TransitionCollection>],
    constructors: [() => TransitionCollection::new],
    properties: [Count: i32 { get: |items: &TransitionCollection| items.count() as i32 }],
    methods: [fn Add(Ref<ThemeTransition>) => |items: &TransitionCollection, item: Ref<ThemeTransition>| items.add(item)],
});

/// The properties the WinUI names of the theme transitions are aliases of.
pub struct ThemeTransitions;

ferro_static_type!(ThemeTransitions);

ferro_properties! {
    impl ThemeTransitions {
        /// Defines the attached `ChildrenTransitions` property of a panel.
        pub fn children_transitions_property() -> AttachedProperty<Option<TransitionCollection>> {
            FerroProperty::register_attached::<ThemeTransitions, Panel, _>("ChildrenTransitions", None)
        }
        /// Defines the attached `ItemContainerTransitions` property of an items control.
        pub fn item_container_transitions_property() -> AttachedProperty<Option<TransitionCollection>> {
            FerroProperty::register_attached::<ThemeTransitions, ItemsControl, _>("ItemContainerTransitions", None)
        }
        /// Defines the attached `ChildTransitions` property of a popup.
        pub fn child_transitions_property() -> AttachedProperty<Option<TransitionCollection>> {
            FerroProperty::register_attached::<ThemeTransitions, Popup, _>("ChildTransitions", None)
        }
        /// Defines the attached `Transitions` property: `UIElement.Transitions` of WinUI.
        pub fn transitions_property() -> AttachedProperty<Option<TransitionCollection>> {
            FerroProperty::register_attached::<ThemeTransitions, Visual, _>("Transitions", None)
        }
    }
}

impl ThemeTransitions {
    /// The value of `ChildrenTransitions` of `element`.
    pub fn get_children_transitions(element: &Panel) -> Option<TransitionCollection> {
        element.get_value(Self::children_transitions_property())
    }

    /// Sets the value of `ChildrenTransitions` of `element`.
    pub fn set_children_transitions(element: &Panel, value: Option<TransitionCollection>) {
        element.set_value(Self::children_transitions_property(), value);
    }

    /// The value of `ItemContainerTransitions` of `element`.
    pub fn get_item_container_transitions(element: &ItemsControl) -> Option<TransitionCollection> {
        element.get_value(Self::item_container_transitions_property())
    }

    /// Sets the value of `ItemContainerTransitions` of `element`.
    pub fn set_item_container_transitions(element: &ItemsControl, value: Option<TransitionCollection>) {
        element.set_value(Self::item_container_transitions_property(), value);
    }

    /// The value of `ChildTransitions` of `element`.
    pub fn get_child_transitions(element: &Popup) -> Option<TransitionCollection> {
        element.get_value(Self::child_transitions_property())
    }

    /// Sets the value of `ChildTransitions` of `element`.
    pub fn set_child_transitions(element: &Popup, value: Option<TransitionCollection>) {
        element.set_value(Self::child_transitions_property(), value);
    }

    /// The value of `Transitions` of `element`.
    pub fn get_transitions(element: &Visual) -> Option<TransitionCollection> {
        element.get_value(Self::transitions_property())
    }

    /// Sets the value of `Transitions` of `element`.
    pub fn set_transitions(element: &Visual, value: Option<TransitionCollection>) {
        element.set_value(Self::transitions_property(), value);
    }

    fn static_constructor() {
        for (property, slot) in [
            (Self::children_transitions_property(), Slot::Children),
            (Self::item_container_transitions_property(), Slot::Items),
            (Self::child_transitions_property(), Slot::Popup),
            (Self::transitions_property(), Slot::Own),
        ] {
            property
                .changed()
                .subscribe(move |change| {
                    let Some(owner) = change.sender().downcast_ref::<Visual>() else { return };
                    let (_, new) = change.get_old_and_new_value::<Option<TransitionCollection>>();
                    Host::set(owner, slot, new);
                })
                .detach();
        }
    }
}

/// What the transitions of a collection do together.
#[derive(Clone, Copy, Default, PartialEq)]
struct Motion {
    /// The element fades in from this distance.
    show: Option<(f64, f64)>,
    stagger_show: bool,
    hide: bool,
    reposition: bool,
    stagger_reposition: bool,
}

impl Motion {
    fn of(collection: &TransitionCollection) -> Self {
        let mut motion = Motion::default();
        for transition in collection.snapshot().iter() {
            match transition.kind {
                Kind::Entrance => {
                    let Some(entrance) = transition.downcast_ref::<EntranceThemeTransition>() else { continue };
                    motion.show.get_or_insert((entrance.from_horizontal_offset(), entrance.from_vertical_offset()));
                    motion.stagger_show |= entrance.is_staggering_enabled();
                }
                Kind::Reposition => {
                    let Some(reposition) = transition.downcast_ref::<RepositionThemeTransition>() else { continue };
                    motion.reposition = true;
                    motion.stagger_reposition |= reposition.is_staggering_enabled();
                }
                Kind::Content => {
                    let Some(content) = transition.downcast_ref::<ContentThemeTransition>() else { continue };
                    motion.show.get_or_insert((content.horizontal_offset(), content.vertical_offset()));
                    motion.hide = true;
                }
                Kind::AddDelete => {
                    motion.show.get_or_insert((0.0, 0.0));
                    motion.hide = true;
                    motion.reposition = true;
                }
                Kind::Popup => {
                    let Some(popup) = transition.downcast_ref::<PopupThemeTransition>() else { continue };
                    motion.show.get_or_insert((popup.from_horizontal_offset(), popup.from_vertical_offset()));
                    motion.hide = true;
                }
            }
        }
        motion
    }
}

/// The animations of a collection on one compositor, shared by its elements.
struct Compiled {
    compositor: Weak<Compositor>,
    motion: Motion,
    show: Vec<(Rc<dyn ICompositionAnimation>, Rc<dyn ICompositionAnimation>)>,
    hide: Option<Rc<dyn ICompositionAnimationBase>>,
    reposition: Vec<Rc<dyn ICompositionAnimationBase>>,
}

fn decelerate(compositor: &Compositor) -> Easing {
    CompositionEasingFunction::create_cubic_bezier_easing_function(
        compositor,
        Vector2::new(0.1, 0.9),
        Vector2::new(0.2, 1.0),
    )
}

fn delay_of(slot: usize) -> Duration {
    STAGGER_STEP * slot as u32
}

/// The delay of a show animation. At least a moment, because the first frame of a key frame
/// animation without a delay is the starting value of the property: the element would show at
/// its place for one frame. Within the delay it holds its first key frame.
fn show_delay_of(slot: usize) -> Duration {
    delay_of(slot) + Duration::from_millis(1)
}

impl Compiled {
    fn build(compositor: &Rc<Compositor>, motion: Motion) -> Self {
        let easing = decelerate(compositor);
        let show = motion.show.map_or_else(Vec::new, |(x, y)| {
            let slots = if motion.stagger_show { STAGGER_SLOTS } else { 1 };
            (0..slots)
                .map(|slot| {
                    let opacity = compositor.create_scalar_key_frame_animation();
                    opacity.set_target(Some("Opacity".to_owned()));
                    opacity.insert_key_frame(0.0, 0.0);
                    opacity.insert_expression_key_frame(1.0, "this.FinalValue", Some(easing.clone()));
                    opacity.set_duration(SHOW_DURATION);
                    opacity.set_delay_time(show_delay_of(slot));
                    opacity.set_delay_behavior(AnimationDelayBehavior::SetInitialValueBeforeDelay);

                    let translation = compositor.create_vector3d_key_frame_animation();
                    translation.set_target(Some("Translation".to_owned()));
                    translation.insert_expression_key_frame(
                        0.0,
                        &format!("this.FinalValue + Vector3({x:?}, {y:?}, 0.0)"),
                        None,
                    );
                    translation.insert_expression_key_frame(1.0, "this.FinalValue", Some(easing.clone()));
                    translation.set_duration(SHOW_DURATION);
                    translation.set_delay_time(show_delay_of(slot));
                    translation.set_delay_behavior(AnimationDelayBehavior::SetInitialValueBeforeDelay);
                    (opacity as Rc<dyn ICompositionAnimation>, translation as Rc<dyn ICompositionAnimation>)
                })
                .collect()
        });
        let hide = motion.hide.then(|| {
            let opacity = compositor.create_scalar_key_frame_animation();
            opacity.set_target(Some("Opacity".to_owned()));
            opacity.insert_expression_key_frame(0.0, "this.StartingValue", None);
            opacity.insert_key_frame(1.0, 0.0);
            opacity.set_duration(HIDE_DURATION);
            let group: Rc<CompositionAnimationGroup> = compositor.create_animation_group();
            group.add(opacity);
            group as Rc<dyn ICompositionAnimationBase>
        });
        let reposition = if motion.reposition {
            let slots = if motion.stagger_reposition { STAGGER_SLOTS } else { 1 };
            (0..slots)
                .map(|slot| {
                    let offset = compositor.create_vector3d_key_frame_animation();
                    offset.set_target(Some(OFFSET.to_owned()));
                    offset.insert_expression_key_frame(1.0, "this.FinalValue", Some(easing.clone()));
                    offset.set_duration(REPOSITION_DURATION);
                    // Within its delay the element stays where it was.
                    offset.set_delay_time(delay_of(slot));
                    offset as Rc<dyn ICompositionAnimationBase>
                })
                .collect()
        } else {
            Vec::new()
        };
        Self { compositor: Rc::downgrade(compositor), motion, show, hide, reposition }
    }

    fn is_on(&self, compositor: &Rc<Compositor>) -> bool {
        std::ptr::eq(self.compositor.as_ptr(), Rc::as_ptr(compositor))
    }
}

fn slot_of<T>(list: &[T], index: usize) -> Option<&T> {
    list.get(index.min(list.len().saturating_sub(1)))
}

fn compositor_of(visual: &Visual) -> Option<Rc<Compositor>> {
    visual.presentation_source().and_then(|source| source.renderer().compositor())
}

/// Which property of the owner a collection was set through.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Slot {
    Children,
    Items,
    Popup,
    Own,
}

struct Target {
    visual: WeakRef<Visual>,
    index: Cell<usize>,
    subscription: Rc<dyn IDisposable>,
}

impl Target {
    fn is(&self, visual: &Visual) -> bool {
        self.visual.upgrade().is_some_and(|known| std::ptr::eq(&*known, visual))
    }
}

ferro_thread_local! {
    /// The elements with a collection of theme transitions.
    static HOSTS: RefCell<Vec<Rc<Host>>> = const { RefCell::new(Vec::new()) };
}

/// One collection on one element, and the elements it animates.
struct Host {
    owner: WeakRef<Visual>,
    slot: Slot,
    collection: TransitionCollection,
    collection_token: Cell<u64>,
    compiled: RefCell<Option<Rc<Compiled>>>,
    targets: RefCell<Vec<Target>>,
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    items_token: Cell<Option<u64>>,
    /// Item containers animate only around a load or a change of the items, not when a
    /// virtualizing panel recycles them while scrolling.
    armed: Cell<bool>,
    arm_count: Cell<u64>,
    disarm_queued: Cell<bool>,
}

impl Host {
    /// Replaces the collection of `owner` in `slot`.
    fn set(owner: &Visual, slot: Slot, collection: Option<TransitionCollection>) {
        let previous = HOSTS.with(|hosts| {
            let mut hosts = hosts.borrow_mut();
            hosts.retain(|host| host.owner.upgrade().is_some());
            let index = hosts
                .iter()
                .position(|host| host.slot == slot && host.owner.upgrade().is_some_and(|known| std::ptr::eq(&*known, owner)));
            index.map(|index| hosts.swap_remove(index))
        });
        if let Some(previous) = previous {
            previous.dispose();
        }
        let Some(collection) = collection else { return };
        let host = Rc::new(Host {
            owner: owner.to_ref().downgrade(),
            slot,
            collection,
            collection_token: Cell::new(0),
            compiled: RefCell::new(None),
            targets: RefCell::new(Vec::new()),
            subscriptions: RefCell::new(Vec::new()),
            items_token: Cell::new(None),
            armed: Cell::new(false),
            arm_count: Cell::new(0),
            disarm_queued: Cell::new(false),
        });
        HOSTS.with(|hosts| hosts.borrow_mut().push(host.clone()));
        host.connect(owner);
    }

    fn connect(self: &Rc<Self>, owner: &Visual) {
        let weak = Rc::downgrade(self);
        let token = self.collection.add_collection_changed(Rc::new(
            move |_: &NotifyCollectionChangedEventArgs<'_, Ref<ThemeTransition>>| {
                if let Some(host) = weak.upgrade() {
                    host.refresh();
                }
            },
        ));
        self.collection_token.set(token);
        for transition in self.collection.snapshot().iter() {
            transition.join(self);
        }

        let weak = Rc::downgrade(self);
        let attached = owner.attached_to_visual_tree(move |_| {
            if let Some(host) = weak.upgrade() {
                host.owner_attached();
            }
        });
        self.subscriptions.borrow_mut().push(attached.detach());

        match self.slot {
            Slot::Children => self.connect_panel(owner),
            Slot::Items => self.connect_items(owner),
            Slot::Popup => self.connect_popup(owner),
            Slot::Own => self.add_target(owner, 0),
        }
    }

    fn connect_panel(self: &Rc<Self>, owner: &Visual) {
        let Some(panel) = owner.downcast_ref::<Panel>() else { return };
        for (index, child) in panel.children().snapshot().iter().enumerate() {
            self.add_target(&child, index);
        }
        let weak = Rc::downgrade(self);
        let children = panel.children();
        let token = children.add_collection_changed(Rc::new(
            move |e: &NotifyCollectionChangedEventArgs<'_, Ref<Control>>| {
                if let Some(host) = weak.upgrade() {
                    host.children_changed(e);
                }
            },
        ));
        let weak_panel = panel.to_ref().downgrade();
        self.subscriptions.borrow_mut().push(ferroui_base::reactive::Disposable::create(move || {
            if let Some(panel) = weak_panel.upgrade() {
                panel.children().remove_collection_changed(token);
            }
        }));
    }

    fn children_changed(self: &Rc<Self>, e: &NotifyCollectionChangedEventArgs<'_, Ref<Control>>) {
        for child in e.old_items {
            self.remove_target(child);
        }
        if e.action == NotifyCollectionChangedAction::Reset {
            self.retain_children();
        }
        let start = e.new_starting_index.max(0) as usize;
        for (offset, child) in e.new_items.iter().enumerate() {
            self.add_target(child, start + offset);
            if child.is_attached_to_visual_tree() {
                self.play(child, start + offset);
            }
        }
    }

    /// After a reset: only the children the panel still has stay targets.
    fn retain_children(&self) {
        let Some(owner) = self.owner.upgrade() else { return };
        let Some(panel) = owner.downcast_ref::<Panel>() else { return };
        let children = panel.children().snapshot();
        let gone: Vec<_> = self
            .targets
            .borrow()
            .iter()
            .filter_map(|target| target.visual.upgrade())
            .filter(|visual| {
                !children.iter().any(|child| {
                    let child: &Visual = child;
                    std::ptr::eq(child, &**visual)
                })
            })
            .collect();
        for visual in gone {
            self.remove_target(&visual);
        }
    }

    fn connect_items(self: &Rc<Self>, owner: &Visual) {
        let Some(items) = owner.downcast_ref::<ItemsControl>() else { return };
        let weak = Rc::downgrade(self);
        let prepared = items.container_prepared(move |e| {
            if let Some(host) = weak.upgrade() {
                host.container_prepared(e.container(), e.index().max(0) as usize);
            }
        });
        let weak = Rc::downgrade(self);
        let clearing = items.container_clearing(move |e| {
            if let Some(host) = weak.upgrade() {
                // A container that leaves while scrolling does not fade.
                if !host.armed.get() {
                    host.remove_target(e.container());
                }
            }
        });
        let weak = Rc::downgrade(self);
        let layout = items.layout_updated(move || {
            if let Some(host) = weak.upgrade() {
                host.queue_disarm();
            }
        });
        // Runs before the panels, which listen to the end of the change.
        let weak = Rc::downgrade(self);
        let token = items.items_view().add_collection_changed(Rc::new(move |_: &ItemsChangedEventArgs<'_>| {
            if let Some(host) = weak.upgrade() {
                host.arm();
            }
        }));
        self.items_token.set(Some(token));
        let mut subscriptions = self.subscriptions.borrow_mut();
        subscriptions.push(prepared.detach());
        subscriptions.push(clearing.detach());
        subscriptions.push(layout.detach());
    }

    fn container_prepared(self: &Rc<Self>, container: &Ref<Control>, index: usize) {
        if !self.armed.get() {
            return;
        }
        self.add_target(container, index);
        self.play(container, index);
    }

    /// The containers animate until the frame after the next layout.
    fn arm(self: &Rc<Self>) {
        self.armed.set(true);
        self.arm_count.set(self.arm_count.get() + 1);
        let Some(owner) = self.owner.upgrade() else { return };
        let Some(items) = owner.downcast_ref::<ItemsControl>() else { return };
        for container in items.get_realized_containers() {
            let index = items.index_from_container(&container).max(0) as usize;
            self.add_target(&container, index);
            self.install_reposition(&container, index);
        }
    }

    fn queue_disarm(self: &Rc<Self>) {
        if !self.armed.get() || self.disarm_queued.replace(true) {
            return;
        }
        let count = self.arm_count.get();
        let weak = Rc::downgrade(self);
        Dispatcher::ui_thread().post_local(
            move || {
                let Some(host) = weak.upgrade() else { return };
                host.disarm_queued.set(false);
                if host.arm_count.get() == count {
                    host.disarm();
                }
            },
            DispatcherPriority::BACKGROUND,
        );
    }

    fn disarm(&self) {
        self.armed.set(false);
        for target in self.targets.borrow().iter() {
            if let Some(visual) = target.visual.upgrade() {
                Self::remove_reposition(&visual);
            }
        }
    }

    fn connect_popup(self: &Rc<Self>, owner: &Visual) {
        let Some(popup) = owner.downcast_ref::<Popup>() else { return };
        let weak = Rc::downgrade(self);
        let weak_popup = popup.to_ref().downgrade();
        let opened = popup.opened(move || {
            let (Some(host), Some(popup)) = (weak.upgrade(), weak_popup.upgrade()) else { return };
            if let Some(child) = popup.child() {
                host.play(&child, 0);
            }
        });
        self.subscriptions.borrow_mut().push(opened.detach());
    }

    fn owner_attached(self: &Rc<Self>) {
        let Some(owner) = self.owner.upgrade() else { return };
        match self.slot {
            Slot::Children => {
                let Some(panel) = owner.downcast_ref::<Panel>() else { return };
                for (index, child) in panel.children().snapshot().iter().enumerate() {
                    self.add_target(&child, index);
                    self.play(&child, index);
                }
            }
            Slot::Items => self.arm(),
            Slot::Popup => {}
            Slot::Own => {
                self.add_target(&owner, 0);
                self.play(&owner, 0);
            }
        }
    }

    fn add_target(self: &Rc<Self>, visual: &Visual, index: usize) {
        {
            let mut targets = self.targets.borrow_mut();
            targets.retain(|target| target.visual.upgrade().is_some());
            if let Some(known) = targets.iter().find(|target| target.is(visual)) {
                known.index.set(index);
            } else {
                let weak = Rc::downgrade(self);
                let weak_visual = visual.to_ref().downgrade();
                let subscription = visual.property_changed(move |e| {
                    let is_bounds = e.property() == Visual::bounds_property().as_property();
                    let is_visible = e.property() == Visual::is_visible_property().as_property();
                    if !is_bounds && !is_visible {
                        return;
                    }
                    let (Some(host), Some(visual)) = (weak.upgrade(), weak_visual.upgrade()) else { return };
                    if is_bounds {
                        let (old, new) = e.get_old_and_new_value::<Rect>();
                        host.bounds_changed(&visual, old, new);
                    } else if e.get_new_value::<bool>() && host.slot != Slot::Items {
                        host.play(&visual, host.index_of(&visual));
                    }
                });
                targets.push(Target { visual: visual.to_ref().downgrade(), index: Cell::new(index), subscription });
            }
        }
        self.install_hide(visual);
    }

    fn remove_target(&self, visual: &Visual) {
        let target = {
            let mut targets = self.targets.borrow_mut();
            targets.iter().position(|target| target.is(visual)).map(|index| targets.swap_remove(index))
        };
        if let Some(target) = target {
            target.subscription.dispose();
            ElementComposition::set_implicit_hide_animation(visual, None);
            Self::remove_reposition(visual);
        }
    }

    fn index_of(&self, visual: &Visual) -> usize {
        self.targets.borrow().iter().find(|target| target.is(visual)).map_or(0, |target| target.index.get())
    }

    /// The compositor of `visual`, or of the owner while `visual` is still being attached: the
    /// owner hears of its attachment before its children are attached.
    fn compositor_for(&self, visual: &Visual) -> Option<Rc<Compositor>> {
        compositor_of(visual).or_else(|| self.owner.upgrade().and_then(|owner| compositor_of(&owner)))
    }

    fn compiled(&self, compositor: &Rc<Compositor>) -> Rc<Compiled> {
        if let Some(compiled) = &*self.compiled.borrow() {
            if compiled.is_on(compositor) {
                return compiled.clone();
            }
        }
        let compiled = Rc::new(Compiled::build(compositor, Motion::of(&self.collection)));
        *self.compiled.borrow_mut() = Some(compiled.clone());
        compiled
    }

    fn install_hide(&self, visual: &Visual) {
        if self.slot == Slot::Popup {
            return;
        }
        let Some(compositor) = self.compositor_for(visual) else { return };
        ElementComposition::set_implicit_hide_animation(visual, self.compiled(&compositor).hide.clone());
    }

    /// Starts the show animation on the composition visual of `visual`, before the next commit.
    fn play(&self, visual: &Visual, index: usize) {
        let Some(compositor) = self.compositor_for(visual) else { return };
        let compiled = self.compiled(&compositor);
        if compiled.motion.show.is_none() {
            return;
        }
        let weak = visual.to_ref().downgrade();
        compositor.request_composition_update(move || {
            let Some(visual) = weak.upgrade() else { return };
            if !visual.is_visible() {
                return;
            }
            let (Some(comp), Some((opacity, translation))) = (visual.composition_visual(), slot_of(&compiled.show, index))
            else {
                return;
            };
            comp.start_animation_with_final_value(
                "Opacity",
                &**opacity,
                Some(ExpressionVariant::create(visual.opacity() as f32)),
            );
            comp.start_animation_with_final_value(
                "Translation",
                &**translation,
                Some(ExpressionVariant::create(visual.translation())),
            );
        });
    }

    fn bounds_changed(&self, visual: &Visual, old: Rect, new: Rect) {
        // The first placement of an element does not animate. Its composition visual is created
        // now, so that the next move starts from a value.
        if old == Rect::default() {
            if self.wants_reposition(visual) {
                let weak = visual.to_ref().downgrade();
                if let Some(compositor) = self.compositor_for(visual) {
                    compositor.request_composition_update(move || {
                        if let Some(visual) = weak.upgrade() {
                            let _ = visual.composition_visual();
                        }
                    });
                }
            }
            return;
        }
        if old.x == new.x && old.y == new.y {
            return;
        }
        self.install_reposition(visual, self.index_of(visual));
    }

    fn wants_reposition(&self, visual: &Visual) -> bool {
        if self.slot == Slot::Items && !self.armed.get() {
            return false;
        }
        self.compositor_for(visual).is_some_and(|compositor| self.compiled(&compositor).motion.reposition)
    }

    /// The implicit animation of `Offset`, in the collection the composition visual already has:
    /// the implicit transitions of an element keep theirs.
    fn install_reposition(&self, visual: &Visual, index: usize) {
        if !self.wants_reposition(visual) {
            return;
        }
        let Some(compositor) = self.compositor_for(visual) else { return };
        let compiled = self.compiled(&compositor);
        let (Some(comp), Some(animation)) = (visual.composition_visual(), slot_of(&compiled.reposition, index)) else {
            return;
        };
        let comp: Rc<CompositionVisual> = (*comp).clone();
        let collection = comp.implicit_animations().unwrap_or_else(|| {
            let collection = compositor.create_implicit_animation_collection();
            comp.set_implicit_animations(Some(collection.clone()));
            collection
        });
        if !collection.contains_key(OFFSET) {
            collection.set(OFFSET, animation.clone());
        }
    }

    fn remove_reposition(visual: &Visual) {
        if !visual.has_composition_visual() {
            return;
        }
        if let Some(collection) = visual.composition_visual().and_then(|comp| comp.implicit_animations()) {
            collection.remove(OFFSET);
        }
    }

    /// The transitions changed: the animations are built again.
    fn refresh(self: &Rc<Self>) {
        *self.compiled.borrow_mut() = None;
        for transition in self.collection.snapshot().iter() {
            transition.join(self);
        }
        let visuals: Vec<_> = self.targets.borrow().iter().filter_map(|target| target.visual.upgrade()).collect();
        for visual in visuals {
            Self::remove_reposition(&visual);
            self.install_hide(&visual);
        }
    }

    fn dispose(&self) {
        self.collection.remove_collection_changed(self.collection_token.get());
        for transition in self.collection.snapshot().iter() {
            transition.leave(self);
        }
        for subscription in self.subscriptions.borrow_mut().drain(..) {
            subscription.dispose();
        }
        if let (Some(token), Some(owner)) = (self.items_token.take(), self.owner.upgrade()) {
            if let Some(items) = owner.downcast_ref::<ItemsControl>() {
                items.items_view().remove_collection_changed(token);
            }
        }
        let targets: Vec<_> = self.targets.borrow_mut().drain(..).collect();
        for target in targets {
            target.subscription.dispose();
            if let Some(visual) = target.visual.upgrade() {
                ElementComposition::set_implicit_hide_animation(&visual, None);
                Self::remove_reposition(&visual);
            }
        }
    }
}
