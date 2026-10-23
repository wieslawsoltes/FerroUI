//! Port of `Diagnostics/Views/MainWindow.xaml.cs`: the class of the document
//! `MainWindow.xaml`, the window of the developer tools.

use crate::diagnostics::controls::Application;
use crate::diagnostics::view_models::MainViewModel;
use crate::diagnostics::{DevToolsOptions, HotKeyConfiguration};
use ferroui_base::controls::ResourceKey;
use ferroui_base::input::raw::{RawKeyEventArgs, RawKeyEventType, RawPointerEventArgs};
use ferroui_base::input::{ICloseable, IInputRoot, InputElement, InputElementImpl, InputManager, Key, KeyGesture, KeyModifiers};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::metadata::{from_markup_value, into_markup_value};
use ferroui_base::reactive::{IDisposable, ObservableExt};
use ferroui_base::styling::{ControlTheme, StyleHostRef};
use ferroui_base::utilities::CancelEventArgs;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObject, FerroObjectImpl, PixelPoint, Ref,
    StyledElementImpl, Visual, VisualImpl,
};
use ferroui_controls::diagnostics::{IPopupHostProvider, ToolTipDiagnostics};
use ferroui_controls::platform::PlatformManager;
use ferroui_controls::primitives::{
    AdornerLayer, FlyoutBase, Popup, PopupFlyoutBase, PopupRoot, TemplatedControlImpl,
};
use ferroui_controls::{
    Button, ContentControlImpl, Control, ControlImpl, TopLevel, TopLevelImpl, TopLevelImplExt, Window,
    WindowBaseImpl, WindowImpl, WindowState,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The popup root of the host of a popup host provider, if it has one and
/// it is a popup root (C# `popupProvider.PopupHost is PopupRoot popupRoot`).
fn popup_root_of(popup_provider: Option<Rc<dyn IPopupHostProvider>>) -> Option<Ref<PopupRoot>> {
    popup_provider.and_then(|popup_provider| popup_provider.popup_host()).and_then(|popup_host| popup_host.as_popup_root())
}

/// The popup host provider of a flyout, if the flyout is one (C#
/// `flyout is IPopupHostProvider`).
fn flyout_popup_host_provider(flyout: Option<Ref<FlyoutBase>>) -> Option<Rc<dyn IPopupHostProvider>> {
    flyout.and_then(|flyout| flyout.cast::<PopupFlyoutBase>()).map(|flyout| flyout.to_popup_host_provider())
}

/// The window of the developer tools: the main view over the view model of
/// the root it inspects, the hot keys that pick the control under the
/// pointer, freeze the popups, freeze the value frames and take screenshots.
#[repr(C)]
pub struct MainWindow {
    base: Window,
    input_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    /// The frozen popups, with the subscriptions of their `Closing` events.
    frozen_popup_states: RefCell<Vec<(Ref<Popup>, Rc<dyn IDisposable>)>>,
    root: RefCell<Option<Ref<FerroObject>>>,
    /// The subscription to the `Closed` event of the root.
    root_closed_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    last_pointer_position: Cell<PixelPoint>,
    hot_keys: RefCell<Option<Rc<HotKeyConfiguration>>>,
}

ferro_class!(MainWindow: Window);
ferro_impl_classes!(
    MainWindow: FerroObjectImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl,
    WindowBaseImpl,
    WindowImpl
);
ferro_class_info!(MainWindow { new: MainWindow::new });

impl StyledElementImpl for MainWindow {
    /// C# `IStyleHost? IStyleHost.StylingParent => null`: the window does
    /// not take the styles of the application it inspects.
    fn styling_parent(_this: &Self) -> Option<StyleHostRef> {
        None
    }
}

impl TopLevelImpl for MainWindow {
    fn on_closed(this: &Self) {
        Self::parent_on_closed(this);
        let input_subscription = this.input_subscription.borrow().clone();
        if let Some(input_subscription) = input_subscription {
            input_subscription.dispose();
        }

        let frozen_popup_states: Vec<_> = this.frozen_popup_states.borrow_mut().drain(..).collect();
        for (_, closing_subscription) in frozen_popup_states {
            closing_subscription.dispose();
        }

        let root = this.root.borrow().clone();
        if root.as_ref().and_then(Self::closeable_of).is_some() {
            let root_closed_subscription = this.root_closed_subscription.borrow_mut().take();
            if let Some(root_closed_subscription) = root_closed_subscription {
                root_closed_subscription.dispose();
            }
            *this.root.borrow_mut() = None;
        }

        if let Some(view_model) = this.view_model() {
            view_model.dispose();
        }
    }
}

impl MainWindow {
    /// The URI of the document of the class.
    pub const DOCUMENT_URI: &'static str = "ferres://FerroUI.Diagnostics/diagnostics/views/MainWindow.xaml";

    pub fn construct() -> Self {
        Self {
            base: Window::construct(PlatformManager::create_window()),
            input_subscription: RefCell::new(None),
            frozen_popup_states: RefCell::new(Vec::new()),
            root: RefCell::new(None),
            root_closed_subscription: RefCell::new(None),
            last_pointer_position: Cell::new(PixelPoint::default()),
            hot_keys: RefCell::new(None),
        }
    }

    /// Creates the window and populates it from its document
    /// (`InitializeComponent()`: the compiled markup of the document, which
    /// the build of the crate generates), with the theme of a window of the
    /// WinUI 3 theme, the observation of the input and the placement on the
    /// startup screen.
    ///
    /// # Panics
    /// Panics when no windowing platform is registered, and when the
    /// document fails to load (an exception of the constructor in the
    /// original).
    pub fn new() -> Ref<Self> {
        crate::register_types();
        let this = instantiate(Self::construct());
        Self::initialize_component(&this);

        // Deviation (DEVIATIONS.md, Developer tools): the platform as a class of the window, for
        // the room the header in the title bar leaves to the caption buttons of the system.
        let platform = if cfg!(target_os = "macos") {
            "macos"
        } else if cfg!(windows) {
            "windows"
        } else {
            "other"
        };
        this.classes().add(platform);

        // Apply the SimpleTheme.Window theme; this must be done after the XAML is parsed as
        // the theme is included in the MainWindow's XAML.
        // Deviation (DEVIATIONS.md, Developer tools): the theme of a window of the WinUI 3 theme,
        // which the document includes in place of the Simple theme.
        if this.theme().is_none() {
            let window_theme = this
                .find_resource(&ResourceKey::Type(Window::TYPE))
                .and_then(|value| value.downcast_ref::<Ref<ControlTheme>>().cloned());
            if let Some(window_theme) = window_theme {
                this.set_theme(window_theme);
            }
        }

        let weak = this.downgrade();
        let input_subscription = InputManager::instance().map(|input_manager| {
            input_manager.process().subscribe_fn(move |x| {
                let Some(this) = weak.upgrade() else {
                    return;
                };
                if let Some(pointer_event_args) = x.downcast_ref::<RawPointerEventArgs>() {
                    if let Some(root) = x.root().try_root_element() {
                        this.last_pointer_position.set(root.point_to_screen(pointer_event_args.position()));
                    }
                } else if let Some(key_event_args) = x.downcast_ref::<RawKeyEventArgs>() {
                    if key_event_args.type_() == RawKeyEventType::KeyDown {
                        this.raw_key_down(key_event_args);
                    }
                }
            })
        });
        *this.input_subscription.borrow_mut() = input_subscription;

        // `this.Opened += lh`, where the handler removes itself.
        let opened_subscription: Rc<RefCell<Option<Rc<dyn IDisposable>>>> = Rc::new(RefCell::new(None));
        let handler_subscription = opened_subscription.clone();
        let weak = this.downgrade();
        let subscription = this.opened(move || {
            let own = handler_subscription.borrow_mut().take();
            if let Some(own) = own {
                own.dispose();
            }
            let Some(this) = weak.upgrade() else {
                return;
            };
            if let Some(index) = this.view_model().and_then(|view_model| view_model.startup_screen_index()) {
                let screens = this.screens();
                if index > -1 && index < screens.screen_count() {
                    let screen = screens.all()[index as usize].clone();
                    this.set_position(screen.bounds().top_left());
                    this.set_window_state(WindowState::Maximized);
                }
            }
        });
        *opened_subscription.borrow_mut() = Some(subscription);

        this
    }

    /// `InitializeComponent()`.
    fn initialize_component(this: &Ref<Self>) {
        if let Err(error) = crate::compiled_diagnostics_views_main_window::populate(None, this) {
            panic!("{}: {error}", Self::DOCUMENT_URI);
        }
    }

    /// The view model of the window (C# `DataContext as MainViewModel`).
    fn view_model(&self) -> Option<Rc<MainViewModel>> {
        from_markup_value::<Rc<MainViewModel>>(&self.data_context())
    }

    /// The root as `ICloseable` (C# `root is ICloseable`): a top level, or
    /// the application of the developer tools.
    fn closeable_of(root: &Ref<FerroObject>) -> Option<Rc<dyn ICloseable>> {
        if let Some(top_level) = root.cast::<TopLevel>() {
            return Some(top_level.as_closeable());
        }
        if let Some(application) = root.cast::<Application>() {
            return Some(Application::as_closeable(application));
        }
        None
    }

    /// The object the window inspects: a top level, or the application of
    /// the developer tools.
    pub fn root(&self) -> Option<Ref<FerroObject>> {
        self.root.borrow().clone()
    }

    /// Sets the object the window inspects; a root that tells when it is
    /// closed gets a view model and closes the window when it closes.
    pub fn set_root(&self, value: Option<Ref<FerroObject>>) {
        if *self.root.borrow() != value {
            let old_closable = self.root.borrow().as_ref().and_then(Self::closeable_of);
            if old_closable.is_some() {
                let root_closed_subscription = self.root_closed_subscription.borrow_mut().take();
                if let Some(root_closed_subscription) = root_closed_subscription {
                    root_closed_subscription.dispose();
                }
            }

            *self.root.borrow_mut() = value.clone();

            match value.as_ref().and_then(|root| Self::closeable_of(root).map(|closeable| (root.clone(), closeable))) {
                Some((root, new_closable)) => {
                    let weak = self.to_ref().downgrade();
                    let subscription = new_closable.closed(Rc::new(move || {
                        if let Some(this) = weak.upgrade() {
                            this.root_closed();
                        }
                    }));
                    *self.root_closed_subscription.borrow_mut() = Some(subscription);
                    self.set_data_context(into_markup_value(MainViewModel::new(root)));
                }
                None => self.set_data_context(None),
            }
        }
    }

    /// The control under the pointer in a top level, the topmost visible
    /// one that is hit test visible, outside of the adorner layer.
    fn get_hovered_control(&self, top_level: &TopLevel) -> Option<Ref<Control>> {
        let point = top_level.point_to_client(self.last_pointer_position.get());

        top_level
            .get_visuals_at_filtered(point, &|x: &Visual| {
                if x.downcast_ref::<AdornerLayer>().is_some() || !x.is_visible() {
                    return false;
                }

                x.downcast_ref::<InputElement>().is_none_or(|ie| ie.is_hit_test_visible())
            })
            .into_iter()
            .next()
            // `(Control?)`: the topmost visual, as a control.
            .and_then(|visual| visual.cast::<Control>())
    }

    /// The roots of the popups that are open in the visual tree of a top
    /// level: of popups, context flyouts and menus, attached flyouts, tool
    /// tips and flyouts of buttons.
    fn get_popup_roots(root: &TopLevel) -> Vec<Ref<PopupRoot>> {
        let mut popup_roots = Vec::new();

        fn process_property(popup_roots: &mut Vec<Ref<PopupRoot>>, popup_provider: Option<Rc<dyn IPopupHostProvider>>) {
            if let Some(popup_root) = popup_root_of(popup_provider) {
                popup_roots.push(popup_root);
            }
        }

        for control in root.get_visual_descendants().filter_map(|visual| visual.cast::<Control>()) {
            if let Some(p) = control.cast::<Popup>() {
                if let Some(popup_root) = p.host().and_then(|host| host.as_popup_root()) {
                    popup_roots.push(popup_root);
                }
            }

            process_property(
                &mut popup_roots,
                flyout_popup_host_provider(control.get_value(Control::context_flyout_property())),
            );
            process_property(
                &mut popup_roots,
                control.get_value(Control::context_menu_property()).map(|context_menu| context_menu.to_popup_host_provider()),
            );
            process_property(
                &mut popup_roots,
                flyout_popup_host_provider(control.get_value(FlyoutBase::attached_flyout_property())),
            );
            process_property(
                &mut popup_roots,
                control.get_value(ToolTipDiagnostics::tool_tip_property()).map(|tool_tip| tool_tip.to_popup_host_provider()),
            );
            process_property(&mut popup_roots, flyout_popup_host_provider(control.get_value(Button::flyout_property())));
        }

        popup_roots
    }

    fn raw_key_down(&self, e: &RawKeyEventArgs) {
        let hot_keys = self.hot_keys.borrow().clone();
        let Some(hot_keys) = hot_keys else {
            return;
        };
        let Some(vm) = self.view_model() else {
            return;
        };
        // `vm.PointerOverRoot is TopLevel root`: the top level of the input root.
        let Some(mut root) = vm.pointer_over_root().and_then(|pointer_over_root| top_level_of_root(&*pointer_over_root))
        else {
            return;
        };

        if let Some(parent_top_level) = root.cast::<PopupRoot>().and_then(|pr| pr.parent_top_level()) {
            root = parent_top_level;
        }

        let modifiers = Self::merge_modifiers(e.key(), e.modifiers().to_key_modifiers());

        if Self::is_matched(&hot_keys.value_frames_freeze(), e.key(), modifiers) {
            self.freeze_value_frames(&vm);
        } else if Self::is_matched(&hot_keys.value_frames_unfreeze(), e.key(), modifiers) {
            self.unfreeze_value_frames(&vm);
        } else if Self::is_matched(&hot_keys.toggle_popup_freeze(), e.key(), modifiers) {
            self.toggle_freeze_popups(&root, &vm);
        } else if Self::is_matched(&hot_keys.screenshot_selected_control(), e.key(), modifiers) {
            self.screenshot_selected_control(&vm);
        } else if Self::is_matched(&hot_keys.inspect_hovered_control(), e.key(), modifiers) {
            self.inspect_hovered_control(&root, &vm);
        }
    }

    fn is_matched(gesture: &KeyGesture, key: Key, modifiers: KeyModifiers) -> bool {
        (gesture.key() == key || gesture.key() == Key::None) && modifiers.contains(gesture.key_modifiers())
    }

    /// When Control, Shift, or Alt are initially pressed, they are the Key and not part of Modifiers
    /// This merges so modifier keys alone can more easily trigger actions
    fn merge_modifiers(key: Key, modifiers: KeyModifiers) -> KeyModifiers {
        match key {
            Key::LeftCtrl | Key::RightCtrl => modifiers | KeyModifiers::CONTROL,
            Key::LeftShift | Key::RightShift => modifiers | KeyModifiers::SHIFT,
            Key::LeftAlt | Key::RightAlt => modifiers | KeyModifiers::ALT,
            _ => modifiers,
        }
    }

    fn freeze_value_frames(&self, vm: &MainViewModel) {
        vm.enable_snapshot_styles(true);
    }

    fn unfreeze_value_frames(&self, vm: &MainViewModel) {
        vm.enable_snapshot_styles(false);
    }

    fn toggle_freeze_popups(&self, root: &TopLevel, vm: &MainViewModel) {
        vm.set_freeze_popups(!vm.freeze_popups());

        for popup_root in Self::get_popup_roots(root) {
            if let Some(popup) = popup_root.parent().and_then(|parent| parent.cast::<Popup>()) {
                if vm.freeze_popups() {
                    let weak = self.to_ref().downgrade();
                    let closing_subscription = popup.closing(move |e| {
                        if let Some(this) = weak.upgrade() {
                            this.popup_on_closing(e);
                        }
                    });
                    self.frozen_popup_states.borrow_mut().push((popup, closing_subscription));
                } else {
                    let removed: Vec<_> = {
                        let mut frozen_popup_states = self.frozen_popup_states.borrow_mut();
                        let (removed, kept) =
                            frozen_popup_states.drain(..).partition(|(frozen, _)| frozen.ptr_eq(&popup));
                        *frozen_popup_states = kept;
                        removed
                    };
                    for (_, closing_subscription) in removed {
                        closing_subscription.dispose();
                    }
                }
            }
        }
    }

    fn screenshot_selected_control(&self, vm: &MainViewModel) {
        vm.shot(None);
    }

    fn inspect_hovered_control(&self, root: &TopLevel, vm: &MainViewModel) {
        let mut control = None;

        for popup_root in Self::get_popup_roots(root) {
            control = self.get_hovered_control(&popup_root);

            if control.is_some() {
                break;
            }
        }

        let control = control.or_else(|| self.get_hovered_control(root));

        if let Some(control) = control {
            vm.select_control(&control);
        }
    }

    fn popup_on_closing(&self, e: &CancelEventArgs) {
        if self.view_model().is_some_and(|vm| vm.freeze_popups()) {
            e.set_cancel(true);
        }
    }

    fn root_closed(&self) {
        self.close();
    }

    /// Applies the options the developer tools were opened with: the hot
    /// keys, the options of the view model and the theme variant.
    pub fn set_options(&self, options: &DevToolsOptions) {
        *self.hot_keys.borrow_mut() = Some(options.hot_keys());

        if let Some(view_model) = self.view_model() {
            view_model.set_options(options);
        }
        if let Some(theme_variant) = options.theme_variant() {
            self.set_requested_theme_variant(Some(theme_variant));
        }
    }

    /// Selects a control in the tree page of the window.
    pub fn selected_control(&self, control: Option<Ref<Control>>) {
        if let Some(control) = control {
            if let Some(view_model) = self.view_model() {
                view_model.select_control(&control);
            }
        }
    }
}

/// The top level of an input root.
///
/// Deviation (DEVIATIONS.md, Developer tools): the root element of the input root of a top level
/// is its host (the `TopLevelHost` that holds the window decorations), with the top level as its
/// visual child; the developer tools were written for the framework version whose input root was
/// the top level itself (`vm.PointerOverRoot is TopLevel root`), and with the host in between the
/// hot keys that need the hovered top level (Ctrl+Shift, Ctrl+Alt+F) did nothing.
pub(crate) fn top_level_of_root(root: &dyn IInputRoot) -> Option<Ref<TopLevel>> {
    let element = root.try_root_element()?;
    if let Some(top_level) = element.clone().cast::<TopLevel>() {
        return Some(top_level);
    }
    element.visual_children().to_vec().into_iter().find_map(|child| child.cast::<TopLevel>())
}
