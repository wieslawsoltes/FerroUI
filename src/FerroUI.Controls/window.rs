use crate::platform::{IWindowBaseImpl, IWindowImpl, IWindowingPlatform, PlatformAllowedWindowActions, PlatformManager, Screen};
use crate::primitives::{TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt};
use crate::window_icon::WindowIcon;
use crate::{
    ContentControlImpl, ControlImpl, SizeToContent, TopLevel, TopLevelImpl, TopLevelImplExt, WindowBase,
    WindowBaseImpl, WindowCloseReason, WindowClosingEventArgs, WindowDecorations, WindowEdge, WindowResizeReason,
    WindowResizedEventArgs, WindowStartupLocation, WindowState,
};
use ferroui_base::input::{IFocusScope, InputElementImpl, PointerPressedEventArgs};
use ferroui_base::interactivity::{InteractiveImpl, RoutedEvent, RoutedEventArgs, RoutingStrategies};
use ferroui_base::layout::{ILayoutManager, Layoutable, LayoutableImpl, LayoutableImplExt, MinMax};
use ferroui_base::media::{Brushes, IBrush};
use ferroui_base::reactive::{CompositeDisposable, Disposable, IDisposable, ObservableExt};
use ferroui_base::styling::ControlTheme;
use ferroui_base::threading::{Dispatcher, DispatcherTask};
use ferroui_base::utilities::{HandlerList, MathUtilities};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, ferro_routed_event, instantiate, BoxedValue, DirectProperty,
    FerroLocator, FerroObject, FerroObjectExtensions, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, LocatorExtensions, Nullable, PixelPoint, PixelRect, PixelSize, PropertyValue, Ref,
    Size, StaticType, StyledElementImpl, StyledProperty, StyledPropertyOptions, Thickness, TypeInfo, VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

/// Describes how the [`Window::closing`] event behaves in the presence of
/// child windows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum WindowClosingBehavior {
    /// When the owner window is closed, the child windows' closing event
    /// will be raised, followed by the owner window's closing events. A
    /// child canceling the close will result in the owner window's close
    /// being cancelled.
    #[default]
    OwnerAndChildWindows = 0,

    /// When the owner window is closed, only the owner window's closing
    /// event will be raised.
    OwnerWindowOnly = 1,
}

/// A top-level window.
#[repr(C)]
pub struct Window {
    base: WindowBase,
    window_impl: RefCell<Option<Rc<dyn IWindowImpl>>>,
    children: RefCell<Vec<(Ref<Window>, bool)>>,
    is_extended_into_window_decorations: Cell<bool>,
    window_decoration_margin: Cell<Thickness>,
    off_screen_margin: Cell<Thickness>,
    can_handle_resized: Cell<bool>,
    arrange_bounds: Cell<Size>,
    is_forced_decoration_mode: Cell<bool>,
    dialog_result: RefCell<Option<BoxedValue>>,
    shown: Cell<bool>,
    showing_as_dialog: Cell<bool>,
    position_was_set: Cell<bool>,
    was_shown_before: Cell<bool>,
    modal_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    allowed_window_actions: Cell<PlatformAllowedWindowActions>,
    last_window_state: Cell<WindowState>,
    closing: HandlerList<dyn Fn(&WindowClosingEventArgs)>,
    allowed_window_actions_changed: HandlerList<dyn Fn(PlatformAllowedWindowActions)>,
}

ferro_class! {
    Window: WindowBase, virtuals WindowImpl: WindowBaseImpl {
        /// Handles a closing notification from the platform implementation.
        /// Returns true if closing is cancelled, otherwise false. `reason`
        /// is the reason the window is closing.
        fn handle_closing(this, reason: WindowCloseReason) -> bool;
        /// Raises the `Closing` event.
        ///
        /// A type that derives from `Window` may override this member. The
        /// override must call the base implementation if the `Closing`
        /// event needs to be raised.
        fn on_closing(this, e: &WindowClosingEventArgs);
    }
}
ferroui_base::ferro_class_info!(Window { new: Window::new });

ferro_impl_classes!(Window: VisualImpl, InteractiveImpl, ContentControlImpl);

impl ControlImpl for Window {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::WindowAutomationPeer::new(this).upcast()
    }
}

impl IFocusScope for Window {}

impl InputElementImpl for Window {
    fn is_focus_scope(_this: &Self) -> bool {
        true
    }
}

impl StyledElementImpl for Window {
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        <Window as StaticType>::TYPE
    }
}

impl FerroObjectImpl for Window {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.initialize();
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::window_decorations_property().as_property() {
            let (_, typed_new_value) = change.get_old_and_new_value::<WindowDecorations>();

            if let Some(platform_impl) = this.platform_impl() {
                platform_impl.set_window_decorations(typed_new_value);
            }
            this.update_drawn_decorations();
        } else if change.property() == Self::window_decorations_theme_property().as_property() {
            this.update_drawn_decorations();
        } else if change.property() == WindowBase::owner_property().as_property() {
            let (old_parent, new_parent) = change.get_old_and_new_value::<Option<Ref<WindowBase>>>();
            let old_parent = old_parent.and_then(|parent| parent.cast::<Window>());
            let new_parent = new_parent.and_then(|parent| parent.cast::<Window>());

            if let Some(old_parent) = &old_parent {
                old_parent.remove_child(this);
            }
            if let Some(new_parent) = &new_parent {
                new_parent.add_child(this, this.showing_as_dialog.get());
            }

            if let Some(platform_impl) = this.platform_impl() {
                platform_impl.set_parent(new_parent.and_then(|parent| parent.platform_impl()));
            }
        } else if change.property() == Self::can_resize_property().as_property() {
            this.coerce_value(Self::can_maximize_property().as_property());
        }
    }
}

impl TemplatedControlImpl for Window {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);
        this.enable_visual_layer_manager_layers();
    }
}

impl LayoutableImpl for Window {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        let size_to_content = this.size_to_content();
        let mut client_size = this.client_size();
        if this.is_forced_decoration_mode.get() {
            client_size = this.platform_impl().map(|platform_impl| platform_impl.client_size()).unwrap_or(client_size);
            let inset = this.top_level_host().decoration_inset();
            client_size = Size::new(
                (client_size.width - inset.left - inset.right).max(0.0),
                (client_size.height - inset.top - inset.bottom).max(0.0),
            );
        }
        let mut max_auto_size =
            this.platform_impl().map(|platform_impl| platform_impl.max_auto_size_hint()).unwrap_or(Size::INFINITY);
        let use_auto_width = size_to_content.contains(SizeToContent::WIDTH);
        let use_auto_height = size_to_content.contains(SizeToContent::HEIGHT);

        let mut constraint = Size::new(
            if use_auto_width || available_size.width.is_infinite() { client_size.width } else { available_size.width },
            if use_auto_height || available_size.height.is_infinite() {
                client_size.height
            } else {
                available_size.height
            },
        );

        if this.max_width() > 0.0 && this.max_width() < max_auto_size.width {
            max_auto_size = max_auto_size.with_width(this.max_width());
        }
        if this.max_height() > 0.0 && this.max_height() < max_auto_size.height {
            max_auto_size = max_auto_size.with_height(this.max_height());
        }

        if use_auto_width {
            constraint = constraint.with_width(max_auto_size.width);
        }

        if use_auto_height {
            constraint = constraint.with_height(max_auto_size.height);
        }

        let mut result = Self::parent_measure_override(this, constraint);

        if !use_auto_width {
            if !available_size.width.is_infinite() {
                result = result.with_width(available_size.width);
            } else {
                result = result.with_width(client_size.width);
            }
        }

        if !use_auto_height {
            if !available_size.height.is_infinite() {
                result = result.with_height(available_size.height);
            } else {
                result = result.with_height(client_size.height);
            }
        }

        result
    }
}

impl TopLevelImpl for Window {
    fn handle_closed(this: &Self) {
        this.shown.set(false);

        Self::parent_handle_closed(this);
        *this.window_impl.borrow_mut() = None;

        this.raise_event(&RoutedEventArgs::with_event(Self::window_closed_event()));

        this.set_owner(None);
    }

    fn handle_resized(this: &Self, mut client_size: Size, reason: WindowResizeReason) {
        // In forced decoration mode, the platform's client size includes the decoration area.
        // Subtract the decoration inset so the window's client size reflects the usable content area.
        if this.is_forced_decoration_mode.get() {
            let inset = this.top_level_host().decoration_inset();
            client_size = Size::new(
                (client_size.width - inset.left - inset.right).max(0.0),
                (client_size.height - inset.top - inset.bottom).max(0.0),
            );
        }

        if this.can_handle_resized.get()
            && (this.client_size() != client_size || this.width().is_nan() || this.height().is_nan())
        {
            let mut size_to_content = this.size_to_content();

            // If auto-sizing is enabled, and the resize came from a user resize (or the reason was
            // unspecified) then turn off auto-resizing for any window dimension that is not equal
            // to the requested size.
            if size_to_content != SizeToContent::MANUAL
                && this.can_resize()
                && reason == WindowResizeReason::Unspecified
                || reason == WindowResizeReason::User
            {
                if client_size.width != this.client_size().width {
                    size_to_content &= !SizeToContent::WIDTH;
                }
                if client_size.height != this.client_size().height {
                    size_to_content &= !SizeToContent::HEIGHT;
                }
                this.set_size_to_content(size_to_content);
            }

            this.set_width(client_size.width);
            this.set_height(client_size.height);
        }

        Self::parent_handle_resized(this, client_size, reason);
    }
}

impl WindowBaseImpl for Window {
    /// Hides the window but does not close it.
    fn hide(this: &Self) {
        let _freeze = this.freeze_visibility_change_handling();

        if !this.shown.get() {
            return;
        }

        this.stop_rendering();

        let children = this.children.borrow().clone();
        for (child, _) in children {
            child.hide();
        }

        this.set_owner(None);
        if let Some(platform_impl) = this.platform_impl() {
            platform_impl.hide();
        }
        this.set_is_visible(false);

        let modal_subscription = this.modal_subscription.borrow().clone();
        if let Some(modal_subscription) = modal_subscription {
            modal_subscription.dispose();
        }
        this.shown.set(false);
    }

    /// Shows the window.
    ///
    /// # Panics
    /// Panics when the window has already been closed.
    fn show(this: &Self) {
        this.show_core(None, false);
    }

    fn arrange_set_bounds(this: &Self, size: Size) -> Size {
        this.arrange_bounds.set(size);
        if this.can_handle_resized.get() {
            this.resize_platform_impl(size, WindowResizeReason::Layout);
        }
        this.client_size()
    }

    fn is_visible_changed(this: &Self, e: &FerroPropertyChangedEventArgs<'_>) {
        if !this.ignore_visibility_changes() {
            let is_visible = e.get_new_value::<bool>();

            if this.shown.get() != is_visible {
                if !this.shown.get() {
                    this.show();
                } else {
                    this.hide();
                }
            }
        }
    }
}

impl WindowImpl for Window {
    fn handle_closing(this: &Self, reason: WindowCloseReason) -> bool {
        if !this.should_cancel_close(&WindowClosingEventArgs::new(reason, false)) {
            this.close_internal();
            return false;
        }

        true
    }

    fn on_closing(this: &Self, e: &WindowClosingEventArgs) {
        for (_, handler) in this.closing.snapshot().iter() {
            handler(e);
        }
    }
}

struct DialogCompletionState {
    result: Option<Option<BoxedValue>>,
    waker: Option<Waker>,
}

/// Completes once, with the result of a dialog, when the dialog closes.
#[derive(Clone)]
struct DialogCompletion(Rc<RefCell<DialogCompletionState>>);

impl DialogCompletion {
    fn new() -> Self {
        Self(Rc::new(RefCell::new(DialogCompletionState { result: None, waker: None })))
    }

    fn set_result(&self, result: Option<BoxedValue>) {
        let waker = {
            let mut state = self.0.borrow_mut();
            state.result = Some(result);
            state.waker.take()
        };
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

impl Future for DialogCompletion {
    type Output = Option<BoxedValue>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self.0.borrow_mut();
        match state.result.take() {
            Some(result) => Poll::Ready(result),
            None => {
                state.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

/// Closes the window when the scope of a close request ends, unless the
/// close was cancelled.
struct CloseScope<'a> {
    window: &'a Window,
    close: Cell<bool>,
    ignore_cancel: bool,
}

impl Drop for CloseScope<'_> {
    fn drop(&mut self) {
        if self.close.get() || self.ignore_cancel {
            self.window.close_internal();
        }
    }
}

ferroui_base::ferro_properties! { impl Window {
    ferro_property!(
        /// Defines the `SizeToContent` property.
        pub fn size_to_content_property() -> StyledProperty<SizeToContent> {
            FerroProperty::register::<Window, _>("SizeToContent", SizeToContent::MANUAL)
        }
    );

    ferro_property!(
        /// Defines the `ExtendClientAreaToDecorationsHint` property.
        pub fn extend_client_area_to_decorations_hint_property() -> StyledProperty<bool> {
            FerroProperty::register::<Window, _>("ExtendClientAreaToDecorationsHint", false)
        }
    );

    ferro_property!(
        /// Defines the `ExtendClientAreaTitleBarHeightHint` property.
        pub fn extend_client_area_title_bar_height_hint_property() -> StyledProperty<f64> {
            FerroProperty::register::<Window, _>("ExtendClientAreaTitleBarHeightHint", -1.0)
        }
    );

    ferro_property!(
        /// Defines the `IsExtendedIntoWindowDecorations` property.
        pub fn is_extended_into_window_decorations_property() -> DirectProperty<Window, bool> {
            FerroProperty::register_direct::<Window, _>(
                "IsExtendedIntoWindowDecorations",
                |o| o.is_extended_into_window_decorations(),
                None,
                false,
            )
        }
    );

    ferro_property!(
        /// Defines the `WindowDecorationMargin` property.
        pub fn window_decoration_margin_property() -> DirectProperty<Window, Thickness> {
            FerroProperty::register_direct::<Window, _>(
                "WindowDecorationMargin",
                |o| o.window_decoration_margin(),
                None,
                Thickness::default(),
            )
        }
    );

    ferro_property!(
        /// Defines the `OffScreenMargin` property.
        pub fn off_screen_margin_property() -> DirectProperty<Window, Thickness> {
            FerroProperty::register_direct::<Window, _>(
                "OffScreenMargin",
                |o| o.off_screen_margin(),
                None,
                Thickness::default(),
            )
        }
    );

    ferro_property!(
        /// Defines the `WindowDecorations` property.
        pub fn window_decorations_property() -> StyledProperty<WindowDecorations> {
            FerroProperty::register::<Window, _>("WindowDecorations", WindowDecorations::Full)
        }
    );

    ferro_property!(
        /// Defines the `WindowDecorationsTheme` property.
        pub fn window_decorations_theme_property() -> StyledProperty<Option<Ref<ControlTheme>>> {
            FerroProperty::register::<Window, _>("WindowDecorationsTheme", None)
        }
    );

    ferro_property!(
        /// Defines the `ShowActivated` property.
        pub fn show_activated_property() -> StyledProperty<bool> {
            FerroProperty::register::<Window, _>("ShowActivated", true)
        }
    );

    ferro_property!(
        /// Enables or disables the taskbar icon.
        pub fn show_in_taskbar_property() -> StyledProperty<bool> {
            FerroProperty::register::<Window, _>("ShowInTaskbar", true)
        }
    );

    ferro_property!(
        /// Defines the `ClosingBehavior` property.
        pub fn closing_behavior_property() -> StyledProperty<WindowClosingBehavior> {
            FerroProperty::register::<Window, _>("ClosingBehavior", WindowClosingBehavior::OwnerAndChildWindows)
        }
    );

    ferro_property!(
        /// Represents the currently effective window state (normal,
        /// minimized, maximized).
        pub fn window_state_property() -> DirectProperty<Window, WindowState> {
            FerroProperty::register_direct::<Window, _>(
                "WindowState",
                |o| o.window_state(),
                Some(|o, v| o.set_window_state(v)),
                WindowState::Normal,
            )
        }
    );

    ferro_property!(
        /// Defines the `Title` property.
        pub fn title_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<Window, _>("Title", Some("Window".to_string()))
        }
    );

    ferro_property!(
        /// Defines the `Icon` property.
        pub fn icon_property() -> StyledProperty<Option<Rc<WindowIcon>>> {
            FerroProperty::register::<Window, _>("Icon", None)
        }
    );

    ferro_property!(
        /// Defines the `WindowStartupLocation` property.
        pub fn window_startup_location_property() -> StyledProperty<WindowStartupLocation> {
            FerroProperty::register::<Window, _>("WindowStartupLocation", WindowStartupLocation::Manual)
        }
    );

    ferro_property!(
        /// Defines the `CanResize` property.
        pub fn can_resize_property() -> StyledProperty<bool> {
            FerroProperty::register::<Window, _>("CanResize", true)
        }
    );

    ferro_property!(
        /// Defines the `CanMinimize` property.
        pub fn can_minimize_property() -> StyledProperty<bool> {
            FerroProperty::register::<Window, _>("CanMinimize", true)
        }
    );

    ferro_property!(
        /// Defines the `CanMaximize` property.
        pub fn can_maximize_property() -> StyledProperty<bool> {
            FerroProperty::register_with::<Window, _>(
                "CanMaximize",
                StyledPropertyOptions::new(true).coerce(|target, value| {
                    value && !target.downcast_ref::<Window>().is_some_and(|window| !window.can_resize())
                }),
            )
        }
    );
} }

impl Window {
    ferro_routed_event!(
        /// Routed event that can be used for global tracking of window
        /// destruction.
        pub fn window_closed_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<Window, _>("WindowClosed", RoutingStrategies::DIRECT)
        }
    );

    ferro_routed_event!(
        /// Routed event that can be used for global tracking of opening
        /// windows.
        pub fn window_opened_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<Window, _>("WindowOpened", RoutingStrategies::DIRECT)
        }
    );

    fn static_constructor() {
        let white: Rc<dyn IBrush> = Brushes::white();
        TemplatedControl::background_property().override_default_value::<Window>(Some(white));
        let _ = Self::extend_client_area_title_bar_height_hint_property()
            .changed()
            .add_class_handler::<Window>(|w, _| w.on_title_bar_height_hint_changed());
    }

    /// Field initialisation of a window over the window implementation
    /// `platform_impl`.
    pub fn construct(platform_impl: Rc<dyn IWindowImpl>) -> Self {
        let window_base_impl: Rc<dyn IWindowBaseImpl> = platform_impl.clone();
        Self {
            base: WindowBase::construct(window_base_impl),
            window_impl: RefCell::new(Some(platform_impl)),
            children: RefCell::new(Vec::new()),
            is_extended_into_window_decorations: Cell::new(false),
            window_decoration_margin: Cell::new(Thickness::default()),
            off_screen_margin: Cell::new(Thickness::default()),
            can_handle_resized: Cell::new(false),
            arrange_bounds: Cell::new(Size::default()),
            is_forced_decoration_mode: Cell::new(false),
            dialog_result: RefCell::new(None),
            shown: Cell::new(false),
            showing_as_dialog: Cell::new(false),
            position_was_set: Cell::new(false),
            was_shown_before: Cell::new(false),
            modal_subscription: RefCell::new(None),
            allowed_window_actions: Cell::new(PlatformAllowedWindowActions::ALL),
            last_window_state: Cell::new(WindowState::Normal),
            closing: HandlerList::new(),
            allowed_window_actions_changed: HandlerList::new(),
        }
    }

    /// Creates a window over a window of the windowing platform.
    ///
    /// # Panics
    /// Panics when no windowing platform is registered.
    pub fn new() -> Ref<Self> {
        Self::with_impl(PlatformManager::create_window())
    }

    /// Creates a window over the window implementation `platform_impl`.
    pub fn with_impl(platform_impl: Rc<dyn IWindowImpl>) -> Ref<Self> {
        instantiate(Self::construct(platform_impl))
    }

    /// The constructor body.
    fn initialize(&self) {
        let platform_impl = self.platform_impl().expect("the window has a platform implementation");
        let weak = self.to_ref().downgrade();

        platform_impl.set_closing(Some(Rc::new({
            let weak = weak.clone();
            move |reason| weak.upgrade().is_some_and(|this| this.handle_closing(reason))
        })));
        platform_impl.set_got_input_when_disabled(Some(Rc::new({
            let weak = weak.clone();
            move || {
                if let Some(this) = weak.upgrade() {
                    this.on_got_input_when_disabled();
                }
            }
        })));
        platform_impl.set_window_state_changed(Some(Rc::new({
            let weak = weak.clone();
            move |state| {
                if let Some(this) = weak.upgrade() {
                    this.handle_window_state_changed(state);
                }
            }
        })));
        platform_impl.set_drawn_decorations_request_changed(Some(Rc::new({
            let weak = weak.clone();
            move || {
                if let Some(this) = weak.upgrade() {
                    this.update_drawn_decorations();
                }
            }
        })));
        platform_impl.set_extend_client_area_to_decorations_changed(Some(Rc::new({
            let weak = weak.clone();
            move |is_extended| {
                if let Some(this) = weak.upgrade() {
                    this.extend_client_area_to_decorations_changed(is_extended);
                }
            }
        })));
        platform_impl.set_allowed_window_actions_changed(Some(Rc::new({
            let weak = weak.clone();
            move |actions| {
                if let Some(this) = weak.upgrade() {
                    this.on_allowed_window_actions_changed(actions);
                }
            }
        })));
        self.allowed_window_actions.set(platform_impl.allowed_window_actions());

        // The first value is the current one; later ones are changes.
        let skip = Cell::new(true);
        let object: &FerroObject = self;
        let _ = FerroObjectExtensions::get_observable(object, TopLevel::client_size_property()).subscribe_fn({
            let weak = weak.clone();
            move |x: Size| {
                if skip.replace(false) {
                    return;
                }
                if let Some(this) = weak.upgrade() {
                    this.resize_platform_impl(x, WindowResizeReason::Application);
                }
            }
        });
        let _ = self.scaling_changed({
            let weak = weak.clone();
            move || {
                if let Some(this) = weak.upgrade() {
                    this.on_scaling_changed_update_decorations();
                }
            }
        });

        self.bind_impl(Self::title_property(), |_, platform_impl, title| platform_impl.set_title(title.as_deref()));
        self.bind_impl(Self::icon_property(), |this, _, icon| this.set_effective_icon(icon));
        self.bind_impl(Self::can_resize_property(), |_, platform_impl, can_resize| platform_impl.can_resize(can_resize));
        self.bind_impl(Self::can_minimize_property(), |_, platform_impl, can_minimize| {
            platform_impl.set_can_minimize(can_minimize)
        });
        self.bind_impl(Self::can_maximize_property(), |_, platform_impl, can_maximize| {
            platform_impl.set_can_maximize(can_maximize)
        });
        self.bind_impl(Self::show_in_taskbar_property(), |_, platform_impl, show| platform_impl.show_taskbar_icon(show));

        self.bind_impl(Self::extend_client_area_to_decorations_hint_property(), |_, platform_impl, hint| {
            platform_impl.set_extend_client_area_to_decorations_hint(hint)
        });
        self.bind_impl(Self::extend_client_area_title_bar_height_hint_property(), |_, platform_impl, height| {
            platform_impl.set_extend_client_area_title_bar_height_hint(height)
        });

        fn update_min_max_size(this: &Window, platform_impl: &Rc<dyn IWindowImpl>, _: f64) {
            platform_impl.set_min_max_size(
                Size::new(this.min_width(), this.min_height()),
                Size::new(this.max_width(), this.max_height()),
            );
        }
        self.bind_impl(Layoutable::min_width_property(), update_min_max_size);
        self.bind_impl(Layoutable::max_width_property(), update_min_max_size);
        self.bind_impl(Layoutable::min_height_property(), update_min_max_size);
        self.bind_impl(Layoutable::max_height_property(), update_min_max_size);
    }

    /// Forwards the value of `property` to the platform implementation, now
    /// and whenever it changes.
    fn bind_impl<T: PropertyValue>(
        &self,
        property: &'static StyledProperty<T>,
        on_value: impl Fn(&Window, &Rc<dyn IWindowImpl>, T) + 'static,
    ) {
        let weak = self.to_ref().downgrade();
        self.create_platform_impl_binding(property, move |value| {
            if let Some(this) = weak.upgrade() {
                if let Some(platform_impl) = this.platform_impl() {
                    on_value(&this, &platform_impl, value);
                }
            }
        });
    }

    /// The platform-specific window implementation; `None` once the window
    /// has closed.
    pub fn platform_impl(&self) -> Option<Rc<dyn IWindowImpl>> {
        WindowBase::platform_impl(self)?;
        self.window_impl.borrow().clone()
    }

    /// The child windows owned by this window.
    pub fn owned_windows(&self) -> Vec<Ref<Window>> {
        self.children.borrow().iter().map(|(child, _)| child.clone()).collect()
    }

    /// A value indicating how the window will size itself to fit its
    /// content.
    ///
    /// If the value is other than [`SizeToContent::MANUAL`], it is
    /// automatically set to [`SizeToContent::MANUAL`] if a user resizes the
    /// window by using the resize grip or dragging the border.
    ///
    /// NOTE: Because of a limitation of X11, the value will be reset on X11
    /// to [`SizeToContent::MANUAL`] on any resize - including the resize
    /// that happens when the window is first shown. This is because X11
    /// resize notifications are asynchronous and there is no way to know
    /// whether a resize came from the user or the layout system. To avoid
    /// this, consider setting `CanResize` to false, which will disable user
    /// resizing of the window.
    pub fn size_to_content(&self) -> SizeToContent {
        self.get_value(Self::size_to_content_property())
    }

    pub fn set_size_to_content(&self, value: SizeToContent) {
        self.set_value(Self::size_to_content_property(), value)
    }

    /// The title of the window.
    pub fn title(&self) -> Option<String> {
        self.get_value(Self::title_property())
    }

    pub fn set_title(&self, value: Option<String>) {
        self.set_value(Self::title_property(), value)
    }

    /// Whether the client area is extended into the window decorations
    /// (chrome or border).
    pub fn extend_client_area_to_decorations_hint(&self) -> bool {
        self.get_value(Self::extend_client_area_to_decorations_hint_property())
    }

    pub fn set_extend_client_area_to_decorations_hint(&self, value: bool) {
        self.set_value(Self::extend_client_area_to_decorations_hint_property(), value)
    }

    /// The title bar height hint for when the client area is extended. A
    /// value of -1 will cause the title bar to be auto sized to the OS
    /// default. Any other positive value will cause the title bar to assume
    /// that height.
    pub fn extend_client_area_title_bar_height_hint(&self) -> f64 {
        self.get_value(Self::extend_client_area_title_bar_height_hint_property())
    }

    pub fn set_extend_client_area_title_bar_height_hint(&self, value: f64) {
        self.set_value(Self::extend_client_area_title_bar_height_hint_property(), value)
    }

    /// Whether the client area is extended into the window decorations.
    pub fn is_extended_into_window_decorations(&self) -> bool {
        self.is_extended_into_window_decorations.get()
    }

    fn set_is_extended_into_window_decorations(&self, value: bool) {
        self.set_and_raise_cell(
            Self::is_extended_into_window_decorations_property(),
            &self.is_extended_into_window_decorations,
            value,
        );
    }

    /// The thickness around the window that is used by borders and the
    /// title bar.
    pub fn window_decoration_margin(&self) -> Thickness {
        self.window_decoration_margin.get()
    }

    fn set_window_decoration_margin(&self, value: Thickness) {
        self.set_and_raise_cell(Self::window_decoration_margin_property(), &self.window_decoration_margin, value);
    }

    /// The window margin that is hidden off the screen area. This is
    /// generally only the case on Windows when maximized, where the window
    /// border is hidden off the screen. This margin may be used to ensure
    /// user content doesn't overlap this space.
    pub fn off_screen_margin(&self) -> Thickness {
        self.off_screen_margin.get()
    }

    fn set_off_screen_margin(&self, value: Thickness) {
        self.set_and_raise_cell(Self::off_screen_margin_property(), &self.off_screen_margin, value);
    }

    /// The window decorations (title bar, border, etc).
    pub fn window_decorations(&self) -> WindowDecorations {
        self.get_value(Self::window_decorations_property())
    }

    pub fn set_window_decorations(&self, value: WindowDecorations) {
        self.set_value(Self::window_decorations_property(), value)
    }

    /// The theme used to render the window decorations when they are not
    /// drawn by the system.
    pub fn window_decorations_theme(&self) -> Option<Ref<ControlTheme>> {
        self.get_value(Self::window_decorations_theme_property())
    }

    pub fn set_window_decorations_theme(&self, value: impl Into<Nullable<ControlTheme>>) {
        self.set_value(Self::window_decorations_theme_property(), value.into().0)
    }

    #[deprecated(note = "Use window_decorations instead.")]
    pub fn system_decorations(&self) -> WindowDecorations {
        self.window_decorations()
    }

    #[deprecated(note = "Use set_window_decorations instead.")]
    pub fn set_system_decorations(&self, value: WindowDecorations) {
        self.set_window_decorations(value)
    }

    /// Whether a window is activated when first shown.
    pub fn show_activated(&self) -> bool {
        self.get_value(Self::show_activated_property())
    }

    pub fn set_show_activated(&self, value: bool) {
        self.set_value(Self::show_activated_property(), value)
    }

    /// Enables or disables the taskbar icon.
    pub fn show_in_taskbar(&self) -> bool {
        self.get_value(Self::show_in_taskbar_property())
    }

    pub fn set_show_in_taskbar(&self, value: bool) {
        self.set_value(Self::show_in_taskbar_property(), value)
    }

    /// A value indicating how the closing event behaves in the presence of
    /// child windows.
    pub fn closing_behavior(&self) -> WindowClosingBehavior {
        self.get_value(Self::closing_behavior_property())
    }

    pub fn set_closing_behavior(&self, value: WindowClosingBehavior) {
        self.set_value(Self::closing_behavior_property(), value)
    }

    /// Represents the currently effective window state (normal, minimized,
    /// maximized).
    pub fn window_state(&self) -> WindowState {
        match self.platform_impl() {
            Some(platform_impl) if platform_impl.window_state_getter_is_usable() => platform_impl.window_state(),
            _ => self.last_window_state.get(),
        }
    }

    pub fn set_window_state(&self, value: WindowState) {
        if let Some(platform_impl) = self.platform_impl() {
            if platform_impl.window_state_getter_is_usable() {
                // Attempt to set the window state to desired value, if it succeeds the platform will
                // trigger the window state changed callback which will set and raise the property
                platform_impl.set_window_state(value);

                // If the request was refused - trigger a synthetic property change notification
                // for data bindings and user state to fix itself.
                if platform_impl.window_state() != value {
                    // Since it's a force notify, we aren't checking for the old value and sometimes
                    // trigger notification with old value = new value.
                    let old_value = self.last_window_state.get();
                    self.last_window_state.set(platform_impl.window_state());
                    self.raise_direct_property_changed(
                        Self::window_state_property(),
                        &old_value,
                        &self.last_window_state.get(),
                    );
                }
            } else {
                // Legacy behavior - update the property and hope for the best that the platform
                // will update it back to match the actual window state
                self.set_and_raise_cell(Self::window_state_property(), &self.last_window_state, value);
                platform_impl.set_window_state(self.last_window_state.get());
            }
        }
    }

    /// Enables or disables resizing of the window.
    pub fn can_resize(&self) -> bool {
        self.get_value(Self::can_resize_property())
    }

    pub fn set_can_resize(&self, value: bool) {
        self.set_value(Self::can_resize_property(), value)
    }

    /// Enables or disables minimizing the window.
    ///
    /// This property might be ignored by some window managers on Linux.
    pub fn can_minimize(&self) -> bool {
        self.get_value(Self::can_minimize_property())
    }

    pub fn set_can_minimize(&self, value: bool) {
        self.set_value(Self::can_minimize_property(), value)
    }

    /// Enables or disables maximizing the window.
    ///
    /// When `CanResize` is false, this property is always false. On macOS,
    /// setting this property to false also disables the full screen mode.
    /// This property might be ignored by some window managers on Linux.
    pub fn can_maximize(&self) -> bool {
        self.get_value(Self::can_maximize_property())
    }

    pub fn set_can_maximize(&self, value: bool) {
        self.set_value(Self::can_maximize_property(), value)
    }

    /// The window actions currently allowed by the underlying platform
    /// (internal upstream: for the window decorations).
    pub fn allowed_window_actions(&self) -> PlatformAllowedWindowActions {
        self.allowed_window_actions.get()
    }

    /// The icon of the window.
    pub fn icon(&self) -> Option<Rc<WindowIcon>> {
        self.get_value(Self::icon_property())
    }

    pub fn set_icon(&self, value: Option<Rc<WindowIcon>>) {
        self.set_value(Self::icon_property(), value)
    }

    /// The startup location of the window.
    pub fn window_startup_location(&self) -> WindowStartupLocation {
        self.get_value(Self::window_startup_location_property())
    }

    pub fn set_window_startup_location(&self, value: WindowStartupLocation) {
        self.set_value(Self::window_startup_location_property(), value)
    }

    /// The window position in screen coordinates.
    pub fn position(&self) -> PixelPoint {
        self.platform_impl().map(|platform_impl| platform_impl.position()).unwrap_or_default()
    }

    pub fn set_position(&self, value: PixelPoint) {
        if let Some(platform_impl) = self.platform_impl() {
            platform_impl.move_(value);
        }
        self.position_was_set.set(true);
    }

    /// Whether this window was opened as a dialog.
    pub fn is_dialog(&self) -> bool {
        self.showing_as_dialog.get()
    }

    /// Starts moving a window with left button being held. Should be called
    /// from a left mouse button press event handler.
    pub fn begin_move_drag(&self, e: &PointerPressedEventArgs) {
        if let Some(platform_impl) = self.platform_impl() {
            platform_impl.begin_move_drag(e);
        }
    }

    /// Starts resizing a window. This function is used if an application
    /// has window resizing controls. Should be called from a left mouse
    /// button press event handler.
    pub fn begin_resize_drag(&self, edge: WindowEdge, e: &PointerPressedEventArgs) {
        if let Some(platform_impl) = self.platform_impl() {
            platform_impl.begin_resize_drag(edge, e);
        }
    }

    /// Fired before a window is closed. Disposing the returned handle
    /// unsubscribes.
    pub fn closing(&self, handler: impl Fn(&WindowClosingEventArgs) + 'static) -> Rc<dyn IDisposable> {
        let token = self.closing.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.closing.remove(token);
            }
        })
    }

    /// Closes the window.
    pub fn close(&self) {
        self.close_core(WindowCloseReason::WindowClosing, true, false);
    }

    /// Closes a dialog window with the specified result.
    ///
    /// When the window is shown with [`show_dialog`](Self::show_dialog) or
    /// [`show_dialog_typed`](Self::show_dialog_typed), the resulting task
    /// will produce the `dialog_result` value when the window is closed.
    pub fn close_with_result(&self, dialog_result: Option<BoxedValue>) {
        *self.dialog_result.borrow_mut() = dialog_result;
        self.close_core(WindowCloseReason::WindowClosing, true, false);
    }

    /// Closes the window for `reason`; with `ignore_cancel` the window
    /// closes even when a closing handler cancels (internal upstream: for
    /// the application lifetime).
    pub fn close_core(&self, reason: WindowCloseReason, is_programmatic: bool, ignore_cancel: bool) {
        let scope = CloseScope { window: self, close: Cell::new(true), ignore_cancel };

        if self.should_cancel_close(&WindowClosingEventArgs::new(reason, is_programmatic)) {
            scope.close.set(false);
        }
    }

    fn close_internal(&self) {
        let children = self.children.borrow().clone();
        for (child, _) in children {
            child.close_internal();
        }

        if let Some(platform_impl) = self.platform_impl() {
            platform_impl.dispose();
        }

        self.showing_as_dialog.set(false);

        self.set_owner(None);
    }

    fn should_cancel_close(&self, args: &WindowClosingEventArgs) -> bool {
        match self.closing_behavior() {
            WindowClosingBehavior::OwnerAndChildWindows => {
                let mut can_close = true;

                let children = self.children.borrow().clone();
                if !children.is_empty() {
                    let owner_closing_args;
                    let child_args = if args.close_reason() == WindowCloseReason::WindowClosing {
                        owner_closing_args =
                            WindowClosingEventArgs::new(WindowCloseReason::OwnerWindowClosing, args.is_programmatic());
                        &owner_closing_args
                    } else {
                        args
                    };

                    for (child, _) in children {
                        if child.should_cancel_close(child_args) {
                            can_close = false;
                        }
                    }
                }

                if can_close {
                    self.on_closing(args);

                    return args.cancel();
                }

                true
            }
            WindowClosingBehavior::OwnerWindowOnly => {
                self.on_closing(args);

                args.cancel()
            }
        }
    }

    fn handle_window_state_changed(&self, state: WindowState) {
        // Check if platform impl doesn't lie about the window state getter being usable
        debug_assert!(self
            .platform_impl()
            .is_none_or(|platform_impl| !platform_impl.window_state_getter_is_usable()
                || platform_impl.window_state() == state));

        self.set_and_raise_cell(Self::window_state_property(), &self.last_window_state, state);

        if state == WindowState::Minimized {
            self.stop_rendering();
        } else {
            self.start_rendering();
        }

        // Update decoration parts and fullscreen popover state for the new window state
        self.update_drawn_decoration_parts();
    }

    /// Fired when the window actions allowed by the platform change
    /// (internal upstream: for the window decorations). Disposing the
    /// returned handle unsubscribes.
    pub fn allowed_window_actions_changed(
        &self,
        handler: impl Fn(PlatformAllowedWindowActions) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.allowed_window_actions_changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.allowed_window_actions_changed.remove(token);
            }
        })
    }

    fn on_allowed_window_actions_changed(&self, actions: PlatformAllowedWindowActions) {
        self.allowed_window_actions.set(actions);
        for (_, handler) in self.allowed_window_actions_changed.snapshot().iter() {
            handler(actions);
        }
    }

    fn extend_client_area_to_decorations_changed(&self, is_extended: bool) {
        self.set_is_extended_into_window_decorations(is_extended);
        self.set_off_screen_margin(
            self.platform_impl().map(|platform_impl| platform_impl.off_screen_margin()).unwrap_or_default(),
        );

        self.update_drawn_decorations();
    }

    fn update_drawn_decorations(&self) {
        let parts = self.compute_decoration_parts();

        // Detect forced mode: platform needs managed decorations but app hasn't opted in
        self.is_forced_decoration_mode.set(parts.is_some() && !self.is_extended_into_window_decorations());

        self.top_level_host().update_drawn_decorations(parts, self.window_state(), self.window_decorations_theme());

        if parts.is_some() {
            // Forward the title bar height hint to the title bar height of the decorations
            if let Some(decorations) = self.top_level_host().decorations() {
                decorations.set_render_scaling(self.render_scaling());

                let hint = self.extend_client_area_title_bar_height_hint();
                if hint >= 0.0 {
                    decorations.set_title_bar_height_override(hint);
                }
            }
        }

        self.update_drawn_decoration_margins();
    }

    fn on_scaling_changed_update_decorations(&self) {
        if let Some(decorations) = self.top_level_host().decorations() {
            decorations.set_render_scaling(self.render_scaling());
        }
    }

    /// Updates decoration parts based on current window state without
    /// re-creating the decorations instance.
    fn update_drawn_decoration_parts(&self) {
        if self.top_level_host().decorations().is_none() {
            return;
        }

        self.top_level_host().update_drawn_decorations(
            self.compute_decoration_parts(),
            self.window_state(),
            self.window_decorations_theme(),
        );
    }

    fn compute_decoration_parts(&self) -> Option<crate::chrome::DrawnWindowDecorationParts> {
        use crate::chrome::DrawnWindowDecorationParts;
        use crate::platform::PlatformRequestedDrawnDecoration;

        let platform_impl = self.platform_impl();
        if !platform_impl.as_ref().is_some_and(|platform_impl| platform_impl.needs_managed_decorations()) {
            return None;
        }

        let platform_needs = platform_impl
            .map(|platform_impl| platform_impl.requested_drawn_decorations())
            .unwrap_or(PlatformRequestedDrawnDecoration::NONE);
        let mut parts = DrawnWindowDecorationParts::NONE;
        if self.window_decorations() != WindowDecorations::None {
            if platform_needs.contains(PlatformRequestedDrawnDecoration::TITLE_BAR)
                && self.window_decorations() == WindowDecorations::Full
            {
                parts |= DrawnWindowDecorationParts::TITLE_BAR;
            }
            if platform_needs.contains(PlatformRequestedDrawnDecoration::SHADOW) {
                parts |= DrawnWindowDecorationParts::SHADOW;
            }
            if platform_needs.contains(PlatformRequestedDrawnDecoration::BORDER) {
                parts |= DrawnWindowDecorationParts::BORDER;
            }
            if platform_needs.contains(PlatformRequestedDrawnDecoration::RESIZE_GRIPS) && self.can_resize() {
                parts |= DrawnWindowDecorationParts::RESIZE_GRIPS;
            }

            // In fullscreen: no shadow, border, resize grips, or titlebar (popover takes over)
            if self.window_state() == WindowState::FullScreen {
                parts &= !(DrawnWindowDecorationParts::SHADOW
                    | DrawnWindowDecorationParts::BORDER
                    | DrawnWindowDecorationParts::RESIZE_GRIPS
                    | DrawnWindowDecorationParts::TITLE_BAR);
            }
            // In maximized: no shadow, border, or resize grips (titlebar stays)
            else if self.window_state() == WindowState::Maximized {
                parts &= !(DrawnWindowDecorationParts::SHADOW
                    | DrawnWindowDecorationParts::BORDER
                    | DrawnWindowDecorationParts::RESIZE_GRIPS);
            }
        }

        Some(parts)
    }

    fn on_title_bar_height_hint_changed(&self) {
        let Some(decorations) = self.top_level_host().decorations() else { return };

        decorations.set_title_bar_height_override(self.extend_client_area_title_bar_height_hint());

        self.update_drawn_decoration_margins();
    }

    /// Called by the host of the window when the effective geometry of the
    /// decorations changes (e.g. the theme changes the default values, or
    /// the enabled parts change).
    pub(crate) fn on_drawn_decorations_geometry_changed(&self) {
        self.update_drawn_decoration_margins();
    }

    fn update_drawn_decoration_margins(&self) {
        if let Some(decorations) = self.top_level_host().decorations() {
            use crate::chrome::DrawnWindowDecorationParts;

            let parts = decorations.enabled_parts();
            let title_bar_height =
                if parts.contains(DrawnWindowDecorationParts::TITLE_BAR) { decorations.title_bar_height() } else { 0.0 };
            let frame = if parts.contains(DrawnWindowDecorationParts::BORDER) {
                decorations.frame_thickness()
            } else {
                Thickness::default()
            };
            let shadow = if parts.contains(DrawnWindowDecorationParts::SHADOW) {
                decorations.shadow_thickness()
            } else {
                Thickness::default()
            };

            if let Some(platform_impl) = self.platform_impl() {
                platform_impl.set_shadow_extents(shadow);
            }

            let margin = Thickness::new(
                frame.left + shadow.left,
                title_bar_height + frame.top + shadow.top,
                frame.right + shadow.right,
                frame.bottom + shadow.bottom,
            );

            if self.is_forced_decoration_mode.get() {
                // In forced mode, app is unaware of decorations.
                // The host insets the window child; the window decoration margin stays zero.
                self.set_window_decoration_margin(Thickness::default());
                self.top_level_host().set_decoration_inset(margin);
            } else {
                // In extended mode, app handles the margin itself.
                self.set_window_decoration_margin(margin);
                self.top_level_host().set_decoration_inset(Thickness::default());
            }
            return;
        }

        // Only use platform margins if drawn decorations are not active
        let platform_impl = self.platform_impl();
        self.set_window_decoration_margin(
            platform_impl.as_ref().map(|platform_impl| platform_impl.extended_margins()).unwrap_or_default(),
        );
        self.top_level_host().set_decoration_inset(Thickness::default());
        if let Some(platform_impl) = platform_impl {
            platform_impl.set_shadow_extents(Thickness::default());
        }
    }

    /// Shows the window as a child of `owner`.
    ///
    /// # Panics
    /// Panics when the window has already been closed, and when the owner
    /// is closed, not visible or the window itself.
    pub fn show_with_owner(&self, owner: &Window) {
        self.show_core(Some(owner), false);
    }

    fn ensure_state_before_show(&self) {
        if self.platform_impl().is_none() {
            panic!("Cannot re-show a closed window.");
        }
    }

    fn ensure_parent_state_before_show(&self, owner: &Window) {
        if owner.platform_impl().is_none() {
            panic!("Cannot show a window with a closed owner.");
        }

        if std::ptr::eq(owner, self) {
            panic!("A Window cannot be its own owner.");
        }

        if !owner.is_visible() {
            panic!("Cannot show window with non-visible owner.");
        }
    }

    fn show_core(&self, owner: Option<&Window>, modal: bool) -> Option<DialogCompletion> {
        let _freeze = self.freeze_visibility_change_handling();

        self.ensure_state_before_show();

        if modal && owner.is_none() {
            panic!("A dialog requires an owner.");
        }
        if let Some(owner) = owner {
            self.ensure_parent_state_before_show(owner);
        }

        if self.shown.get() {
            if modal {
                panic!("The window is already being shown.");
            }
            return None;
        }

        self.showing_as_dialog.set(modal);
        self.raise_event(&RoutedEventArgs::with_event(Self::window_opened_event()));

        self.ensure_initialized();
        self.apply_styling();

        // Enable drawn decorations before layout so margins are computed
        self.update_drawn_decorations();

        // In forced mode, adjust the client size to reflect usable content area
        if self.is_forced_decoration_mode.get() {
            let inset = self.top_level_host().decoration_inset();
            self.set_client_size(Size::new(
                (self.client_size().width - inset.left - inset.right).max(0.0),
                (self.client_size().height - inset.top - inset.bottom).max(0.0),
            ));
        }

        self.shown.set(true);
        self.set_is_visible(true);

        self.set_effective_icon(self.icon());

        // If window position was not set before then platform may provide incorrect scaling at this time,
        // but we need it for proper calculation of position and in some cases size (size to content)
        self.set_expected_scaling(owner);

        let mut initial_size = Size::new(
            if self.width().is_nan() { self.client_size().width } else { self.width() },
            if self.height().is_nan() { self.client_size().height } else { self.height() },
        );

        let min_max = MinMax::new(self);

        initial_size = Size::new(
            MathUtilities::clamp(initial_size.width, min_max.min_width, min_max.max_width),
            MathUtilities::clamp(initial_size.height, min_max.min_height, min_max.max_height),
        );

        let mut client_size_changed = initial_size != self.client_size();
        self.set_client_size(initial_size); // The client size is required for measure and arrange

        // this will call arrange_set_bounds
        self.layout_manager().execute_initial_layout_pass();

        if self.size_to_content().contains(SizeToContent::WIDTH) {
            initial_size = initial_size.with_width(MathUtilities::clamp(
                self.arrange_bounds.get().width,
                min_max.min_width,
                min_max.max_width,
            ));
            client_size_changed |= initial_size != self.client_size();
            self.set_client_size(initial_size);
        }

        if self.size_to_content().contains(SizeToContent::HEIGHT) {
            initial_size = initial_size.with_height(MathUtilities::clamp(
                self.arrange_bounds.get().height,
                min_max.min_height,
                min_max.max_height,
            ));
            client_size_changed |= initial_size != self.client_size();
            self.set_client_size(initial_size);
        }

        self.set_owner(owner.map(|owner| owner.to_ref().upcast()));

        self.apply_window_startup_location(owner);

        self.set_desktop_scaling_override(None);

        // In forced mode, compare against adjusted platform size
        let platform_client_size =
            self.platform_impl().map(|platform_impl| platform_impl.client_size()).unwrap_or_default();
        let comparable_client_size = if self.is_forced_decoration_mode.get() {
            let inset = self.top_level_host().decoration_inset();
            Size::new(
                (platform_client_size.width - inset.left - inset.right).max(0.0),
                (platform_client_size.height - inset.top - inset.bottom).max(0.0),
            )
        } else {
            platform_client_size
        };

        if client_size_changed || self.client_size() != comparable_client_size {
            // Previously it was called before the initial layout pass
            self.resize_platform_impl(self.client_size(), WindowResizeReason::Layout);

            // we do not want the platform resize to trigger handle_resized yet because it will set the width
            // and height. So perform some important actions from handle_resized

            self.renderer().resized(self.client_size());
            self.on_resized(&WindowResizedEventArgs::new(self.client_size(), WindowResizeReason::Layout));

            if !self.width().is_nan() {
                self.set_width(self.client_size().width);
            }
            if !self.height().is_nan() {
                self.set_height(self.client_size().height);
            }
        }

        self.set_frame_size(self.platform_impl().and_then(|platform_impl| platform_impl.frame_size()));

        self.can_handle_resized.set(true);

        self.start_rendering();
        if let Some(platform_impl) = self.platform_impl() {
            platform_impl.show(self.show_activated(), modal);
        }

        let mut result = None;
        if let (true, Some(owner)) = (modal, owner) {
            let completion = DialogCompletion::new();
            let weak = self.to_ref().downgrade();

            let taken = Cell::new(false);
            let closed_subscription = TopLevel::closed(self, {
                let weak = weak.clone();
                move || {
                    if taken.replace(true) {
                        return;
                    }
                    let modal_subscription =
                        weak.upgrade().and_then(|this| this.modal_subscription.borrow().clone());
                    if let Some(modal_subscription) = modal_subscription {
                        modal_subscription.dispose();
                    }
                }
            });
            let complete = Disposable::create({
                let owner = owner.to_ref().downgrade();
                let completion = completion.clone();
                move || {
                    let this = weak.upgrade();
                    if let Some(this) = &this {
                        *this.modal_subscription.borrow_mut() = None;
                    }
                    if let Some(owner) = owner.upgrade() {
                        owner.activate();
                    }
                    completion.set_result(this.and_then(|this| this.dialog_result.borrow().clone()));
                }
            });

            let disposables: Rc<dyn IDisposable> =
                Rc::new(CompositeDisposable::from_disposables([closed_subscription, complete]));
            *self.modal_subscription.borrow_mut() = Some(disposables);
            result = Some(completion);
        }

        self.on_opened();
        if !modal {
            self.was_shown_before.set(true);
        }

        result
    }

    fn resize_platform_impl(&self, mut size: Size, reason: WindowResizeReason) {
        let Some(platform_impl) = self.platform_impl() else { return };

        // In forced mode, add decoration inset so platform gets full frame size
        if self.is_forced_decoration_mode.get() {
            let inset = self.top_level_host().decoration_inset();
            size = Size::new(size.width + inset.left + inset.right, size.height + inset.top + inset.bottom);
            if platform_impl.client_size() != size {
                platform_impl.resize(size, reason);
            }
        } else {
            platform_impl.resize(size, reason);
        }
    }

    /// Shows the window as a dialog owned by `owner`.
    ///
    /// Returns a task that can be used to track the lifetime of the dialog:
    /// it completes, from a dispatcher job, with the dialog result (see
    /// [`close_with_result`](Self::close_with_result)) once the dialog has
    /// closed. Nothing blocks: the caller keeps running and the task is
    /// observed through the dispatcher.
    ///
    /// # Panics
    /// Panics when the window has already been closed or is already shown,
    /// and when the owner is closed, not visible or the window itself.
    pub fn show_dialog(&self, owner: &Window) -> DispatcherTask<Option<BoxedValue>> {
        let completion = self.show_core(Some(owner), true).expect("showing a dialog creates its completion");
        Dispatcher::ui_thread().invoke_async_task_local(move || completion)
    }

    /// Shows the window as a dialog owned by `owner`; `TResult` is the type
    /// of the result produced by the dialog.
    ///
    /// Returns a task that can be used to retrieve the result of the dialog
    /// when it closes: the value passed to
    /// [`close_with_result`](Self::close_with_result), or the default of
    /// `TResult` when the dialog closed without a result. The task fails
    /// when the dialog result is not a `TResult`.
    ///
    /// # Panics
    /// See [`show_dialog`](Self::show_dialog).
    pub fn show_dialog_typed<TResult: Clone + Default + 'static>(&self, owner: &Window) -> DispatcherTask<TResult> {
        let completion = self.show_core(Some(owner), true).expect("showing a dialog creates its completion");
        Dispatcher::ui_thread().invoke_async_task_local(move || async move {
            match completion.await {
                Some(dialog_result) => match dialog_result.downcast_ref::<TResult>() {
                    Some(dialog_result) => dialog_result.clone(),
                    None => panic!(
                        "Unable to cast the dialog result of type {} to {}.",
                        dialog_result.any_value_type_name(),
                        std::any::type_name::<TResult>()
                    ),
                },
                None => TResult::default(),
            }
        })
    }

    /// Sorts the windows ascending by their Z order - the topmost window
    /// will be the last in the list.
    ///
    /// # Panics
    /// Panics when one of the windows is closed, and when no windowing
    /// platform is registered.
    pub fn sort_windows_by_z_order(windows: &mut [Ref<Window>]) {
        if windows.len() <= 1 {
            return;
        }

        let platform = FerroLocator::current().get_required_service::<dyn IWindowingPlatform>();

        let window_impls: Vec<Rc<dyn IWindowImpl>> = windows
            .iter()
            .enumerate()
            .map(|(i, window)| window.platform_impl().unwrap_or_else(|| panic!("Invalid window at index {i}")))
            .collect();

        let mut z_order = vec![0i64; windows.len()];
        platform.get_windows_z_order(&window_impls, &mut z_order);

        let mut keyed: Vec<(i64, Ref<Window>)> = z_order.into_iter().zip(windows.iter().cloned()).collect();
        keyed.sort_by_key(|(z, _)| *z);
        for (slot, (_, window)) in windows.iter_mut().zip(keyed) {
            *slot = window;
        }
    }

    fn update_enabled(&self) {
        let is_enabled = !self.children.borrow().iter().any(|(_, is_dialog)| *is_dialog);

        if let Some(platform_impl) = self.platform_impl() {
            platform_impl.set_enabled(is_enabled);
        }
    }

    fn add_child(&self, window: &Window, is_dialog: bool) {
        self.children.borrow_mut().push((window.to_ref(), is_dialog));
        self.update_enabled();
    }

    fn remove_child(&self, window: &Window) {
        self.children.borrow_mut().retain(|(child, _)| !std::ptr::eq(&**child, window));

        self.update_enabled();
    }

    fn on_got_input_when_disabled(&self) {
        let first_dialog_child =
            self.children.borrow().iter().find(|(_, is_dialog)| *is_dialog).map(|(child, _)| child.clone());

        if let Some(first_dialog_child) = first_dialog_child {
            first_dialog_child.on_got_input_when_disabled();
        } else {
            self.activate();
        }
    }

    fn set_expected_scaling(&self, owner: Option<&Window>) {
        if self.was_shown_before.get() {
            return;
        }

        let location = self.get_effective_window_startup_location(owner);

        match location {
            WindowStartupLocation::CenterOwner => {
                self.set_desktop_scaling_override(owner.map(|owner| owner.desktop_scaling()));
            }
            WindowStartupLocation::CenterScreen => {
                let scaling = owner
                    .map(|owner| owner.desktop_scaling())
                    .or_else(|| self.screens().screen_from_point(self.position()).map(|screen| screen.scaling()))
                    .or_else(|| self.screens().primary().map(|screen| screen.scaling()));
                self.set_desktop_scaling_override(scaling);
            }
            WindowStartupLocation::Manual => {
                self.set_desktop_scaling_override(
                    self.screens().screen_from_point(self.position()).map(|screen| screen.scaling()),
                );
            }
        }
    }

    fn get_effective_window_startup_location(&self, owner: Option<&Window>) -> WindowStartupLocation {
        let mut startup_location = self.window_startup_location();

        if startup_location == WindowStartupLocation::CenterOwner
            && owner.is_none_or(|owner| owner.window_state() == WindowState::Minimized)
        {
            // If startup location is CenterOwner, but owner is null or minimized then fall back
            // to CenterScreen. This behavior is consistent with WPF.
            startup_location = WindowStartupLocation::CenterScreen;
        }

        startup_location
    }

    fn apply_window_startup_location(&self, owner: Option<&Window>) {
        if self.was_shown_before.get() {
            return;
        }

        let startup_location = self.get_effective_window_startup_location(owner);

        // Use frame size, falling back to client size if the platform can't give it to us.
        let platform_frame = self
            .platform_impl()
            .and_then(|platform_impl| platform_impl.frame_size().map(|frame_size| (frame_size, platform_impl.client_size())));
        let rect = match platform_frame {
            Some((frame_size, platform_client_size)) => {
                // Platform may calculate the frame size with incorrect scaling, so do not trust the value.
                let diff = frame_size - platform_client_size;
                PixelRect::from_size(PixelSize::from_size(self.client_size() + diff, self.desktop_scaling()))
            }
            None => PixelRect::from_size(PixelSize::from_size(self.client_size(), self.desktop_scaling())),
        };

        let apply_screen_constraint = |screen: Option<&Rc<Screen>>, mut child_rect: PixelRect| -> PixelRect {
            if let Some(constraint) = screen.map(|screen| screen.working_area()) {
                let max_x = constraint.right() - rect.width;
                let max_y = constraint.bottom() - rect.height;

                if constraint.x <= max_x {
                    child_rect = child_rect.with_x(MathUtilities::clamp_i32(child_rect.x, constraint.x, max_x));
                }
                if constraint.y <= max_y {
                    child_rect = child_rect.with_y(MathUtilities::clamp_i32(child_rect.y, constraint.y, max_y));
                }
            }

            child_rect
        };

        if startup_location == WindowStartupLocation::CenterScreen {
            let mut screen: Option<Rc<Screen>> = None;

            if let Some(owner) = owner {
                screen = self
                    .screens()
                    .screen_from_window(owner)
                    .or_else(|| self.screens().screen_from_point(owner.position()));
            }

            let screen = screen
                .or_else(|| self.screens().screen_from_point(self.position()))
                .or_else(|| self.screens().primary());

            if let Some(screen) = screen {
                let mut child_rect = screen.working_area().center_rect(rect);

                if self.screens().screen_from_point(child_rect.position()).is_none() {
                    child_rect = apply_screen_constraint(Some(&screen), child_rect);
                }

                self.set_position(child_rect.position());
            }
        } else if startup_location == WindowStartupLocation::CenterOwner {
            let owner = owner.expect("the effective startup location is the owner's center only with an owner");
            let owner_size = owner.frame_size().unwrap_or_else(|| owner.client_size());
            let owner_rect = PixelRect::from_position_size(
                owner.position(),
                PixelSize::from_size(owner_size, owner.desktop_scaling()),
            );
            let mut child_rect = owner_rect.center_rect(rect);

            let screen = self.screens().screen_from_window(owner);

            child_rect = apply_screen_constraint(screen.as_ref(), child_rect);

            self.set_position(child_rect.position());
        }

        // Platform returns incorrect scaling, forcing setting position may fix it
        if !self.position_was_set.get() {
            if let Some(platform_impl) = self.platform_impl() {
                if self.desktop_scaling() != platform_impl.desktop_scaling() {
                    platform_impl.move_(self.position());
                }
            }
        }
    }

    fn set_effective_icon(&self, icon: Option<Rc<WindowIcon>>) {
        if let Some(platform_impl) = self.platform_impl() {
            platform_impl.set_icon(icon.map(|icon| icon.platform_impl()));
        }
    }
}
