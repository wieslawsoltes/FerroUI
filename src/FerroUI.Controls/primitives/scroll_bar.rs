use super::{
    RangeBase, ScrollBarVisibility, ScrollEventType, TemplateAppliedEventArgs, TemplatedControlImpl, Thumb, Track,
};
use crate::metadata::{PseudoClassesAttribute, TemplatePartAttribute};
use crate::{Button, ControlImpl, ScrollViewer};
use ferroui_base::animation::TimeSpan;
use ferroui_base::data::BindingPriority;
use ferroui_base::input::{
    ContextRequestedEventArgs, InputElement, InputElementImpl, InputElementImplExt, Key, KeyEventArgs,
    PointerEventArgs, PointerPointProperties, PointerPressedEventArgs, PointerType, PointerUpdateKind,
    PointerWheelEventArgs,
    RawInputModifiers, VectorEventArgs,
};
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventHandlerToken, RoutingStrategies};
use ferroui_base::layout::{LayoutableImpl, Orientation};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, DirectProperty, FerroObject, FerroObjectExtensions,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Point, Ref, Size, StaticType,
    StyledElementImpl, StyledProperty, Vector, Visual, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
    WeakRef,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

const PC_VERTICAL: &str = ":vertical";
const PC_HORIZONTAL: &str = ":horizontal";

/// Provides data for the scroll event of a [`ScrollBar`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollEventArgs {
    new_value: f64,
    scroll_event_type: ScrollEventType,
}

impl ScrollEventArgs {
    pub fn new(event_type: ScrollEventType, new_value: f64) -> Self {
        Self { new_value, scroll_event_type: event_type }
    }

    /// The new value of the scroll bar.
    pub fn new_value(&self) -> f64 {
        self.new_value
    }

    /// The type of the scroll event.
    pub fn scroll_event_type(&self) -> ScrollEventType {
        self.scroll_event_type
    }
}

/// The action invoked when the expand/collapse timer elapses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DelayedAction {
    Collapse,
    Expand,
}

/// A scrollbar control.
#[repr(C)]
pub struct ScrollBar {
    base: RangeBase,
    line_up_button: RefCell<Option<(Ref<Button>, RoutedEventHandlerToken)>>,
    line_down_button: RefCell<Option<(Ref<Button>, RoutedEventHandlerToken)>>,
    page_up_button: RefCell<Option<(Ref<Button>, RoutedEventHandlerToken)>>,
    page_down_button: RefCell<Option<(Ref<Button>, RoutedEventHandlerToken)>>,
    timer: RefCell<Option<Rc<DispatcherTimer>>>,
    timer_action: Cell<Option<DelayedAction>>,
    is_expanded: Cell<bool>,
    owner_subscriptions: RefCell<Option<Vec<Rc<dyn IDisposable>>>>,
    owner: RefCell<Option<WeakRef<ScrollViewer>>>,
    is_dragging: Cell<bool>,
    last_right_click_position: Cell<Point>,
    scroll: HandlerList<dyn Fn(&ScrollEventArgs)>,
}

ferro_class!(ScrollBar: RangeBase);
// The `markup:` part is declared here and not generated: the public methods of the class are
// declared for markup, the commands of the context menu of the scroll bar in the control themes
// (`Command="{Binding $parent[ScrollBar].ScrollHere}"`).
ferroui_base::ferro_class_info!(ScrollBar {
    new: ScrollBar::new,
    markup: {
        methods: [
            fn ScrollHere() => ScrollBar::scroll_here,
            fn ScrollToHome() => ScrollBar::scroll_to_home,
            fn ScrollToEnd() => ScrollBar::scroll_to_end,
            fn PageUp() => ScrollBar::page_up,
            fn PageDown() => ScrollBar::page_down,
            fn PageLeft() => ScrollBar::page_left,
            fn PageRight() => ScrollBar::page_right,
            fn LineUp() => ScrollBar::line_up,
            fn LineDown() => ScrollBar::line_down,
            fn LineLeft() => ScrollBar::line_left,
            fn LineRight() => ScrollBar::line_right,
        ],
        events: [
            Scroll(Option<ferroui_base::BoxedValue>, crate::primitives::ScrollEventArgs) => |this: &ferroui_base::Ref<ScrollBar>, handler: ferroui_base::metadata::MarkupDelegate| {
                let sender = this.downgrade();
                this.scroll(move |e: &crate::primitives::ScrollEventArgs| {
                    handler.invoke(&[ferroui_base::metadata::into_markup_value(sender.upgrade()), ferroui_base::metadata::into_markup_value(e.clone())]);
                })
            },
        ],
        attributes: [
            TemplatePart("PART_LineDownButton", type(ferroui_base::Ref<crate::Button>)),
            TemplatePart("PART_LineUpButton", type(ferroui_base::Ref<crate::Button>)),
            TemplatePart("PART_PageDownButton", type(ferroui_base::Ref<crate::Button>)),
            TemplatePart("PART_PageUpButton", type(ferroui_base::Ref<crate::Button>)),
            PseudoClasses(":vertical", ":horizontal"),
        ],
    },
});
ferro_impl_classes!(ScrollBar: StyledElementImpl, LayoutableImpl, InteractiveImpl);

impl ControlImpl for ScrollBar {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::ScrollBarAutomationPeer::new(this).upcast()
    }
}

impl VisualImpl for ScrollBar {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        this.attach_to_scroll_viewer();
    }
}

impl FerroObjectImpl for ScrollBar {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.update_pseudo_classes(this.orientation());
        this.add_handler(InputElement::context_requested_event(), |sender, e| {
            if let Some(this) = sender.downcast_ref::<ScrollBar>() {
                this.on_context_requested(e);
            }
        });
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        let property = change.property();

        if property == Self::orientation_property().as_property() {
            this.update_pseudo_classes(change.get_new_value::<Orientation>());
            if this.is_attached_to_visual_tree() {
                // There's no way to manually refresh bindings, so reapply
                // them.
                this.attach_to_scroll_viewer();
            }
        } else if property == Self::allow_auto_hide_property().as_property() {
            this.update_is_expanded_state();
        } else if property == RangeBase::value_property().as_property() {
            let value = change.get_new_value::<f64>();
            if let Some(owner) = this.owner() {
                let offset = owner.offset();
                owner.set_current_value(
                    ScrollViewer::offset_property(),
                    if this.orientation() == Orientation::Horizontal {
                        offset.with_x(value)
                    } else {
                        offset.with_y(value)
                    },
                );
            }
        } else if property == RangeBase::minimum_property().as_property()
            || property == RangeBase::maximum_property().as_property()
            || property == Self::viewport_size_property().as_property()
            || property == Self::visibility_property().as_property()
        {
            this.update_is_visible();
        }
    }
}

impl InputElementImpl for ScrollBar {
    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        if this.orientation() == Orientation::Vertical {
            if e.key == Key::PageUp {
                this.large_decrement();
                e.set_handled(true);
            } else if e.key == Key::PageDown {
                this.large_increment();
                e.set_handled(true);
            } else if e.key == Key::Up {
                this.small_decrement();
                e.set_handled(true);
            } else if e.key == Key::Down {
                this.small_increment();
                e.set_handled(true);
            }
        } else if this.orientation() == Orientation::Horizontal {
            if e.key == Key::Left {
                this.small_decrement();
                e.set_handled(true);
            } else if e.key == Key::Right {
                this.small_increment();
                e.set_handled(true);
            }
        }
    }

    fn on_pointer_wheel_changed(this: &Self, e: &PointerWheelEventArgs) {
        Self::parent_on_pointer_wheel_changed(this, e);

        // We need to handle the pointer wheel event to allow scrolling with
        // the pointer wheel. So we raise the event on the scroll viewer's
        // presenter.
        if e.handled() {
            return;
        }

        let presenter = this.owner().and_then(|owner| owner.presenter());
        if let (Some(presenter), Some(root)) = (presenter, this.visual_root()) {
            e.set_handled(true);
            let key_modifiers = e.key_modifiers();
            let e = PointerWheelEventArgs::new(
                this.to_ref(),
                e.pointer().clone(),
                &root,
                e.get_position(Some(&root)),
                e.timestamp(),
                PointerPointProperties::new(
                    RawInputModifiers::from_bits_truncate(key_modifiers.bits()),
                    PointerUpdateKind::Other,
                ),
                key_modifiers,
                e.delta(),
            );
            presenter.raise_event(&e);
        }
    }

    fn on_pointer_entered(this: &Self, e: &PointerEventArgs) {
        Self::parent_on_pointer_entered(this, e);

        if this.allow_auto_hide() {
            this.expand_after_delay();
        }
    }

    fn on_pointer_exited(this: &Self, e: &PointerEventArgs) {
        Self::parent_on_pointer_exited(this, e);

        if this.allow_auto_hide() && !this.is_dragging.get() {
            this.collapse_after_delay();
        }
    }

    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        Self::parent_on_pointer_pressed(this, e);

        if e.get_current_point(Some(this)).properties.is_right_button_pressed {
            this.last_right_click_position.set(e.get_position(Some(this)));
        }
    }
}

impl TemplatedControlImpl for ScrollBar {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        for slot in [&this.line_up_button, &this.line_down_button, &this.page_up_button, &this.page_down_button] {
            let previous = slot.borrow_mut().take();
            if let Some((button, token)) = previous {
                button.remove_handler(Button::click_event(), token);
            }
        }

        let attach = |slot: &RefCell<Option<(Ref<Button>, RoutedEventHandlerToken)>>,
                      name: &str,
                      action: fn(&ScrollBar)| {
            if let Some(button) = e.name_scope().find_as::<Button>(name) {
                let weak = this.to_ref().downgrade();
                let token = button.click(move |_, _| {
                    if let Some(this) = weak.upgrade() {
                        action(&this);
                    }
                });
                *slot.borrow_mut() = Some((button, token));
            }
        };

        attach(&this.line_up_button, "PART_LineUpButton", Self::small_decrement);
        attach(&this.line_down_button, "PART_LineDownButton", Self::small_increment);
        attach(&this.page_up_button, "PART_PageUpButton", Self::large_decrement);
        attach(&this.page_down_button, "PART_PageDownButton", Self::large_increment);
    }
}

impl ScrollBar {
    /// The named parts expected in the control template.
    pub const TEMPLATE_PARTS: &'static [TemplatePartAttribute] = &[
        TemplatePartAttribute::new("PART_LineDownButton", <Button as StaticType>::TYPE),
        TemplatePartAttribute::new("PART_LineUpButton", <Button as StaticType>::TYPE),
        TemplatePartAttribute::new("PART_PageDownButton", <Button as StaticType>::TYPE),
        TemplatePartAttribute::new("PART_PageUpButton", <Button as StaticType>::TYPE),
    ];

    /// The pseudoclasses set by the class.
    pub const PSEUDO_CLASSES: PseudoClassesAttribute = PseudoClassesAttribute::new(&[PC_VERTICAL, PC_HORIZONTAL]);
}

ferroui_base::ferro_properties! { impl ScrollBar {
    ferro_property!(
        /// Defines the `ViewportSize` property.
        pub fn viewport_size_property() -> StyledProperty<f64> {
            FerroProperty::register::<ScrollBar, _>("ViewportSize", f64::NAN)
        }
    );

    ferro_property!(
        /// Defines the `Visibility` property.
        pub fn visibility_property() -> StyledProperty<ScrollBarVisibility> {
            FerroProperty::register::<ScrollBar, _>("Visibility", ScrollBarVisibility::Visible)
        }
    );

    ferro_property!(
        /// Defines the `Orientation` property.
        pub fn orientation_property() -> StyledProperty<Orientation> {
            FerroProperty::register::<ScrollBar, _>("Orientation", Orientation::Vertical)
        }
    );

    ferro_property!(
        /// Defines the `IsExpanded` property.
        pub fn is_expanded_property() -> DirectProperty<ScrollBar, bool> {
            FerroProperty::register_direct::<ScrollBar, _>("IsExpanded", |o| o.is_expanded(), None, false)
        }
    );

    ferro_property!(
        /// Defines the `AllowAutoHide` property.
        pub fn allow_auto_hide_property() -> StyledProperty<bool> {
            FerroProperty::register::<ScrollBar, _>("AllowAutoHide", true)
        }
    );

    ferro_property!(
        /// Defines the `HideDelay` property.
        pub fn hide_delay_property() -> StyledProperty<TimeSpan> {
            FerroProperty::register::<ScrollBar, _>("HideDelay", TimeSpan::from_seconds(2.0))
        }
    );

    ferro_property!(
        /// Defines the `ShowDelay` property.
        pub fn show_delay_property() -> StyledProperty<TimeSpan> {
            FerroProperty::register::<ScrollBar, _>("ShowDelay", TimeSpan::from_seconds(0.5))
        }
    );
} }

impl ScrollBar {
    fn static_constructor() {
        Thumb::drag_delta_event().add_class_handler_with::<ScrollBar>(
            |x, e| x.on_thumb_drag_delta(e),
            RoutingStrategies::BUBBLE,
            false,
        );
        Thumb::drag_started_event().add_class_handler_with::<ScrollBar>(
            |x, e| x.on_thumb_drag_start(e),
            RoutingStrategies::BUBBLE,
            false,
        );
        Thumb::drag_completed_event().add_class_handler_with::<ScrollBar>(
            |x, e| x.on_thumb_drag_complete(e),
            RoutingStrategies::BUBBLE,
            false,
        );

        InputElement::focusable_property().override_default_value::<ScrollBar>(false);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: RangeBase::construct(),
            line_up_button: RefCell::new(None),
            line_down_button: RefCell::new(None),
            page_up_button: RefCell::new(None),
            page_down_button: RefCell::new(None),
            timer: RefCell::new(None),
            timer_action: Cell::new(None),
            is_expanded: Cell::new(false),
            owner_subscriptions: RefCell::new(None),
            owner: RefCell::new(None),
            is_dragging: Cell::new(false),
            last_right_click_position: Cell::new(Point::default()),
            scroll: HandlerList::new(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The amount of the scrollable content that is currently visible.
    pub fn viewport_size(&self) -> f64 {
        self.get_value(Self::viewport_size_property())
    }

    pub fn set_viewport_size(&self, value: f64) {
        self.set_value(Self::viewport_size_property(), value)
    }

    /// Indicates whether the scrollbar should hide itself when it is not
    /// needed.
    pub fn visibility(&self) -> ScrollBarVisibility {
        self.get_value(Self::visibility_property())
    }

    pub fn set_visibility(&self, value: ScrollBarVisibility) {
        self.set_value(Self::visibility_property(), value)
    }

    /// The orientation of the scrollbar.
    pub fn orientation(&self) -> Orientation {
        self.get_value(Self::orientation_property())
    }

    pub fn set_orientation(&self, value: Orientation) {
        self.set_value(Self::orientation_property(), value)
    }

    /// Indicates whether the scrollbar is expanded.
    pub fn is_expanded(&self) -> bool {
        self.is_expanded.get()
    }

    fn set_is_expanded(&self, value: bool) {
        self.set_and_raise_cell(Self::is_expanded_property(), &self.is_expanded, value);
    }

    /// Indicates whether the scrollbar can hide itself when the user is not
    /// interacting with it.
    pub fn allow_auto_hide(&self) -> bool {
        self.get_value(Self::allow_auto_hide_property())
    }

    pub fn set_allow_auto_hide(&self, value: bool) {
        self.set_value(Self::allow_auto_hide_property(), value)
    }

    /// Determines how long the hide delay will be after the user stops
    /// interacting with the scrollbar.
    pub fn hide_delay(&self) -> TimeSpan {
        self.get_value(Self::hide_delay_property())
    }

    pub fn set_hide_delay(&self, value: TimeSpan) {
        self.set_value(Self::hide_delay_property(), value)
    }

    /// Determines how long the show delay will be when the user starts
    /// interacting with the scrollbar.
    pub fn show_delay(&self) -> TimeSpan {
        self.get_value(Self::show_delay_property())
    }

    pub fn set_show_delay(&self, value: TimeSpan) {
        self.set_value(Self::show_delay_property(), value)
    }

    /// Raised when the scrollbar scrolls. The handler receives the event
    /// data, as the handlers of the other plain events do.
    pub fn scroll(&self, handler: impl Fn(&ScrollEventArgs) + 'static) -> Rc<dyn IDisposable> {
        let token = self.scroll.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.scroll.remove(token);
            }
        })
    }

    /// Calculates and updates whether the scrollbar should be visible.
    fn update_is_visible(&self) {
        let is_visible = match self.visibility() {
            ScrollBarVisibility::Visible => true,
            ScrollBarVisibility::Disabled => false,
            ScrollBarVisibility::Hidden => false,
            ScrollBarVisibility::Auto => self.viewport_size().is_nan() || self.maximum() > 0.0,
        };

        self.set_current_value(Visual::is_visible_property(), is_visible);
    }

    fn owner(&self) -> Option<Ref<ScrollViewer>> {
        self.owner.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    fn dispose_owner_subscriptions(&self) {
        let subscriptions = self.owner_subscriptions.borrow_mut().take();
        if let Some(subscriptions) = subscriptions {
            for subscription in subscriptions {
                subscription.dispose();
            }
        }
    }

    /// Tries to attach to the templated parent if it is a [`ScrollViewer`]
    /// and binds to its properties. Properties which have been set through
    /// other means are not bound.
    ///
    /// This method is automatically called when the control is attached to
    /// a visual tree.
    pub(crate) fn attach_to_scroll_viewer(&self) {
        let owner = self.templated_parent().and_then(|parent| parent.cast::<ScrollViewer>());

        let Some(owner) = owner else {
            *self.owner.borrow_mut() = None;
            self.dispose_owner_subscriptions();
            return;
        };

        if self.owner().as_ref() == Some(&owner) {
            return;
        }

        self.dispose_owner_subscriptions();

        let is_horizontal = self.orientation() == Orientation::Horizontal;

        let weak = self.to_ref().downgrade();
        let is_horizontal_now = move || {
            weak.upgrade().map_or(is_horizontal, |this| this.orientation() == Orientation::Horizontal)
        };
        let extract_vector = {
            let is_horizontal_now = is_horizontal_now.clone();
            move |v: Vector| if is_horizontal_now() { v.x } else { v.y }
        };
        let extract_size = move |v: Size| if is_horizontal_now() { v.width } else { v.height };

        let visibility_source = if is_horizontal {
            ScrollViewer::horizontal_scroll_bar_visibility_property()
        } else {
            ScrollViewer::vertical_scroll_bar_visibility_property()
        };

        let priority = BindingPriority::Template;
        let source: &FerroObject = &owner;
        let mut subscriptions: Vec<Rc<dyn IDisposable>> = Vec::new();

        if !self.is_set(RangeBase::maximum_property().as_property()) {
            subscriptions.push(self.bind_typed(
                RangeBase::maximum_property(),
                FerroObjectExtensions::get_observable_with(source, ScrollViewer::scroll_bar_maximum_property(), extract_vector.clone()),
                priority,
            ));
        }
        if !self.is_set(RangeBase::value_property().as_property()) {
            subscriptions.push(self.bind_typed(
                RangeBase::value_property(),
                FerroObjectExtensions::get_observable_with(source, ScrollViewer::offset_property(), extract_vector),
                priority,
            ));
        }
        if !self.is_set(ScrollViewer::is_deferred_scrolling_enabled_property().as_property()) {
            subscriptions.push(self.bind_typed(
                ScrollViewer::is_deferred_scrolling_enabled_property(),
                FerroObjectExtensions::get_observable(source, ScrollViewer::is_deferred_scrolling_enabled_property()),
                priority,
            ));
        }
        if !self.is_set(Self::viewport_size_property().as_property()) {
            subscriptions.push(self.bind_typed(
                Self::viewport_size_property(),
                FerroObjectExtensions::get_observable_with(source, ScrollViewer::viewport_property(), extract_size.clone()),
                priority,
            ));
        }
        if !self.is_set(Self::visibility_property().as_property()) {
            subscriptions.push(self.bind_typed(
                Self::visibility_property(),
                FerroObjectExtensions::get_observable(source, visibility_source),
                priority,
            ));
        }
        if !self.is_set(Self::allow_auto_hide_property().as_property()) {
            subscriptions.push(self.bind_typed(
                Self::allow_auto_hide_property(),
                FerroObjectExtensions::get_observable(source, ScrollViewer::allow_auto_hide_property()),
                priority,
            ));
        }
        if !self.is_set(RangeBase::large_change_property().as_property()) {
            subscriptions.push(self.bind_typed(
                RangeBase::large_change_property(),
                FerroObjectExtensions::get_observable_with(source, ScrollViewer::large_change_property(), extract_size.clone()),
                priority,
            ));
        }
        if !self.is_set(RangeBase::small_change_property().as_property()) {
            subscriptions.push(self.bind_typed(
                RangeBase::small_change_property(),
                FerroObjectExtensions::get_observable_with(source, ScrollViewer::small_change_property(), extract_size),
                priority,
            ));
        }

        *self.owner.borrow_mut() = Some(owner.downgrade());
        *self.owner_subscriptions.borrow_mut() = Some(subscriptions);
    }

    fn invoke_after_delay(&self, handler: DelayedAction, delay: TimeSpan) {
        let existing = self.timer.borrow().clone();
        let timer = match existing {
            Some(timer) => {
                timer.stop();
                timer
            }
            None => {
                let timer = DispatcherTimer::with_priority(DispatcherPriority::NORMAL);
                let weak = self.to_ref().downgrade();
                // The subscription lives as long as the timer.
                let _ = timer.tick(move |sender_timer| {
                    if let Some(this) = weak.upgrade() {
                        match this.timer_action.get() {
                            Some(DelayedAction::Collapse) => this.collapse(),
                            Some(DelayedAction::Expand) => this.expand(),
                            None => {}
                        }
                    }

                    sender_timer.stop();
                });
                *self.timer.borrow_mut() = Some(timer.clone());
                timer
            }
        };

        self.timer_action.set(Some(handler));
        timer.set_interval(delay.to_duration().unwrap_or(Duration::ZERO));

        timer.start();
    }

    fn update_is_expanded_state(&self) {
        if !self.allow_auto_hide() {
            let timer = self.timer.borrow().clone();
            if let Some(timer) = timer {
                timer.stop();
            }

            self.set_is_expanded(true);
        } else {
            self.set_is_expanded(self.is_pointer_over());
        }
    }

    fn collapse_after_delay(&self) {
        self.invoke_after_delay(DelayedAction::Collapse, self.hide_delay());
    }

    fn expand_after_delay(&self) {
        self.invoke_after_delay(DelayedAction::Expand, self.show_delay());
    }

    fn collapse(&self) {
        self.set_is_expanded(false);
    }

    fn expand(&self) {
        self.set_is_expanded(true);
    }

    fn small_decrement(&self) {
        self.set_current_value(
            RangeBase::value_property(),
            f64::max(self.value() - self.small_change(), self.minimum()),
        );
        self.on_scroll(ScrollEventType::SmallDecrement);
    }

    fn small_increment(&self) {
        self.set_current_value(
            RangeBase::value_property(),
            f64::min(self.value() + self.small_change(), self.maximum()),
        );
        self.on_scroll(ScrollEventType::SmallIncrement);
    }

    fn large_decrement(&self) {
        self.set_current_value(
            RangeBase::value_property(),
            f64::max(self.value() - self.large_change(), self.minimum()),
        );
        self.on_scroll(ScrollEventType::LargeDecrement);
    }

    fn large_increment(&self) {
        self.set_current_value(
            RangeBase::value_property(),
            f64::min(self.value() + self.large_change(), self.maximum()),
        );
        self.on_scroll(ScrollEventType::LargeIncrement);
    }

    fn on_thumb_drag_delta(&self, _e: &VectorEventArgs) {
        self.on_scroll(ScrollEventType::ThumbTrack);
    }

    fn on_thumb_drag_start(&self, _e: &VectorEventArgs) {
        self.is_dragging.set(true);
    }

    fn on_thumb_drag_complete(&self, _e: &VectorEventArgs) {
        self.is_dragging.set(false);

        if self.allow_auto_hide() && !self.is_pointer_over() {
            self.collapse_after_delay();
        }

        self.on_scroll(ScrollEventType::EndScroll);
    }

    /// Raises the scroll event.
    pub fn on_scroll(&self, scroll_event_type: ScrollEventType) {
        if self.scroll.is_empty() {
            return;
        }

        let e = ScrollEventArgs::new(scroll_event_type, self.value());
        for (_, handler) in self.scroll.snapshot().iter() {
            handler(&e);
        }
    }

    fn update_pseudo_classes(&self, o: Orientation) {
        self.pseudo_classes().set(PC_VERTICAL, o == Orientation::Vertical);
        self.pseudo_classes().set(PC_HORIZONTAL, o == Orientation::Horizontal);
    }

    fn on_context_requested(&self, e: &ContextRequestedEventArgs) {
        if matches!(e.pointer_type(), Some(PointerType::Touch | PointerType::Pen)) {
            e.set_handled(true);
            return;
        }

        if let Some(position) = e.try_get_position(Some(self)) {
            self.last_right_click_position.set(position);
        }
    }

    /// Scrolls to the location at which the context menu was most recently
    /// requested.
    pub fn scroll_here(&self) {
        let Some(track) = self.track() else {
            return;
        };

        let is_vertical = self.orientation() == Orientation::Vertical;
        let last_right_click_position = self.last_right_click_position.get();
        let thumb_bounds = track.thumb().map(|thumb| thumb.bounds());

        let track_length = if is_vertical { track.bounds().height } else { track.bounds().width };
        let thumb_length =
            thumb_bounds.map_or(0.0, |bounds| if is_vertical { bounds.height } else { bounds.width });
        let click_position = if is_vertical { last_right_click_position.y } else { last_right_click_position.x };

        if track_length > thumb_length {
            let ratio = click_position / track_length;
            let range = self.maximum() - self.minimum();
            let value = self.minimum() + (ratio * range);
            self.set_current_value(
                RangeBase::value_property(),
                f64::max(self.minimum(), f64::min(self.maximum(), value)),
            );
            self.on_scroll(ScrollEventType::ThumbTrack);
        }
    }

    /// Scrolls to the top (or left edge) of the scrollbar by setting the
    /// value to the minimum.
    pub fn scroll_to_home(&self) {
        self.set_current_value(RangeBase::value_property(), self.minimum());
        self.on_scroll(ScrollEventType::LargeDecrement);
    }

    /// Scrolls to the bottom (or right edge) of the scrollbar by setting the
    /// value to the maximum.
    pub fn scroll_to_end(&self) {
        self.set_current_value(RangeBase::value_property(), self.maximum());
        self.on_scroll(ScrollEventType::LargeIncrement);
    }

    /// Scrolls up by one large change (a page) on a vertical scrollbar.
    pub fn page_up(&self) {
        self.large_decrement()
    }

    /// Scrolls down by one large change (a page) on a vertical scrollbar.
    pub fn page_down(&self) {
        self.large_increment()
    }

    /// Scrolls left by one large change (a page) on a horizontal scrollbar.
    pub fn page_left(&self) {
        self.large_decrement()
    }

    /// Scrolls right by one large change (a page) on a horizontal scrollbar.
    pub fn page_right(&self) {
        self.large_increment()
    }

    /// Scrolls up by one small change on a vertical scrollbar.
    pub fn line_up(&self) {
        self.small_decrement()
    }

    /// Scrolls down by one small change on a vertical scrollbar.
    pub fn line_down(&self) {
        self.small_increment()
    }

    /// Scrolls left by one small change on a horizontal scrollbar.
    pub fn line_left(&self) {
        self.small_decrement()
    }

    /// Scrolls right by one small change on a horizontal scrollbar.
    pub fn line_right(&self) {
        self.small_increment()
    }

    fn track(&self) -> Option<Ref<Track>> {
        self.get_template_descendants().into_iter().find_map(|visual| visual.cast::<Track>())
    }
}
