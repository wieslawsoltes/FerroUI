//! WinUI's `SwipeControl` (`controls/dev/SwipeControl/SwipeControl.cpp`,
//! `SwipeControlInteractionTrackerOwner.cpp`, `SwipeControl.idl` of microsoft-ui-xaml commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md), on FerroUI's composition interactions (docs/porting/winui-calculator-gaps.md,
//! section 6.4).

use super::{SwipeItem, SwipeItems, SwipeMode};
use crate::primitives::{TemplateAppliedEventArgs, TemplatedControlImpl, TemplatedControlImplExt};
use crate::{
    Button, CommandBarButton, ContentControl, ContentControlImpl, Control, ControlImpl, Grid, ListBoxItem, StackPanel,
};
use ferroui_base::controls::ResourceKey;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::input::{InputElement, InputElementImpl, InputElementImplExt, PointerPressedEventArgs, PointerType};
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventHandlerToken};
use ferroui_base::layout::{HorizontalAlignment, LayoutableImpl, VerticalAlignment};
use ferroui_base::media::IBrush;
use ferroui_base::numerics::Vector3;
use ferroui_base::rendering::composition::animations::ExpressionAnimation;
use ferroui_base::rendering::composition::interactions::{
    IInteractionTrackerOwner, InteractionChainingMode, InteractionSourceMode, InteractionTracker,
    InteractionTrackerIdleStateEnteredArgs, InteractionTrackerInertiaModifier, InteractionTrackerInertiaRestingValue,
    InteractionTrackerInertiaStateEnteredArgs, InteractionTrackerInteractingStateEnteredArgs,
    InteractionTrackerValuesChangedArgs, VisualInteractionSource, VisualInteractionSourceRedirectionMode,
};
use ferroui_base::rendering::composition::{AsCompositionObject, ElementComposition, ICompositionObjectAnimations};
use ferroui_base::styling::ControlTheme;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElementImpl, StyledProperty,
    VisualImpl, WeakRef,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// WinUI's `c_epsilon`.
const EPSILON: f32 = 0.0001;
/// WinUI's `c_ThresholdValue`: the distance past which items open (or execute).
const THRESHOLD_VALUE: f32 = 100.0;
/// WinUI's `c_MinimumCloseVelocity`.
const MINIMUM_CLOSE_VELOCITY: f32 = 31.0;
/// WinUI's swipe animation of the content: a side that is open, or whose opposite side is
/// blocked, moves no further than its items; a side without items does not move.
const SWIPE_EXPRESSION: &str = concat!(
    "isHorizontal ? ",
    "Vector3(tracker.isFarOpen || tracker.blockNearContent ? Clamp(-tracker.Position.X, -this.Target.Size.X, 0) : ",
    "tracker.isNearOpen || tracker.blockFarContent ? Clamp(-tracker.Position.X, 0, this.Target.Size.X) : ",
    "Clamp(-tracker.Position.X, (tracker.hasRightContent ? -10000 : 0), ",
    "(tracker.hasLeftContent ? 10000 : 0)), 0, 0) : ",
    "Vector3(0, tracker.isFarOpen || tracker.blockNearContent ? Clamp(-tracker.Position.Y, -this.Target.Size.Y, 0) : ",
    "tracker.isNearOpen || tracker.blockFarContent ? Clamp(-tracker.Position.Y, 0, this.Target.Size.Y) : ",
    "Clamp(-tracker.Position.Y, (tracker.hasBottomContent ? -10000 : 0), ",
    "(tracker.hasTopContent ? 10000 : 0)), 0)",
);
/// The style of the buttons of the items, a resource of the theme.
const SWIPE_ITEM_STYLE: &str = "SwipeItemStyle";

/// The side whose items are shown (WinUI's `CreatedContent`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum CreatedContent {
    #[default]
    None,
    Left,
    Top,
    Right,
    Bottom,
}

/// The state of the interaction (the members of WinUI's `SwipeControl`).
#[derive(Default)]
struct SwipeState {
    is_horizontal: bool,
    created_content: CreatedContent,
    current_items: Option<Ref<SwipeItems>>,
    is_open: bool,
    is_idle: bool,
    is_interacting: bool,
    last_action_was_closing: bool,
    last_action_was_opening: bool,
    block_near_content: bool,
    block_far_content: bool,
    threshold_reached: bool,
}

/// The template parts.
#[derive(Default)]
struct Parts {
    root_grid: Option<Ref<Grid>>,
    swipe_content_root: Option<Ref<Grid>>,
    swipe_content_stack_panel: Option<Ref<StackPanel>>,
    content_root: Option<Ref<Grid>>,
    input_eater: Option<Ref<Grid>>,
}

/// The composition objects of the interaction.
struct Interaction {
    tracker: Rc<InteractionTracker>,
    source: Rc<VisualInteractionSource>,
}

ferroui_base::ferro_thread_local! {
    // WinUI's `s_lastInteractedWithSwipeControl`: one swipe control is open at a time.
    static LAST_INTERACTED: RefCell<Option<WeakRef<SwipeControl>>> = const { RefCell::new(None) };
}

/// A container that reveals commands when its content is swiped (WinUI's `SwipeControl`).
///
/// [`left_items`](Self::left_items) and [`right_items`](Self::right_items) open with a
/// horizontal swipe, [`top_items`](Self::top_items) and [`bottom_items`](Self::bottom_items) with
/// a vertical one (a control has items of one direction). [`SwipeMode::Reveal`] items stay open
/// for the user to invoke one; [`SwipeMode::Execute`] items invoke their single item when the swipe
/// passes the threshold. One swipe control is open at a time; [`close`](Self::close) closes it.
///
/// The interaction is WinUI's: an `InteractionTracker` with a `VisualInteractionSource` (on the
/// nearest `ListBoxItem` within five parents, else the control), redirected for touch and pen
/// (decision D7: no mouse drag), with the resting values of WinUI's `ConfigurePositionInertiaRestingValues`;
/// the content follows the tracker through an expression animation of its `Translation`, on the
/// compositor. The template parts are `RootGrid`, `SwipeContentRoot`, `SwipeContentStackPanel`,
/// `ContentRoot` and `InputEater`; each item is a `CommandBarButton` with the theme's
/// `SwipeItemStyle`.
#[repr(C)]
pub struct SwipeControl {
    base: ContentControl,
    state: RefCell<SwipeState>,
    parts: RefCell<Parts>,
    interaction: RefCell<Option<Interaction>>,
    item_clicks: RefCell<Vec<(WeakRef<Button>, RoutedEventHandlerToken)>>,
    template_applied: Cell<bool>,
}

ferro_class!(SwipeControl: ContentControl);
ferro_class_info!(SwipeControl {
    new: SwipeControl::new,
    markup: {
        namespace: "FerroUI.Controls",
        content: Content,
        attributes: [
            TemplatePart("RootGrid", type(Ref<Grid>)),
            TemplatePart("SwipeContentRoot", type(Ref<Grid>)),
            TemplatePart("SwipeContentStackPanel", type(Ref<StackPanel>)),
            TemplatePart("ContentRoot", type(Ref<Grid>)),
            TemplatePart("InputEater", type(Ref<Grid>)),
        ],
    },
});
ferro_impl_classes!(SwipeControl: StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl, ContentControlImpl);

ferro_properties! { impl SwipeControl {
    /// Defines the `LeftItems` property: the items revealed by swiping to the right.
    pub fn left_items_property() -> StyledProperty<Option<Ref<SwipeItems>>> {
        FerroProperty::register::<SwipeControl, _>("LeftItems", None)
    }

    /// Defines the `RightItems` property: the items revealed by swiping to the left.
    pub fn right_items_property() -> StyledProperty<Option<Ref<SwipeItems>>> {
        FerroProperty::register::<SwipeControl, _>("RightItems", None)
    }

    /// Defines the `TopItems` property: the items revealed by swiping down.
    pub fn top_items_property() -> StyledProperty<Option<Ref<SwipeItems>>> {
        FerroProperty::register::<SwipeControl, _>("TopItems", None)
    }

    /// Defines the `BottomItems` property: the items revealed by swiping up.
    pub fn bottom_items_property() -> StyledProperty<Option<Ref<SwipeItems>>> {
        FerroProperty::register::<SwipeControl, _>("BottomItems", None)
    }
} }

impl FerroObjectImpl for SwipeControl {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        let property = change.property();
        let horizontal = property == Self::left_items_property().as_property()
            || property == Self::right_items_property().as_property();
        let vertical = property == Self::top_items_property().as_property()
            || property == Self::bottom_items_property().as_property();
        if horizontal || vertical {
            // WinUI's `On*ItemsCollectionChanged`.
            this.check_direction(horizontal);
            this.update_has_content();
            this.close_without_animation();
        }
    }
}

impl ControlImpl for SwipeControl {}

impl TemplatedControlImpl for SwipeControl {
    fn on_apply_template(this: &Self, args: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, args);
        let scope = args.name_scope();
        {
            let mut parts = this.parts.borrow_mut();
            parts.root_grid = scope.find_as::<Grid>("RootGrid");
            parts.swipe_content_root = scope.find_as::<Grid>("SwipeContentRoot");
            parts.swipe_content_stack_panel = scope.find_as::<StackPanel>("SwipeContentStackPanel");
            parts.content_root = scope.find_as::<Grid>("ContentRoot");
            parts.input_eater = scope.find_as::<Grid>("InputEater");
        }
        *this.interaction.borrow_mut() = None;
        this.template_applied.set(true);
        {
            let mut state = this.state.borrow_mut();
            state.is_idle = true;
            state.created_content = CreatedContent::None;
            state.current_items = None;
        }
        this.ensure_interaction();
    }
}

impl InputElementImpl for SwipeControl {
    /// WinUI's `OnPointerPressedEvent`: touch (and pen) are redirected to the interaction tracker.
    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        Self::parent_on_pointer_pressed(this, e);
        if !matches!(e.pointer().type_(), PointerType::Touch | PointerType::Pen) {
            return;
        }
        this.ensure_interaction();
        let source = this.interaction.borrow().as_ref().map(|interaction| interaction.source.clone());
        if let Some(source) = source {
            source.try_redirect_for_manipulation(e);
        }
    }
}

impl SwipeControl {
    pub fn construct() -> Self {
        Self {
            base: ContentControl::construct(),
            state: RefCell::new(SwipeState { is_horizontal: true, is_idle: true, ..SwipeState::default() }),
            parts: RefCell::new(Parts::default()),
            interaction: RefCell::new(None),
            item_clicks: RefCell::new(Vec::new()),
            template_applied: Cell::new(false),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn left_items(&self) -> Option<Ref<SwipeItems>> {
        self.get_value(Self::left_items_property())
    }

    pub fn set_left_items(&self, value: Option<Ref<SwipeItems>>) {
        self.set_value(Self::left_items_property(), value);
    }

    pub fn right_items(&self) -> Option<Ref<SwipeItems>> {
        self.get_value(Self::right_items_property())
    }

    pub fn set_right_items(&self, value: Option<Ref<SwipeItems>>) {
        self.set_value(Self::right_items_property(), value);
    }

    pub fn top_items(&self) -> Option<Ref<SwipeItems>> {
        self.get_value(Self::top_items_property())
    }

    pub fn set_top_items(&self, value: Option<Ref<SwipeItems>>) {
        self.set_value(Self::top_items_property(), value);
    }

    pub fn bottom_items(&self) -> Option<Ref<SwipeItems>> {
        self.get_value(Self::bottom_items_property())
    }

    pub fn set_bottom_items(&self, value: Option<Ref<SwipeItems>>) {
        self.set_value(Self::bottom_items_property(), value);
    }

    /// Whether the items of a side are open.
    pub fn is_open(&self) -> bool {
        self.state.borrow().is_open
    }

    /// Whether the interaction is at rest.
    pub fn is_idle(&self) -> bool {
        self.state.borrow().is_idle
    }

    /// The swipe control the user interacted with last (WinUI's test hook
    /// `GetLastInteractedWithSwipeControl`).
    pub fn last_interacted_with_swipe_control() -> Option<Ref<SwipeControl>> {
        LAST_INTERACTED.with(|last| last.borrow().as_ref().and_then(WeakRef::upgrade))
    }

    /// The side whose items are shown.
    #[cfg(test)]
    pub(crate) fn created_content(&self) -> CreatedContent {
        self.state.borrow().created_content
    }

    /// The items of the side that is shown.
    pub fn current_items(&self) -> Option<Ref<SwipeItems>> {
        self.state.borrow().current_items.clone()
    }

    /// Whether the swipe has passed the threshold (execute items change their colours then).
    pub fn is_threshold_reached(&self) -> bool {
        self.state.borrow().threshold_reached
    }

    /// Closes the open items, animated (WinUI's `Close`).
    pub fn close(&self) {
        let (is_open, closing, interacting, idle, created) = {
            let state = self.state.borrow();
            (state.is_open, state.last_action_was_closing, state.is_interacting, state.is_idle, state.created_content)
        };
        if !is_open || closing || interacting {
            return;
        }
        self.state.borrow_mut().last_action_was_closing = true;
        let tracker = self.tracker();
        let Some(tracker) = tracker else {
            // No compositor: the items close at once, as at rest.
            self.idle_state_entered(Vector3::new(0.0, 0.0, 0.0));
            return;
        };
        let (width, height) = self.stack_panel_size();
        if !idle {
            let initial = match created {
                CreatedContent::Left => Vector3::new(-width, 0.0, 0.0),
                CreatedContent::Top => Vector3::new(0.0, -height, 0.0),
                CreatedContent::Right => Vector3::new(width, 0.0, 0.0),
                CreatedContent::Bottom => Vector3::new(0.0, height, 0.0),
                CreatedContent::None => Vector3::new(0.0, 0.0, 0.0),
            };
            tracker.try_update_position(initial);
        }
        let velocity = match created {
            CreatedContent::Left => Vector3::new(MINIMUM_CLOSE_VELOCITY, 0.0, 0.0),
            CreatedContent::Top => Vector3::new(0.0, MINIMUM_CLOSE_VELOCITY, 0.0),
            CreatedContent::Right => Vector3::new(-MINIMUM_CLOSE_VELOCITY, 0.0, 0.0),
            CreatedContent::Bottom => Vector3::new(0.0, -MINIMUM_CLOSE_VELOCITY, 0.0),
            CreatedContent::None => Vector3::new(0.0, 0.0, 0.0),
        };
        tracker.try_update_position_with_additional_velocity(velocity);
    }

    fn tracker(&self) -> Option<Rc<InteractionTracker>> {
        self.interaction.borrow().as_ref().map(|interaction| interaction.tracker.clone())
    }

    fn set_tracker_bool(&self, name: &str, value: bool) {
        if let Some(tracker) = self.tracker() {
            tracker.properties().insert_boolean(name, value);
        }
    }

    fn stack_panel_size(&self) -> (f32, f32) {
        self.parts
            .borrow()
            .swipe_content_stack_panel
            .as_ref()
            .map(|panel| {
                let bounds = panel.bounds();
                (bounds.width as f32, bounds.height as f32)
            })
            .unwrap_or((0.0, 0.0))
    }

    fn items_of(&self, side: CreatedContent) -> Option<Ref<SwipeItems>> {
        match side {
            CreatedContent::Left => self.left_items(),
            CreatedContent::Right => self.right_items(),
            CreatedContent::Top => self.top_items(),
            CreatedContent::Bottom => self.bottom_items(),
            CreatedContent::None => None,
        }
    }

    fn has_items(&self, side: CreatedContent) -> bool {
        self.items_of(side).is_some_and(|items| items.count() > 0)
    }

    /// WinUI's `ThrowIfHasVerticalAndHorizontalContent`: a control has items of one direction; the
    /// direction follows the items set last (WinUI throws when both are set).
    fn check_direction(&self, horizontal: bool) {
        let has_horizontal = self.has_items(CreatedContent::Left) || self.has_items(CreatedContent::Right);
        let has_vertical = self.has_items(CreatedContent::Top) || self.has_items(CreatedContent::Bottom);
        if has_horizontal && has_vertical {
            warn("SwipeControl: the control has both horizontal and vertical items; only one direction is supported.");
        }
        let is_horizontal = if has_horizontal != has_vertical { has_horizontal } else { horizontal };
        let changed = self.state.borrow().is_horizontal != is_horizontal;
        self.state.borrow_mut().is_horizontal = is_horizontal;
        if changed {
            // The rails of the interaction change with the direction.
            *self.interaction.borrow_mut() = None;
            self.ensure_interaction();
        }
    }

    fn update_has_content(&self) {
        self.set_tracker_bool("hasLeftContent", self.has_items(CreatedContent::Left));
        self.set_tracker_bool("hasRightContent", self.has_items(CreatedContent::Right));
        self.set_tracker_bool("hasTopContent", self.has_items(CreatedContent::Top));
        self.set_tracker_bool("hasBottomContent", self.has_items(CreatedContent::Bottom));
    }

    // --- the interaction tracker (WinUI's `InitializeInteractionTracker`) ---

    /// The element whose visual is the source of the interaction: the nearest list item within five
    /// parents (WinUI looks for the `ListViewItemPresenter`), else the control.
    fn interaction_source_element(&self) -> Ref<InputElement> {
        let mut current = self.visual_parent();
        for _ in 0..5 {
            let Some(element) = current else {
                break;
            };
            if let Some(item) = element.cast::<ListBoxItem>() {
                return item.upcast();
            }
            current = element.visual_parent();
        }
        self.to_ref().upcast()
    }

    /// Creates the interaction when the control is composited and has its template.
    fn ensure_interaction(&self) {
        if !self.template_applied.get() || self.interaction.borrow().is_some() {
            return;
        }
        let root = self.parts.borrow().root_grid.clone();
        let Some(root_visual) = root.as_ref().and_then(|root| ElementComposition::get_element_visual(root)) else {
            return;
        };
        let element = self.interaction_source_element();
        let Some(element_visual) = ElementComposition::get_element_visual(&element) else {
            return;
        };
        let compositor = root_visual.compositor().clone();
        let horizontal = self.state.borrow().is_horizontal;
        let source = VisualInteractionSource::create_for_element(&element_visual, &element);
        source.set_is_position_x_rails_enabled(horizontal);
        source.set_is_position_y_rails_enabled(!horizontal);
        source.set_manipulation_redirection_mode(VisualInteractionSourceRedirectionMode::CapableTouchpadOnly);
        source.set_position_x_source_mode(if horizontal {
            InteractionSourceMode::EnabledWithInertia
        } else {
            InteractionSourceMode::Disabled
        });
        source.set_position_y_source_mode(if horizontal {
            InteractionSourceMode::Disabled
        } else {
            InteractionSourceMode::EnabledWithInertia
        });
        if horizontal {
            source.set_position_x_chaining_mode(InteractionChainingMode::Never);
        } else {
            source.set_position_y_chaining_mode(InteractionChainingMode::Never);
        }
        let owner: Rc<dyn IInteractionTrackerOwner> = Rc::new(TrackerOwner(self.to_ref().downgrade()));
        let tracker = InteractionTracker::create_with_owner(&compositor, owner);
        tracker.interaction_sources().add(source.clone());
        for name in ["isFarOpen", "isNearOpen", "blockNearContent", "blockFarContent"] {
            tracker.properties().insert_boolean(name, false);
        }
        tracker.set_max_position(Vector3::new(f32::INFINITY, f32::INFINITY, 0.0));
        tracker.set_min_position(Vector3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, 0.0));
        *self.interaction.borrow_mut() = Some(Interaction { tracker: tracker.clone(), source });
        self.update_has_content();

        // The content follows the tracker (WinUI's swipe animation): a side that is open, or whose
        // opposite side is blocked, moves no further than its items; a side without items does not
        // move.
        let content = self.parts.borrow().content_root.clone();
        if let Some(content) = content {
            ElementComposition::set_is_translation_enabled(&content, true);
            if let Some(content_visual) = ElementComposition::get_element_visual(&content) {
                let animation = compositor.create_expression_animation_with(SWIPE_EXPRESSION);
                let reference: Rc<dyn AsCompositionObject> = tracker.clone();
                animation.set_reference_parameter("tracker", reference);
                animation.set_boolean_parameter("isHorizontal", horizontal);
                content_visual.start_animation("Translation", &*animation);
            }
        }
        self.configure_resting_values();
    }

    /// WinUI's `ConfigurePositionInertiaRestingValues`: past the threshold (or the size of the
    /// items when they are open) the tracker rests open at the size of the items, else closed.
    fn configure_resting_values(&self) {
        let Some(tracker) = self.tracker() else {
            return;
        };
        let compositor = tracker.compositor().clone();
        let horizontal = self.state.borrow().is_horizontal;
        let (width, height) = self.stack_panel_size();
        let size = if horizontal { width } else { height };
        let (near, far, axis) = if horizontal {
            ("hasLeftContent", "hasRightContent", "x")
        } else {
            ("hasTopContent", "hasBottomContent", "y")
        };
        let resting = |condition: String, value: &str| {
            let condition_animation: Rc<ExpressionAnimation> = compositor.create_expression_animation_with(&condition);
            condition_animation.set_scalar_parameter("swipeContentSize", size);
            condition_animation.set_scalar_parameter("maxThreshold", THRESHOLD_VALUE);
            let value_animation = compositor.create_expression_animation_with(value);
            value_animation.set_scalar_parameter("swipeContentSize", size);
            let modifier = InteractionTrackerInertiaRestingValue::create(&compositor);
            modifier.set_condition(Some(condition_animation));
            modifier.set_resting_value(Some(value_animation));
            InteractionTrackerInertiaModifier::RestingValue(modifier)
        };
        let near_open = resting(
            format!(
                "this.Target.{near} && !this.Target.isFarOpen && this.Target.NaturalRestingPosition.{axis} <= \
                 -1 * (this.Target.isNearOpen ? swipeContentSize : min(swipeContentSize, maxThreshold))"
            ),
            "-swipeContentSize",
        );
        let far_open = resting(
            format!(
                "this.Target.{far} && !this.Target.isNearOpen && this.Target.NaturalRestingPosition.{axis} >= \
                 (this.Target.isFarOpen ? swipeContentSize : min(swipeContentSize, maxThreshold))"
            ),
            "swipeContentSize",
        );
        let neutral = resting("true".to_string(), "0");
        let modifiers = [near_open, far_open, neutral];
        if horizontal {
            tracker.configure_position_x_inertia_modifiers(modifiers);
        } else {
            tracker.configure_position_y_inertia_modifiers(modifiers);
        }
    }

    // --- the states of the tracker (WinUI's `IInteractionTrackerOwner` handlers) ---

    /// WinUI's `IdleStateEntered`: at rest; open items of execute mode invoke their item, closed
    /// items are taken away. `position` is the position of the tracker.
    pub(crate) fn idle_state_entered(&self, position: Vector3) {
        {
            self.state.borrow_mut().is_interacting = false;
        }
        self.update_is_open(position != Vector3::new(0.0, 0.0, 0.0));
        let (is_open, items) = {
            let state = self.state.borrow();
            (state.is_open, state.current_items.clone())
        };
        if is_open {
            if let Some(items) = items.filter(|items| items.mode() == SwipeMode::Execute) {
                if let Some(item) = items.get(0) {
                    item.invoke_swipe(&self.to_ref());
                }
            }
        } else {
            self.clear_content();
            let mut state = self.state.borrow_mut();
            state.current_items = None;
            state.created_content = CreatedContent::None;
        }
        self.state.borrow_mut().is_idle = true;
    }

    /// WinUI's `InteractingStateEntered`: the user swipes.
    pub(crate) fn interacting_state_entered(&self) {
        let was_open = {
            let mut state = self.state.borrow_mut();
            state.is_idle = false;
            state.last_action_was_closing = false;
            state.last_action_was_opening = false;
            state.is_interacting = true;
            state.is_open
        };
        // Once the user starts with the control closed, both sides may open.
        if !was_open {
            {
                let mut state = self.state.borrow_mut();
                state.block_near_content = false;
                state.block_far_content = false;
            }
            self.set_tracker_bool("blockNearContent", false);
            self.set_tracker_bool("blockFarContent", false);
        }
    }

    /// WinUI's `InertiaStateEntered`: the swipe was released; `position` is the position of the
    /// tracker and `resting` the position it comes to rest at.
    pub(crate) fn inertia_state_entered(&self, position: Vector3, resting: Vector3) {
        let horizontal = {
            let mut state = self.state.borrow_mut();
            state.is_interacting = false;
            state.is_idle = false;
            state.is_horizontal
        };
        // A flick from one side to the other closes the control.
        let opposite = if horizontal { position.x * resting.x < 0.0 } else { position.y * resting.y < 0.0 };
        if opposite {
            self.close_without_animation();
            return;
        }
        self.update_is_open(resting != Vector3::new(0.0, 0.0, 0.0));
        let (is_open, created) = {
            let state = self.state.borrow();
            (state.is_open, state.created_content)
        };
        if is_open {
            let (near, far) = match created {
                CreatedContent::Bottom | CreatedContent::Right => (true, false),
                CreatedContent::Top | CreatedContent::Left => (false, true),
                CreatedContent::None => (false, false),
            };
            {
                let mut state = self.state.borrow_mut();
                state.block_near_content = near;
                state.block_far_content = far;
            }
            self.set_tracker_bool("blockNearContent", near);
            self.set_tracker_bool("blockFarContent", far);
        }
    }

    /// WinUI's `ValuesChanged`: the content of the side the swipe goes to is created, the last
    /// control the user swiped closes, and the threshold is followed.
    pub(crate) fn values_changed(&self, position: Vector3) {
        let interacting = self.state.borrow().is_interacting;
        if interacting {
            let this = self.to_ref();
            let last = Self::last_interacted_with_swipe_control();
            if last.as_ref() != Some(&this) {
                if let Some(last) = last {
                    last.close_if_not_remain_open_execute_item();
                }
                LAST_INTERACTED.with(|slot| *slot.borrow_mut() = Some(this.downgrade()));
            }
        }
        let (horizontal, block_near, block_far, created) = {
            let state = self.state.borrow();
            (state.is_horizontal, state.block_near_content, state.block_far_content, state.created_content)
        };
        let value = if horizontal { position.x } else { position.y };
        let (near, far) = if horizontal {
            (CreatedContent::Left, CreatedContent::Right)
        } else {
            (CreatedContent::Top, CreatedContent::Bottom)
        };
        if !block_near && created != near && value < -EPSILON {
            self.create_content(near);
        } else if !block_far && created != far && value > EPSILON {
            self.create_content(far);
        }
        self.update_threshold_reached(value);
    }

    /// WinUI's `CloseWithoutAnimation`.
    fn close_without_animation(&self) {
        let was_idle = self.state.borrow().is_idle;
        if let Some(tracker) = self.tracker() {
            tracker.try_update_position(Vector3::new(0.0, 0.0, 0.0));
        }
        if was_idle {
            self.idle_state_entered(Vector3::new(0.0, 0.0, 0.0));
        }
    }

    /// WinUI's `CloseIfNotRemainOpenExecuteItem`.
    fn close_if_not_remain_open_execute_item(&self) {
        let (is_open, items) = {
            let state = self.state.borrow();
            (state.is_open, state.current_items.clone())
        };
        let remain_open = items.is_some_and(|items| {
            items.mode() == SwipeMode::Execute
                && items
                    .get(0)
                    .is_some_and(|item| item.behavior_on_invoked() == super::SwipeBehaviorOnInvoked::RemainOpen)
        });
        if remain_open && is_open {
            return;
        }
        self.close();
    }

    /// WinUI's `UpdateIsOpen`.
    fn update_is_open(&self, is_open: bool) {
        let (was_open, created) = {
            let state = self.state.borrow();
            (state.is_open, state.created_content)
        };
        if is_open && !was_open {
            {
                let mut state = self.state.borrow_mut();
                state.is_open = true;
                state.last_action_was_opening = true;
            }
            let (far, near) = match created {
                CreatedContent::Right | CreatedContent::Bottom => (true, false),
                CreatedContent::Left | CreatedContent::Top => (false, true),
                CreatedContent::None => (false, false),
            };
            self.set_tracker_bool("isFarOpen", far);
            self.set_tracker_bool("isNearOpen", near);
        } else if !is_open && was_open {
            {
                let mut state = self.state.borrow_mut();
                state.is_open = false;
                state.last_action_was_closing = true;
            }
            self.set_tracker_bool("isFarOpen", false);
            self.set_tracker_bool("isNearOpen", false);
        }
    }

    /// WinUI's `UpdateThresholdReached`.
    fn update_threshold_reached(&self, value: f32) {
        let (horizontal, is_open, opening, old) = {
            let state = self.state.borrow();
            (state.is_horizontal, state.is_open, state.last_action_was_opening, state.threshold_reached)
        };
        let (width, height) = self.stack_panel_size();
        let effective = (if horizontal { width } else { height }) - 1.0;
        let reached = if !is_open || opening {
            value.abs() > effective.min(THRESHOLD_VALUE)
        } else {
            value.abs() < effective
        };
        self.state.borrow_mut().threshold_reached = reached;
        if reached != old {
            self.update_colors();
        }
    }

    // --- the content of the items (WinUI's `CreateContent`) ---

    fn clear_content(&self) {
        for (button, token) in self.item_clicks.borrow_mut().drain(..) {
            if let Some(button) = button.upgrade() {
                button.remove_handler(&Button::click_event(), token);
            }
        }
        let parts = self.parts.borrow();
        if let Some(panel) = &parts.swipe_content_stack_panel {
            panel.set_background(None);
            panel.children().clear();
        }
        if let Some(root) = &parts.swipe_content_root {
            root.set_background(None);
        }
    }

    /// Shows the items of `side` (WinUI's `CreateLeftContent` and its siblings).
    fn create_content(&self, side: CreatedContent) {
        self.clear_content();
        let items = self.items_of(side);
        {
            let mut state = self.state.borrow_mut();
            state.created_content = side;
            state.current_items = items.clone();
        }
        let Some(items) = items else {
            return;
        };
        self.align_stack_panel(&items, side);
        self.populate_content_items(&items);
        self.update_colors();
        self.configure_resting_values();
    }

    /// WinUI's `AlignStackPanel`.
    fn align_stack_panel(&self, items: &SwipeItems, side: CreatedContent) {
        let Some(panel) = self.parts.borrow().swipe_content_stack_panel.clone() else {
            return;
        };
        if items.count() == 0 {
            return;
        }
        let horizontal = self.state.borrow().is_horizontal;
        panel.set_orientation(if horizontal {
            ferroui_base::layout::Orientation::Horizontal
        } else {
            ferroui_base::layout::Orientation::Vertical
        });
        match (items.mode(), horizontal) {
            (SwipeMode::Execute, true) => {
                panel.set_horizontal_alignment(HorizontalAlignment::Stretch);
                panel.set_vertical_alignment(VerticalAlignment::Center);
            }
            (SwipeMode::Execute, false) => {
                panel.set_horizontal_alignment(HorizontalAlignment::Center);
                panel.set_vertical_alignment(VerticalAlignment::Stretch);
            }
            (SwipeMode::Reveal, true) => {
                panel.set_horizontal_alignment(match side {
                    CreatedContent::Left => HorizontalAlignment::Left,
                    CreatedContent::Right => HorizontalAlignment::Right,
                    _ => HorizontalAlignment::Stretch,
                });
                panel.set_vertical_alignment(VerticalAlignment::Center);
            }
            (SwipeMode::Reveal, false) => {
                panel.set_horizontal_alignment(HorizontalAlignment::Center);
                panel.set_vertical_alignment(match side {
                    CreatedContent::Top => VerticalAlignment::Top,
                    CreatedContent::Bottom => VerticalAlignment::Bottom,
                    _ => VerticalAlignment::Stretch,
                });
            }
        }
    }

    /// WinUI's `PopulateContentItems` and `SwipeItem::GenerateControl`: a button per item.
    fn populate_content_items(&self, items: &SwipeItems) {
        let Some(panel) = self.parts.borrow().swipe_content_stack_panel.clone() else {
            return;
        };
        let style = self.swipe_item_style();
        for item in items.items().snapshot().iter() {
            let button = self.swipe_item_button(item, style.clone());
            panel.children().add(button);
        }
    }

    fn swipe_item_style(&self) -> Option<Ref<ControlTheme>> {
        self.try_find_resource(&ResourceKey::from(SWIPE_ITEM_STYLE), self.actual_theme_variant().as_ref())
            .flatten()
            .and_then(|value| ValueTypes::try_cast(&value, ValueType::of::<Option<Ref<ControlTheme>>>()))
            .and_then(|value| value.downcast_ref::<Option<Ref<ControlTheme>>>().cloned().flatten())
    }

    fn swipe_item_button(&self, item: &Ref<SwipeItem>, style: Option<Ref<ControlTheme>>) -> Ref<CommandBarButton> {
        let button = CommandBarButton::new();
        if let Some(style) = style {
            button.set_theme(style);
        }
        if let Some(background) = item.background() {
            button.set_background(Some(background));
        }
        if let Some(foreground) = item.foreground() {
            button.set_foreground(Some(foreground));
        }
        if let Some(icon) = item.icon_source().and_then(|source| source.create_icon_element()) {
            button.set_icon(Some(Control::boxed(icon)));
        }
        let text = item.text();
        button.set_label(Some(&text));
        let weak_item = item.downgrade();
        let weak_this = self.to_ref().downgrade();
        let token = button.click(move |_, _| {
            if let (Some(item), Some(this)) = (weak_item.upgrade(), weak_this.upgrade()) {
                item.invoke_swipe(&this);
            }
        });
        self.item_clicks.borrow_mut().push((button.clone().upcast::<Button>().downgrade(), token));
        button
    }

    /// WinUI's `UpdateColors`: the backgrounds of reveal items, and of an execute item before and
    /// after the threshold.
    fn update_colors(&self) {
        let (items, threshold_reached) = {
            let state = self.state.borrow();
            (state.current_items.clone(), state.threshold_reached)
        };
        let Some(items) = items else {
            return;
        };
        let parts = self.parts.borrow();
        let (Some(panel), Some(root)) = (&parts.swipe_content_stack_panel, &parts.swipe_content_root) else {
            return;
        };
        let resource = |key: &str| -> Option<Rc<dyn IBrush>> {
            self.try_find_resource(&ResourceKey::from(key), self.actual_theme_variant().as_ref())
                .flatten()
                .and_then(|value| ValueTypes::try_cast(&value, ValueType::of::<Option<Rc<dyn IBrush>>>()))
                .and_then(|value| value.downcast_ref::<Option<Rc<dyn IBrush>>>().cloned().flatten())
        };
        match items.mode() {
            SwipeMode::Execute => {
                let item = items.get(0);
                let background = if threshold_reached {
                    item.as_ref()
                        .and_then(|item| item.background())
                        .or_else(|| resource("SwipeItemPostThresholdExecuteBackground"))
                } else {
                    resource("SwipeItemPreThresholdExecuteBackground")
                };
                let foreground = if threshold_reached {
                    item.as_ref()
                        .and_then(|item| item.foreground())
                        .or_else(|| resource("SwipeItemPostThresholdExecuteForeground"))
                } else {
                    resource("SwipeItemPreThresholdExecuteForeground")
                };
                root.set_background(background.clone());
                panel.set_background(background);
                for child in panel.children().snapshot().iter() {
                    if let Some(button) = child.cast::<CommandBarButton>() {
                        button.set_background(None);
                        button.set_foreground(foreground.clone());
                    }
                }
            }
            SwipeMode::Reveal => {
                let background = resource("SwipeItemBackground");
                let foreground = resource("SwipeItemForeground");
                // The root takes the background of the item nearest the content.
                let nearest = items.get(items.count().saturating_sub(1)).and_then(|item| item.background());
                root.set_background(nearest.or_else(|| background.clone()));
                panel.set_background(None);
                for (index, child) in panel.children().snapshot().iter().enumerate() {
                    let Some(button) = child.cast::<CommandBarButton>() else {
                        continue;
                    };
                    let item = items.get(index);
                    if item.as_ref().and_then(|item| item.background()).is_none() {
                        button.set_background(background.clone());
                    }
                    if item.as_ref().and_then(|item| item.foreground()).is_none() {
                        button.set_foreground(foreground.clone());
                    }
                }
            }
        }
    }
}

/// The owner of the tracker (WinUI's `SwipeControlInteractionTrackerOwner`): forwards the states
/// to the control, which it holds weakly.
struct TrackerOwner(WeakRef<SwipeControl>);

impl IInteractionTrackerOwner for TrackerOwner {
    fn idle_state_entered(&self, sender: &Rc<InteractionTracker>, _args: &InteractionTrackerIdleStateEnteredArgs) {
        if let Some(control) = self.0.upgrade() {
            control.idle_state_entered(sender.position());
        }
    }

    fn interacting_state_entered(
        &self,
        _sender: &Rc<InteractionTracker>,
        _args: &InteractionTrackerInteractingStateEnteredArgs,
    ) {
        if let Some(control) = self.0.upgrade() {
            control.interacting_state_entered();
        }
    }

    fn inertia_state_entered(&self, sender: &Rc<InteractionTracker>, args: &InteractionTrackerInertiaStateEnteredArgs) {
        if let Some(control) = self.0.upgrade() {
            let resting = args.modified_resting_position.unwrap_or(args.natural_resting_position);
            control.inertia_state_entered(sender.position(), resting);
        }
    }

    fn values_changed(&self, _sender: &Rc<InteractionTracker>, args: &InteractionTrackerValuesChangedArgs) {
        if let Some(control) = self.0.upgrade() {
            control.values_changed(args.position);
        }
    }
}

/// Logs a warning of the swipe control (area `Control`).
pub(crate) fn warn(message: &str) {
    use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
    if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::CONTROL) {
        logger.log(None, message);
    }
}
