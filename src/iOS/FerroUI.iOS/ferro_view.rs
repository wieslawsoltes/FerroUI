//! The view: a `UIView` with a Metal layer that hosts a top-level of the
//! framework and can be embedded into the view tree of an application.
//!
//! Stage 3 of `docs/porting/ios-platform.md` adds what the reference's view
//! has beyond this file: accessibility. The swipe gestures of a
//! remote, which the reference adds on tvOS, are not ported.

use crate::input_handler::InputHandler;
use crate::insets_manager::InsetsManager;
use crate::ios_launcher::IosLauncher;
use crate::ios_platform_feedback::IosPlatformFeedback;
use crate::metal::{FrameCapture, MetalPlatformSurface, SurfaceShared};
use crate::native_control_host_impl::{NativeControlHostImpl, UIViewControlHandle};
use crate::clipboard::clipboard_impl::ClipboardImpl;
use crate::platform::Platform;
use crate::storage::ios_storage_provider::IosStorageProvider;
use crate::text_input_responder::{current_ferro_responder, set_current_ferro_responder, TextInputResponder};
use crate::ui_kit_input_pane::UIKitInputPane;
use crate::view_controller::{IFerroViewController, StatusBarStyle};
use ferroui_base::data::BindingPriority;
use ferroui_base::input::platform::{Clipboard, IClipboard};
use ferroui_base::input::raw::IRawInputEventArgs;
use ferroui_base::input::text_input::{ITextInputMethodImpl, TextInputMethodClient, TextInputOptions};
use ferroui_base::input::IInputRoot;
use ferroui_base::platform::storage::{ILauncher, IStorageProvider};
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::platform::{ICursorImpl, IOptionalFeatureProvider};
use ferroui_base::reactive::IDisposable;
use ferroui_base::rendering::composition::Compositor;
use ferroui_base::{FerroLocator, LocatorExtensions, PixelPoint, PixelSize, Point, Rect, Ref, Size};
use ferroui_controls::embedding::EmbeddableControlRoot;
use ferroui_controls::platform::{
    IInputPane, IInsetsManager, INativeControlHostImpl, IPlatformFeedback, IPlatformHandle, IPopupImpl, IScreenImpl, ITopLevelImpl, PlatformThemeVariant,
};
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::{
    AcrylicPlatformCompensationLevels, Control, TopLevel, WindowResizeReason, WindowTransparencyLevel,
};
use objc2::rc::{Retained, Weak};
use objc2::runtime::AnyClass;
use objc2::{define_class, msg_send, sel, ClassType, DefinedClass, MainThreadMarker, MainThreadOnly, Message};
use objc2_foundation::{NSObjectProtocol, NSSet};
use objc2_quartz_core::{CADisplayLink, CAMetalLayer};
use objc2_ui_kit::{
    UIEvent, UIPanGestureRecognizer, UIPasteboard, UIPress, UIPressesEvent, UIResponder, UIScreen, UIScrollTypeMask, UITouch,
    UITraitCollection, UIView,
};
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
    client: RefCell<Option<Rc<dyn TextInputMethodClient>>>,
    cursor_rect: Cell<Rect>,
    options: RefCell<Option<TextInputOptions>>,
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

        #[unsafe(method(becomeFirstResponder))]
        fn become_first_responder(&self) -> bool {
            // SAFETY: the method of the superclass this one overrides.
            let res: bool = unsafe { msg_send![super(self), becomeFirstResponder] };
            if res {
                let this: Retained<UIView> = Retained::into_super(Message::retain(self));
                set_current_ferro_responder(Some(Retained::into_super(this)));
            }
            res
        }

        #[unsafe(method(resignFirstResponder))]
        fn resign_first_responder(&self) -> bool {
            // SAFETY: the method of the superclass this one overrides.
            let res: bool = unsafe { msg_send![super(self), resignFirstResponder] };
            if res && self.is_current_ferro_responder() {
                set_current_ferro_responder(None);
            }
            res
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

        #[unsafe(method(pressesBegan:withEvent:))]
        fn presses_began(&self, presses: &NSSet<UIPress>, evt: Option<&UIPressesEvent>) {
            if !self.handle_presses(presses, evt) {
                // SAFETY: the method of the superclass this one overrides,
                // with its arguments.
                let _: () = unsafe { msg_send![super(self), pressesBegan: presses, withEvent: evt] };
            }
        }

        #[unsafe(method(pressesChanged:withEvent:))]
        fn presses_changed(&self, presses: &NSSet<UIPress>, evt: Option<&UIPressesEvent>) {
            if !self.handle_presses(presses, evt) {
                // The reference passes a changed press on as one that
                // began (DEVIATIONS.md, iOS platform).
                // SAFETY: as above.
                let _: () = unsafe { msg_send![super(self), pressesChanged: presses, withEvent: evt] };
            }
        }

        #[unsafe(method(pressesEnded:withEvent:))]
        fn presses_ended(&self, presses: &NSSet<UIPress>, evt: Option<&UIPressesEvent>) {
            if !self.handle_presses(presses, evt) {
                // SAFETY: as above.
                let _: () = unsafe { msg_send![super(self), pressesEnded: presses, withEvent: evt] };
            }
        }

        #[unsafe(method(pressesCancelled:withEvent:))]
        fn presses_cancelled(&self, presses: &NSSet<UIPress>, evt: Option<&UIPressesEvent>) {
            if !self.handle_presses(presses, evt) {
                // SAFETY: as above.
                let _: () = unsafe { msg_send![super(self), pressesCancelled: presses, withEvent: evt] };
            }
        }

        // Deprecated since iOS 17 in favour of the registration of trait
        // changes; it is what the reference overrides, and it is called
        // on every system the port runs on.
        #[unsafe(method(traitCollectionDidChange:))]
        fn trait_collection_did_change(&self, previous_trait_collection: Option<&UITraitCollection>) {
            // SAFETY: the method of the superclass this one overrides,
            // with its argument.
            let _: () = unsafe { msg_send![super(self), traitCollectionDidChange: previous_trait_collection] };

            if let Some(settings) = Platform::settings() {
                settings.trait_collection_did_change();
            }
        }

        #[unsafe(method(tintColorDidChange))]
        fn tint_color_did_change(&self) {
            // SAFETY: the method of the superclass this one overrides.
            let _: () = unsafe { msg_send![super(self), tintColorDidChange] };

            if let Some(settings) = Platform::settings() {
                settings.trait_collection_did_change();
            }
        }

        /// The action of the pan gesture that only takes scroll events.
        #[unsafe(method(ferroHandleScrollWheel:))]
        fn ferro_handle_scroll_wheel(&self, recognizer: &UIPanGestureRecognizer) {
            if let Some(state) = self.ivars().state.get() {
                state.input.handle_scroll_wheel(recognizer);
            }
        }

        /// The tick of the display link of inertia scrolling.
        #[unsafe(method(ferroUpdateInertiaScrolling:))]
        fn ferro_update_inertia_scrolling(&self, _link: &CADisplayLink) {
            if let Some(state) = self.ivars().state.get() {
                state.input.update_inertia_scrolling();
            }
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

        // The reference adds the swipe gestures of a remote on tvOS,
        // which is not a target of the port.
        this.setMultipleTouchEnabled(true);

        // SAFETY: the view responds to the selector (its class declares
        // `ferroHandleScrollWheel:`, which takes the recognizer, the one
        // argument of an action of a gesture recognizer); a recognizer
        // does not retain its target, and the view, which is the target,
        // owns the recognizer.
        let scroll_gesture_recognizer = unsafe {
            UIPanGestureRecognizer::initWithTarget_action(
                mtm.alloc(),
                Some(&this),
                Some(sel!(ferroHandleScrollWheel:)),
            )
        };
        // Only respond to scroll events, not touches
        scroll_gesture_recognizer.setMaximumNumberOfTouches(0);
        scroll_gesture_recognizer.setAllowedScrollTypesMask(UIScrollTypeMask::Discrete | UIScrollTypeMask::Continuous);
        this.addGestureRecognizer(&scroll_gesture_recognizer);

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

    fn is_current_ferro_responder(&self) -> bool {
        current_ferro_responder().is_some_and(|current| {
            std::ptr::eq(Retained::as_ptr(&current).cast::<FerroView>(), self as *const FerroView)
        })
    }

    /// The text input responder that is the first responder, when it is
    /// one of this view.
    fn driving_responder(&self) -> Option<Retained<TextInputResponder>> {
        let responder = current_ferro_responder()?.downcast::<TextInputResponder>().ok()?;
        responder.is_of_view(self).then_some(responder)
    }

    /// Whether a text input responder of this view is the first
    /// responder: whether the keyboard writes into a client of this view.
    pub fn is_driving_text(&self) -> bool {
        self.driving_responder().is_some()
    }

    /// The responder UIKit sends the text of the keyboard to, while a
    /// client of this view has the text input: an object that implements
    /// `UITextInput`. An addition of the port, for an application that
    /// tests itself.
    pub fn text_input_responder(&self) -> Option<Retained<UIResponder>> {
        self.driving_responder().map(Retained::into_super)
    }

    /// The marked text of the text input responder of this view. An
    /// addition of the port, for an application that tests itself.
    pub fn marked_text(&self) -> Option<String> {
        self.driving_responder().and_then(|responder| responder.marked_text())
    }

    /// The rectangle of the caret the text input method was given.
    pub(crate) fn cursor_rect(&self) -> Rect {
        self.ivars().cursor_rect.get()
    }

    /// The text input options the text input method was given.
    pub(crate) fn text_input_options(&self) -> Option<TextInputOptions> {
        self.ivars().options.borrow().clone()
    }

    /// Passes a raw input event to the top-level.
    pub(crate) fn invoke_input(&self, args: Rc<dyn IRawInputEventArgs>) {
        if let Some(input) = self.state().top_level_impl.input() {
            input(args);
        }
    }

    fn set_client(&self, client: Option<Rc<dyn TextInputMethodClient>>) {
        *self.ivars().client.borrow_mut() = client.clone();
        if client.is_none() && self.is_driving_text() {
            self.becomeFirstResponder();
        }

        if let Some(client) = client {
            TextInputResponder::new(self, client).becomeFirstResponder();
        }
    }

    fn reset_text_input(&self) {
        if self.is_driving_text() {
            self.becomeFirstResponder();
        }
    }

    fn handle_presses(&self, presses: &NSSet<UIPress>, evt: Option<&UIPressesEvent>) -> bool {
        match self.ivars().state.get() {
            Some(state) => state.input.handle_presses(presses, evt),
            None => false,
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

/// The text input method of a view. The reference's view is the text
/// input method itself; a view of the port is an object of the
/// Objective-C runtime, and the contract is implemented by this object,
/// which forwards to its view.
struct ViewTextInputMethod {
    view: Weak<FerroView>,
}

impl ITextInputMethodImpl for ViewTextInputMethod {
    fn set_client(&self, client: Option<Rc<dyn TextInputMethodClient>>) {
        if let Some(view) = self.view.load() {
            view.set_client(client);
        }
    }

    fn set_cursor_rect(&self, rect: Rect) {
        if let Some(view) = self.view.load() {
            view.ivars().cursor_rect.set(rect);
        }
    }

    fn set_options(&self, options: &TextInputOptions) {
        if let Some(view) = self.view.load() {
            *view.ivars().options.borrow_mut() = Some(options.clone());
        }
    }

    fn reset(&self) {
        if let Some(view) = self.view.load() {
            view.reset_text_input();
        }
    }
}

/// The top-level implementation of a view.
pub struct TopLevelImpl {
    view: Weak<FerroView>,
    pub(crate) insets_manager: Rc<InsetsManager>,
    feedback: Rc<dyn IPlatformFeedback>,
    text_input_method: Rc<dyn ITextInputMethodImpl>,
    input_pane: Rc<dyn IInputPane>,
    storage_provider: Rc<dyn IStorageProvider>,
    clipboard: Rc<dyn IClipboard>,
    native_control_host: Rc<dyn INativeControlHostImpl>,
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
            feedback: Rc::new(IosPlatformFeedback::new(Weak::from_retained(view))),
            text_input_method: Rc::new(ViewTextInputMethod { view: Weak::from_retained(view) }),
            input_pane: UIKitInputPane::instance(),
            storage_provider: IosStorageProvider::new(Weak::from_retained(view)),
            native_control_host: NativeControlHostImpl::new(Weak::from_retained(view)),
            clipboard: Clipboard::new(Rc::new(ClipboardImpl::new(UIPasteboard::generalPasteboard()))),
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
        if feature_type == TypeId::of::<dyn ITextInputMethodImpl>() {
            return Some(Rc::new(self.text_input_method.clone()));
        }

        if feature_type == TypeId::of::<dyn IClipboard>() {
            return Some(Rc::new(self.clipboard.clone()));
        }

        if feature_type == TypeId::of::<dyn IStorageProvider>() {
            return Some(Rc::new(self.storage_provider.clone()));
        }

        if feature_type == TypeId::of::<dyn IInputPane>() {
            return Some(Rc::new(self.input_pane.clone()));
        }

        if feature_type == TypeId::of::<dyn INativeControlHostImpl>() {
            return Some(Rc::new(self.native_control_host.clone()));
        }

        if feature_type == TypeId::of::<dyn IInsetsManager>() {
            let insets_manager: Rc<dyn IInsetsManager> = self.insets_manager.clone();
            return Some(Rc::new(insets_manager));
        }

        if feature_type == TypeId::of::<dyn ILauncher>() {
            let launcher: Rc<dyn ILauncher> = Rc::new(IosLauncher::new());
            return Some(Rc::new(launcher));
        }

        if feature_type == TypeId::of::<dyn IScreenImpl>() {
            let service = FerroLocator::current().get_required_service::<dyn IScreenImpl>();
            return Some(Rc::new(service));
        }

        if feature_type == TypeId::of::<dyn IPlatformFeedback>() {
            return Some(Rc::new(self.feedback.clone()));
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
