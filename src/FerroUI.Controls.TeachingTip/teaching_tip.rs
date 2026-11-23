//! WinUI's `TeachingTip` (`controls/dev/TeachingTip/TeachingTip.cpp`) on a `Popup`: opening and
//! closing with their reasons, the placement around the target, and the side of the tail.

use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventHandlerToken};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::reactive::{Disposable, Subscription};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, Nullable, Ref, StyledElementImpl, StyledProperty, VisualImpl,
};
use ferroui_controls::primitives::{Popup, TemplateAppliedEventArgs, TemplatedControlImpl, TemplatedControlImplExt};
use ferroui_controls::{Button, ContentControl, ContentControlImpl, Control, ControlImpl, PlacementMode};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const PC_TAIL_TOP: &str = ":tail-top";
const PC_TAIL_BOTTOM: &str = ":tail-bottom";
const PC_TAIL_LEFT: &str = ":tail-left";
const PC_TAIL_RIGHT: &str = ":tail-right";
const PC_LIGHT_DISMISS: &str = ":light-dismiss";
const PC_FOOTER_CLOSE: &str = ":footer-close";

/// Why a tip closed (WinUI `TeachingTipCloseReason`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum TeachingTipCloseReason {
    /// The close button was clicked.
    #[default]
    CloseButton = 0,
    /// The tip was dismissed by a click outside it.
    LightDismiss = 1,
    /// `IsOpen` was set to false.
    Programmatic = 2,
}

/// Where a tip is placed around its target (WinUI `TeachingTipPlacementMode`). The first word is
/// the side of the target; the second, the direction the tip extends along that side.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum TeachingTipPlacementMode {
    /// Above the target.
    #[default]
    Auto = 0,
    Top = 1,
    Bottom = 2,
    Left = 3,
    Right = 4,
    TopRight = 5,
    TopLeft = 6,
    BottomRight = 7,
    BottomLeft = 8,
    LeftTop = 9,
    LeftBottom = 10,
    RightTop = 11,
    RightBottom = 12,
}

impl TeachingTipPlacementMode {
    /// The placement of the popup.
    pub(crate) fn popup_placement(self) -> PlacementMode {
        match self {
            Self::Auto | Self::Top => PlacementMode::Top,
            Self::Bottom => PlacementMode::Bottom,
            Self::Left => PlacementMode::Left,
            Self::Right => PlacementMode::Right,
            Self::TopRight => PlacementMode::TopEdgeAlignedLeft,
            Self::TopLeft => PlacementMode::TopEdgeAlignedRight,
            Self::BottomRight => PlacementMode::BottomEdgeAlignedLeft,
            Self::BottomLeft => PlacementMode::BottomEdgeAlignedRight,
            Self::LeftTop => PlacementMode::LeftEdgeAlignedBottom,
            Self::LeftBottom => PlacementMode::LeftEdgeAlignedTop,
            Self::RightTop => PlacementMode::RightEdgeAlignedBottom,
            Self::RightBottom => PlacementMode::RightEdgeAlignedTop,
        }
    }

    /// The pseudoclass of the edge of the tip that faces the target.
    fn tail_class(self) -> &'static str {
        match self {
            Self::Auto | Self::Top | Self::TopRight | Self::TopLeft => PC_TAIL_BOTTOM,
            Self::Bottom | Self::BottomRight | Self::BottomLeft => PC_TAIL_TOP,
            Self::Left | Self::LeftTop | Self::LeftBottom => PC_TAIL_RIGHT,
            Self::Right | Self::RightTop | Self::RightBottom => PC_TAIL_LEFT,
        }
    }
}

/// Whether a tip shows its tail (WinUI `TeachingTipTailVisibility`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum TeachingTipTailVisibility {
    /// With a target.
    #[default]
    Auto = 0,
    Visible = 1,
    Collapsed = 2,
}

/// The arguments of [`TeachingTip::closing`].
#[derive(Debug)]
pub struct TeachingTipClosingEventArgs {
    reason: TeachingTipCloseReason,
    cancel: Cell<bool>,
}

impl TeachingTipClosingEventArgs {
    /// Why the tip is closing.
    pub fn reason(&self) -> TeachingTipCloseReason {
        self.reason
    }

    /// Whether the tip stays open.
    pub fn cancel(&self) -> bool {
        self.cancel.get()
    }

    pub fn set_cancel(&self, value: bool) {
        self.cancel.set(value);
    }
}

/// The arguments of [`TeachingTip::closed`].
#[derive(Clone, Copy, Debug)]
pub struct TeachingTipClosedEventArgs {
    reason: TeachingTipCloseReason,
}

impl TeachingTipClosedEventArgs {
    /// Why the tip closed.
    pub fn reason(&self) -> TeachingTipCloseReason {
        self.reason
    }
}

/// A dismissible message that points at a target.
#[repr(C)]
pub struct TeachingTip {
    base: ContentControl,
    popup: RefCell<Option<Ref<Popup>>>,
    popup_subscriptions: RefCell<Vec<Subscription>>,
    buttons: RefCell<Vec<(Ref<Button>, RoutedEventHandlerToken)>>,
    /// The reason of the close in progress; none for a close the popup started by itself.
    close_reason: Cell<Option<TeachingTipCloseReason>>,
    last_close_reason: Cell<TeachingTipCloseReason>,
    /// Set while the control writes `IsOpen` to follow the popup.
    syncing: Cell<bool>,
    opened_handlers: Rc<HandlerList<dyn Fn()>>,
    closing_handlers: Rc<HandlerList<dyn Fn(&TeachingTipClosingEventArgs)>>,
    closed_handlers: Rc<HandlerList<dyn Fn(&TeachingTipClosedEventArgs)>>,
    action_handlers: Rc<HandlerList<dyn Fn()>>,
    close_button_handlers: Rc<HandlerList<dyn Fn()>>,
}

ferro_class!(TeachingTip: ContentControl);
ferroui_base::ferro_class_info!(TeachingTip { new: TeachingTip::new });
ferro_impl_classes!(
    TeachingTip: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    ContentControlImpl
);

impl FerroObjectImpl for TeachingTip {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.update_classes();
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        let property = change.property();
        if property == Self::is_open_property().as_property() {
            if !this.syncing.get() {
                this.apply_is_open(change.get_new_value::<bool>());
            }
        } else if property == Self::target_property().as_property()
            || property == Self::preferred_placement_property().as_property()
        {
            this.update_popup();
            this.update_classes();
        } else if property == Self::is_light_dismiss_enabled_property().as_property() {
            this.update_popup();
            this.update_classes();
        } else if property == Self::tail_visibility_property().as_property()
            || property == Self::close_button_content_property().as_property()
        {
            this.update_classes();
        }
    }
}

impl TemplatedControlImpl for TeachingTip {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);

        for subscription in this.popup_subscriptions.take() {
            subscription.dispose();
        }
        for (button, token) in this.buttons.take() {
            button.remove_handler(Button::click_event(), token);
        }

        let popup = e.name_scope().find_as::<Popup>("PART_Popup");
        if let Some(popup) = &popup {
            let weak = this.to_ref().downgrade();
            let closing = {
                let weak = weak.clone();
                popup.closing(move |args| {
                    if let Some(this) = weak.upgrade() {
                        let reason = this.close_reason.get().unwrap_or(TeachingTipCloseReason::LightDismiss);
                        if this.raise_closing(reason) {
                            args.set_cancel(true);
                        } else {
                            this.last_close_reason.set(reason);
                        }
                    }
                })
            };
            let closed = {
                let weak = weak.clone();
                popup.closed(move || {
                    if let Some(this) = weak.upgrade() {
                        this.on_popup_closed();
                    }
                })
            };
            let opened = popup.opened(move || {
                if let Some(this) = weak.upgrade() {
                    for (_, handler) in this.opened_handlers.snapshot().iter() {
                        handler();
                    }
                }
            });
            *this.popup_subscriptions.borrow_mut() = vec![closing, closed, opened];
        }
        *this.popup.borrow_mut() = popup;

        let mut buttons = Vec::new();
        for (name, is_action) in [("ActionButton", true), ("CloseButton", false), ("AlternateCloseButton", false)] {
            if let Some(button) = e.name_scope().find_as::<Button>(name) {
                let weak = this.to_ref().downgrade();
                let token = button.click(move |_, _| {
                    if let Some(this) = weak.upgrade() {
                        if is_action {
                            for (_, handler) in this.action_handlers.snapshot().iter() {
                                handler();
                            }
                        } else {
                            this.on_close_button();
                        }
                    }
                });
                buttons.push((button, token));
            }
        }
        *this.buttons.borrow_mut() = buttons;

        this.update_popup();
        if this.is_open() {
            this.apply_is_open(true);
        }
    }
}

ferro_properties! {
    impl TeachingTip {
        /// Defines the `IsOpen` property.
        pub fn is_open_property() -> StyledProperty<bool> {
            FerroProperty::register::<TeachingTip, _>("IsOpen", false)
        }

        /// Defines the `Target` property.
        pub fn target_property() -> StyledProperty<Option<Ref<Control>>> {
            FerroProperty::register::<TeachingTip, _>("Target", None)
        }

        /// Defines the `Title` property.
        pub fn title_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<TeachingTip, _>("Title", None)
        }

        /// Defines the `Subtitle` property.
        pub fn subtitle_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<TeachingTip, _>("Subtitle", None)
        }

        /// Defines the `PreferredPlacement` property.
        pub fn preferred_placement_property() -> StyledProperty<TeachingTipPlacementMode> {
            FerroProperty::register::<TeachingTip, _>("PreferredPlacement", TeachingTipPlacementMode::Auto)
        }

        /// Defines the `TailVisibility` property.
        pub fn tail_visibility_property() -> StyledProperty<TeachingTipTailVisibility> {
            FerroProperty::register::<TeachingTip, _>("TailVisibility", TeachingTipTailVisibility::Auto)
        }

        /// Defines the `IsLightDismissEnabled` property.
        pub fn is_light_dismiss_enabled_property() -> StyledProperty<bool> {
            FerroProperty::register::<TeachingTip, _>("IsLightDismissEnabled", false)
        }

        /// Defines the `ActionButtonContent` property.
        pub fn action_button_content_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<TeachingTip, _>("ActionButtonContent", None)
        }

        /// Defines the `CloseButtonContent` property.
        pub fn close_button_content_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<TeachingTip, _>("CloseButtonContent", None)
        }

        /// Defines the `HeroContent` property.
        pub fn hero_content_property() -> StyledProperty<Option<Ref<Control>>> {
            FerroProperty::register::<TeachingTip, _>("HeroContent", None)
        }
    }
}

impl TeachingTip {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: ContentControl::construct(),
            popup: RefCell::new(None),
            popup_subscriptions: RefCell::new(Vec::new()),
            buttons: RefCell::new(Vec::new()),
            close_reason: Cell::new(None),
            last_close_reason: Cell::new(TeachingTipCloseReason::Programmatic),
            syncing: Cell::new(false),
            opened_handlers: Rc::new(HandlerList::new()),
            closing_handlers: Rc::new(HandlerList::new()),
            closed_handlers: Rc::new(HandlerList::new()),
            action_handlers: Rc::new(HandlerList::new()),
            close_button_handlers: Rc::new(HandlerList::new()),
        }
    }

    /// Creates a closed tip.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Whether the tip is open.
    pub fn is_open(&self) -> bool {
        self.get_value(Self::is_open_property())
    }

    pub fn set_is_open(&self, value: bool) {
        self.set_value(Self::is_open_property(), value);
    }

    /// The control the tip points at. Without one the tip is centered on the element it is
    /// declared in and has no tail.
    pub fn target(&self) -> Option<Ref<Control>> {
        self.get_value(Self::target_property())
    }

    pub fn set_target(&self, value: impl Into<Nullable<Control>>) {
        self.set_value(Self::target_property(), value.into().0);
    }

    /// The title of the tip.
    pub fn title(&self) -> Option<String> {
        self.get_value(Self::title_property())
    }

    pub fn set_title(&self, value: Option<&str>) {
        self.set_value(Self::title_property(), value.map(str::to_owned));
    }

    /// The text under the title.
    pub fn subtitle(&self) -> Option<String> {
        self.get_value(Self::subtitle_property())
    }

    pub fn set_subtitle(&self, value: Option<&str>) {
        self.set_value(Self::subtitle_property(), value.map(str::to_owned));
    }

    /// Where the tip is placed around its target.
    pub fn preferred_placement(&self) -> TeachingTipPlacementMode {
        self.get_value(Self::preferred_placement_property())
    }

    pub fn set_preferred_placement(&self, value: TeachingTipPlacementMode) {
        self.set_value(Self::preferred_placement_property(), value);
    }

    /// Whether the tail is shown.
    pub fn tail_visibility(&self) -> TeachingTipTailVisibility {
        self.get_value(Self::tail_visibility_property())
    }

    pub fn set_tail_visibility(&self, value: TeachingTipTailVisibility) {
        self.set_value(Self::tail_visibility_property(), value);
    }

    /// Whether a click outside the tip closes it.
    pub fn is_light_dismiss_enabled(&self) -> bool {
        self.get_value(Self::is_light_dismiss_enabled_property())
    }

    pub fn set_is_light_dismiss_enabled(&self, value: bool) {
        self.set_value(Self::is_light_dismiss_enabled_property(), value);
    }

    /// The content of the action button, which is shown when it has one.
    pub fn action_button_content(&self) -> Option<BoxedValue> {
        self.get_value(Self::action_button_content_property())
    }

    pub fn set_action_button_content(&self, value: Option<BoxedValue>) {
        self.set_value(Self::action_button_content_property(), value);
    }

    /// The content of the close button in the footer. Without one the close button is the small
    /// button in the corner.
    pub fn close_button_content(&self) -> Option<BoxedValue> {
        self.get_value(Self::close_button_content_property())
    }

    pub fn set_close_button_content(&self, value: Option<BoxedValue>) {
        self.set_value(Self::close_button_content_property(), value);
    }

    /// The control shown at the top of the tip.
    pub fn hero_content(&self) -> Option<Ref<Control>> {
        self.get_value(Self::hero_content_property())
    }

    pub fn set_hero_content(&self, value: impl Into<Nullable<Control>>) {
        self.set_value(Self::hero_content_property(), value.into().0);
    }

    fn subscribe<F: ?Sized + 'static>(handlers: &Rc<HandlerList<F>>, handler: Rc<F>) -> Subscription {
        let handlers = handlers.clone();
        let token = handlers.add(handler);
        Subscription::new(Disposable::create(move || {
            handlers.remove(token);
        }))
    }

    /// Occurs when the tip has opened.
    pub fn opened(&self, handler: impl Fn() + 'static) -> Subscription {
        Self::subscribe::<dyn Fn()>(&self.opened_handlers, Rc::new(handler))
    }

    /// Occurs before the tip closes; a handler can keep it open.
    pub fn closing(&self, handler: impl Fn(&TeachingTipClosingEventArgs) + 'static) -> Subscription {
        Self::subscribe::<dyn Fn(&TeachingTipClosingEventArgs)>(&self.closing_handlers, Rc::new(handler))
    }

    /// Occurs when the tip has closed.
    pub fn closed(&self, handler: impl Fn(&TeachingTipClosedEventArgs) + 'static) -> Subscription {
        Self::subscribe::<dyn Fn(&TeachingTipClosedEventArgs)>(&self.closed_handlers, Rc::new(handler))
    }

    /// Occurs when the action button is clicked.
    pub fn action_button_click(&self, handler: impl Fn() + 'static) -> Subscription {
        Self::subscribe::<dyn Fn()>(&self.action_handlers, Rc::new(handler))
    }

    /// Occurs when a close button is clicked, before the tip closes.
    pub fn close_button_click(&self, handler: impl Fn() + 'static) -> Subscription {
        Self::subscribe::<dyn Fn()>(&self.close_button_handlers, Rc::new(handler))
    }

    fn on_close_button(&self) {
        for (_, handler) in self.close_button_handlers.snapshot().iter() {
            handler();
        }
        self.close_with(TeachingTipCloseReason::CloseButton);
    }

    /// Raises `Closing`; returns whether a handler kept the tip open.
    fn raise_closing(&self, reason: TeachingTipCloseReason) -> bool {
        let args = TeachingTipClosingEventArgs { reason, cancel: Cell::new(false) };
        for (_, handler) in self.closing_handlers.snapshot().iter() {
            handler(&args);
        }
        args.cancel()
    }

    fn close_with(&self, reason: TeachingTipCloseReason) {
        let Some(popup) = self.popup.borrow().clone() else {
            self.write_is_open(false);
            return;
        };
        self.close_reason.set(Some(reason));
        popup.close();
        self.close_reason.set(None);
        // A handler of `Closing` kept it open.
        self.write_is_open(popup.is_open());
    }

    fn apply_is_open(&self, open: bool) {
        let popup = self.popup.borrow().clone();
        let Some(popup) = popup else { return };
        if open {
            self.update_popup();
            popup.open();
            // A popup needs a host to open; `IsOpen` says what is shown.
            if !popup.is_open() {
                self.write_is_open(false);
            }
        } else if popup.is_open() {
            self.close_with(TeachingTipCloseReason::Programmatic);
        }
    }

    fn on_popup_closed(&self) {
        self.write_is_open(false);
        let args = TeachingTipClosedEventArgs { reason: self.last_close_reason.get() };
        for (_, handler) in self.closed_handlers.snapshot().iter() {
            handler(&args);
        }
    }

    fn write_is_open(&self, value: bool) {
        if self.is_open() != value {
            self.syncing.set(true);
            self.set_current_value(Self::is_open_property(), value);
            self.syncing.set(false);
        }
    }

    fn update_popup(&self) {
        let Some(popup) = self.popup.borrow().clone() else { return };
        match self.target() {
            Some(target) => {
                popup.set_placement_target(target);
                popup.set_placement(self.preferred_placement().popup_placement());
            }
            None => {
                popup.set_placement_target(None::<Ref<Control>>);
                popup.set_placement(PlacementMode::Center);
            }
        }
        popup.set_is_light_dismiss_enabled(self.is_light_dismiss_enabled());
    }

    fn has_tail(&self) -> bool {
        match self.tail_visibility() {
            TeachingTipTailVisibility::Auto => self.target().is_some(),
            TeachingTipTailVisibility::Visible => true,
            TeachingTipTailVisibility::Collapsed => false,
        }
    }

    fn update_classes(&self) {
        let tail = self.has_tail().then(|| self.preferred_placement().tail_class());
        let classes = self.pseudo_classes();
        for class in [PC_TAIL_TOP, PC_TAIL_BOTTOM, PC_TAIL_LEFT, PC_TAIL_RIGHT] {
            classes.set(class, tail == Some(class));
        }
        classes.set(PC_LIGHT_DISMISS, self.is_light_dismiss_enabled());
        classes.set(PC_FOOTER_CLOSE, self.close_button_content().is_some());
    }
}
