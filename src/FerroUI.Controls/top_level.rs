use crate::metadata::TemplatePartAttribute;
use crate::platform::{
    IInputPane, IInsetsManager, IPlatformHandle, IScreenImpl, ITopLevelImpl, PlatformThemeVariant, SafeAreaChangedArgs,
};
use crate::presentation_source::{try_get_service, ITopLevelRenderer, PresentationSource};
use crate::primitives::{TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt};
use crate::top_level_host::TopLevelHost;
use crate::{IToolTipService, ToolTip};
use ferroui_base::{Point, Vector};
use crate::{
    Border, ContentControl, ContentControlImpl, Control, ControlImpl, PlatformInhibitionType, Screens, WindowResizeReason,
    WindowTransparencyLevel, WindowTransparencyLevelCollection,
};
use ferroui_base::data::BindingPriority;
use ferroui_base::input::platform::{ClipboardType, IClipboard, IPlatformClipboardManagerImpl};
use ferroui_base::input::raw::{IRawInputEventArgs, RawKeyEventArgs, RawKeyEventType, RawPointerEventArgs, RawPointerEventType};
use ferroui_base::input::{
    FocusManager, IAccessKeyHandler, IInputManager, IInputRoot, IKeyboardNavigationHandler, InputElementImpl,
    KeyEventArgs, KeyModifiers, KeyboardDevice, KeyboardNavigation, KeyboardNavigationMode, NavigationMethod,
};
use ferroui_base::interactivity::{
    Interactive, InteractiveImpl, RoutedEvent, RoutedEventArgs, RoutedEventHandlerToken, RoutingStrategies,
};
use ferroui_base::layout::{ILayoutManager, LayoutHelper, LayoutManager, Layoutable, LayoutableImpl};
use ferroui_base::logical_tree::LogicalTreeAttachmentEventArgs;
use ferroui_base::media::{Brushes, IBrush, MediaContext, SolidColorBrush};
use ferroui_base::platform::{
    IOptionalFeatureProvider, IPlatformBehaviorInhibition, IPlatformSettings, ISystemNavigationManagerImpl,
};
use ferroui_base::reactive::{AnonymousObserver, Disposable, IDisposable};
use ferroui_base::rendering::composition::CompositionTransparencyLevel;
use ferroui_base::rendering::{IHitTester, IRenderer, RendererDiagnostics};
use ferroui_base::styling::{
    Container, ContainerSizing, IGlobalStyles, IStyle, IStyleHost, IThemeVariantHost, StyleHostRef, ThemeVariant,
};
use ferroui_base::threading::{Dispatcher, DispatcherTask};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, ferro_routed_event, AttachedProperty, DirectProperty,
    FerroLocator, FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs,
    IFerroDependencyResolver, PixelPoint, Rect, Ref, Size, StaticType, StyledElementImpl, StyledProperty,
    StyledPropertyOptions, Visual, VisualImpl,
};
use std::any::Any;
use std::cell::{Cell, OnceCell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

type PlatformImplBindings = HashMap<usize, Vec<Rc<dyn Fn()>>>;

/// Base class for top-level widgets.
///
/// This class acts as a base for top level widget. It handles scheduling
/// layout, styling and rendering as well as tracking the widget's
/// [`client_size`](TopLevel::client_size).
#[repr(C)]
pub struct TopLevel {
    base: ContentControl,
    dependency_resolver: RefCell<Option<Rc<dyn IFerroDependencyResolver>>>,
    input_manager: RefCell<Option<Rc<dyn IInputManager>>>,
    tooltip_service: RefCell<Option<Rc<dyn IToolTipService>>>,
    access_key_handler: RefCell<Option<Rc<dyn IAccessKeyHandler>>>,
    keyboard_navigation_handler: RefCell<Option<Rc<dyn IKeyboardNavigationHandler>>>,
    global_styles: RefCell<Option<Rc<dyn IGlobalStyles>>>,
    global_styles_subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    application_theme_host: RefCell<Option<Rc<dyn IThemeVariantHost>>>,
    application_theme_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    back_gesture_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    resources_changed_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    platform_impl_bindings: RefCell<PlatformImplBindings>,
    scaling: Cell<f64>,
    is_closed: Cell<bool>,
    client_size: Cell<Size>,
    frame_size: Cell<Option<Size>>,
    actual_transparency_level: Cell<WindowTransparencyLevel>,
    transparency_fallback_border: RefCell<Option<Ref<Border>>>,
    visual_layer_manager: RefCell<Option<Ref<crate::primitives::VisualLayerManager>>>,
    screens: RefCell<Option<Rc<Screens>>>,
    storage_provider: RefCell<Option<Rc<dyn ferroui_base::platform::storage::IStorageProvider>>>,
    source: OnceCell<Rc<PresentationSource>>,
    top_level_host: OnceCell<Ref<TopLevelHost>>,
    platform_impl: RefCell<Option<Rc<dyn ITopLevelImpl>>>,
    insets_paddings: RefCell<Option<Rc<dyn IDisposable>>>,
    insets_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    opened: HandlerList<dyn Fn()>,
    closed: HandlerList<dyn Fn()>,
    scaling_changed: HandlerList<dyn Fn()>,
    platform_lost_focus: HandlerList<dyn Fn()>,
    platform_deactivated: HandlerList<dyn Fn()>,
    platform_position_changed: HandlerList<dyn Fn(PixelPoint)>,
    opened_popups: RefCell<Vec<Ref<crate::primitives::Popup>>>,
}

ferro_class! {
    TopLevel: ContentControl, virtuals TopLevelImpl: ContentControlImpl {
        /// Handles a closed notification from the platform implementation.
        fn handle_closed(this);
        /// Handles a resize notification from the platform implementation:
        /// `client_size` is the new client size and `reason` the reason for
        /// the resize.
        fn handle_resized(this, client_size: Size, reason: WindowResizeReason);
        /// Raises the `Opened` event.
        fn on_opened(this);
        /// Raises the `Closed` event.
        fn on_closed(this);
        /// The popups that are currently open directly in this top level,
        /// in the order they were opened.
        fn opened_popups(this) -> Vec<Ref<crate::primitives::Popup>>;
    }
}

ferro_impl_classes!(TopLevel: LayoutableImpl, InteractiveImpl, InputElementImpl, ControlImpl, ContentControlImpl);

impl FerroObjectImpl for TopLevel {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.initialize();
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == ContentControl::content_property().as_property() {
            this.invalidate_child_insets_padding();
        } else {
            let actions = this.platform_impl_bindings.borrow().get(&property_key(change.property())).cloned();
            if let Some(actions) = actions {
                for action in actions {
                    action();
                }
            }
        }
    }
}

impl StyledElementImpl for TopLevel {
    fn is_logical_root(_this: &Self) -> bool {
        true
    }

    fn styling_parent(this: &Self) -> Option<StyleHostRef> {
        let global_styles = this.global_styles.borrow().clone()?;
        let host: Rc<dyn IStyleHost> = global_styles;
        Some(StyleHostRef::Other(host))
    }

    fn is_theme_variant_root(this: &Self) -> bool {
        this.application_theme_host.borrow().is_none()
    }
}

impl VisualImpl for TopLevel {
    fn bypass_flow_direction_policies(_this: &Self) -> bool {
        true
    }

    fn invalidate_mirror_transform(_this: &Self) {
        // Do nothing because TopLevel shouldn't apply MirrorTransform on himself.
    }
}

impl TemplatedControlImpl for TopLevel {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);

        {
            *this.visual_layer_manager.borrow_mut() =
                e.name_scope().find_as::<crate::primitives::VisualLayerManager>("PART_VisualLayerManager");
        }

        let Some(platform_impl) = this.platform_impl() else { return };

        *this.transparency_fallback_border.borrow_mut() = e.name_scope().find_as::<Border>("PART_TransparencyFallback");
        this.handle_transparency_level_changed(platform_impl.transparency_level());
    }
}

impl TopLevelImpl for TopLevel {
    fn handle_closed(this: &Self) {
        let source = this.presentation_source().clone();
        source.dispose();
        this.stop_rendering();

        debug_assert!(this.platform_impl.borrow().is_some());
        // The PlatformImpl is completely invalid at this point
        let platform_impl = this.platform_impl.borrow_mut().take();
        if let Some(platform_impl) = platform_impl {
            // The implementation may outlive the top-level it no longer
            // belongs to: let go of the callbacks, one of which holds the
            // top-level.
            platform_impl.set_closed(None);
            platform_impl.set_paint(None);
            platform_impl.set_resized(None);
            platform_impl.set_transparency_level_changed(None);
            platform_impl.set_platform_specific_scene_info_changed(None);
        }
        this.scaling.set(1.0);

        for subscription in this.global_styles_subscriptions.borrow_mut().drain(..) {
            subscription.dispose();
        }
        if let Some(subscription) = this.application_theme_subscription.borrow_mut().take() {
            subscription.dispose();
        }

        if let Some(subscription) = this.back_gesture_subscription.borrow_mut().take() {
            subscription.dispose();
        }

        let this_ref = this.to_ref();
        let logical_args = LogicalTreeAttachmentEventArgs::new(this_ref.clone().upcast(), this_ref.clone().upcast(), None);
        this.notify_detached_from_logical_tree(&logical_args);

        source.set_root_visual(None);

        this.on_closed();

        ILayoutManager::dispose(&**source.layout_manager());
        this.platform_impl_bindings.borrow_mut().clear();

        // The host and the top-level reference each other (visual child and
        // logical child); nothing collects the pair, so the closed top-level
        // leaves its host.
        if let Some(host) = this.top_level_host.get() {
            host.visual_children().remove(&this_ref.upcast());
        }
    }

    fn handle_resized(this: &Self, client_size: Size, _reason: WindowResizeReason) {
        this.set_client_size(client_size);
        this.set_width(client_size.width);
        this.set_height(client_size.height);
        this.layout_manager().execute_layout_pass();
        this.renderer().resized(client_size);
    }

    fn on_opened(this: &Self) {
        raise(&this.opened);
    }

    fn on_closed(this: &Self) {
        raise(&this.closed);
    }

    fn opened_popups(this: &Self) -> Vec<Ref<crate::primitives::Popup>> {
        this.opened_popups.borrow().clone()
    }
}

/// Raises a plain event through the dispatcher, the way the reference
/// implementation sends its event invocations.
fn raise(handlers: &HandlerList<dyn Fn()>) {
    if handlers.is_empty() {
        return;
    }
    let snapshot = handlers.snapshot();
    let _ = Dispatcher::ui_thread().invoke_local(move || {
        for (_, handler) in snapshot.iter() {
            handler();
        }
    });
}

fn property_key(property: &'static FerroProperty) -> usize {
    property as *const FerroProperty as usize
}

fn subscribe(handlers: &HandlerList<dyn Fn()>, owner: &TopLevel, select: fn(&TopLevel) -> &HandlerList<dyn Fn()>, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
    let token = handlers.add(handler);
    let weak = owner.to_ref().downgrade();
    Disposable::create(move || {
        if let Some(owner) = weak.upgrade() {
            select(&owner).remove(token);
        }
    })
}

/// Chains `handler` onto the lost-focus callback of a platform
/// implementation (C# `impl.LostFocus += handler`).
fn add_lost_focus(platform_impl: &Rc<dyn ITopLevelImpl>, handler: Rc<dyn Fn()>) {
    let previous = platform_impl.lost_focus();
    platform_impl.set_lost_focus(Some(Rc::new(move || {
        if let Some(previous) = &previous {
            previous();
        }
        handler();
    })));
}

impl TopLevel {
    /// The named parts expected in the control template.
    pub const TEMPLATE_PARTS: &'static [TemplatePartAttribute] = &[
        TemplatePartAttribute::new("PART_TransparencyFallback", <Border as StaticType>::TYPE),
        TemplatePartAttribute::new(
            "PART_VisualLayerManager",
            <crate::primitives::VisualLayerManager as StaticType>::TYPE,
        ),
    ];

    /// The layer manager of the control template, once the template is
    /// applied (private protected upstream: for derived classes).
    pub fn visual_layer_manager(&self) -> Option<Ref<crate::primitives::VisualLayerManager>> {
        self.visual_layer_manager.borrow().clone()
    }

    /// Enables the overlay, popup overlay and text selector layers of the
    /// layer manager of the control template (private protected upstream:
    /// for the top-levels that host their popups and overlays themselves).
    pub fn enable_visual_layer_manager_layers(&self) {
        if let Some(vlm) = self.visual_layer_manager() {
            vlm.set_enable_overlay_layer(true);
            vlm.set_enable_popup_overlay_layer(true);
            vlm.set_enable_text_selector_layer(true);
        }
    }
}

ferroui_base::ferro_properties! { impl TopLevel, also [
    TopLevel::system_bar_color_property,
    TopLevel::auto_safe_area_padding_property,
] {
    ferro_property!(
        /// Defines the `ClientSize` property.
        pub fn client_size_property() -> DirectProperty<TopLevel, Size> {
            FerroProperty::register_direct::<TopLevel, _>("ClientSize", |o| o.client_size(), None, Size::default())
        }
    );

    ferro_property!(
        /// Defines the `FrameSize` property.
        pub fn frame_size_property() -> DirectProperty<TopLevel, Option<Size>> {
            FerroProperty::register_direct::<TopLevel, _>("FrameSize", |o| o.frame_size(), None, None)
        }
    );

    ferro_property!(
        /// Defines the `TransparencyLevelHint` property.
        pub fn transparency_level_hint_property() -> StyledProperty<WindowTransparencyLevelCollection> {
            FerroProperty::register::<TopLevel, _>("TransparencyLevelHint", WindowTransparencyLevelCollection::default())
        }
    );

    ferro_property!(
        /// Defines the `ActualTransparencyLevel` property.
        pub fn actual_transparency_level_property() -> DirectProperty<TopLevel, WindowTransparencyLevel> {
            FerroProperty::register_direct::<TopLevel, _>(
                "ActualTransparencyLevel",
                |o| o.actual_transparency_level(),
                None,
                WindowTransparencyLevel::none(),
            )
        }
    );

    ferro_property!(
        /// Defines the `TransparencyBackgroundFallback` property.
        pub fn transparency_background_fallback_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            let white: Rc<dyn IBrush> = Brushes::white();
            FerroProperty::register::<TopLevel, _>("TransparencyBackgroundFallback", Some(white))
        }
    );
} }

impl TopLevel {
    /// Defines the `ActualThemeVariant` property.
    pub fn actual_theme_variant_property() -> &'static StyledProperty<Option<ThemeVariant>> {
        ThemeVariant::actual_theme_variant_property()
    }

    /// Defines the `RequestedThemeVariant` property.
    pub fn requested_theme_variant_property() -> &'static StyledProperty<Option<ThemeVariant>> {
        ThemeVariant::requested_theme_variant_property()
    }

    ferro_property!(for TopLevel;
        /// Defines the SystemBarColor attached property.
        pub fn system_bar_color_property() -> AttachedProperty<Option<Ref<SolidColorBrush>>> {
            FerroProperty::register_attached_with::<TopLevel, Control, _>(
                "SystemBarColor",
                StyledPropertyOptions::new(None).inherits(true),
            )
        }
    );

    ferro_property!(for TopLevel;
        /// Defines the AutoSafeAreaPadding attached property.
        pub fn auto_safe_area_padding_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached::<TopLevel, Control, _>("AutoSafeAreaPadding", true)
        }
    );

    ferro_routed_event!(
        /// Defines the `BackRequested` event.
        pub fn back_requested_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<TopLevel, _>("BackRequested", RoutingStrategies::BUBBLE)
        }
    );

    fn static_constructor() {
        KeyboardNavigation::tab_navigation_property().override_default_value::<TopLevel>(KeyboardNavigationMode::Cycle);
        Container::sizing_property().override_default_value::<TopLevel>(ContainerSizing::WidthAndHeight);

        Layoutable::affects_measure::<TopLevel>(&[Self::client_size_property().as_property()]);

        Self::system_bar_color_property().changed().add_class_handler::<Control>(|view, e| {
            let (_, new_value) = e.get_old_and_new_value::<Option<Ref<SolidColorBrush>>>();
            if let Some(color_brush) = new_value {
                if let Some(insets_manager) =
                    view.parent().and_then(|parent| parent.cast::<TopLevel>()).and_then(|tl| tl.insets_manager())
                {
                    insets_manager.set_system_bar_color(Some(color_brush.color()));
                }

                if let Some(insets) = view.downcast_ref::<TopLevel>().and_then(|tl| tl.insets_manager()) {
                    insets.set_system_bar_color(Some(color_brush.color()));
                }
            }
        });

        Self::auto_safe_area_padding_property().changed().add_class_handler::<Control>(|view, _| {
            let top_level = view
                .to_ref()
                .cast::<TopLevel>()
                .or_else(|| view.parent().and_then(|parent| parent.cast::<TopLevel>()));
            if let Some(top_level) = top_level {
                top_level.invalidate_child_insets_padding();
            }
        });

        // The subscription lives as long as the property.
        let _ = ToolTip::service_enabled_property().changed().subscribe(Self::on_tool_tip_service_enabled_changed);
    }

    fn on_tool_tip_service_enabled_changed(args: &FerroPropertyChangedEventArgs<'_>) {
        if args.get_new_value::<bool>() && args.priority() != BindingPriority::Inherited {
            let Some(visual) = args.sender().downcast_ref::<Visual>() else { return };
            let Some(top_level) = Self::get_top_level(Some(visual)) else { return };
            let offset = visual
                .translate_point(Point::default(), &top_level)
                .expect("the top-level is an ancestor of the visual");
            top_level.update_tool_tip(visual.bounds().translate(Vector::new(offset.x, offset.y)));
        }
    }

    fn update_tool_tip(&self, dirty_rect: Rect) {
        let tooltip_service = self.tooltip_service.borrow().clone();
        let Some(tooltip_service) = tooltip_service else { return };
        if !self.is_pointer_over() {
            return;
        }
        let source = self.presentation_source();
        let Some(last_pos) = source.get_last_pointer_position(self) else { return };

        let client_point = Visual::point_to_client(self, last_pos);
        if dirty_rect.contains(client_point) {
            let input_root: Rc<dyn IInputRoot> = source.clone();
            let candidate = self.input_hit_test_with(client_point, false).map(|element| element.upcast::<Visual>());
            tooltip_service.update(&input_root, candidate);
        }
    }

    /// Field initialisation of a top-level over the platform-specific
    /// implementation `platform_impl`, with the services of the current
    /// service locator.
    pub fn construct(platform_impl: Rc<dyn ITopLevelImpl>) -> Self {
        Self::construct_with_resolver(platform_impl, None)
    }

    /// Field initialisation of a top-level; `dependency_resolver` is the
    /// dependency resolver to use, the current service locator when `None`.
    pub fn construct_with_resolver(
        platform_impl: Rc<dyn ITopLevelImpl>,
        dependency_resolver: Option<Rc<dyn IFerroDependencyResolver>>,
    ) -> Self {
        Self {
            base: ContentControl::construct(),
            dependency_resolver: RefCell::new(dependency_resolver),
            input_manager: RefCell::new(None),
            tooltip_service: RefCell::new(None),
            access_key_handler: RefCell::new(None),
            keyboard_navigation_handler: RefCell::new(None),
            global_styles: RefCell::new(None),
            global_styles_subscriptions: RefCell::new(Vec::new()),
            application_theme_host: RefCell::new(None),
            application_theme_subscription: RefCell::new(None),
            back_gesture_subscription: RefCell::new(None),
            resources_changed_subscription: RefCell::new(None),
            platform_impl_bindings: RefCell::new(HashMap::new()),
            scaling: Cell::new(1.0),
            is_closed: Cell::new(false),
            client_size: Cell::new(Size::default()),
            frame_size: Cell::new(None),
            actual_transparency_level: Cell::new(WindowTransparencyLevel::none()),
            transparency_fallback_border: RefCell::new(None),
            visual_layer_manager: RefCell::new(None),
            screens: RefCell::new(None),
            storage_provider: RefCell::new(None),
            source: OnceCell::new(),
            top_level_host: OnceCell::new(),
            platform_impl: RefCell::new(Some(platform_impl)),
            insets_paddings: RefCell::new(None),
            insets_subscription: RefCell::new(None),
            opened: HandlerList::new(),
            closed: HandlerList::new(),
            scaling_changed: HandlerList::new(),
            platform_lost_focus: HandlerList::new(),
            platform_deactivated: HandlerList::new(),
            platform_position_changed: HandlerList::new(),
            opened_popups: RefCell::new(Vec::new()),
        }
    }

    /// The constructor body.
    fn initialize(&self) {
        let this = self.to_ref();
        let platform_impl = self.platform_impl().expect(
            "Could not create window implementation: maybe no windowing subsystem was initialized?",
        );
        let dependency_resolver =
            self.dependency_resolver.borrow_mut().take().unwrap_or_else(FerroLocator::current);
        let dependency_resolver: &dyn IFerroDependencyResolver = &*dependency_resolver;

        let host_visual = TopLevelHost::new(&this);
        if self.top_level_host.set(host_visual.clone()).is_err() {
            unreachable!("the top-level is initialized once");
        }
        host_visual.set_parent(this.clone());
        self.logical_children().add(host_visual.clone().upcast());

        let source = PresentationSource::new(
            host_visual.upcast(),
            &this.clone().upcast(),
            platform_impl.clone(),
            dependency_resolver,
        );
        if self.source.set(source.clone()).is_err() {
            unreachable!("the top-level is initialized once");
        }
        // The handler lives as long as the renderer; it does nothing once
        // the top-level is gone.
        let _ = source.typed_renderer().scene_invalidated(Rc::new({
            let weak = this.downgrade();
            move |e| {
                if let Some(this) = weak.upgrade() {
                    this.update_tool_tip(e.dirty_rect());
                }
            }
        }));

        self.scaling.set(LayoutHelper::validate_scaling(platform_impl.render_scaling()));
        self.actual_transparency_level.set(platform_impl.transparency_level());

        let renderer = source.typed_renderer();
        renderer.set_transparency_level(to_composition_transparency_level(self.actual_transparency_level.get()));
        renderer.set_platform_specific_scene_info(platform_impl.platform_specific_scene_info());

        *self.access_key_handler.borrow_mut() = try_get_service::<dyn IAccessKeyHandler>(dependency_resolver);
        *self.input_manager.borrow_mut() = try_get_service::<dyn IInputManager>(dependency_resolver);
        *self.tooltip_service.borrow_mut() = try_get_service::<dyn IToolTipService>(dependency_resolver);
        *self.keyboard_navigation_handler.borrow_mut() =
            try_get_service::<dyn IKeyboardNavigationHandler>(dependency_resolver);
        *self.global_styles.borrow_mut() = try_get_service::<dyn IGlobalStyles>(dependency_resolver);
        *self.application_theme_host.borrow_mut() = try_get_service::<dyn IThemeVariantHost>(dependency_resolver);

        let weak = this.downgrade();
        // The closed callback holds the top-level: as with the delegates of
        // the reference, the platform implementation keeps its top-level
        // alive until it reports that it has closed. The callbacks are
        // released in `handle_closed`.
        platform_impl.set_closed(Some(Rc::new({
            let this = this.clone();
            move || this.ensure_closed()
        })));
        platform_impl.set_paint(Some(Rc::new({
            let weak = weak.clone();
            move |rect| {
                if let Some(this) = weak.upgrade() {
                    this.handle_paint(rect);
                }
            }
        })));
        platform_impl.set_resized(Some(Rc::new({
            let weak = weak.clone();
            move |client_size, reason| {
                if let Some(this) = weak.upgrade() {
                    this.handle_resized(client_size, reason);
                }
            }
        })));
        // The handler lives as long as the platform implementation; it does
        // nothing once the top-level is gone or closed.
        let _ = crate::presentation_source::add_scaling_changed(
            &platform_impl,
            Rc::new({
                let weak = weak.clone();
                move |scaling| {
                    if let Some(this) = weak.upgrade() {
                        this.handle_scaling_changed(scaling);
                    }
                }
            }),
        );
        platform_impl.set_transparency_level_changed(Some(Rc::new({
            let weak = weak.clone();
            move |level| {
                if let Some(this) = weak.upgrade() {
                    this.handle_transparency_level_changed(level);
                }
            }
        })));
        platform_impl.set_platform_specific_scene_info_changed(Some(Rc::new({
            let weak = weak.clone();
            move |scene_info| {
                if let Some(this) = weak.upgrade() {
                    this.handle_platform_specific_scene_info_changed(scene_info);
                }
            }
        })));

        self.create_platform_impl_binding(Self::transparency_level_hint_property(), {
            let weak = weak.clone();
            move |hint: WindowTransparencyLevelCollection| {
                if let Some(platform_impl) = weak.upgrade().and_then(|this| this.platform_impl()) {
                    platform_impl.set_transparency_level_hint(&hint);
                }
            }
        });

        let keyboard_navigation_handler = self.keyboard_navigation_handler.borrow().clone();
        if let Some(handler) = keyboard_navigation_handler {
            handler.set_owner(&this.clone().upcast());
        }
        let access_key_handler = self.access_key_handler.borrow().clone();
        if let Some(handler) = access_key_handler {
            handler.set_owner(&this.clone().upcast());
        }

        let global_styles = self.global_styles.borrow().clone();
        if let Some(global_styles) = &global_styles {
            let added = global_styles.global_styles_added(Rc::new({
                let weak = weak.clone();
                move |styles: &[Rc<dyn IStyle>]| {
                    if let Some(this) = weak.upgrade() {
                        this.styles_added(styles);
                    }
                }
            }));
            let removed = global_styles.global_styles_removed(Rc::new({
                let weak = weak.clone();
                move |styles: &[Rc<dyn IStyle>]| {
                    if let Some(this) = weak.upgrade() {
                        this.styles_removed(styles);
                    }
                }
            }));
            self.global_styles_subscriptions.borrow_mut().extend([added, removed]);
        }

        let application_theme_host = self.application_theme_host.borrow().clone();
        if let Some(host) = &application_theme_host {
            let _ = self.set_value_with_priority(
                Self::actual_theme_variant_property(),
                host.actual_theme_variant(),
                BindingPriority::Template,
            );
            let weak_host = Rc::downgrade(host);
            let subscription = host.actual_theme_variant_changed(Rc::new({
                let weak = weak.clone();
                move || {
                    if let (Some(this), Some(host)) = (weak.upgrade(), weak_host.upgrade()) {
                        this.global_actual_theme_variant_changed(&*host);
                    }
                }
            }));
            *self.application_theme_subscription.borrow_mut() = Some(subscription);
        } else {
            ThemeVariant::update_actual_theme_variant(self);
        }

        self.create_platform_impl_binding(Self::actual_theme_variant_property(), {
            let weak = weak.clone();
            move |variant: Option<ThemeVariant>| {
                if let Some(platform_impl) = weak.upgrade().and_then(|this| this.platform_impl()) {
                    platform_impl.set_frame_theme_variant(variant.as_ref().and_then(to_platform_theme_variant));
                }
            }
        });

        self.set_client_size(platform_impl.client_size());

        if let Some(application_resources) = &global_styles {
            let subscription = application_resources.resources_changed(Rc::new({
                let weak = weak.clone();
                move |e| {
                    if let Some(this) = weak.upgrade() {
                        this.notify_resources_changed(*e, true);
                    }
                }
            }));
            *self.resources_changed_subscription.borrow_mut() = Some(subscription);
        }

        add_lost_focus(
            &platform_impl,
            Rc::new({
                let weak = weak.clone();
                move || {
                    if let Some(this) = weak.upgrade() {
                        this.platform_impl_lost_focus();
                        for (_, handler) in this.platform_lost_focus.snapshot().iter() {
                            handler();
                        }
                    }
                }
            }),
        );

        // The handlers that others add to the window callbacks of the
        // platform implementation go through the top-level (see
        // `platform_deactivated`); a window base replaces these callbacks
        // with its own, which raise the same lists.
        if let Some(window_base_impl) = platform_impl.as_window_base_impl() {
            window_base_impl.set_deactivated(Some(Rc::new({
                let weak = weak.clone();
                move || {
                    if let Some(this) = weak.upgrade() {
                        this.raise_platform_deactivated();
                    }
                }
            })));
            window_base_impl.set_position_changed(Some(Rc::new({
                let weak = weak.clone();
                move |position| {
                    if let Some(this) = weak.upgrade() {
                        this.raise_platform_position_changed(position);
                    }
                }
            })));
        }

        if let Some(system_navigation_manager) = self.optional_feature::<dyn ISystemNavigationManagerImpl>() {
            // As in the reference, the handler stays for the life of the
            // platform implementation.
            let _ = system_navigation_manager.back_requested(Rc::new({
                let weak = weak.clone();
                move |e: &Rc<RoutedEventArgs>| {
                    if let Some(this) = weak.upgrade() {
                        e.set_routed_event(Some(Self::back_requested_event()));
                        let e = e.clone();
                        let _ = Dispatcher::ui_thread().invoke_local(move || this.raise_event(&*e));
                    }
                }
            }));
        }

        let input_manager = self.input_manager.borrow().clone();
        *self.back_gesture_subscription.borrow_mut() = input_manager.map(|input_manager| {
            input_manager.pre_process().subscribe(Rc::new(AnonymousObserver::new({
                let weak = weak.clone();
                move |e: Rc<dyn IRawInputEventArgs>| {
                    if let Some(this) = weak.upgrade() {
                        this.back_gesture_pre_process(&*e);
                    }
                }
            })))
        });
    }

    fn back_gesture_pre_process(&self, e: &dyn IRawInputEventArgs) {
        let input_root = self.input_root();
        if !std::ptr::addr_eq(Rc::as_ptr(e.root()), Rc::as_ptr(&input_root)) {
            return;
        }

        let mut back_requested = false;

        if let Some(raw_key_event_args) = e.downcast_ref::<RawKeyEventArgs>() {
            if raw_key_event_args.type_() == RawKeyEventType::KeyDown {
                if let Some(platform_settings) = self.platform_settings() {
                    let hotkeys = platform_settings.hotkey_configuration();

                    let mut key_event = KeyEventArgs::new();
                    key_event.key_modifiers = KeyModifiers::from_bits_retain(raw_key_event_args.modifiers().bits());
                    key_event.key = raw_key_event_args.key();
                    key_event.physical_key = raw_key_event_args.physical_key();
                    key_event.key_device_type = raw_key_event_args.key_device_type();
                    key_event.key_symbol = raw_key_event_args.key_symbol();

                    back_requested = hotkeys.back.iter().any(|key| key.matches(Some(&key_event)));
                }
            }
        } else if let Some(pointer_event_args) = e.downcast_ref::<RawPointerEventArgs>() {
            back_requested = pointer_event_args.type_() == RawPointerEventType::XButton1Down;
        }

        if back_requested {
            let back_requested_event_args = Rc::new(RoutedEventArgs::with_event(Self::back_requested_event()));
            let this = self.to_ref();
            let args = back_requested_event_args.clone();
            let _ = Dispatcher::ui_thread().invoke_local(move || this.raise_event(&*args));

            e.set_handled(back_requested_event_args.handled());
        }
    }

    /// Fired when the window is opened. Disposing the returned handle
    /// unsubscribes.
    pub fn opened(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        subscribe(&self.opened, self, |tl| &tl.opened, Rc::new(handler))
    }

    /// Fired when the window is closed. Disposing the returned handle
    /// unsubscribes.
    pub fn closed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        subscribe(&self.closed, self, |tl| &tl.closed, Rc::new(handler))
    }

    /// Fired when the top-level's scaling changes. Disposing the returned
    /// handle unsubscribes.
    pub fn scaling_changed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        subscribe(&self.scaling_changed, self, |tl| &tl.scaling_changed, Rc::new(handler))
    }

    /// Adds a handler to the lost-focus notification of the platform
    /// implementation (C# `PlatformImpl.LostFocus += handler`); it runs
    /// after the top-level's own handling. Disposing the returned handle
    /// removes it.
    ///
    /// The callbacks of a platform implementation hold a single function,
    /// so handlers that come and go (an open popup watching its parent) are
    /// kept in a list of the top-level instead of being chained onto the
    /// callback, where a removed handler would stay behind.
    pub fn platform_lost_focus(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        subscribe(&self.platform_lost_focus, self, |tl| &tl.platform_lost_focus, Rc::new(handler))
    }

    /// Adds a handler to the deactivated notification of a window platform
    /// implementation (C# `impl.Deactivated += handler`); see
    /// [`platform_lost_focus`](Self::platform_lost_focus).
    pub fn platform_deactivated(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        subscribe(&self.platform_deactivated, self, |tl| &tl.platform_deactivated, Rc::new(handler))
    }

    /// Adds a handler to the position-changed notification of a window
    /// platform implementation (C# `impl.PositionChanged += handler`); see
    /// [`platform_lost_focus`](Self::platform_lost_focus).
    pub fn platform_position_changed(&self, handler: impl Fn(PixelPoint) + 'static) -> Rc<dyn IDisposable> {
        let token = self.platform_position_changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.platform_position_changed.remove(token);
            }
        })
    }

    pub(crate) fn raise_platform_deactivated(&self) {
        for (_, handler) in self.platform_deactivated.snapshot().iter() {
            handler();
        }
    }

    pub(crate) fn raise_platform_position_changed(&self, position: PixelPoint) {
        for (_, handler) in self.platform_position_changed.snapshot().iter() {
            handler(position);
        }
    }

    /// The client size of the window.
    pub fn client_size(&self) -> Size {
        self.client_size.get()
    }

    /// Sets the client size of the window (protected upstream: for derived
    /// classes).
    pub fn set_client_size(&self, value: Size) {
        self.set_and_raise_cell(Self::client_size_property(), &self.client_size, value);
    }

    /// The total size of the window.
    pub fn frame_size(&self) -> Option<Size> {
        self.frame_size.get()
    }

    /// Sets the total size of the window (protected upstream: for derived
    /// classes).
    pub fn set_frame_size(&self, value: Option<Size>) {
        self.set_and_raise_cell(Self::frame_size_property(), &self.frame_size, value);
    }

    /// The transparency levels that the top-level should use when possible.
    /// Accepts multiple values which are applied in a fallback order. For
    /// instance, with "Mica, Blur" Mica will be applied only on platforms
    /// where it is possible, and Blur will be used on the rest of them.
    /// Default value is an empty collection or "None".
    pub fn transparency_level_hint(&self) -> WindowTransparencyLevelCollection {
        self.get_value(Self::transparency_level_hint_property())
    }

    pub fn set_transparency_level_hint(&self, value: WindowTransparencyLevelCollection) {
        self.set_value(Self::transparency_level_hint_property(), value)
    }

    /// The achieved transparency level that the platform was able to
    /// provide.
    pub fn actual_transparency_level(&self) -> WindowTransparencyLevel {
        self.actual_transparency_level.get()
    }

    fn set_actual_transparency_level(&self, value: WindowTransparencyLevel) {
        self.set_and_raise_cell(Self::actual_transparency_level_property(), &self.actual_transparency_level, value);
    }

    /// The brush that transparency will blend with when transparency is not
    /// supported. By default this is a solid white brush.
    pub fn transparency_background_fallback(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::transparency_background_fallback_property())
    }

    pub fn set_transparency_background_fallback(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::transparency_background_fallback_property(), value)
    }

    /// The theme variant requested for the top-level and its descendants.
    pub fn requested_theme_variant(&self) -> Option<ThemeVariant> {
        self.get_value(Self::requested_theme_variant_property())
    }

    pub fn set_requested_theme_variant(&self, value: Option<ThemeVariant>) {
        self.set_value(Self::requested_theme_variant_property(), value)
    }

    /// Occurs when physical Back Button is pressed or a back navigation has
    /// been requested.
    pub fn back_requested(&self, handler: impl Fn(&Interactive, &RoutedEventArgs) + 'static) -> RoutedEventHandlerToken {
        self.add_handler(Self::back_requested_event(), handler)
    }

    /// The layout manager of the top-level.
    pub fn layout_manager(&self) -> Rc<LayoutManager> {
        self.presentation_source().layout_manager().clone()
    }

    /// The platform-specific window implementation; `None` once the
    /// top-level has closed.
    pub fn platform_impl(&self) -> Option<Rc<dyn ITopLevelImpl>> {
        self.platform_impl.borrow().clone()
    }

    /// Tries to get the platform handle for the top-level-derived control.
    ///
    /// Returns the handle describing the window, or `None` if the handle
    /// could not be retrieved.
    pub fn try_get_platform_handle(&self) -> Option<Rc<dyn IPlatformHandle>> {
        self.platform_impl().and_then(|platform_impl| platform_impl.handle())
    }

    /// Calls `on_value` with the value of `property` now and whenever the
    /// property changes, for as long as the top-level has a platform
    /// implementation (private protected upstream: for derived classes).
    pub fn create_platform_impl_binding<T: ferroui_base::PropertyValue>(
        &self,
        property: &'static StyledProperty<T>,
        on_value: impl Fn(T) + 'static,
    ) {
        let weak = self.to_ref().downgrade();
        let update_platform_impl: Rc<dyn Fn()> = Rc::new(move || {
            if let Some(this) = weak.upgrade() {
                if this.platform_impl.borrow().is_some() {
                    on_value(this.get_value(property));
                }
            }
        });
        self.platform_impl_bindings
            .borrow_mut()
            .entry(property_key(property.as_property()))
            .or_default()
            .push(update_platform_impl.clone());

        // execute the action now to handle the default value, which may have been overridden
        update_platform_impl();
    }

    /// The presentation source of the top-level.
    pub fn presentation_source(&self) -> &Rc<PresentationSource> {
        self.source.get().expect("the presentation source is created with the top-level")
    }

    /// The host of the top-level: the root of its visual tree.
    pub(crate) fn top_level_host(&self) -> &Ref<TopLevelHost> {
        self.top_level_host.get().expect("the host is created with the top-level")
    }

    /// The input root of the top-level.
    pub fn input_root(&self) -> Rc<dyn IInputRoot> {
        self.presentation_source().clone()
    }

    /// The renderer for the window.
    pub fn renderer(&self) -> Rc<dyn ITopLevelRenderer> {
        self.presentation_source().typed_renderer().clone()
    }

    /// The hit tester used instead of the renderer's. This is here purely
    /// for unit tests that don't want to set up a proper hit-testable
    /// visual tree.
    pub fn hit_tester_override(&self) -> Option<Rc<dyn IHitTester>> {
        self.presentation_source().hit_tester_override()
    }

    pub fn set_hit_tester_override(&self, value: Option<Rc<dyn IHitTester>>) {
        self.presentation_source().set_hit_tester_override(value)
    }

    /// A value controlling whether the renderer should draw specific
    /// diagnostics.
    pub fn renderer_diagnostics(&self) -> Rc<RendererDiagnostics> {
        self.renderer().diagnostics()
    }

    /// The last known position of the pointer, in screen coordinates.
    pub fn last_pointer_position(&self) -> Option<PixelPoint> {
        self.presentation_source().get_last_pointer_position(self)
    }

    /// The access key handler for the window.
    pub fn access_key_handler(&self) -> Option<Rc<dyn IAccessKeyHandler>> {
        self.access_key_handler.borrow().clone()
    }

    /// The keyboard navigation handler of the window (for unit tests).
    pub fn tests_keyboard_navigation_handler(&self) -> Option<Rc<dyn IKeyboardNavigationHandler>> {
        self.keyboard_navigation_handler.borrow().clone()
    }

    /// Helper for setting the color of the platform's system bars.
    /// `control` is the main view attached to the top-level, or the
    /// top-level.
    pub fn set_system_bar_color(control: &Control, color: Option<Ref<SolidColorBrush>>) {
        control.set_value(Self::system_bar_color_property(), color);
    }

    /// Helper for getting the color of the platform's system bars.
    /// `control` is the main view attached to the top-level, or the
    /// top-level.
    pub fn get_system_bar_color(control: &Control) -> Option<Ref<SolidColorBrush>> {
        control.get_value(Self::system_bar_color_property())
    }

    /// Enables or disables whenever the top-level should automatically
    /// adjust paddings depending on the safe area. `control` is the main
    /// view attached to the top-level, or the top-level.
    pub fn set_auto_safe_area_padding(control: &Control, value: bool) {
        control.set_value(Self::auto_safe_area_padding_property(), value);
    }

    /// Gets if auto safe area padding is enabled. `control` is the main
    /// view attached to the top-level, or the top-level.
    pub fn get_auto_safe_area_padding(control: &Control) -> bool {
        control.get_value(Self::auto_safe_area_padding_property())
    }

    /// The scaling factor to use in rendering.
    pub fn render_scaling(&self) -> f64 {
        self.scaling.get()
    }

    fn optional_feature<T: ?Sized + 'static>(&self) -> Option<Rc<T>> {
        let platform_impl = self.platform_impl()?;
        let provider: &dyn IOptionalFeatureProvider = &*platform_impl;
        provider.try_get::<T>()
    }

    /// File System storage service used for file pickers and bookmarks:
    /// the provider of the registered storage provider factory, else the
    /// one of the platform, else one without pickers. The provider is
    /// created on first use and kept.
    pub fn storage_provider(&self) -> Rc<dyn ferroui_base::platform::storage::IStorageProvider> {
        use ferroui_base::platform::storage::{IStorageProvider, NoopStorageProvider};
        use ferroui_base::LocatorExtensions;

        if let Some(storage_provider) = self.storage_provider.borrow().clone() {
            return storage_provider;
        }

        let storage_provider = FerroLocator::current()
            .get_service::<dyn crate::platform::IStorageProviderFactory>()
            .map(|factory| factory.create_provider(&self.to_ref()))
            .or_else(|| self.optional_feature::<dyn IStorageProvider>())
            .unwrap_or_else(|| Rc::new(NoopStorageProvider));
        *self.storage_provider.borrow_mut() = Some(storage_provider.clone());
        storage_provider
    }

    /// The insets manager of the platform, if it has one.
    pub fn insets_manager(&self) -> Option<Rc<dyn IInsetsManager>> {
        self.optional_feature::<dyn IInsetsManager>()
    }

    /// The input pane of the platform, if it has one.
    pub fn input_pane(&self) -> Option<Rc<dyn IInputPane>> {
        self.optional_feature::<dyn IInputPane>()
    }

    /// The launcher of the platform: starts the default app associated
    /// with a file or a URI. A launcher that launches nothing when the
    /// platform has none.
    pub fn launcher(&self) -> Rc<dyn ferroui_base::platform::storage::ILauncher> {
        use ferroui_base::platform::storage::{ILauncher, NoopLauncher};

        self.optional_feature::<dyn ILauncher>().unwrap_or_else(|| Rc::new(NoopLauncher))
    }

    /// The platform screens implementation.
    pub fn screens(&self) -> Option<Rc<Screens>> {
        if let Some(screens) = self.screens.borrow().clone() {
            return Some(screens);
        }
        let screens = Screens::new(self.optional_feature::<dyn IScreenImpl>()?);
        *self.screens.borrow_mut() = Some(screens.clone());
        Some(screens)
    }

    /// The platform's clipboard implementation.
    pub fn clipboard(&self) -> Option<Rc<dyn IClipboard>> {
        self.try_get_clipboard(ClipboardType::Default)
    }

    /// Gets the platform's clipboard of the specified type, or `None` if
    /// the platform doesn't provide it.
    pub fn try_get_clipboard(&self, type_: ClipboardType) -> Option<Rc<dyn IClipboard>> {
        if let Some(clipboard_manager) = self.optional_feature::<dyn IPlatformClipboardManagerImpl>() {
            return clipboard_manager.try_get_clipboard(type_);
        }

        if type_ == ClipboardType::Default {
            self.optional_feature::<dyn IClipboard>()
        } else {
            None
        }
    }

    /// The focus manager of the root.
    pub fn focus_manager(&self) -> Rc<FocusManager> {
        self.presentation_source().focus_manager().clone()
    }

    /// The platform-specific settings.
    fn platform_settings(&self) -> Option<Rc<dyn IPlatformSettings>> {
        use ferroui_base::LocatorExtensions;
        FerroLocator::current().get_service::<dyn IPlatformSettings>()
    }

    /// Gets the top-level in which the given visual is hosted.
    pub fn get_top_level(visual: Option<&Visual>) -> Option<Ref<TopLevel>> {
        let mut visual = visual.map(|visual| visual.to_ref());
        while let Some(current) = visual {
            if let Some(tl) = current.clone().cast::<TopLevel>() {
                return Some(tl);
            }
            visual = current.visual_parent();
        }

        None
    }

    /// Requests a [`PlatformInhibitionType`] to be inhibited. The behavior
    /// remains inhibited until the returned value is disposed. The
    /// available set of [`PlatformInhibitionType`]s depends on the platform.
    /// If a behavior is inhibited on a platform where this type is not
    /// supported the request will have no effect.
    ///
    /// Like the asynchronous method of the reference, the request is sent to
    /// the platform before this returns; the returned task completes, from
    /// a dispatcher job unless the platform completed at once, when the
    /// platform has applied it. Disposing the result starts the request
    /// that lifts the inhibition and returns without waiting for the
    /// platform to complete it.
    pub fn request_platform_inhibition(
        &self,
        type_: PlatformInhibitionType,
        reason: &str,
    ) -> DispatcherTask<Rc<dyn IDisposable>> {
        let platform_behavior_inhibition = self.optional_feature::<dyn IPlatformBehaviorInhibition>();
        let reason = reason.to_owned();

        Dispatcher::ui_thread().to_task_scheduler().start_local(async move {
            let Some(platform_behavior_inhibition) = platform_behavior_inhibition else {
                return Disposable::create(|| {});
            };

            match type_ {
                PlatformInhibitionType::AppSleep => {
                    platform_behavior_inhibition.set_inhibit_app_sleep(true, &reason).await;
                    Disposable::create(move || {
                        let lifted = platform_behavior_inhibition.set_inhibit_app_sleep(false, &reason);
                        drop(Dispatcher::ui_thread().to_task_scheduler().start_local(lifted));
                    })
                }
            }
        })
    }

    /// Enqueues a callback to be called on the next animation tick.
    pub fn request_animation_frame(&self, action: impl FnOnce(ferroui_base::animation::TimeSpan) + 'static) {
        Dispatcher::ui_thread().verify_access();
        MediaContext::instance().request_animation_frame(action);
    }

    fn invalidate_child_insets_padding(&self) {
        let Some(child) = self.content().and_then(|content| Control::from_boxed(&content)) else { return };
        let Some(insets_manager) = self.insets_manager() else { return };

        if let Some(subscription) = self.insets_subscription.borrow_mut().take() {
            subscription.dispose();
        }
        if let Some(insets_paddings) = self.insets_paddings.borrow_mut().take() {
            insets_paddings.dispose();
        }

        if child.get_value(Self::auto_safe_area_padding_property()) {
            let weak = self.to_ref().downgrade();
            let subscription = insets_manager.safe_area_changed(Rc::new(move |_: &SafeAreaChangedArgs| {
                if let Some(this) = weak.upgrade() {
                    this.invalidate_child_insets_padding();
                }
            }));
            *self.insets_subscription.borrow_mut() = Some(subscription);
            // lower priority, so it can be redefined by user
            let insets_paddings = child.set_value_with_priority(
                TemplatedControl::padding_property(),
                insets_manager.safe_area_padding(),
                BindingPriority::Style,
            );
            *self.insets_paddings.borrow_mut() = insets_paddings;
        }
    }

    /// Handles a paint notification from the platform implementation;
    /// `rect` is the dirty area.
    fn handle_paint(&self, rect: Rect) {
        self.renderer().paint(rect);
    }

    /// Registers the top-level with the media context, which starts its
    /// renderer (private protected upstream: for derived classes).
    pub fn start_rendering(&self) {
        let renderer: Rc<dyn IRenderer> = self.renderer();
        MediaContext::instance().add_top_level(self.media_context_key(), renderer);
    }

    /// Unregisters the top-level from the media context, which stops its
    /// renderer (private protected upstream: for derived classes).
    pub fn stop_rendering(&self) {
        MediaContext::instance().remove_top_level(self.media_context_key());
    }

    /// The key that identifies the top-level in the media context.
    pub fn media_context_key(&self) -> usize {
        self as *const TopLevel as usize
    }

    /// Runs the top level teardown exactly once, no matter whether it was
    /// initiated by the platform via the closed notification or by a
    /// managed dispose (private protected upstream: for derived classes).
    pub fn ensure_closed(&self) {
        if self.is_closed.replace(true) {
            return;
        }
        self.handle_closed();
    }

    /// Handles a window scaling change notification from the platform
    /// implementation.
    fn handle_scaling_changed(&self, scaling: f64) {
        if self.is_closed.get() {
            return;
        }
        self.scaling.set(LayoutHelper::validate_scaling(scaling));
        LayoutHelper::invalidate_self_and_children_measure(self);
        raise(&self.scaling_changed);

        self.invalidate_child_insets_padding();
    }

    fn handle_transparency_level_changed(&self, transparency_level: WindowTransparencyLevel) {
        let border = self.transparency_fallback_border.borrow().clone();
        if let Some(border) = border {
            if transparency_level == WindowTransparencyLevel::none() {
                border.set_background(self.transparency_background_fallback());
            } else {
                border.set_background(None);
            }
        }

        self.set_actual_transparency_level(transparency_level);
        self.renderer().set_transparency_level(to_composition_transparency_level(transparency_level));
    }

    fn handle_platform_specific_scene_info_changed(&self, scene_info: Option<std::sync::Arc<dyn Any + Send + Sync>>) {
        self.renderer().set_platform_specific_scene_info(scene_info);
    }

    fn global_actual_theme_variant_changed(&self, sender: &dyn IThemeVariantHost) {
        let _ = self.set_value_with_priority(
            Self::actual_theme_variant_property(),
            sender.actual_theme_variant(),
            BindingPriority::Template,
        );
    }

    /// Records a popup opened directly in this top level (internal
    /// upstream).
    pub fn add_opened_popup(&self, popup: &Ref<crate::primitives::Popup>) {
        self.opened_popups.borrow_mut().push(popup.clone());
    }

    /// Forgets a popup opened directly in this top level (internal
    /// upstream).
    pub fn remove_opened_popup(&self, popup: &Ref<crate::primitives::Popup>) {
        let mut opened_popups = self.opened_popups.borrow_mut();
        if let Some(index) = opened_popups.iter().position(|p| p == popup) {
            opened_popups.remove(index);
        }
    }

    fn platform_impl_lost_focus(&self) {
        let focused = self.focus_manager().get_focused_element();
        let focused = Self::get_top_level(focused.as_deref().map(|element| -> &Visual { element }));

        if focused.is_some_and(|focused| std::ptr::eq(&*focused, self)) {
            if let Some(keyboard_device) = KeyboardDevice::instance() {
                keyboard_device.set_focused_element_with(None, NavigationMethod::Unspecified, KeyModifiers::NONE, false);
            }
        }
    }
}

impl Drop for TopLevel {
    fn drop(&mut self) {
        if let Some(subscription) = self.resources_changed_subscription.borrow_mut().take() {
            subscription.dispose();
        }
    }
}

/// The frame theme variant of a theme variant: light and dark map to
/// themselves, a custom variant to what it inherits from, and the default
/// variant to nothing (follow the system).
fn to_platform_theme_variant(theme_variant: &ThemeVariant) -> Option<PlatformThemeVariant> {
    if *theme_variant == ThemeVariant::light() {
        Some(PlatformThemeVariant::Light)
    } else if *theme_variant == ThemeVariant::dark() {
        Some(PlatformThemeVariant::Dark)
    } else if let Some(inherit_variant) = theme_variant.inherit_variant() {
        to_platform_theme_variant(&inherit_variant)
    } else {
        None
    }
}

fn to_composition_transparency_level(level: WindowTransparencyLevel) -> CompositionTransparencyLevel {
    if level == WindowTransparencyLevel::transparent() {
        return CompositionTransparencyLevel::Transparent;
    }
    if level == WindowTransparencyLevel::blur() {
        return CompositionTransparencyLevel::Blur;
    }
    if level == WindowTransparencyLevel::acrylic_blur() {
        return CompositionTransparencyLevel::AcrylicBlur;
    }
    if level == WindowTransparencyLevel::mica() {
        return CompositionTransparencyLevel::Mica;
    }
    CompositionTransparencyLevel::None
}

/// The top-level as something that tells when it has been closed.
struct TopLevelCloseable(Ref<TopLevel>);

impl ferroui_base::input::ICloseable for TopLevelCloseable {
    fn closed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.0.closed(move || handler())
    }
}

impl TopLevel {
    /// The top-level as something that tells when it has been closed.
    pub fn as_closeable(&self) -> Rc<dyn ferroui_base::input::ICloseable> {
        Rc::new(TopLevelCloseable(self.to_ref()))
    }
}
