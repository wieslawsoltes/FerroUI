use super::{
    DrawnWindowDecorationParts, IWindowDrawnDecorationsTemplate, TitleBarDecorations, WindowDrawnDecorationsContent,
};
use crate::automation::AutomationProperties;
use crate::platform::PlatformAllowedWindowActions;
use crate::primitives::TemplatedControl;
use crate::{Border, Button, Control, Window, WindowState};
use ferroui_base::controls::NameScopeRef;
use ferroui_base::interactivity::{RoutedEventArgs, RoutedEventHandlerToken};
use ferroui_base::layout::LayoutHelper;
use ferroui_base::reactive::{CompositeDisposable, Disposable, IDisposable, ObservableExt};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, AttachedProperty, DirectProperty, FerroObject,
    FerroObjectExtensions, FerroObjectImpl, FerroProperty, Ref, StyledElement, StyledElementImpl,
    StyledProperty, StyledPropertyOptions, Thickness, WeakRef,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Manages client-side window decorations (app-drawn window frame).
///
/// This is a logical element that holds the decorations template and
/// properties. The host of the top-level extracts the overlay, underlay and
/// popover visuals from the template content and inserts them into its own
/// visual tree.
///
/// Pseudo-classes: `:normal`, `:maximized`, `:fullscreen`, `:has-shadow`,
/// `:has-border`, `:has-titlebar`, `:has-maximize`, `:has-fullscreen`,
/// `:has-minimize`, `:has-close`, `:has-title`.
#[repr(C)]
pub struct WindowDrawnDecorations {
    base: StyledElement,
    applied_template: RefCell<Option<Rc<dyn IWindowDrawnDecorationsTemplate>>>,
    template_name_scope: RefCell<Option<NameScopeRef>>,
    close_button: CaptionButton,
    minimize_button: CaptionButton,
    maximize_button: CaptionButton,
    full_screen_button: CaptionButton,
    popover_close_button: CaptionButton,
    popover_full_screen_button: CaptionButton,
    window_subscriptions: RefCell<Option<Rc<dyn IDisposable>>>,
    host_window: RefCell<Option<WeakRef<Window>>>,
    title_bar_height_override: Cell<f64>,
    frame_thickness_override: Cell<Option<Thickness>>,
    shadow_thickness_override: Cell<Option<Thickness>>,
    render_scaling: Cell<f64>,
    content: RefCell<Option<Ref<WindowDrawnDecorationsContent>>>,
    title_bar_height: Cell<f64>,
    frame_thickness: Cell<Thickness>,
    shadow_thickness: Cell<Thickness>,
    has_shadow: Cell<bool>,
    has_border: Cell<bool>,
    has_title_bar: Cell<bool>,
    effective_geometry_changed: HandlerList<dyn Fn()>,
}

/// A caption button of the applied template with the token of its click
/// handler.
type CaptionButton = RefCell<Option<(Ref<Button>, RoutedEventHandlerToken)>>;

ferro_class!(WindowDrawnDecorations: StyledElement);
ferroui_base::ferro_class_info!(WindowDrawnDecorations { new: WindowDrawnDecorations::new });
ferro_impl_classes!(WindowDrawnDecorations: StyledElementImpl);

impl FerroObjectImpl for WindowDrawnDecorations {}

impl WindowDrawnDecorations {
    pub const PC_NORMAL: &'static str = ":normal";
    pub const PC_MAXIMIZED: &'static str = ":maximized";
    pub const PC_FULLSCREEN: &'static str = ":fullscreen";
    pub const PC_HAS_SHADOW: &'static str = ":has-shadow";
    pub const PC_HAS_BORDER: &'static str = ":has-border";
    pub const PC_HAS_TITLEBAR: &'static str = ":has-titlebar";
    pub const PC_HAS_MAXIMIZE: &'static str = ":has-maximize";
    pub const PC_HAS_FULLSCREEN: &'static str = ":has-fullscreen";
    pub const PC_HAS_MINIMIZE: &'static str = ":has-minimize";
    pub const PC_HAS_CLOSE: &'static str = ":has-close";
    pub const PC_HAS_TITLE: &'static str = ":has-title";

    // Template part names for caption buttons
    pub const PART_CLOSE_BUTTON: &'static str = "PART_CloseButton";
    pub const PART_MINIMIZE_BUTTON: &'static str = "PART_MinimizeButton";
    pub const PART_MAXIMIZE_BUTTON: &'static str = "PART_MaximizeButton";
    pub const PART_FULL_SCREEN_BUTTON: &'static str = "PART_FullScreenButton";
    // Popover caption buttons (separate names to avoid name scope conflicts)
    pub const PART_POPOVER_CLOSE_BUTTON: &'static str = "PART_PopoverCloseButton";
    pub const PART_POPOVER_FULL_SCREEN_BUTTON: &'static str = "PART_PopoverFullScreenButton";
    // Titlebar panel
    pub const PART_TITLE_BAR: &'static str = "PART_TitleBar";
}

ferroui_base::ferro_properties! { impl WindowDrawnDecorations {
    ferro_property!(
        /// Defines the `Template` property.
        pub fn template_property() -> StyledProperty<Option<Rc<dyn IWindowDrawnDecorationsTemplate>>> {
            FerroProperty::register::<WindowDrawnDecorations, _>("Template", None)
        }
    );

    ferro_property!(
        /// Defines the `DefaultTitleBarHeight` property.
        pub fn default_title_bar_height_property() -> StyledProperty<f64> {
            FerroProperty::register::<WindowDrawnDecorations, _>("DefaultTitleBarHeight", 0.0)
        }
    );

    ferro_property!(
        /// Defines the `DefaultFrameThickness` property.
        pub fn default_frame_thickness_property() -> StyledProperty<Thickness> {
            FerroProperty::register_with::<WindowDrawnDecorations, _>(
                "DefaultFrameThickness",
                StyledPropertyOptions::new(Thickness::default()).validate(validate_thickness),
            )
        }
    );

    ferro_property!(
        /// Defines the `DefaultShadowThickness` property.
        pub fn default_shadow_thickness_property() -> StyledProperty<Thickness> {
            FerroProperty::register_with::<WindowDrawnDecorations, _>(
                "DefaultShadowThickness",
                StyledPropertyOptions::new(Thickness::default()).validate(validate_thickness),
            )
        }
    );

    ferro_property!(
        /// Defines the `TitleBarHeight` property.
        pub fn title_bar_height_property() -> DirectProperty<WindowDrawnDecorations, f64> {
            FerroProperty::register_direct::<WindowDrawnDecorations, _>(
                "TitleBarHeight",
                |o| o.title_bar_height(),
                None,
                0.0,
            )
        }
    );

    ferro_property!(
        /// Defines the `FrameThickness` property.
        pub fn frame_thickness_property() -> DirectProperty<WindowDrawnDecorations, Thickness> {
            FerroProperty::register_direct::<WindowDrawnDecorations, _>(
                "FrameThickness",
                |o| o.frame_thickness(),
                None,
                Thickness::default(),
            )
        }
    );

    ferro_property!(
        /// Defines the `ShadowThickness` property.
        pub fn shadow_thickness_property() -> DirectProperty<WindowDrawnDecorations, Thickness> {
            FerroProperty::register_direct::<WindowDrawnDecorations, _>(
                "ShadowThickness",
                |o| o.shadow_thickness(),
                None,
                Thickness::default(),
            )
        }
    );

    ferro_property!(
        /// Defines the `HasShadow` property.
        pub fn has_shadow_property() -> DirectProperty<WindowDrawnDecorations, bool> {
            FerroProperty::register_direct::<WindowDrawnDecorations, _>("HasShadow", |o| o.has_shadow(), None, false)
        }
    );

    ferro_property!(
        /// Defines the `HasBorder` property.
        pub fn has_border_property() -> DirectProperty<WindowDrawnDecorations, bool> {
            FerroProperty::register_direct::<WindowDrawnDecorations, _>("HasBorder", |o| o.has_border(), None, false)
        }
    );

    ferro_property!(
        /// Defines the `HasTitleBar` property.
        pub fn has_title_bar_property() -> DirectProperty<WindowDrawnDecorations, bool> {
            FerroProperty::register_direct::<WindowDrawnDecorations, _>(
                "HasTitleBar",
                |o| o.has_title_bar(),
                None,
                false,
            )
        }
    );

    ferro_property!(
        /// Defines the `Title` property.
        pub fn title_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<WindowDrawnDecorations, _>("Title", None)
        }
    );

    ferro_property!(
        /// Defines the `TitleBarDecorations` attached property.
        pub fn title_bar_decorations_property() -> AttachedProperty<TitleBarDecorations> {
            FerroProperty::register_attached::<WindowDrawnDecorations, StyledElement, _>(
                "TitleBarDecorations",
                TitleBarDecorations::ALL,
            )
        }
    );

    ferro_property!(
        /// Defines the `EnabledParts` property.
        pub(crate) fn enabled_parts_property() -> StyledProperty<DrawnWindowDecorationParts> {
            FerroProperty::register::<WindowDrawnDecorations, _>("EnabledParts", DrawnWindowDecorationParts::NONE)
        }
    );
} }

impl WindowDrawnDecorations {
    fn static_constructor() {
        let _ = Self::template_property()
            .changed()
            .add_class_handler::<WindowDrawnDecorations>(|x, _| x.invalidate_template());
        let _ = Self::enabled_parts_property().changed().add_class_handler::<WindowDrawnDecorations>(|x, _| {
            x.update_enabled_parts_pseudo_classes();
            x.update_effective_geometry();
        });

        let _ = Self::title_bar_decorations_property()
            .changed()
            .add_class_handler::<WindowDrawnDecorations>(|x, _| x.update_title_bar_decorations_pseudo_classes());
        let _ = Self::default_title_bar_height_property()
            .changed()
            .add_class_handler::<WindowDrawnDecorations>(|x, _| x.update_effective_geometry());
        let _ = Self::default_frame_thickness_property()
            .changed()
            .add_class_handler::<WindowDrawnDecorations>(|x, _| x.update_effective_geometry());
        let _ = Self::default_shadow_thickness_property()
            .changed()
            .add_class_handler::<WindowDrawnDecorations>(|x, _| x.update_effective_geometry());
    }

    /// Field initialisation only.
    pub fn construct() -> Self {
        Self {
            base: StyledElement::construct(),
            applied_template: RefCell::new(None),
            template_name_scope: RefCell::new(None),
            close_button: RefCell::new(None),
            minimize_button: RefCell::new(None),
            maximize_button: RefCell::new(None),
            full_screen_button: RefCell::new(None),
            popover_close_button: RefCell::new(None),
            popover_full_screen_button: RefCell::new(None),
            window_subscriptions: RefCell::new(None),
            host_window: RefCell::new(None),
            title_bar_height_override: Cell::new(-1.0),
            frame_thickness_override: Cell::new(None),
            shadow_thickness_override: Cell::new(None),
            render_scaling: Cell::new(1.0),
            content: RefCell::new(None),
            title_bar_height: Cell::new(0.0),
            frame_thickness: Cell::new(Thickness::default()),
            shadow_thickness: Cell::new(Thickness::default()),
            has_shadow: Cell::new(false),
            has_border: Cell::new(false),
            has_title_bar: Cell::new(false),
            effective_geometry_changed: HandlerList::new(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Raised when any property affecting the effective geometry changes
    /// (effective titlebar height, frame thickness, or shadow thickness).
    /// Disposing the returned handle unsubscribes.
    pub(crate) fn effective_geometry_changed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.effective_geometry_changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.effective_geometry_changed.remove(token);
            }
        })
    }

    /// The current render scaling factor used for pixel-aligning decoration
    /// geometry (title bar height, frame/shadow thickness).
    pub fn render_scaling(&self) -> f64 {
        self.render_scaling.get()
    }

    pub fn set_render_scaling(&self, value: f64) {
        if self.render_scaling.get() == value {
            return;
        }
        self.render_scaling.set(value);
        self.update_effective_geometry();
    }

    /// The decorations template.
    pub fn template(&self) -> Option<Rc<dyn IWindowDrawnDecorationsTemplate>> {
        self.get_value(Self::template_property())
    }

    pub fn set_template(&self, value: Option<Rc<dyn IWindowDrawnDecorationsTemplate>>) {
        self.set_value(Self::template_property(), value)
    }

    /// The theme-set default titlebar height.
    pub fn default_title_bar_height(&self) -> f64 {
        self.get_value(Self::default_title_bar_height_property())
    }

    pub fn set_default_title_bar_height(&self, value: f64) {
        self.set_value(Self::default_title_bar_height_property(), value)
    }

    /// The theme-set default frame thickness.
    pub fn default_frame_thickness(&self) -> Thickness {
        self.get_value(Self::default_frame_thickness_property())
    }

    pub fn set_default_frame_thickness(&self, value: Thickness) {
        self.set_value(Self::default_frame_thickness_property(), value)
    }

    /// The theme-set default shadow thickness.
    pub fn default_shadow_thickness(&self) -> Thickness {
        self.get_value(Self::default_shadow_thickness_property())
    }

    pub fn set_default_shadow_thickness(&self, value: Thickness) {
        self.set_value(Self::default_shadow_thickness_property(), value)
    }

    /// The titlebar height override. When -1, falls back to
    /// [`default_title_bar_height`](Self::default_title_bar_height).
    pub fn title_bar_height_override(&self) -> f64 {
        self.title_bar_height_override.get()
    }

    pub fn set_title_bar_height_override(&self, value: f64) {
        self.title_bar_height_override.set(value);
        self.update_effective_geometry();
    }

    /// The frame thickness override. When set, takes precedence over
    /// [`default_frame_thickness`](Self::default_frame_thickness).
    pub fn frame_thickness_override(&self) -> Option<Thickness> {
        self.frame_thickness_override.get()
    }

    pub fn set_frame_thickness_override(&self, value: Option<Thickness>) {
        self.frame_thickness_override.set(value);
        self.update_effective_geometry();
    }

    /// The shadow thickness override. When set, takes precedence over
    /// [`default_shadow_thickness`](Self::default_shadow_thickness).
    pub fn shadow_thickness_override(&self) -> Option<Thickness> {
        self.shadow_thickness_override.get()
    }

    pub fn set_shadow_thickness_override(&self, value: Option<Thickness>) {
        self.shadow_thickness_override.set(value);
        self.update_effective_geometry();
    }

    /// The window title displayed in the decorations. Mirrors the value set
    /// on the host window.
    pub fn title(&self) -> Option<String> {
        self.get_value(Self::title_property())
    }

    pub fn set_title(&self, value: Option<String>) {
        self.set_value(Self::title_property(), value)
    }

    /// Gets which title bar elements to display.
    pub fn get_title_bar_decorations(element: &StyledElement) -> TitleBarDecorations {
        element.get_value(Self::title_bar_decorations_property())
    }

    /// Sets which title bar elements to display.
    ///
    /// Set this property on a window to control the elements displayed in
    /// its drawn title bar.
    pub fn set_title_bar_decorations(element: &StyledElement, value: TitleBarDecorations) {
        element.set_value(Self::title_bar_decorations_property(), value)
    }

    /// Which title bar elements are requested to be displayed. Mirrors the
    /// value set on the host window.
    pub fn title_bar_decorations(&self) -> TitleBarDecorations {
        self.get_value(Self::title_bar_decorations_property())
    }

    /// Sets which title bar elements are requested to be displayed on these
    /// decorations (the instance property; the associated function
    /// [`set_title_bar_decorations`](Self::set_title_bar_decorations) is the
    /// attached property setter).
    pub fn set_own_title_bar_decorations(&self, value: TitleBarDecorations) {
        self.set_value(Self::title_bar_decorations_property(), value)
    }

    /// Which decoration parts are enabled. Set by the window based on
    /// platform capabilities and user preferences.
    pub(crate) fn enabled_parts(&self) -> DrawnWindowDecorationParts {
        self.get_value(Self::enabled_parts_property())
    }

    pub(crate) fn set_enabled_parts(&self, value: DrawnWindowDecorationParts) {
        self.set_value(Self::enabled_parts_property(), value)
    }

    /// The built template content.
    pub fn content(&self) -> Option<Ref<WindowDrawnDecorationsContent>> {
        self.content.borrow().clone()
    }

    /// The effective titlebar height, resolving a -1 override to the
    /// default. 0 if the titlebar part is disabled.
    pub fn title_bar_height(&self) -> f64 {
        self.title_bar_height.get()
    }

    fn set_title_bar_height(&self, value: f64) {
        self.set_and_raise_cell(Self::title_bar_height_property(), &self.title_bar_height, value);
    }

    /// The effective frame thickness: the override if explicitly set,
    /// otherwise the default. Zero if the border part is disabled.
    pub fn frame_thickness(&self) -> Thickness {
        self.frame_thickness.get()
    }

    fn set_frame_thickness(&self, value: Thickness) {
        self.set_and_raise_cell(Self::frame_thickness_property(), &self.frame_thickness, value);
    }

    /// The effective shadow thickness: the override if explicitly set,
    /// otherwise the default. Zero if the shadow part is disabled.
    pub fn shadow_thickness(&self) -> Thickness {
        self.shadow_thickness.get()
    }

    fn set_shadow_thickness(&self, value: Thickness) {
        self.set_and_raise_cell(Self::shadow_thickness_property(), &self.shadow_thickness, value);
    }

    /// Whether the shadow decoration part is enabled.
    pub fn has_shadow(&self) -> bool {
        self.has_shadow.get()
    }

    fn set_has_shadow(&self, value: bool) {
        self.set_and_raise_cell(Self::has_shadow_property(), &self.has_shadow, value);
    }

    /// Whether the border decoration part is enabled.
    pub fn has_border(&self) -> bool {
        self.has_border.get()
    }

    fn set_has_border(&self, value: bool) {
        self.set_and_raise_cell(Self::has_border_property(), &self.has_border, value);
    }

    /// Whether the title bar decoration part is enabled.
    pub fn has_title_bar(&self) -> bool {
        self.has_title_bar.get()
    }

    fn set_has_title_bar(&self, value: bool) {
        self.set_and_raise_cell(Self::has_title_bar_property(), &self.has_title_bar, value);
    }

    /// Applies the template if it has changed.
    pub(crate) fn apply_template(&self) {
        let template = self.template();
        if template == *self.applied_template.borrow() {
            return;
        }

        // Clean up old content
        let old_content = self.content.borrow().clone();
        if let Some(old_content) = old_content {
            self.detach_caption_buttons();
            self.logical_children().remove(&old_content.clone().upcast());
            old_content.set_parent(None);
            *self.content.borrow_mut() = None;
            *self.template_name_scope.borrow_mut() = None;
        }

        *self.applied_template.borrow_mut() = template.clone();

        let Some(template) = template else { return };

        let (content, name_scope) = template.build_typed().deconstruct();
        *self.content.borrow_mut() = Some(content.clone());
        *self.template_name_scope.borrow_mut() = Some(name_scope);

        let this = self.to_ref();
        TemplatedControl::apply_templated_parent(&content, Some(&this.clone().upcast::<FerroObject>()));
        self.logical_children().add(content.clone().upcast());
        content.set_parent(this.upcast::<StyledElement>());

        self.attach_caption_buttons();
    }

    /// Attaches to the specified window for caption button interactions and
    /// state tracking.
    pub(crate) fn attach(&self, window: &Ref<Window>) {
        if self.host_window().as_ref() == Some(window) {
            return;
        }

        self.detach();
        *self.host_window.borrow_mut() = Some(window.downgrade());

        let weak = self.to_ref().downgrade();
        let object: &FerroObject = window;

        let subscriptions: [Rc<dyn IDisposable>; 6] = [
            window.allowed_window_actions_changed({
                let weak = weak.clone();
                move |actions| {
                    if let Some(this) = weak.upgrade() {
                        this.on_allowed_window_actions_changed(actions);
                    }
                }
            }),
            FerroObjectExtensions::get_observable(object, Window::title_property()).subscribe_fn({
                let weak = weak.clone();
                move |title| {
                    if let Some(this) = weak.upgrade() {
                        this.set_current_value(Self::title_property(), title);
                    }
                }
            }),
            FerroObjectExtensions::get_observable(object, Self::title_bar_decorations_property()).subscribe_fn({
                let weak = weak.clone();
                move |hints| {
                    if let Some(this) = weak.upgrade() {
                        this.set_current_value(Self::title_bar_decorations_property(), hints);
                    }
                }
            }),
            FerroObjectExtensions::get_observable(object, Window::can_maximize_property()).subscribe_fn({
                let weak = weak.clone();
                move |_| {
                    if let Some(this) = weak.upgrade() {
                        this.update_maximize_button_state();
                        this.update_full_screen_button_state();
                    }
                }
            }),
            FerroObjectExtensions::get_observable(object, Window::can_minimize_property()).subscribe_fn({
                let weak = weak.clone();
                move |_| {
                    if let Some(this) = weak.upgrade() {
                        this.update_minimize_button_state();
                    }
                }
            }),
            FerroObjectExtensions::get_observable(object, Window::window_state_property()).subscribe_fn({
                let weak = weak.clone();
                move |state| {
                    if let Some(this) = weak.upgrade() {
                        let pseudo_classes = this.pseudo_classes();
                        pseudo_classes.set(Self::PC_NORMAL, state == WindowState::Normal);
                        pseudo_classes.set(Self::PC_MAXIMIZED, state == WindowState::Maximized);
                        pseudo_classes.set(Self::PC_FULLSCREEN, state == WindowState::FullScreen);
                        this.update_maximize_button_state();
                        this.update_minimize_button_state();
                        this.update_full_screen_button_state();
                    }
                }
            }),
        ];
        let subscriptions: Rc<dyn IDisposable> = Rc::new(CompositeDisposable::from_disposables(subscriptions));
        *self.window_subscriptions.borrow_mut() = Some(subscriptions);

        self.update_title_bar_decorations_pseudo_classes();
        self.update_maximize_button_state();
        self.update_minimize_button_state();
        self.update_full_screen_button_state();
    }

    /// Detaches from the current window.
    pub(crate) fn detach(&self) {
        let subscriptions = self.window_subscriptions.borrow_mut().take();
        if let Some(subscriptions) = subscriptions {
            subscriptions.dispose();
        }
        *self.host_window.borrow_mut() = None;
    }

    fn host_window(&self) -> Option<Ref<Window>> {
        self.host_window.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    fn invalidate_template(&self) {
        *self.applied_template.borrow_mut() = None;
    }

    /// Finds a caption button of the applied template and adds its click
    /// handler.
    fn attach_caption_button(
        &self,
        name_scope: &NameScopeRef,
        name: &str,
        field: &CaptionButton,
        automation_id: &str,
        automation_name: &str,
        on_click: fn(&Self, &RoutedEventArgs),
    ) -> bool {
        let Some(button) = name_scope.find_as::<Button>(name) else {
            *field.borrow_mut() = None;
            return false;
        };
        AutomationProperties::set_automation_id(&button, Some(automation_id));
        AutomationProperties::set_name(&button, Some(automation_name));
        let weak = self.to_ref().downgrade();
        let token = button.click(move |_, e| {
            if let Some(this) = weak.upgrade() {
                on_click(&this, e);
            }
        });
        *field.borrow_mut() = Some((button, token));
        true
    }

    fn attach_caption_buttons(&self) {
        let Some(name_scope) = self.template_name_scope.borrow().clone() else { return };

        let title_bar = name_scope.find_as::<Control>(Self::PART_TITLE_BAR);
        if let Some(title_bar) = title_bar {
            AutomationProperties::set_is_control_element_override(&title_bar, Some(true));
            AutomationProperties::set_automation_id(&title_bar, Some("FerroTitleBar"));
            AutomationProperties::set_name(&title_bar, Some("TitleBar"));
        }

        self.attach_caption_button(
            &name_scope,
            Self::PART_CLOSE_BUTTON,
            &self.close_button,
            "Close",
            "Close",
            Self::on_close_button_click,
        );
        let has_minimize = self.attach_caption_button(
            &name_scope,
            Self::PART_MINIMIZE_BUTTON,
            &self.minimize_button,
            "Minimize",
            "Minimize",
            Self::on_minimize_button_click,
        );
        let has_maximize = self.attach_caption_button(
            &name_scope,
            Self::PART_MAXIMIZE_BUTTON,
            &self.maximize_button,
            "Maximize",
            "Maximize",
            Self::on_maximize_button_click,
        );
        let has_full_screen = self.attach_caption_button(
            &name_scope,
            Self::PART_FULL_SCREEN_BUTTON,
            &self.full_screen_button,
            "Fullscreen",
            "Fullscreen",
            Self::on_full_screen_button_click,
        );
        self.attach_caption_button(
            &name_scope,
            Self::PART_POPOVER_CLOSE_BUTTON,
            &self.popover_close_button,
            "FullscreenClose",
            "Close",
            Self::on_close_button_click,
        );
        self.attach_caption_button(
            &name_scope,
            Self::PART_POPOVER_FULL_SCREEN_BUTTON,
            &self.popover_full_screen_button,
            "ExitFullscreen",
            "ExitFullscreen",
            Self::on_full_screen_button_click,
        );

        if has_minimize {
            self.update_minimize_button_state();
        }
        if has_maximize {
            self.update_maximize_button_state();
        }
        if has_full_screen {
            self.update_full_screen_button_state();
        }
    }

    fn detach_caption_buttons(&self) {
        for field in [
            &self.close_button,
            &self.minimize_button,
            &self.maximize_button,
            &self.full_screen_button,
            &self.popover_close_button,
            &self.popover_full_screen_button,
        ] {
            let attached = field.borrow_mut().take();
            if let Some((button, token)) = attached {
                button.remove_handler(Button::click_event(), token);
            }
        }
    }

    fn on_close_button_click(&self, e: &RoutedEventArgs) {
        if let Some(host_window) = self.host_window() {
            host_window.close();
        }
        e.set_handled(true);
    }

    fn on_minimize_button_click(&self, e: &RoutedEventArgs) {
        if let Some(host_window) = self.host_window() {
            host_window.set_window_state(WindowState::Minimized);
        }
        e.set_handled(true);
    }

    fn on_maximize_button_click(&self, e: &RoutedEventArgs) {
        if let Some(host_window) = self.host_window() {
            host_window.set_window_state(if host_window.window_state() == WindowState::Maximized {
                WindowState::Normal
            } else {
                WindowState::Maximized
            });
        }
        e.set_handled(true);
    }

    fn on_full_screen_button_click(&self, e: &RoutedEventArgs) {
        if let Some(host_window) = self.host_window() {
            host_window.set_window_state(if host_window.window_state() == WindowState::FullScreen {
                WindowState::Normal
            } else {
                WindowState::FullScreen
            });
        }
        e.set_handled(true);
    }

    fn effective_allowed_actions(&self) -> PlatformAllowedWindowActions {
        self.host_window().map(|window| window.allowed_window_actions()).unwrap_or(PlatformAllowedWindowActions::ALL)
    }

    fn update_maximize_button_state(&self) {
        let Some(maximize_button) = self.maximize_button.borrow().as_ref().map(|(button, _)| button.clone()) else {
            return;
        };
        let host_window = self.host_window();
        maximize_button.set_is_enabled(
            self.effective_allowed_actions().contains(PlatformAllowedWindowActions::MAXIMIZE)
                && match &host_window {
                    Some(host_window) => match host_window.window_state() {
                        WindowState::Maximized | WindowState::FullScreen => host_window.can_resize(),
                        WindowState::Normal => host_window.can_maximize(),
                        _ => true,
                    },
                    None => true,
                },
        );
    }

    fn update_minimize_button_state(&self) {
        let Some(minimize_button) = self.minimize_button.borrow().as_ref().map(|(button, _)| button.clone()) else {
            return;
        };
        minimize_button.set_is_enabled(
            self.effective_allowed_actions().contains(PlatformAllowedWindowActions::MINIMIZE)
                && self.host_window().is_none_or(|host_window| host_window.can_minimize()),
        );
    }

    fn update_full_screen_button_state(&self) {
        let Some(full_screen_button) = self.full_screen_button.borrow().as_ref().map(|(button, _)| button.clone())
        else {
            return;
        };
        full_screen_button.set_is_enabled(
            self.effective_allowed_actions().contains(PlatformAllowedWindowActions::FULLSCREEN)
                && self.host_window().is_none_or(|host_window| {
                    if host_window.window_state() == WindowState::FullScreen {
                        host_window.can_resize()
                    } else {
                        host_window.can_maximize()
                    }
                }),
        );
    }

    fn on_allowed_window_actions_changed(&self, _actions: PlatformAllowedWindowActions) {
        self.update_title_bar_decorations_pseudo_classes();
        self.update_maximize_button_state();
        self.update_minimize_button_state();
        self.update_full_screen_button_state();
    }

    fn update_title_bar_decorations_pseudo_classes(&self) {
        let actions = self.effective_allowed_actions();
        let hints = self.title_bar_decorations();
        let pseudo_classes = self.pseudo_classes();

        pseudo_classes.set(
            Self::PC_HAS_MAXIMIZE,
            actions.contains(PlatformAllowedWindowActions::MAXIMIZE) && hints.contains(TitleBarDecorations::MAXIMIZE_BUTTON),
        );
        pseudo_classes.set(
            Self::PC_HAS_FULLSCREEN,
            actions.contains(PlatformAllowedWindowActions::FULLSCREEN)
                && hints.contains(TitleBarDecorations::FULL_SCREEN_BUTTON),
        );
        pseudo_classes.set(
            Self::PC_HAS_MINIMIZE,
            actions.contains(PlatformAllowedWindowActions::MINIMIZE) && hints.contains(TitleBarDecorations::MINIMIZE_BUTTON),
        );
        pseudo_classes.set(Self::PC_HAS_CLOSE, hints.contains(TitleBarDecorations::CLOSE_BUTTON));
        pseudo_classes.set(Self::PC_HAS_TITLE, hints.contains(TitleBarDecorations::TITLE));
    }

    fn update_effective_geometry(&self) {
        let scale = self.render_scaling.get();
        let enabled_parts = self.enabled_parts();

        self.set_title_bar_height(if enabled_parts.contains(DrawnWindowDecorationParts::TITLE_BAR) {
            LayoutHelper::round_layout_value(
                if self.title_bar_height_override() == -1.0 {
                    self.default_title_bar_height()
                } else {
                    self.title_bar_height_override()
                },
                scale,
            )
        } else {
            0.0
        });

        self.set_frame_thickness(if enabled_parts.contains(DrawnWindowDecorationParts::BORDER) {
            LayoutHelper::round_layout_thickness(
                self.frame_thickness_override().unwrap_or_else(|| self.default_frame_thickness()),
                scale,
            )
        } else {
            Thickness::default()
        });

        self.set_shadow_thickness(if enabled_parts.contains(DrawnWindowDecorationParts::SHADOW) {
            LayoutHelper::round_layout_thickness(
                self.shadow_thickness_override().unwrap_or_else(|| self.default_shadow_thickness()),
                scale,
            )
        } else {
            Thickness::default()
        });

        for (_, handler) in self.effective_geometry_changed.snapshot().iter() {
            handler();
        }
    }

    fn update_enabled_parts_pseudo_classes(&self) {
        let parts = self.enabled_parts();
        let has_shadow = parts.contains(DrawnWindowDecorationParts::SHADOW);
        let has_border = parts.contains(DrawnWindowDecorationParts::BORDER);
        let has_title_bar = parts.contains(DrawnWindowDecorationParts::TITLE_BAR);
        self.set_has_shadow(has_shadow);
        self.set_has_border(has_border);
        self.set_has_title_bar(has_title_bar);
        let pseudo_classes = self.pseudo_classes();
        pseudo_classes.set(Self::PC_HAS_SHADOW, has_shadow);
        pseudo_classes.set(Self::PC_HAS_BORDER, has_border);
        pseudo_classes.set(Self::PC_HAS_TITLEBAR, has_title_bar);
    }
}

fn validate_thickness(value: &Thickness) -> bool {
    Border::border_thickness_property().validate_value().is_none_or(|validate| validate(value))
}
