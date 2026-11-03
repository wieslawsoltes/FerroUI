//! The sample windows of the SystemBackdrops and TitleBar examples of the pinned WinUI Gallery
//! (`SampleSupport/SamplePages`: `SampleBuiltInSystemBackdropsWindow`,
//! `SampleSystemBackdropsWindow`, `TitleBarDragRegionsWindow` and `TitleBarWindow`), with their
//! C# handlers in Rust. The window, its system backdrop, the backdrop controllers and the title
//! bar are the framework's; the content of each window is its markup in `Pages/`.
//!
//! A window opened by a "Show window" button lives until it is closed, as a WinUI window does:
//! the sample keeps it in [`opened_windows`] and releases it when it closes.

use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{Brushes, Color, IBrush};
use ferroui_base::platform::{ColorContrastPreference, IPlatformSettings};
use ferroui_base::reactive::Subscription;
use ferroui_base::rendering::composition::system_backdrops::{
    DesktopAcrylicController, DesktopAcrylicKind, ICompositionSupportsSystemBackdrop, MicaController, MicaKind,
    SystemBackdropConfiguration, SystemBackdropTheme,
};
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{BoxedValue, FerroLocator, LocatorExtensions, ObjectType, Ref, WeakRef};
use ferroui_controls::media::{DesktopAcrylicBackdrop, MicaBackdrop, SystemBackdrop};
use ferroui_controls::primitives::SelectingItemsControl;
use ferroui_controls::{
    Button, ComboBox, Control, Frame, Grid, NavigationView, NavigationViewItem, RadioButtons, StackPanel,
    TextBlock, TitleBar, TopLevel, Window,
};
use ferroui_markup_xaml::XamlLoadError;
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

ferroui_base::ferro_thread_local! {
    /// The sample windows that are open (strong: a window lives until it is closed).
    static OPENED: RefCell<Vec<Ref<Window>>> = const { RefCell::new(Vec::new()) };
}

/// The sample windows the pages opened that are not closed yet, oldest first.
pub(crate) fn opened_windows() -> Vec<Ref<Window>> {
    OPENED.with(|opened| opened.borrow().clone())
}

pub(crate) fn wire(root: &Ref<Control>) {
    wire_system_backdrops_page(root);
    wire_title_bar_page(root);
}

fn find<T: ObjectType>(root: &Ref<Control>, name: &str) -> Option<Ref<T>> {
    root.find_control::<Control>(name).and_then(|control| control.cast())
}

fn on_selection(control: &Ref<SelectingItemsControl>, handler: impl Fn(i32) + 'static) {
    let weak = control.downgrade();
    control.property_changed(move |change| {
        if change.property() != SelectingItemsControl::selected_index_property().as_property() {
            return;
        }
        if let Some(control) = weak.upgrade() {
            handler(control.selected_index());
        }
    });
}

/// Creates a window with `xaml` as its content, extended into the title bar
/// (`ExtendsContentIntoTitleBar = true`), with the theme of the window of `opener`
/// (`RequestedTheme = ThemeHelper.RootTheme`).
fn create_window(xaml: &str, title: &str, opener: &Ref<Control>) -> Result<(Ref<Window>, Ref<Control>), XamlLoadError> {
    let content = FerroRuntimeXamlLoader::parse_as::<Ref<Control>>(xaml, None)?;
    let window = Window::new();
    window.set_title(Some(title.to_string()));
    window.set_width(800.0);
    window.set_height(600.0);
    window.set_extend_client_area_to_decorations_hint(true);
    if let Some(top_level) = TopLevel::get_top_level(Some(opener)) {
        window.set_requested_theme_variant(top_level.requested_theme_variant());
    }
    window.set_content(Some(Control::boxed(content.clone())));
    Ok((window, content))
}

/// `Activate`: shows the window and keeps it until it closes.
fn show(window: &Ref<Window>) {
    OPENED.with(|opened| opened.borrow_mut().push(window.clone()));
    let weak = window.downgrade();
    window
        .closed(move || {
            if let Some(window) = weak.upgrade() {
                OPENED.with(|opened| opened.borrow_mut().retain(|open| open != &window));
            }
        })
        .detach();
    window.show();
}

/// Wires a "Show window" button: `open` creates the window; a failure to load its markup is
/// shown in the page's `WindowSampleError`.
fn wire_show_button(
    root: &Ref<Control>,
    button: &str,
    open: impl Fn(&Ref<Control>) -> Result<Ref<Window>, XamlLoadError> + 'static,
) {
    let button = root.get_control::<Button>(button);
    let weak_root = root.downgrade();
    button.click(move |_, _| {
        let Some(root) = weak_root.upgrade() else { return };
        let error = find::<TextBlock>(&root, "WindowSampleError");
        match open(&root) {
            Ok(_) => {
                if let Some(error) = error {
                    error.set_text(None);
                    error.set_is_visible(false);
                }
            }
            Err(reason) => {
                if let Some(error) = error {
                    error.set_text(Some(&format!("The sample window failed to load: {reason}")));
                    error.set_is_visible(true);
                }
            }
        }
    });
}

// --- SystemBackdrops -------------------------------------------------------------------------

/// `BackdropType` of the two sample windows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BackdropType {
    None,
    Mica,
    MicaAlt,
    Acrylic,
    AcrylicThin,
}

impl BackdropType {
    /// The name the ComboBox shows (the C# enum's).
    pub(crate) fn name(self) -> &'static str {
        match self {
            BackdropType::None => "None",
            BackdropType::Mica => "Mica",
            BackdropType::MicaAlt => "MicaAlt",
            BackdropType::Acrylic => "Acrylic",
            BackdropType::AcrylicThin => "AcrylicThin",
        }
    }
}

fn wire_system_backdrops_page(root: &Ref<Control>) {
    if find::<Button>(root, "ShowBuiltInBackdropWindow").is_none() {
        return;
    }
    // createBuiltInWindow_Click.
    wire_show_button(root, "ShowBuiltInBackdropWindow", |root| {
        let window = BackdropWindow::open_built_in(root)?;
        window.set_backdrop(BackdropType::Mica);
        Ok(window.window())
    });
    // createCustomMicaWindow_Click and createCustomDesktopAcrylicWindow_Click.
    wire_show_button(root, "ShowMicaControllerWindow", |root| {
        let window = BackdropWindow::open_with_controllers(
            root,
            &[BackdropType::Mica, BackdropType::MicaAlt, BackdropType::None],
        )?;
        Ok(window.window())
    });
    wire_show_button(root, "ShowDesktopAcrylicControllerWindow", |root| {
        let window = BackdropWindow::open_with_controllers(
            root,
            &[BackdropType::Acrylic, BackdropType::AcrylicThin, BackdropType::None],
        )?;
        Ok(window.window())
    });
}

/// How a backdrop window applies its backdrop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BackdropMode {
    /// `SampleBuiltInSystemBackdropsWindow`: `Window.SystemBackdrop` with `MicaBackdrop` and
    /// `DesktopAcrylicBackdrop`.
    BuiltIn,
    /// `SampleSystemBackdropsWindow`: `MicaController` and `DesktopAcrylicController` with a
    /// `SystemBackdropConfiguration` the window keeps up to date.
    Controllers,
}

/// The state of one backdrop sample window (the fields of the C# window classes).
pub(crate) struct BackdropWindow {
    window: WeakRef<Window>,
    content: WeakRef<Grid>,
    status: WeakRef<TextBlock>,
    announcement: WeakRef<TextBlock>,
    theme_box: WeakRef<ComboBox>,
    mode: BackdropMode,
    allowed: Vec<BackdropType>,
    current: Cell<BackdropType>,
    mica: RefCell<Option<MicaController>>,
    acrylic: RefCell<Option<DesktopAcrylicController>>,
    configuration: RefCell<Option<Rc<SystemBackdropConfiguration>>>,
    subscriptions: RefCell<Vec<Subscription>>,
}

fn is_high_contrast() -> bool {
    FerroLocator::current()
        .get_service::<dyn IPlatformSettings>()
        .is_some_and(|settings| settings.get_color_values().contrast_preference() == ColorContrastPreference::High)
}

impl BackdropWindow {
    fn open(root: &Ref<Control>, mode: BackdropMode, allowed: &[BackdropType]) -> Result<Rc<BackdropWindow>, XamlLoadError> {
        // Both sample windows have the title of SampleBuiltInSystemBackdropsWindow.xaml.
        let title = "SystemBackdrop sample window";
        let (window, content) = create_window(include_str!("Pages/SystemBackdropsWindow.xaml"), title, root)?;
        let grid = content
            .cast::<Grid>()
            .ok_or_else(|| XamlLoadError::with_message("The sample window's root is not a Grid"))?;
        content.get_control::<TextBlock>("AppTitle").set_text(Some(title));
        let backdrop_box = content.get_control::<ComboBox>("backdropComboBox");
        let theme_box = content.get_control::<ComboBox>("themeComboBox");
        for backdrop in allowed {
            backdrop_box.items().add(Some(Rc::new(backdrop.name().to_string()) as BoxedValue));
        }
        let state = Rc::new(BackdropWindow {
            window: window.downgrade(),
            content: grid.downgrade(),
            status: content.get_control::<TextBlock>("tbChangeStatus").downgrade(),
            announcement: content.get_control::<TextBlock>("BackdropAnnouncement").downgrade(),
            theme_box: theme_box.downgrade(),
            mode,
            allowed: allowed.to_vec(),
            current: Cell::new(BackdropType::None),
            mica: RefCell::new(None),
            acrylic: RefCell::new(None),
            configuration: RefCell::new(None),
            subscriptions: RefCell::new(Vec::new()),
        });
        if mode == BackdropMode::Controllers {
            state.hook_configuration(&window);
        }
        {
            // BackdropComboBox_SelectionChanged.
            let state = state.clone();
            on_selection(backdrop_box.upcast_ref::<SelectingItemsControl>(), move |index| {
                let backdrop = match state.mode {
                    BackdropMode::BuiltIn => match index {
                        1 => BackdropType::MicaAlt,
                        2 => BackdropType::Acrylic,
                        3 => BackdropType::None,
                        _ => BackdropType::Mica,
                    },
                    BackdropMode::Controllers => {
                        match usize::try_from(index).ok().and_then(|index| state.allowed.get(index)) {
                            Some(backdrop) => *backdrop,
                            None => return,
                        }
                    }
                };
                state.set_backdrop(backdrop);
            });
        }
        {
            // ThemeComboBox_SelectionChanged.
            let weak = Rc::downgrade(&state);
            on_selection(theme_box.upcast_ref::<SelectingItemsControl>(), move |index| {
                let Some(state) = weak.upgrade() else { return };
                let Some(window) = state.window.upgrade() else { return };
                window.set_requested_theme_variant(match index {
                    1 => Some(ThemeVariant::light()),
                    2 => Some(ThemeVariant::dark()),
                    _ => None,
                });
                state.set_none_backdrop_background();
            });
        }
        backdrop_box.set_selected_index(0);
        theme_box.set_selected_index(0);
        show(&window);
        Ok(state)
    }

    /// `new SampleBuiltInSystemBackdropsWindow()` and `Activate()`.
    pub(crate) fn open_built_in(root: &Ref<Control>) -> Result<Rc<BackdropWindow>, XamlLoadError> {
        let all = [BackdropType::Mica, BackdropType::MicaAlt, BackdropType::Acrylic, BackdropType::None];
        Self::open(root, BackdropMode::BuiltIn, &all)
    }

    /// `new SampleSystemBackdropsWindow { AllowedBackdrops = allowed }` and `Activate()`.
    pub(crate) fn open_with_controllers(
        root: &Ref<Control>,
        allowed: &[BackdropType],
    ) -> Result<Rc<BackdropWindow>, XamlLoadError> {
        Self::open(root, BackdropMode::Controllers, allowed)
    }

    pub(crate) fn window(&self) -> Ref<Window> {
        self.window.upgrade().expect("the sample window is alive while its state is used")
    }

    pub(crate) fn current(&self) -> BackdropType {
        self.current.get()
    }

    pub(crate) fn mica_controller(&self) -> Option<MicaController> {
        self.mica.borrow().clone()
    }

    pub(crate) fn acrylic_controller(&self) -> Option<DesktopAcrylicController> {
        self.acrylic.borrow().clone()
    }

    pub(crate) fn configuration(&self) -> Option<Rc<SystemBackdropConfiguration>> {
        self.configuration.borrow().clone()
    }

    /// The Activated, Closed and ActualThemeChanged handlers of the controller window. The C#
    /// window adds them again for every backdrop it sets; they are added once here and read the
    /// current configuration.
    fn hook_configuration(self: &Rc<Self>, window: &Ref<Window>) {
        let mut subscriptions = self.subscriptions.borrow_mut();
        for active in [true, false] {
            let weak = Rc::downgrade(self);
            let handler = move || {
                // Window_Activated.
                if let Some(configuration) = weak.upgrade().and_then(|state| state.configuration()) {
                    configuration.set_is_input_active(active);
                }
            };
            subscriptions.push(if active { window.activated(handler) } else { window.deactivated(handler) });
        }
        let weak = Rc::downgrade(self);
        subscriptions.push(window.closed(move || {
            // Window_Closed: no controller uses the closed window.
            if let Some(state) = weak.upgrade() {
                state.dispose_controllers();
            }
        }));
        let weak = Rc::downgrade(self);
        subscriptions.push(window.actual_theme_variant_changed(move || {
            // Window_ThemeChanged.
            if let Some(state) = weak.upgrade() {
                state.set_configuration_source_theme();
            }
        }));
    }

    fn dispose_controllers(&self) {
        if let Some(mica) = self.mica.borrow_mut().take() {
            mica.close();
        }
        if let Some(acrylic) = self.acrylic.borrow_mut().take() {
            acrylic.close();
        }
        *self.configuration.borrow_mut() = None;
    }

    /// `SetConfigurationSourceTheme`.
    fn set_configuration_source_theme(&self) {
        let (Some(configuration), Some(window)) = (self.configuration(), self.window.upgrade()) else {
            return;
        };
        configuration.set_is_high_contrast(is_high_contrast());
        configuration.set_theme(if window.actual_theme_variant() == Some(ThemeVariant::dark()) {
            SystemBackdropTheme::Dark
        } else {
            SystemBackdropTheme::Light
        });
    }

    fn append_status(&self, text: &str) {
        if let Some(status) = self.status.upgrade() {
            let current = status.text().unwrap_or_default();
            status.set_text(Some(&format!("{current}{text}")));
        }
    }

    /// `SetBackdrop`: removes the previous backdrop, then sets `requested`, falling back from Mica
    /// to acrylic and from acrylic to the default colour where the platform has no such material.
    pub(crate) fn set_backdrop(&self, requested: BackdropType) {
        let Some(window) = self.window.upgrade() else { return };
        self.current.set(BackdropType::None);
        if let Some(status) = self.status.upgrade() {
            status.set_text(Some(""));
        }
        match self.mode {
            BackdropMode::BuiltIn => window.set_system_backdrop(None),
            BackdropMode::Controllers => self.dispose_controllers(),
        }
        let mut backdrop = requested;
        if matches!(backdrop, BackdropType::Mica | BackdropType::MicaAlt) {
            let alt = backdrop == BackdropType::MicaAlt;
            if self.try_set_mica_backdrop(&window, alt) {
                self.current.set(backdrop);
            } else {
                // Mica isn't supported. Try Acrylic.
                self.append_status(if alt {
                    "  MicaAlt isn't supported. Trying Acrylic."
                } else {
                    "  Mica isn't supported. Trying Acrylic."
                });
                backdrop = BackdropType::Acrylic;
            }
        }
        if matches!(backdrop, BackdropType::Acrylic | BackdropType::AcrylicThin) {
            let thin = backdrop == BackdropType::AcrylicThin;
            if self.try_set_acrylic_backdrop(&window, thin) {
                self.current.set(backdrop);
            } else {
                // Acrylic isn't supported: the default colour, which is already set.
                self.append_status(match (self.mode, thin) {
                    (BackdropMode::BuiltIn, _) => "  Acrylic isn't supported. Switching to default color.",
                    (BackdropMode::Controllers, false) => "  Acrylic Base isn't supported. Switching to default color.",
                    (BackdropMode::Controllers, true) => "  Acrylic Thin isn't supported. Switching to default color.",
                });
            }
        }
        self.set_none_backdrop_background();
        // Announce visual change to automation.
        if let Some(announcement) = self.announcement.upgrade() {
            announcement.set_text(Some(&format!("Background changed to {}", self.current.get().name())));
        }
    }

    /// `TrySetMicaBackdrop`.
    fn try_set_mica_backdrop(&self, window: &Ref<Window>, use_mica_alt: bool) -> bool {
        if !MicaController::is_supported() {
            return false;
        }
        let kind = if use_mica_alt { MicaKind::BaseAlt } else { MicaKind::Base };
        match self.mode {
            BackdropMode::BuiltIn => {
                let mica = MicaBackdrop::new();
                mica.set_kind(kind);
                window.set_system_backdrop(Some(mica.upcast::<SystemBackdrop>()));
            }
            BackdropMode::Controllers => {
                let configuration = self.new_configuration();
                let controller = MicaController::new();
                controller.set_kind(kind);
                let target: Rc<dyn ICompositionSupportsSystemBackdrop> = window.system_backdrop_target();
                controller.add_system_backdrop_target(target);
                controller.set_system_backdrop_configuration(Some(configuration));
                *self.mica.borrow_mut() = Some(controller);
            }
        }
        true
    }

    /// `TrySetAcrylicBackdrop`. `DesktopAcrylicBackdrop` always uses the Base kind; the Thin kind
    /// needs the controller.
    fn try_set_acrylic_backdrop(&self, window: &Ref<Window>, use_acrylic_thin: bool) -> bool {
        if !DesktopAcrylicController::is_supported() {
            return false;
        }
        match self.mode {
            BackdropMode::BuiltIn => {
                window.set_system_backdrop(Some(DesktopAcrylicBackdrop::new().upcast::<SystemBackdrop>()));
            }
            BackdropMode::Controllers => {
                let configuration = self.new_configuration();
                let controller = DesktopAcrylicController::new();
                controller.set_kind(if use_acrylic_thin { DesktopAcrylicKind::Thin } else { DesktopAcrylicKind::Base });
                let target: Rc<dyn ICompositionSupportsSystemBackdrop> = window.system_backdrop_target();
                controller.add_system_backdrop_target(target);
                controller.set_system_backdrop_configuration(Some(configuration));
                *self.acrylic.borrow_mut() = Some(controller);
            }
        }
        true
    }

    /// The policy object of the controllers: input active, the theme and the high contrast
    /// setting of the window.
    fn new_configuration(&self) -> Rc<SystemBackdropConfiguration> {
        let configuration = SystemBackdropConfiguration::new();
        configuration.set_is_high_contrast(is_high_contrast());
        configuration.set_is_input_active(true);
        *self.configuration.borrow_mut() = Some(configuration.clone());
        self.set_configuration_source_theme();
        configuration
    }

    /// `SetNoneBackdropBackground`: without a backdrop and with an explicit theme, the content is
    /// white or black; otherwise transparent.
    fn set_none_backdrop_background(&self) {
        let (Some(content), Some(theme_box)) = (self.content.upgrade(), self.theme_box.upgrade()) else {
            return;
        };
        let index = theme_box.selected_index();
        let brush: Rc<dyn IBrush> = if self.current.get() == BackdropType::None && index > 0 {
            let color = if index == 1 { Color::from_rgb(0xFF, 0xFF, 0xFF) } else { Color::from_rgb(0x00, 0x00, 0x00) };
            Rc::new(ImmutableSolidColorBrush::new(color)) as Rc<dyn IBrush>
        } else {
            Brushes::transparent() as Rc<dyn IBrush>
        };
        content.set_background(Some(brush));
    }
}

// --- TitleBar --------------------------------------------------------------------------------

fn wire_title_bar_page(root: &Ref<Control>) {
    if find::<TitleBar>(root, "TitleBarControl").is_none() {
        return;
    }
    // CreateTitleBarDragRegionsWindowClick and CreateTitleBarWindowClick.
    wire_show_button(root, "ShowTitleBarDragRegionsWindow", open_drag_regions_window);
    wire_show_button(root, "ShowTitleBarWindow", open_title_bar_window);
}

/// The window chrome both TitleBar windows share: Mica, the content in the title bar and the tall
/// title bar (`TitleBarHeightOption.Tall`, 48 pixels).
fn title_bar_window(xaml: &str, title: &str, opener: &Ref<Control>) -> Result<(Ref<Window>, Ref<Control>), XamlLoadError> {
    let (window, content) = create_window(xaml, title, opener)?;
    window.set_system_backdrop(Some(MicaBackdrop::new().upcast::<SystemBackdrop>()));
    window.set_extend_client_area_title_bar_height_hint(48.0);
    Ok((window, content))
}

/// `new TitleBarDragRegionsWindow()` and `Activate()`.
pub(crate) fn open_drag_regions_window(opener: &Ref<Control>) -> Result<Ref<Window>, XamlLoadError> {
    let (window, content) = title_bar_window(
        include_str!("Pages/TitleBarDragRegionsWindow.xaml"),
        "TitleBar drag regions sample",
        opener,
    )?;
    let title_bar = content.get_control::<TitleBar>("titleBar");
    let badge = content.get_control::<Button>("StatusBadge");
    let panel = content.get_control::<StackPanel>("RightHeaderPanel");
    let status = content.get_control::<TextBlock>("StatusText");
    // BadgeIsDragRegionRadios_SelectionChanged: unset (ClearValue), true or false.
    let radios = content.get_control::<RadioButtons>("BadgeIsDragRegionRadios");
    let weak_badge = badge.downgrade();
    on_selection(radios.upcast_ref::<SelectingItemsControl>(), move |index| {
        let Some(badge) = weak_badge.upgrade() else { return };
        let value = match index {
            0 => None,
            1 => Some(true),
            2 => Some(false),
            _ => return,
        };
        TitleBar::set_is_drag_region(&badge, value);
    });
    // StatusBadge_Click: only raised while the badge is not a drag region.
    let weak_status = status.downgrade();
    badge.click(move |_, _| {
        if let Some(status) = weak_status.upgrade() {
            status.set_text(Some("Status badge clicked"));
        }
    });
    // ToggleExtraButton_Click.
    let extra: Rc<RefCell<Option<Ref<Button>>>> = Rc::new(RefCell::new(None));
    let weak_panel = panel.downgrade();
    let weak_status = status.downgrade();
    content.get_control::<Button>("ToggleExtraButton").click(move |_, _| {
        let (Some(panel), Some(status)) = (weak_panel.upgrade(), weak_status.upgrade()) else { return };
        let mut extra = extra.borrow_mut();
        match extra.take() {
            None => {
                let button = Button::new();
                button.set_content(Some(Rc::new("Extra".to_string()) as BoxedValue));
                button.set_vertical_alignment(ferroui_base::layout::VerticalAlignment::Center);
                panel.children().insert(0, button.clone());
                *extra = Some(button);
                status.set_text(Some(
                    "Added a Button to TitleBar.Content. Call RecomputeDragRegions() to refresh drag regions.",
                ));
            }
            Some(button) => {
                panel.children().remove(button);
                status.set_text(Some("Removed the Button. Call RecomputeDragRegions() to refresh drag regions."));
            }
        }
    });
    // RecomputeDragRegions_Click.
    let weak_bar = title_bar.downgrade();
    let weak_status = status.downgrade();
    content.get_control::<Button>("RecomputeDragRegionsButton").click(move |_, _| {
        if let Some(bar) = weak_bar.upgrade() {
            bar.recompute_drag_regions();
        }
        if let Some(status) = weak_status.upgrade() {
            status.set_text(Some("RecomputeDragRegions() called."));
        }
    });
    show(&window);
    Ok(window)
}

/// `new TitleBarWindow()` and `Activate()`: the title bar's back button follows the frame's
/// CanGoBack (a Binding in the markup), its pane toggle button opens and closes the navigation
/// pane, and the navigation view's selection navigates the frame.
pub(crate) fn open_title_bar_window(opener: &Ref<Control>) -> Result<Ref<Window>, XamlLoadError> {
    let (window, content) = title_bar_window(include_str!("Pages/TitleBarWindow.xaml"), "TitleBarWindow", opener)?;
    let title_bar = content.get_control::<TitleBar>("titleBar");
    let nav = content.get_control::<NavigationView>("navView");
    let frame = content.get_control::<Frame>("navFrame");
    crate::navigation_view_samples::register_pages(&frame)
        .map_err(|error| XamlLoadError::with_message(error.to_string()))?;
    // TitleBar_PaneToggleRequested.
    let weak_nav = nav.downgrade();
    title_bar
        .pane_toggle_requested(move |_| {
            if let Some(nav) = weak_nav.upgrade() {
                nav.set_is_pane_open(!nav.is_pane_open());
            }
        })
        .detach();
    // TitleBar_BackRequested.
    let weak_frame = frame.downgrade();
    title_bar
        .back_requested(move |_| {
            if let Some(frame) = weak_frame.upgrade() {
                let _ = frame.go_back();
            }
        })
        .detach();
    // navView_SelectionChanged.
    let weak_nav = nav.downgrade();
    let weak_frame = frame.downgrade();
    nav.selection_changed(move |args| {
        let (Some(nav), Some(frame)) = (weak_nav.upgrade(), weak_frame.upgrade()) else { return };
        let Some(item) = args
            .selected_item()
            .as_ref()
            .and_then(Control::from_boxed)
            .and_then(|item| item.cast::<NavigationViewItem>())
        else {
            return;
        };
        let Some(tag) = item.tag().and_then(|tag| tag.downcast_ref::<String>().cloned()) else { return };
        nav.set_header(Some(Rc::new(format!("Sample Page {}", tag.chars().last().unwrap_or_default()))));
        if let Some(page) = crate::navigation_view_samples::page_type(&tag) {
            let _ = frame.navigate(page, None);
        }
    })
    .detach();
    // navView.SelectedItem = the first menu item.
    nav.set_selected_item(nav.menu_items().get_at(0));
    show(&window);
    Ok(window)
}
