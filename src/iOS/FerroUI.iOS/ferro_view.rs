//! The view: a `UIView` with a Metal layer that hosts a top-level of the
//! framework and can be embedded into the view tree of an application.
//!
//! Later stages of `docs/porting/ios-platform.md` add what the reference's
//! view has beyond this file: key presses and the scroll wheel (stage 2,
//! with the input handler), the text input method (stage 2:
//! `ITextInputMethodImpl`, the first responder rules), the platform
//! settings that follow the traits of the view (stage 2), the features of
//! a top-level that are services of their own (stage 2: the storage
//! provider, the clipboard, the input pane, the launcher, the native
//! control host, the feedback), and accessibility (stage 3).

use crate::input_handler::InputHandler;
use crate::insets_manager::InsetsManager;
use crate::metal::{FrameCapture, MetalPlatformSurface, SurfaceShared};
use crate::native_control_host_impl::UIViewControlHandle;
use crate::platform::Platform;
use crate::view_controller::{IFerroViewController, StatusBarStyle};
use ferroui_base::data::BindingPriority;
use ferroui_base::input::raw::IRawInputEventArgs;
use ferroui_base::input::IInputRoot;
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::platform::{ICursorImpl, IOptionalFeatureProvider};
use ferroui_base::reactive::IDisposable;
use ferroui_base::rendering::composition::Compositor;
use ferroui_base::{FerroLocator, LocatorExtensions, PixelPoint, PixelSize, Point, Rect, Ref, Size};
use ferroui_controls::embedding::EmbeddableControlRoot;
use ferroui_controls::platform::{
    IInsetsManager, IPlatformHandle, IPopupImpl, IScreenImpl, ITopLevelImpl, PlatformThemeVariant,
};
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::{
    AcrylicPlatformCompensationLevels, Control, TopLevel, WindowResizeReason, WindowTransparencyLevel,
};
use objc2::rc::{Retained, Weak};
use objc2::runtime::AnyClass;
use objc2::{define_class, msg_send, ClassType, DefinedClass, MainThreadMarker, MainThreadOnly};
use objc2_foundation::{NSObjectProtocol, NSSet};
use objc2_quartz_core::CAMetalLayer;
use objc2_ui_kit::{UIEvent, UIScreen, UITouch, UIView};
use std::any::{Any, TypeId};
use std::cell::{Cell, OnceCell, RefCell};
use std::rc::Rc;
use std::sync::{Arc, Mutex, PoisonError};

type Callback<T> = RefCell<Option<Rc<T>>>;

/// What a view holds once it is created.
struct ViewState {
    top_level_impl: Rc<TopLevelImpl>,
    top_level: Ref<EmbeddableControlRoot>,
    input: InputHandler,
}

/// The instance variables of the view.
#[derive(Default)]
pub struct FerroViewIvars {
    state: OnceCell<ViewState>,
    controller: RefCell<Option<Rc<dyn IFerroViewController>>>,
    input_root: RefCell<Option<Rc<dyn IInputRoot>>>,
    latest_scaling: Cell<f64>,
    disposed_value: Cell<bool>,
}

define_class!(
    // SAFETY: `UIView` may be subclassed; the overrides call the superclass
    // where UIKit asks for it (`layoutSubviews`), and the class does not
    // implement `Drop`.
    #[unsafe(super(UIView))]
    #[thread_kind = MainThreadOnly]
    #[name = "FerroView"]
    #[ivars = FerroViewIvars]
    pub struct FerroView;

    impl FerroView {
        #[unsafe(method(layerClass))]
        fn layer_class() -> &'static AnyClass {
            // The reference answers with the layer of OpenGL ES when the
            // graphics of the platform are EAGL (stage 4).
            CAMetalLayer::class()
        }

        #[unsafe(method(canBecomeFirstResponder))]
        fn can_become_first_responder(&self) -> bool {
            true
        }

        #[unsafe(method(canResignFirstResponder))]
        fn can_resign_first_responder(&self) -> bool {
            true
        }

        #[unsafe(method(touchesBegan:withEvent:))]
        fn touches_began(&self, touches: &NSSet<UITouch>, evt: Option<&UIEvent>) {
            self.handle_touches(touches, evt);
        }

        #[unsafe(method(touchesMoved:withEvent:))]
        fn touches_moved(&self, touches: &NSSet<UITouch>, evt: Option<&UIEvent>) {
            self.handle_touches(touches, evt);
        }

        #[unsafe(method(touchesEnded:withEvent:))]
        fn touches_ended(&self, touches: &NSSet<UITouch>, evt: Option<&UIEvent>) {
            self.handle_touches(touches, evt);
        }

        #[unsafe(method(touchesCancelled:withEvent:))]
        fn touches_cancelled(&self, touches: &NSSet<UITouch>, evt: Option<&UIEvent>) {
            self.handle_touches(touches, evt);
        }

        #[unsafe(method(layoutSubviews))]
        fn layout_subviews(&self) {
            if let Some(state) = self.ivars().state.get() {
                let top_level_impl = &state.top_level_impl;
                let resized = top_level_impl.resized.borrow().clone();
                if let Some(resized) = resized {
                    resized(top_level_impl.client_size(), WindowResizeReason::Layout);
                }
                let scaling = self.contentScaleFactor();
                if self.ivars().latest_scaling.get() != scaling {
                    let scaling_changed = top_level_impl.scaling_changed.borrow().clone();
                    if let Some(scaling_changed) = scaling_changed {
                        scaling_changed(scaling);
                    }
                }

                self.ivars().latest_scaling.set(scaling);
                let bounds = self.bounds();
                top_level_impl.shared.set_pending_layout((
                    layout_pixel_size(bounds.size.width, bounds.size.height, scaling),
                    scaling,
                ));
            }

            // SAFETY: the method of the superclass this one overrides.
            let _: () = unsafe { msg_send![super(self), layoutSubviews] };
        }
    }

    unsafe impl NSObjectProtocol for FerroView {}
);

/// The size in pixels of a view of a size in points at a scaling.
pub(crate) fn layout_pixel_size(width: f64, height: f64, scaling: f64) -> PixelSize {
    PixelSize::new((width * scaling) as i32, (height * scaling) as i32)
}

impl FerroView {
    /// Creates a view with a top-level of its own, ready for content.
    ///
    /// # Panics
    /// Panics when the platform was not initialized (`use_ios`).
    pub fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(FerroViewIvars::default());
        // SAFETY: `init` of the superclass, on the object that was just
        // allocated and whose instance variables are set.
        let this: Retained<Self> = unsafe { msg_send![super(this), init] };

        let top_level_impl = TopLevelImpl::new(&this);
        let input = InputHandler::new(Weak::from_retained(&this), top_level_impl.clone());
        let platform_impl: Rc<dyn ITopLevelImpl> = top_level_impl.clone();
        let top_level = EmbeddableControlRoot::with_impl(platform_impl);
        top_level_impl.set_top_level(&top_level);
        let _ = this.ivars().state.set(ViewState { top_level_impl, top_level: top_level.clone(), input });

        top_level.prepare();

        top_level.start_rendering();

        this.init_layer_surface(mtm);

        // The reference adds the swipe gestures of a remote on tvOS and,
        // on iOS, a pan gesture that only takes scroll events; both are
        // stage 2, with the rest of the input handler.
        this.setMultipleTouchEnabled(true);

        this
    }

    fn state(&self) -> &ViewState {
        match self.ivars().state.get() {
            Some(state) => state,
            None => panic!("The view was not created by FerroView::new."),
        }
    }

    fn init_layer_surface(&self, mtm: MainThreadMarker) {
        let l = self.layer();
        // The main screen, as the reference: the view is in no window yet.
        #[allow(deprecated)]
        let scale = UIScreen::mainScreen(mtm).scale();
        l.setContentsScale(scale);
        l.setOpaque(true);
        if let Ok(metal_layer) = l.downcast::<CAMetalLayer>() {
            metal_layer.setOpaque(false);
            let top_level_impl = &self.state().top_level_impl;
            let surface: Arc<dyn IPlatformRenderSurface> =
                MetalPlatformSurface::new(metal_layer, top_level_impl.shared.clone());
            top_level_impl.set_surfaces(vec![surface]);
        }
    }

    fn handle_touches(&self, touches: &NSSet<UITouch>, evt: Option<&UIEvent>) {
        if let Some(state) = self.ivars().state.get() {
            state.input.handle(touches, evt);
        }
    }

    /// The input root of the top-level.
    ///
    /// # Panics
    /// Panics before the top-level set its input root.
    pub(crate) fn input_root(&self) -> Rc<dyn IInputRoot> {
        match self.ivars().input_root.borrow().clone() {
            Some(input_root) => input_root,
            None => panic!("set_input_root must have been called"),
        }
    }

    /// The top-level of the view.
    pub fn top_level(&self) -> Ref<TopLevel> {
        self.state().top_level.clone().upcast()
    }

    /// Gives the view the view controller that shows it.
    pub fn init_with_controller(&self, controller: Rc<dyn IFerroViewController>) {
        *self.ivars().controller.borrow_mut() = Some(controller.clone());
        self.state().top_level_impl.insets_manager.init_with_controller(controller);
    }

    /// The content of the view.
    ///
    /// # Panics
    /// Panics when the content of the top-level is not a control.
    pub fn content(&self) -> Option<Ref<Control>> {
        let content = self.state().top_level.content()?;
        match Control::from_boxed(&content) {
            Some(control) => Some(control),
            None => panic!("Unable to cast the content of the view to a control."),
        }
    }

    /// Sets the content of the view.
    pub fn set_content(&self, value: Option<Ref<Control>>) {
        self.state().top_level.set_content(value.map(Control::boxed));
    }

    /// Releases the top-level of the view.
    pub fn dispose(&self) {
        if !self.ivars().disposed_value.replace(true) {
            self.state().top_level.dispose();
        }
    }

    /// The number of frames that were presented on the layer of the view.
    /// An addition of the port, for an application that tests itself.
    pub fn frames_presented(&self) -> u64 {
        self.state().top_level_impl.shared.frames_presented()
    }

    /// The number of frames that were begun on the layer of the view (a
    /// frame that was begun and not presented was not finished by the
    /// renderer). An addition of the port, for diagnostics.
    pub fn frames_begun(&self) -> u64 {
        self.state().top_level_impl.shared.frames_begun()
    }

    /// The layout the next frame is drawn for: the size in pixels and the
    /// scaling. An addition of the port, for diagnostics.
    pub fn pending_layout(&self) -> (PixelSize, f64) {
        self.state().top_level_impl.shared.pending_layout()
    }

    /// Asks for the pixels of the next frame that is drawn; `callback` is
    /// called with them on the thread that renders, before the frame is
    /// presented. An addition of the port, for an application that tests
    /// itself.
    pub fn capture_next_frame(&self, callback: impl FnOnce(FrameCapture) + Send + 'static) {
        self.state().top_level_impl.shared.request_capture(Box::new(callback));
    }
}

/// The top-level implementation of a view.
pub struct TopLevelImpl {
    view: Weak<FerroView>,
    pub(crate) insets_manager: Rc<InsetsManager>,
    pub(crate) shared: Arc<SurfaceShared>,
    top_level: RefCell<Option<ferroui_base::WeakRef<EmbeddableControlRoot>>>,
    padding_insets: Rc<RefCell<Option<Rc<dyn IDisposable>>>>,
    surfaces: Arc<Mutex<Vec<Arc<dyn IPlatformRenderSurface>>>>,
    acrylic_compensation_levels: AcrylicPlatformCompensationLevels,
    // The subscription of the reference lives as long as the manager.
    _edge_to_edge_subscription: RefCell<Option<Rc<dyn IDisposable>>>,

    input: Callback<dyn Fn(Rc<dyn IRawInputEventArgs>)>,
    paint: Callback<dyn Fn(Rect)>,
    pub(crate) resized: Callback<dyn Fn(Size, WindowResizeReason)>,
    pub(crate) scaling_changed: Callback<dyn Fn(f64)>,
    transparency_level_changed: Callback<dyn Fn(WindowTransparencyLevel)>,
    closed: Callback<dyn Fn()>,
    lost_focus: Callback<dyn Fn()>,
}

impl TopLevelImpl {
    fn new(view: &Retained<FerroView>) -> Rc<Self> {
        let insets_manager = InsetsManager::new();
        let this = Rc::new(Self {
            view: Weak::from_retained(view),
            insets_manager: insets_manager.clone(),
            shared: SurfaceShared::new(),
            top_level: RefCell::new(None),
            padding_insets: Rc::new(RefCell::new(None)),
            surfaces: Arc::new(Mutex::new(Vec::new())),
            acrylic_compensation_levels: AcrylicPlatformCompensationLevels::default(),
            _edge_to_edge_subscription: RefCell::new(None),
            input: RefCell::new(None),
            paint: RefCell::new(None),
            resized: RefCell::new(None),
            scaling_changed: RefCell::new(None),
            transparency_level_changed: RefCell::new(None),
            closed: RefCell::new(None),
            lost_focus: RefCell::new(None),
        });

        let weak = Rc::downgrade(&this);
        let subscription = insets_manager.display_edge_to_edge_changed(Rc::new(move |edge_to_edge| {
            let Some(this) = weak.upgrade() else {
                return;
            };
            // iOS doesn't add any paddings/margins to the application by
            // itself. The application is fully responsible for safe area
            // paddings. So, unlike on Android, the safe area insets are
            // "faked" when edge to edge is disabled.
            let old = this.padding_insets.borrow_mut().take();
            if let Some(old) = old {
                old.dispose();
            }
            let controller = this.view().and_then(|view| view.ivars().controller.borrow().clone());
            let top_level = this.top_level.borrow().as_ref().and_then(|top_level| top_level.upgrade());
            if let (false, Some(controller), Some(top_level)) = (edge_to_edge, controller, top_level) {
                // A lower priority, so that it can be redefined by the
                // user.
                let padding_insets = top_level.set_value_with_priority(
                    TemplatedControl::padding_property(),
                    controller.safe_area_padding(),
                    BindingPriority::Style,
                );
                *this.padding_insets.borrow_mut() = padding_insets;
            }
        }));
        *this._edge_to_edge_subscription.borrow_mut() = Some(subscription);

        this
    }

    fn set_top_level(&self, top_level: &Ref<EmbeddableControlRoot>) {
        *self.top_level.borrow_mut() = Some(top_level.downgrade());
    }

    fn set_surfaces(&self, surfaces: Vec<Arc<dyn IPlatformRenderSurface>>) {
        *self.surfaces.lock().unwrap_or_else(PoisonError::into_inner) = surfaces;
    }

    /// The view of the top-level, while it lives.
    pub fn view(&self) -> Option<Retained<FerroView>> {
        self.view.load()
    }
}

impl IDisposable for TopLevelImpl {
    fn dispose(&self) {
        // No-op
    }
}

impl IOptionalFeatureProvider for TopLevelImpl {
    fn try_get_feature(&self, feature_type: TypeId) -> Option<Rc<dyn Any>> {
        if feature_type == TypeId::of::<dyn IInsetsManager>() {
            let insets_manager: Rc<dyn IInsetsManager> = self.insets_manager.clone();
            return Some(Rc::new(insets_manager));
        }

        if feature_type == TypeId::of::<dyn IScreenImpl>() {
            let service = FerroLocator::current().get_required_service::<dyn IScreenImpl>();
            return Some(Rc::new(service));
        }

        None
    }
}

impl ITopLevelImpl for TopLevelImpl {
    fn desktop_scaling(&self) -> f64 {
        self.render_scaling()
    }

    fn handle(&self) -> Option<Rc<dyn IPlatformHandle>> {
        // The reference creates the handle with the top-level. A handle
        // retains its view, and the view holds its top-level, so the port
        // makes the handle when it is asked for.
        let view: Retained<UIView> = Retained::into_super(self.view()?);
        Some(Rc::new(UIViewControlHandle::new(view)))
    }

    fn client_size(&self) -> Size {
        match self.view() {
            Some(view) => {
                let bounds = view.bounds();
                Size::new(bounds.size.width, bounds.size.height)
            }
            None => Size::default(),
        }
    }

    fn render_scaling(&self) -> f64 {
        self.view().map_or(1.0, |view| view.contentScaleFactor())
    }

    fn surfaces(&self) -> Vec<Arc<dyn IPlatformRenderSurface>> {
        self.surfaces.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    fn render_surfaces(&self) -> Arc<dyn Fn() -> Vec<Arc<dyn IPlatformRenderSurface>> + Send + Sync> {
        let surfaces = self.surfaces.clone();
        Arc::new(move || surfaces.lock().unwrap_or_else(PoisonError::into_inner).clone())
    }

    fn compositor(&self) -> Option<Rc<Compositor>> {
        match Platform::compositor() {
            Some(compositor) => Some(compositor),
            None => panic!("iOS backend wasn't initialized. Make sure use_ios was called."),
        }
    }

    fn input(&self) -> Option<Rc<dyn Fn(Rc<dyn IRawInputEventArgs>)>> {
        self.input.borrow().clone()
    }

    fn set_input(&self, value: Option<Rc<dyn Fn(Rc<dyn IRawInputEventArgs>)>>) {
        *self.input.borrow_mut() = value;
    }

    fn paint(&self) -> Option<Rc<dyn Fn(Rect)>> {
        self.paint.borrow().clone()
    }

    fn set_paint(&self, value: Option<Rc<dyn Fn(Rect)>>) {
        *self.paint.borrow_mut() = value;
    }

    fn resized(&self) -> Option<Rc<dyn Fn(Size, WindowResizeReason)>> {
        self.resized.borrow().clone()
    }

    fn set_resized(&self, value: Option<Rc<dyn Fn(Size, WindowResizeReason)>>) {
        *self.resized.borrow_mut() = value;
    }

    fn scaling_changed(&self) -> Option<Rc<dyn Fn(f64)>> {
        self.scaling_changed.borrow().clone()
    }

    fn set_scaling_changed(&self, value: Option<Rc<dyn Fn(f64)>>) {
        *self.scaling_changed.borrow_mut() = value;
    }

    fn transparency_level_changed(&self) -> Option<Rc<dyn Fn(WindowTransparencyLevel)>> {
        self.transparency_level_changed.borrow().clone()
    }

    fn set_transparency_level_changed(&self, value: Option<Rc<dyn Fn(WindowTransparencyLevel)>>) {
        *self.transparency_level_changed.borrow_mut() = value;
    }

    fn set_input_root(&self, input_root: Rc<dyn IInputRoot>) {
        if let Some(view) = self.view() {
            *view.ivars().input_root.borrow_mut() = Some(input_root);
        }
    }

    fn point_to_client(&self, point: PixelPoint) -> Point {
        Point::new(f64::from(point.x), f64::from(point.y))
    }

    fn point_to_screen(&self, point: Point) -> PixelPoint {
        PixelPoint::new(point.x as i32, point.y as i32)
    }

    fn set_cursor(&self, _cursor: Option<Rc<dyn ICursorImpl>>) {
        // no-op
    }

    fn closed(&self) -> Option<Rc<dyn Fn()>> {
        self.closed.borrow().clone()
    }

    fn set_closed(&self, value: Option<Rc<dyn Fn()>>) {
        *self.closed.borrow_mut() = value;
    }

    fn lost_focus(&self) -> Option<Rc<dyn Fn()>> {
        self.lost_focus.borrow().clone()
    }

    fn set_lost_focus(&self, value: Option<Rc<dyn Fn()>>) {
        *self.lost_focus.borrow_mut() = value;
    }

    fn create_popup(&self) -> Option<Rc<dyn IPopupImpl>> {
        // In-window popups
        None
    }

    fn set_transparency_level_hint(&self, _transparency_levels: &[WindowTransparencyLevel]) {
        // No-op
    }

    fn transparency_level(&self) -> WindowTransparencyLevel {
        WindowTransparencyLevel::none()
    }

    fn acrylic_compensation_levels(&self) -> AcrylicPlatformCompensationLevels {
        self.acrylic_compensation_levels
    }

    fn set_frame_theme_variant(&self, theme_variant: Option<PlatformThemeVariant>) {
        // TODO adjust status bar depending on full screen mode.
        let controller = self.view().and_then(|view| view.ivars().controller.borrow().clone());
        if let Some(controller) = controller {
            controller.set_preferred_status_bar_style(match theme_variant {
                Some(PlatformThemeVariant::Light) => StatusBarStyle::DarkContent,
                Some(PlatformThemeVariant::Dark) => StatusBarStyle::LightContent,
                None => StatusBarStyle::Default,
            });
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
