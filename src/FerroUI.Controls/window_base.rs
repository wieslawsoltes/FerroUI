use crate::platform::{ITopLevelImpl, IWindowBaseImpl};
use crate::primitives::TemplatedControlImpl;
use crate::{
    ContentControlImpl, ControlImpl, PixelPointEventArgs, Screens, TopLevel, TopLevelImpl, TopLevelImplExt,
    WindowResizeReason, WindowResizedEventArgs,
};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{ILayoutManager, LayoutHelper, LayoutableImpl};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::styling::Container;
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, DirectProperty, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, IFerroDependencyResolver, PixelPoint, Rect,
    Ref, Size, StyledElementImpl, StyledProperty, Visual, VisualImpl, WeakRef,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Base class for top-level windows.
///
/// This class acts as a base for top level windows such as `Window` and
/// `PopupRoot`. It handles scheduling layout, styling and rendering as well
/// as tracking the window client size and window activation.
#[repr(C)]
pub struct WindowBase {
    base: TopLevel,
    window_base_impl: RefCell<Option<Rc<dyn IWindowBaseImpl>>>,
    /// Whether the single-argument constructor was used: it binds `Topmost`
    /// to the platform implementation and takes its frame size.
    default_resolver: Cell<bool>,
    has_executed_initial_layout_pass: Cell<bool>,
    is_active: Cell<bool>,
    ignore_visibility_changes: Cell<i32>,
    owner: RefCell<Option<WeakRef<WindowBase>>>,
    desktop_scaling_override: Cell<Option<f64>>,
    activated: HandlerList<dyn Fn()>,
    deactivated: HandlerList<dyn Fn()>,
    position_changed: HandlerList<dyn Fn(&PixelPointEventArgs)>,
    resized: HandlerList<dyn Fn(&WindowResizedEventArgs)>,
}

ferro_class! {
    WindowBase: TopLevel, virtuals WindowBaseImpl: TopLevelImpl {
        /// Hides the window.
        fn hide(this);
        /// Shows the window.
        fn show(this);
        /// Raises the `Resized` event.
        fn on_resized(this, e: &WindowResizedEventArgs);
        /// Called during the arrange pass to set the size of the window;
        /// returns the size to arrange the content with.
        fn arrange_set_bounds(this, size: Size) -> Size;
        /// Called when the `IsVisible` property changes.
        fn is_visible_changed(this, e: &FerroPropertyChangedEventArgs<'_>);
    }
}

ferro_impl_classes!(
    WindowBase: StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);

impl FerroObjectImpl for WindowBase {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        let window_base_impl = this.window_base_impl.borrow().clone().expect("the window has a platform implementation");
        let weak = this.to_ref().downgrade();
        window_base_impl.set_activated(Some(Rc::new({
            let weak = weak.clone();
            move || {
                if let Some(this) = weak.upgrade() {
                    this.handle_activated();
                }
            }
        })));
        window_base_impl.set_deactivated(Some(Rc::new({
            let weak = weak.clone();
            move || {
                if let Some(this) = weak.upgrade() {
                    this.handle_deactivated();
                    this.raise_platform_deactivated();
                }
            }
        })));
        window_base_impl.set_position_changed(Some(Rc::new({
            let weak = weak.clone();
            move |pos| {
                if let Some(this) = weak.upgrade() {
                    this.handle_position_changed(pos);
                    this.raise_platform_position_changed(pos);
                }
            }
        })));

        if this.default_resolver.get() {
            this.create_platform_impl_binding(Self::topmost_property(), move |topmost: bool| {
                if let Some(platform_impl) = weak.upgrade().and_then(|this| this.platform_impl()) {
                    platform_impl.set_topmost(topmost);
                }
            });
            this.set_frame_size(window_base_impl.frame_size());
        }
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Visual::is_visible_property().as_property() {
            let (_, new_value) = change.get_old_and_new_value::<bool>();
            if let Some(visual_root) = this.visual_root() {
                visual_root.set_is_visible(new_value);
            }
            this.is_visible_changed(change);
        }
    }
}

impl LayoutableImpl for WindowBase {
    fn measure_core(this: &Self, available_size: Size) -> Size {
        this.apply_styling();
        this.apply_template();

        let constraint = LayoutHelper::apply_layout_constraints(this, available_size);

        Container::set_query_provider_size(this, constraint.width, constraint.height, Container::get_sizing(this));

        this.measure_override(constraint)
    }

    fn arrange_core(this: &Self, final_rect: Rect) {
        let constraint = this.arrange_set_bounds(final_rect.size());
        let arrange_size = this.arrange_override(constraint);
        this.set_bounds(Rect::from_position_size(final_rect.position(), arrange_size));
    }
}

impl TopLevelImpl for WindowBase {
    fn on_closed(this: &Self) {
        // Window must manually raise Loaded/Unloaded events as it is a visual root and
        // does not raise the attached/detached visual tree notifications
        this.on_unloaded_core();

        Self::parent_on_closed(this);
    }

    fn on_opened(this: &Self) {
        this.set_frame_size(this.platform_impl().and_then(|platform_impl| platform_impl.frame_size()));

        // Window must manually raise Loaded/Unloaded events as it is a visual root and
        // does not raise the attached/detached visual tree notifications
        this.schedule_on_loaded_core();

        Self::parent_on_opened(this);
    }

    fn handle_closed(this: &Self) {
        let _freeze = this.freeze_visibility_change_handling();

        this.set_is_visible(false);

        if this.is_active() {
            this.handle_deactivated();
        }

        if this.is_focus_scope() {
            this.focus_manager().remove_focus_root(this);
        }

        Self::parent_handle_closed(this);
        *this.window_base_impl.borrow_mut() = None;
    }

    fn handle_resized(this: &Self, client_size: Size, reason: WindowResizeReason) {
        this.set_frame_size(this.platform_impl().and_then(|platform_impl| platform_impl.frame_size()));

        let client_size_changed = this.client_size() != client_size;

        this.set_client_size(client_size);

        this.on_resized(&WindowResizedEventArgs::new(client_size, reason));

        if client_size_changed {
            this.layout_manager().execute_layout_pass();
            this.renderer().resized(client_size);
        }
    }
}

impl WindowBaseImpl for WindowBase {
    fn hide(this: &Self) {
        let _freeze = this.freeze_visibility_change_handling();

        this.stop_rendering();
        if let Some(platform_impl) = this.platform_impl() {
            platform_impl.hide();
        }
        this.set_is_visible(false);
    }

    fn show(this: &Self) {
        let _freeze = this.freeze_visibility_change_handling();

        this.ensure_initialized();
        this.apply_styling();
        this.set_is_visible(true);

        if !this.has_executed_initial_layout_pass.get() {
            this.layout_manager().execute_initial_layout_pass();
            this.has_executed_initial_layout_pass.set(true);
        }

        if let Some(platform_impl) = this.platform_impl() {
            platform_impl.show(true, false);
        }
        this.start_rendering();
        this.on_opened();
    }

    fn on_resized(this: &Self, e: &WindowResizedEventArgs) {
        for (_, handler) in this.resized.snapshot().iter() {
            handler(e);
        }
    }

    fn arrange_set_bounds(_this: &Self, size: Size) -> Size {
        size
    }

    fn is_visible_changed(this: &Self, e: &FerroPropertyChangedEventArgs<'_>) {
        if this.ignore_visibility_changes.get() == 0 {
            let (_, new_value) = e.get_old_and_new_value::<bool>();
            if new_value {
                this.show();
            } else {
                this.hide();
            }
        }
    }
}

/// Keeps visibility changes from showing or hiding the window while it is
/// alive.
pub struct IgnoreVisibilityChangesGuard {
    window_base: Ref<WindowBase>,
}

impl Drop for IgnoreVisibilityChangesGuard {
    fn drop(&mut self) {
        let count = &self.window_base.ignore_visibility_changes;
        count.set(count.get() - 1);
    }
}

ferroui_base::ferro_properties! { impl WindowBase {
    ferro_property!(
        /// Defines the `IsActive` property.
        pub fn is_active_property() -> DirectProperty<WindowBase, bool> {
            FerroProperty::register_direct::<WindowBase, _>("IsActive", |o| o.is_active(), None, false)
        }
    );

    ferro_property!(
        /// Defines the `Owner` property.
        pub fn owner_property() -> DirectProperty<WindowBase, Option<Ref<WindowBase>>> {
            FerroProperty::register_direct::<WindowBase, _>("Owner", |o| o.owner(), None, None)
        }
    );

    ferro_property!(
        /// Defines the `Topmost` property.
        pub fn topmost_property() -> StyledProperty<bool> {
            FerroProperty::register::<WindowBase, _>("Topmost", false)
        }
    );
} }

impl WindowBase {
    fn static_constructor() {
        Visual::is_visible_property().override_default_value::<WindowBase>(false);
    }

    /// Field initialisation of a window over the platform implementation
    /// `platform_impl`, with the services of the current service locator.
    pub fn construct(platform_impl: Rc<dyn IWindowBaseImpl>) -> Self {
        let mut this = Self::construct_with_resolver(platform_impl, None);
        this.default_resolver = Cell::new(true);
        this
    }

    /// Field initialisation of a window; `dependency_resolver` is the
    /// dependency resolver to use, the current service locator when `None`.
    ///
    /// Unlike [`construct`](Self::construct) this does not forward `Topmost`
    /// to the platform implementation or take its frame size.
    pub fn construct_with_resolver(
        platform_impl: Rc<dyn IWindowBaseImpl>,
        dependency_resolver: Option<Rc<dyn IFerroDependencyResolver>>,
    ) -> Self {
        let top_level_impl: Rc<dyn ITopLevelImpl> = platform_impl.clone();
        Self {
            base: TopLevel::construct_with_resolver(top_level_impl, dependency_resolver),
            window_base_impl: RefCell::new(Some(platform_impl)),
            default_resolver: Cell::new(false),
            has_executed_initial_layout_pass: Cell::new(false),
            is_active: Cell::new(false),
            ignore_visibility_changes: Cell::new(0),
            owner: RefCell::new(None),
            desktop_scaling_override: Cell::new(None),
            activated: HandlerList::new(),
            deactivated: HandlerList::new(),
            position_changed: HandlerList::new(),
            resized: HandlerList::new(),
        }
    }

    /// Creates a window base over a platform implementation (C#
    /// `new WindowBase(impl)`).
    pub fn new(platform_impl: Rc<dyn IWindowBaseImpl>) -> Ref<Self> {
        instantiate(Self::construct(platform_impl))
    }

    /// Whether visibility changes are currently ignored (protected
    /// upstream).
    pub fn ignore_visibility_changes(&self) -> bool {
        self.ignore_visibility_changes.get() > 0
    }

    /// Ignores visibility changes until the returned guard is dropped
    /// (private protected upstream: for derived classes).
    pub fn freeze_visibility_change_handling(&self) -> IgnoreVisibilityChangesGuard {
        self.ignore_visibility_changes.set(self.ignore_visibility_changes.get() + 1);
        IgnoreVisibilityChangesGuard { window_base: self.to_ref() }
    }

    fn subscribe<F: ?Sized + 'static>(
        &self,
        select: fn(&WindowBase) -> &HandlerList<F>,
        handler: Rc<F>,
    ) -> Rc<dyn IDisposable> {
        let token = select(self).add(handler);
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                select(&this).remove(token);
            }
        })
    }

    /// Fired when the window is activated. Disposing the returned handle
    /// unsubscribes.
    pub fn activated(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        self.subscribe::<dyn Fn()>(|w| &w.activated, Rc::new(handler))
    }

    /// Fired when the window is deactivated. Disposing the returned handle
    /// unsubscribes.
    pub fn deactivated(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        self.subscribe::<dyn Fn()>(|w| &w.deactivated, Rc::new(handler))
    }

    /// Fired when the window position is changed. Disposing the returned
    /// handle unsubscribes.
    pub fn position_changed(&self, handler: impl Fn(&PixelPointEventArgs) + 'static) -> Rc<dyn IDisposable> {
        self.subscribe::<dyn Fn(&PixelPointEventArgs)>(|w| &w.position_changed, Rc::new(handler))
    }

    /// Fired when the window is resized. Disposing the returned handle
    /// unsubscribes.
    pub fn resized(&self, handler: impl Fn(&WindowResizedEventArgs) + 'static) -> Rc<dyn IDisposable> {
        self.subscribe::<dyn Fn(&WindowResizedEventArgs)>(|w| &w.resized, Rc::new(handler))
    }

    /// The platform-specific window implementation; `None` once the window
    /// has closed.
    pub fn platform_impl(&self) -> Option<Rc<dyn IWindowBaseImpl>> {
        TopLevel::platform_impl(self)?;
        self.window_base_impl.borrow().clone()
    }

    /// Whether the window is active.
    pub fn is_active(&self) -> bool {
        self.is_active.get()
    }

    fn set_is_active(&self, value: bool) {
        self.set_and_raise_cell(Self::is_active_property(), &self.is_active, value);
    }

    /// The screens of the platform.
    ///
    /// # Panics
    /// Panics when the windowing backend does not provide screens.
    pub fn screens(&self) -> Rc<Screens> {
        TopLevel::screens(self).expect("Windowing backend wasn't properly initialized.")
    }

    /// The owner of the window.
    pub fn owner(&self) -> Option<Ref<WindowBase>> {
        self.owner.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    /// Sets the owner of the window (protected upstream: for derived
    /// classes).
    pub fn set_owner(&self, value: Option<Ref<WindowBase>>) {
        let old = self.owner();
        if old == value {
            return;
        }
        *self.owner.borrow_mut() = value.as_ref().map(Ref::downgrade);
        self.raise_direct_property_changed(Self::owner_property(), &old, &value);
    }

    /// Whether this window appears on top of all other windows.
    pub fn topmost(&self) -> bool {
        self.get_value(Self::topmost_property())
    }

    pub fn set_topmost(&self, value: bool) {
        self.set_value(Self::topmost_property(), value)
    }

    /// The scaling factor for window positioning and sizing.
    pub fn desktop_scaling(&self) -> f64 {
        self.desktop_scaling_override
            .get()
            .or_else(|| self.platform_impl().map(|platform_impl| platform_impl.desktop_scaling()))
            .unwrap_or(1.0)
    }

    /// The desktop scaling used instead of the platform's (private
    /// protected upstream: for derived classes).
    pub fn desktop_scaling_override(&self) -> Option<f64> {
        self.desktop_scaling_override.get()
    }

    pub fn set_desktop_scaling_override(&self, value: Option<f64>) {
        self.desktop_scaling_override.set(value)
    }

    /// Activates the window.
    pub fn activate(&self) {
        if let Some(platform_impl) = self.platform_impl() {
            platform_impl.activate();
        }
    }

    /// Ensures that the window is initialized (protected upstream: for
    /// derived classes).
    pub fn ensure_initialized(&self) {
        if !self.is_initialized() {
            self.begin_init();
            self.end_init();
        }
    }

    fn handle_position_changed(&self, pos: PixelPoint) {
        let e = PixelPointEventArgs::new(pos);
        for (_, handler) in self.position_changed.snapshot().iter() {
            handler(&e);
        }
    }

    fn handle_activated(&self) {
        for (_, handler) in self.activated.snapshot().iter() {
            handler();
        }

        if self.is_focus_scope() {
            self.focus_manager().set_focus_scope(&self.to_ref().upcast());
        }

        self.set_is_active(true);
    }

    fn handle_deactivated(&self) {
        self.set_is_active(false);

        for (_, handler) in self.deactivated.snapshot().iter() {
            handler();
        }
    }
}
