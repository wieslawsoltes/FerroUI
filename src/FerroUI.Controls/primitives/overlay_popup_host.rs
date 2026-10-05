use super::i_popup_host::subscribe_template_applied;
use super::popup_positioning::{
    IManagedPopupPositionerPopup, IPopupPositioner, ManagedPopupPositioner, ManagedPopupPositionerScreenInfo,
    PopupPositionRequest,
};
use super::{IPopupHost, PopupOverlayLayer, PopupRoot, TemplateAppliedEventArgs, TemplatedControlImpl};
use crate::presenters::ContentPresenter;
use crate::{Canvas, ContentControl, ContentControlImpl, Control, ControlImpl, TopLevel};
use ferroui_base::input::{
    IFocusScope, IKeyboardNavigationHandler, InputElementImpl, KeyboardNavigation, KeyboardNavigationMode,
};
use ferroui_base::interactivity::{Interactive, InteractiveImpl};
use ferroui_base::layout::{LayoutableImpl, LayoutableImplExt};
use ferroui_base::media::{MediaContext, Transform};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroLocator, FerroObjectImpl, FerroObjectImplExt,
    IFerroDependencyResolver, LocatorExtensions, Nullable, Point, Rect, Ref, Size, StyledElementImpl, StyledProperty,
    Thickness, Visual, VisualImpl, WeakRef,
};
use std::cell::{Cell, OnceCell, RefCell};
use std::rc::Rc;

/// The popup host that shows a popup inside its top-level, on the popup
/// overlay layer.
#[repr(C)]
pub struct OverlayPopupHost {
    base: ContentControl,
    overlay_layer: WeakRef<PopupOverlayLayer>,
    positioner: OnceCell<Rc<dyn IPopupPositioner>>,
    keyboard_navigation_handler: RefCell<Option<Rc<dyn IKeyboardNavigationHandler>>>,
    last_requested_position: Cell<Point>,
    popup_position_request: RefCell<Option<PopupPositionRequest>>,
    popup_size: Cell<Size>,
    child_margin: Cell<Thickness>,
    needs_update: Cell<bool>,
}

ferro_class!(OverlayPopupHost: ContentControl);
ferro_impl_classes!(
    OverlayPopupHost: StyledElementImpl,
    VisualImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);

impl IFocusScope for OverlayPopupHost {}

impl FerroObjectImpl for OverlayPopupHost {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        let popup: Rc<dyn IManagedPopupPositionerPopup> =
            Rc::new(OverlayPopupHostPositionerPopup(this.to_ref().downgrade()));
        let positioner: Rc<dyn IPopupPositioner> = Rc::new(ManagedPopupPositioner::new(popup));
        if this.positioner.set(positioner).is_err() {
            unreachable!("the host is constructed once");
        }

        let keyboard_navigation_handler = FerroLocator::current().get_service::<dyn IKeyboardNavigationHandler>();
        if let Some(handler) = &keyboard_navigation_handler {
            handler.set_owner(&this.to_ref().upcast());
        }
        *this.keyboard_navigation_handler.borrow_mut() = keyboard_navigation_handler;
    }
}

impl InteractiveImpl for OverlayPopupHost {
    fn interactive_parent(this: &Self) -> Option<Ref<Interactive>> {
        this.parent().and_then(|parent| parent.cast::<Interactive>())
    }
}

impl InputElementImpl for OverlayPopupHost {
    fn is_focus_scope(_this: &Self) -> bool {
        true
    }
}

impl LayoutableImpl for OverlayPopupHost {
    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let mut reposition = false;

        let new_margin = this
            .presenter()
            .and_then(|presenter| presenter.child())
            .map(|child| child.margin())
            .unwrap_or_default();
        if new_margin != this.child_margin.get() {
            this.child_margin.set(new_margin);
            reposition = true;
        }

        if this.popup_size.get() != final_size {
            this.popup_size.set(final_size);
            reposition = true;
        }

        if reposition {
            this.needs_update.set(true);
            this.update_position();
        }
        Self::parent_arrange_override(this, final_size)
    }
}

ferroui_base::ferro_properties! { impl OverlayPopupHost {
    ferro_property!(
        /// Defines the `Transform` property.
        pub fn transform_property() -> StyledProperty<Option<Ref<Transform>>> {
            PopupRoot::transform_property().add_owner::<OverlayPopupHost>()
        }
    );
} }

impl OverlayPopupHost {
    fn static_constructor() {
        KeyboardNavigation::tab_navigation_property()
            .override_default_value::<OverlayPopupHost>(KeyboardNavigationMode::Cycle);
    }

    /// Creates a host that is shown on `overlay_layer` (internal upstream).
    pub fn new(overlay_layer: &Ref<PopupOverlayLayer>) -> Ref<Self> {
        instantiate(Self {
            base: ContentControl::construct(),
            overlay_layer: overlay_layer.downgrade(),
            positioner: OnceCell::new(),
            keyboard_navigation_handler: RefCell::new(None),
            last_requested_position: Cell::new(Point::default()),
            popup_position_request: RefCell::new(None),
            popup_size: Cell::new(Size::default()),
            child_margin: Cell::new(Thickness::default()),
            needs_update: Cell::new(false),
        })
    }

    /// The keyboard navigation handler of the host (for unit tests).
    pub fn tests_keyboard_navigation_handler(&self) -> Option<Rc<dyn IKeyboardNavigationHandler>> {
        self.keyboard_navigation_handler.borrow().clone()
    }

    /// Sets the control to display in the popup.
    pub fn set_child(&self, control: impl Into<Nullable<Control>>) {
        self.set_content(control.into().0.map(Control::boxed));
    }

    /// The root of the separate visual tree of the popup: none, an overlay
    /// popup lives in the visual tree of its top-level.
    pub fn hosted_visual_tree_root(&self) -> Option<Ref<Visual>> {
        None
    }

    /// A transform that will be applied to the popup.
    pub fn transform(&self) -> Option<Ref<Transform>> {
        self.get_value(Self::transform_property())
    }

    pub fn set_transform(&self, value: impl Into<Nullable<Transform>>) {
        self.set_value(Self::transform_property(), value.into().0)
    }

    /// Hides the popup.
    pub fn dispose(&self) {
        self.hide();
    }

    /// Shows the popup: adds the host to its overlay layer.
    pub fn show(&self) {
        if let Some(overlay_layer) = self.overlay_layer.upgrade() {
            overlay_layer.children().add(self.to_ref());
        }

        let content = self.content().and_then(|content| Control::from_boxed(&content));
        if content.is_some_and(|content| !content.is_attached_to_visual_tree()) {
            // We need to force a measure pass so any descendants are built, for focus to work.
            self.update_layout();
        }
    }

    /// Hides the popup: removes the host from its overlay layer.
    pub fn hide(&self) {
        if let Some(overlay_layer) = self.overlay_layer.upgrade() {
            overlay_layer.children().remove(self.to_ref());
        }
    }

    /// Takes focus from any currently focused native control.
    pub fn take_focus(&self) {
        // Nothing to do here: overlay popups are implemented inside the window.
    }

    /// Configures the position of the popup according to a target control
    /// and a set of placement parameters.
    pub fn configure_position(&self, position_request: PopupPositionRequest) {
        *self.popup_position_request.borrow_mut() = Some(position_request);
        self.needs_update.set(true);
        self.update_position();
    }

    fn update_position(&self) {
        if !self.needs_update.get() {
            return;
        }
        let Some(popup_position_request) = self.popup_position_request.borrow().clone() else { return };

        self.needs_update.set(false);
        let overlay_layer = self.overlay_layer.upgrade();
        let top_level = overlay_layer
            .and_then(|overlay_layer| TopLevel::get_top_level(Some(&overlay_layer)))
            .expect("the popup overlay layer is in a top-level");
        let positioner = self.positioner.get().expect("the positioner is created with the host");
        positioner.update_request(
            &top_level,
            &popup_position_request,
            self.popup_size.get(),
            self.child_margin.get(),
            self.flow_direction(),
        );
    }

    /// The host as a popup host (the popup host contract it implements
    /// upstream).
    pub fn to_popup_host(&self) -> Rc<dyn IPopupHost> {
        Rc::new(OverlayPopupHostHost(self.to_ref()))
    }

    /// Creates the popup host for a popup placed relative to `target`: a
    /// native popup root when the platform of the top-level of `target`
    /// creates popups and `should_use_overlay_layer` is false, otherwise an
    /// overlay popup host on the popup overlay layer of `target` (internal
    /// upstream).
    ///
    /// # Panics
    ///
    /// Panics when neither can be created.
    pub fn create_popup_host(
        target: &Visual,
        dependency_resolver: Option<Rc<dyn IFerroDependencyResolver>>,
        should_use_overlay_layer: bool,
    ) -> Rc<dyn IPopupHost> {
        if !should_use_overlay_layer {
            if let Some(top_level) = TopLevel::get_top_level(Some(target)) {
                let popup_impl = top_level.platform_impl().and_then(|platform_impl| platform_impl.create_popup());
                if let Some(popup_impl) = popup_impl {
                    return PopupRoot::new_with_resolver(&top_level, popup_impl, dependency_resolver).to_popup_host();
                }
            }
        }

        if let Some(overlay_layer) = PopupOverlayLayer::get_popup_overlay_layer(target) {
            return OverlayPopupHost::new(&overlay_layer).to_popup_host();
        }

        panic!("Unable to create IPopupImpl and no overlay layer is found for the target control");
    }
}

/// The popup the managed positioner of an overlay popup host positions:
/// the host itself, within its overlay layer.
struct OverlayPopupHostPositionerPopup(WeakRef<OverlayPopupHost>);

impl IManagedPopupPositionerPopup for OverlayPopupHostPositionerPopup {
    fn screens(&self) -> Vec<ManagedPopupPositionerScreenInfo> {
        let Some(host) = self.0.upgrade() else { return Vec::new() };
        let available_size =
            host.overlay_layer.upgrade().map(|overlay_layer| overlay_layer.available_size()).unwrap_or_default();
        let mut rc = Rect::from_size(available_size);
        if let Some(top_level) = TopLevel::get_top_level(Some(&host)) {
            let padding =
                top_level.insets_manager().map(|insets_manager| insets_manager.safe_area_padding()).unwrap_or_default();
            rc = rc.deflate_thickness(padding);
        }

        vec![ManagedPopupPositionerScreenInfo::new(rc, rc)]
    }

    fn parent_client_area_screen_geometry(&self) -> Rect {
        let size = self
            .0
            .upgrade()
            .and_then(|host| host.overlay_layer.upgrade())
            .map(|overlay_layer| overlay_layer.bounds().size())
            .unwrap_or_default();
        Rect::from_size(size)
    }

    fn move_and_resize(&self, device_point: Point, _virtual_size: Size) {
        let Some(host) = self.0.upgrade() else { return };
        host.last_requested_position.set(device_point);
        let weak = self.0.clone();
        MediaContext::instance().begin_invoke_on_render(Rc::new(move || {
            if let Some(host) = weak.upgrade() {
                let position = host.last_requested_position.get();
                Canvas::set_left(&host, position.x);
                Canvas::set_top(&host, position.y);
            }
        }));
    }

    fn scaling(&self) -> f64 {
        1.0
    }
}

/// The popup host contract of an overlay popup host.
struct OverlayPopupHostHost(Ref<OverlayPopupHost>);

impl IFocusScope for OverlayPopupHostHost {}

impl IDisposable for OverlayPopupHostHost {
    fn dispose(&self) {
        self.0.dispose();
    }
}

impl IPopupHost for OverlayPopupHostHost {
    fn width(&self) -> f64 {
        self.0.width()
    }

    fn set_width(&self, value: f64) {
        self.0.set_width(value)
    }

    fn min_width(&self) -> f64 {
        self.0.min_width()
    }

    fn set_min_width(&self, value: f64) {
        self.0.set_min_width(value)
    }

    fn max_width(&self) -> f64 {
        self.0.max_width()
    }

    fn set_max_width(&self, value: f64) {
        self.0.set_max_width(value)
    }

    fn height(&self) -> f64 {
        self.0.height()
    }

    fn set_height(&self, value: f64) {
        self.0.set_height(value)
    }

    fn min_height(&self) -> f64 {
        self.0.min_height()
    }

    fn set_min_height(&self, value: f64) {
        self.0.set_min_height(value)
    }

    fn max_height(&self) -> f64 {
        self.0.max_height()
    }

    fn set_max_height(&self, value: f64) {
        self.0.set_max_height(value)
    }

    fn presenter(&self) -> Option<Ref<ContentPresenter>> {
        self.0.presenter()
    }

    fn topmost(&self) -> bool {
        false
    }

    fn set_topmost(&self, _value: bool) {
        // Not currently supported in overlay popups
    }

    fn is_hit_test_visible(&self) -> bool {
        self.0.is_hit_test_visible()
    }

    fn set_is_hit_test_visible(&self, value: bool) {
        self.0.set_is_hit_test_visible(value)
    }

    fn transform(&self) -> Option<Ref<Transform>> {
        self.0.transform()
    }

    fn set_transform(&self, value: Option<Ref<Transform>>) {
        self.0.set_transform(Nullable(value))
    }

    fn hosted_visual_tree_root(&self) -> Option<Ref<Visual>> {
        self.0.hosted_visual_tree_root()
    }

    fn template_applied(&self, handler: Rc<dyn Fn(&TemplateAppliedEventArgs)>) -> Rc<dyn IDisposable> {
        subscribe_template_applied(&self.0.clone().upcast(), handler)
    }

    fn configure_position(&self, position_request: PopupPositionRequest) {
        self.0.configure_position(position_request)
    }

    fn set_child(&self, control: Option<Ref<Control>>) {
        self.0.set_child(Nullable(control))
    }

    fn show(&self) {
        self.0.show()
    }

    fn hide(&self) {
        self.0.hide()
    }

    fn take_focus(&self) {
        self.0.take_focus()
    }

    fn as_control(&self) -> Ref<Control> {
        self.0.clone().upcast()
    }

    fn as_overlay_popup_host(&self) -> Option<Ref<OverlayPopupHost>> {
        Some(self.0.clone())
    }
}
