use super::i_popup_host::subscribe_template_applied;
use super::popup_positioning::PopupPositionRequest;
use super::{IPopupHost, Popup, TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl};
use crate::platform::{IPopupImpl, IWindowBaseImpl};
use crate::presenters::ContentPresenter;
use crate::{
    ContentControlImpl, Control, ControlImpl, TopLevel, TopLevelImpl, WindowBase, WindowBaseImpl,
};
use ferroui_base::input::{IFocusScope, InputElement, InputElementImpl};
use ferroui_base::interactivity::{Interactive, InteractiveImpl};
use ferroui_base::layout::{LayoutableImpl, LayoutableImplExt};
use ferroui_base::media::{Brushes, IBrush, Transform};
use ferroui_base::reactive::IDisposable;
use ferroui_base::styling::StyleHostRef;
use ferroui_base::visual_tree::IHostedVisualTreeRoot;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, IFerroDependencyResolver, Nullable, Ref, Size, StyledElementImpl, StyledProperty,
    Thickness, Visual, VisualImpl, WeakRef,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The root window of a [`Popup`].
#[repr(C)]
pub struct PopupRoot {
    base: WindowBase,
    popup_impl: RefCell<Option<Rc<dyn IPopupImpl>>>,
    parent_top_level: WeakRef<TopLevel>,
    popup_position_request: RefCell<Option<PopupPositionRequest>>,
    popup_size: Cell<Size>,
    child_margin: Cell<Thickness>,
    needs_update: Cell<bool>,
}

ferro_class!(PopupRoot: WindowBase);
ferro_impl_classes!(PopupRoot: VisualImpl, TemplatedControlImpl, ContentControlImpl);

impl ControlImpl for PopupRoot {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::PopupRootAutomationPeer::new(this).upcast()
    }
}

impl TopLevelImpl for PopupRoot {
    /// The popups that are currently open directly inside the popup of
    /// this root, in the order they were opened.
    fn opened_popups(this: &Self) -> Vec<Ref<Popup>> {
        this.parent().and_then(|parent| parent.cast::<Popup>()).map(|popup| popup.opened_popups()).unwrap_or_default()
    }
}

impl IFocusScope for PopupRoot {}

impl FerroObjectImpl for PopupRoot {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        if let Some(platform_impl) = this.platform_impl() {
            platform_impl.set_window_manager_add_shadow_hint(this.window_manager_add_shadow_hint());
            platform_impl.set_hit_test_visible(this.is_hit_test_visible());
        }
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::window_manager_add_shadow_hint_property().as_property() {
            let (_, new_value) = change.get_old_and_new_value::<bool>();
            if let Some(platform_impl) = this.platform_impl() {
                platform_impl.set_window_manager_add_shadow_hint(new_value);
            }
        } else if change.property() == WindowBase::topmost_property().as_property() {
            let (_, new_value) = change.get_old_and_new_value::<bool>();
            if let Some(platform_impl) = this.platform_impl() {
                platform_impl.set_topmost(new_value);
            }
        } else if change.property() == InputElement::is_hit_test_visible_property().as_property() {
            let (_, new_value) = change.get_old_and_new_value::<bool>();
            if let Some(platform_impl) = this.platform_impl() {
                platform_impl.set_hit_test_visible(new_value);
            }
        }
    }
}

impl StyledElementImpl for PopupRoot {
    /// The styling parent of the popup root: its logical parent.
    fn styling_parent(this: &Self) -> Option<StyleHostRef> {
        this.parent().map(StyleHostRef::Element)
    }
}

impl InteractiveImpl for PopupRoot {
    /// The parent control in the event route.
    ///
    /// Popup events are passed to their parent window. This facilitates
    /// this.
    fn interactive_parent(this: &Self) -> Option<Ref<Interactive>> {
        this.parent().and_then(|parent| parent.cast::<Interactive>())
    }
}

impl InputElementImpl for PopupRoot {
    fn is_focus_scope(_this: &Self) -> bool {
        true
    }

    fn as_hosted_visual_tree_root(this: &Self) -> Option<&dyn IHostedVisualTreeRoot> {
        Some(this)
    }
}

impl IHostedVisualTreeRoot for PopupRoot {
    /// The control that is hosting the popup root.
    fn host(&self) -> Option<Ref<Visual>> {
        // If the parent is attached to a visual tree, then return that. However the parent
        // will possibly be a standalone Popup (i.e. a Popup not attached to a visual tree,
        // created by e.g. a ContextMenu): if this is the case, return the ParentTopLevel
        // if set. This helps to allow the focus manager to restore the focus to the outer
        // scope when the popup is closed.
        let parent_visual = self.parent().and_then(|parent| parent.cast::<Visual>());
        if let Some(parent_visual) = &parent_visual {
            if parent_visual.is_attached_to_visual_tree() {
                return Some(parent_visual.clone());
            }
        }
        self.parent_top_level().map(Ref::upcast).or(parent_visual)
    }
}

impl LayoutableImpl for PopupRoot {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        let max_auto_size =
            this.platform_impl().map(|platform_impl| platform_impl.max_auto_size_hint()).unwrap_or(Size::INFINITY);
        let mut constraint = available_size;

        if constraint.width.is_infinite() {
            constraint = constraint.with_width(max_auto_size.width);
        }

        if constraint.height.is_infinite() {
            constraint = constraint.with_height(max_auto_size.height);
        }

        let measured = Self::parent_measure_override(this, constraint);
        let mut width = measured.width;
        let mut height = measured.height;
        let width_cache = this.width();
        let height_cache = this.height();

        if !width_cache.is_nan() {
            width = width_cache;
        }

        width = width.min(this.max_width());
        width = width.max(this.min_width());

        if !height_cache.is_nan() {
            height = height_cache;
        }

        height = height.min(this.max_height());
        height = height.max(this.min_height());

        Size::new(width, height)
    }
}

impl WindowBaseImpl for PopupRoot {
    fn arrange_set_bounds(this: &Self, size: Size) -> Size {
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

        if this.popup_size.get() != size {
            this.popup_size.set(size);
            reposition = true;
        }

        if reposition {
            this.needs_update.set(true);
            this.update_position();
        }

        this.client_size()
    }
}

ferroui_base::ferro_properties! { impl PopupRoot {
    ferro_property!(
        /// Defines the `Transform` property.
        pub fn transform_property() -> StyledProperty<Option<Ref<Transform>>> {
            FerroProperty::register::<PopupRoot, _>("Transform", None)
        }
    );

    ferro_property!(
        /// Defines the `WindowManagerAddShadowHint` property.
        pub fn window_manager_add_shadow_hint_property() -> StyledProperty<bool> {
            Popup::window_manager_add_shadow_hint_property().add_owner::<PopupRoot>()
        }
    );
} }

impl PopupRoot {
    fn static_constructor() {
        let white: Rc<dyn IBrush> = Brushes::white();
        TemplatedControl::background_property().override_default_value::<PopupRoot>(Some(white));
    }

    /// Creates a popup root: `parent` is the popup parent and
    /// `platform_impl` the popup implementation.
    pub fn new(parent: &Ref<TopLevel>, platform_impl: Rc<dyn IPopupImpl>) -> Ref<Self> {
        Self::new_with_resolver(parent, platform_impl, None)
    }

    /// Creates a popup root: `parent` is the popup parent, `platform_impl`
    /// the popup implementation and `dependency_resolver` the dependency
    /// resolver to use; if `None` the default dependency resolver will be
    /// used.
    pub fn new_with_resolver(
        parent: &Ref<TopLevel>,
        platform_impl: Rc<dyn IPopupImpl>,
        dependency_resolver: Option<Rc<dyn IFerroDependencyResolver>>,
    ) -> Ref<Self> {
        let window_base_impl: Rc<dyn IWindowBaseImpl> = platform_impl.clone();
        instantiate(Self {
            base: WindowBase::construct_with_resolver(window_base_impl, dependency_resolver),
            popup_impl: RefCell::new(Some(platform_impl)),
            parent_top_level: parent.downgrade(),
            popup_position_request: RefCell::new(None),
            popup_size: Cell::new(Size::default()),
            child_margin: Cell::new(Thickness::default()),
            needs_update: Cell::new(false),
        })
    }

    /// The platform-specific window implementation; `None` once the popup
    /// root has closed.
    pub fn platform_impl(&self) -> Option<Rc<dyn IPopupImpl>> {
        TopLevel::platform_impl(self)?;
        self.popup_impl.borrow().clone()
    }

    /// A transform that will be applied to the popup.
    pub fn transform(&self) -> Option<Ref<Transform>> {
        self.get_value(Self::transform_property())
    }

    pub fn set_transform(&self, value: impl Into<Nullable<Transform>>) {
        self.set_value(Self::transform_property(), value.into().0)
    }

    /// A hint to the window manager that a shadow should be added to the
    /// popup.
    pub fn window_manager_add_shadow_hint(&self) -> bool {
        self.get_value(Self::window_manager_add_shadow_hint_property())
    }

    pub fn set_window_manager_add_shadow_hint(&self, value: bool) {
        self.set_value(Self::window_manager_add_shadow_hint_property(), value)
    }

    /// The top-level the popup belongs to; `None` once that top-level is
    /// gone (the popup root does not keep its parent alive).
    pub fn parent_top_level(&self) -> Option<Ref<TopLevel>> {
        self.parent_top_level.upgrade()
    }

    /// Releases the platform implementation and closes the popup root.
    pub fn dispose(&self) {
        if let Some(platform_impl) = self.platform_impl() {
            platform_impl.dispose();
        }
        self.ensure_closed();
    }

    fn update_position(&self) {
        if !self.needs_update.get() {
            return;
        }
        let Some(popup_position_request) = self.popup_position_request.borrow().clone() else { return };

        self.needs_update.set(false);
        let positioner = self.platform_impl().and_then(|platform_impl| platform_impl.popup_positioner());
        if let (Some(positioner), Some(parent_top_level)) = (positioner, self.parent_top_level()) {
            positioner.update_request(
                &parent_top_level,
                &popup_position_request,
                self.popup_size.get(),
                self.child_margin.get(),
                self.flow_direction(),
            );
        }
    }

    /// Configures the position of the popup according to a target control
    /// and a set of placement parameters.
    pub fn configure_position(&self, request: PopupPositionRequest) {
        *self.popup_position_request.borrow_mut() = Some(request);
        self.needs_update.set(true);
        self.update_position();
    }

    /// Sets the control to display in the popup.
    pub fn set_child(&self, control: impl Into<Nullable<Control>>) {
        self.set_content(control.into().0.map(Control::boxed));
    }

    /// Takes focus from any currently focused native control.
    pub fn take_focus(&self) {
        if let Some(platform_impl) = self.platform_impl() {
            platform_impl.take_focus();
        }
    }

    /// The popup root as a popup host (the popup host contract it
    /// implements upstream).
    pub fn to_popup_host(&self) -> Rc<dyn IPopupHost> {
        Rc::new(PopupRootHost(self.to_ref()))
    }
}

/// The popup host contract of a popup root.
struct PopupRootHost(Ref<PopupRoot>);

impl IFocusScope for PopupRootHost {}

impl IDisposable for PopupRootHost {
    fn dispose(&self) {
        self.0.dispose();
    }
}

impl IPopupHost for PopupRootHost {
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
        self.0.topmost()
    }

    fn set_topmost(&self, value: bool) {
        self.0.set_topmost(value)
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
        Some(self.0.clone().upcast())
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

    fn as_popup_root(&self) -> Option<Ref<PopupRoot>> {
        Some(self.0.clone())
    }
}
