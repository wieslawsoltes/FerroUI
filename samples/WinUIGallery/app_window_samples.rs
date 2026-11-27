//! The code behind of the AppWindow, AppWindowTitleBar and Windowing pages of the pinned WinUI
//! Gallery (`AppWindowPage.xaml.cs`, `AppWindowTitleBarPage.xaml.cs`, `WindowingPage.xaml.cs`) and
//! of their sample windows (`SampleSupport/SamplePages`) in Rust, over FerroUI's `Window`.
//!
//! The `AppWindow` of a WinUI window is the window itself: `Title`, `Resize` (the size),
//! `Move` (`Position`), `Hide` and `Show`, the overlapped presenter's `IsAlwaysOnTop` (`Topmost`),
//! `IsMaximizable`, `IsMinimizable`, `IsResizable` (`CanMaximize`, `CanMinimize`, `CanResize`),
//! `SetBorderAndTitleBar` (`WindowDecorations`), `Maximize`, `Minimize` and `Restore`
//! (`WindowState`), its preferred minimum and maximum sizes (`MinWidth` to `MaxHeight`),
//! `IsModal` with an owner (`ShowDialog`), the full screen presenter (`WindowState::FullScreen`)
//! and `DisplayArea` (the working area of the window's screen). The sizes of `AppWindow` are
//! physical pixels: a window takes them over the render scaling of the gallery's window.
//!
//! Opening a window needs a windowing platform: where the gallery runs in a single view (the
//! browser, iOS, Android) the page is not in a `Window`, and the examples say so instead of
//! opening one. The windows are drawn by the window's renderer like any other; nothing of these
//! pages leaves the GPU path of the backend.

use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{Color, IBrush, TextAlignment, TextWrapping};
use ferroui_base::styling::ThemeVariant;
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_base::{PixelPoint, PixelRect, Ref, Thickness, WeakRef};
use ferroui_controls::media::{MicaBackdrop, SystemBackdrop};
use ferroui_controls::primitives::{SelectingItemsControl, ToggleButton};
use ferroui_controls::{
    Border, Button, CheckBox, ComboBox, Control, InfoBar, ItemsSource, NumberBox, SplitButton,
    TextBlock, TextBox, TitleBar, ToggleSwitch, TopLevel, Window, WindowDecorations, WindowState,
};
use ferroui_controls_color_picker::ColorView;
use ferroui_markup_xaml::XamlLoadError;
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

/// The message of a page that is not in a window of a windowing platform.
pub(crate) const NO_WINDOWS: &str =
    "This platform shows the gallery in a single view: it has no windows to open.";

/// The delay of the source's `Task.Delay(3000)`.
pub(crate) const RESTORE_DELAY: Duration = Duration::from_secs(3);

/// The members of `CompactOverlaySize` with the share of the display's work area each one is, as
/// the source's `InitialSize_SelectionChanged` describes them.
pub(crate) const COMPACT_OVERLAY_SIZES: [(&str, &str, f64); 3] = [
    ("Small", "5%", 0.05),
    ("Medium", "15%", 0.15),
    ("Large", "25%", 0.25),
];

/// The members of `TitleBarHeightOption` with the title bar height hint each one is: Standard is
/// the platform's height, Tall the 48 pixels of WinUI and Collapsed no title bar.
pub(crate) const TITLE_BAR_HEIGHT_OPTIONS: [(&str, f64); 3] =
    [("Standard", -1.0), ("Tall", 48.0), ("Collapsed", 0.0)];

/// The members of `TitleBarTheme`, the source's `titleBarThemes`.
pub(crate) const TITLE_BAR_THEMES: [&str; 4] = ["Legacy", "UseDefaultAppMode", "Light", "Dark"];

/// The theme of a window whose title bar has the `TitleBarTheme` at `index`: Legacy is the light
/// title bar of a classic window, UseDefaultAppMode the application's theme.
pub(crate) fn title_bar_theme(index: i32) -> Option<ThemeVariant> {
    match index {
        0 | 2 => Some(ThemeVariant::light()),
        3 => Some(ThemeVariant::dark()),
        _ => None,
    }
}

/// The colour options of the AppWindowTitleBar page (the source's `ColorSelector`s), in the
/// source's order, with their initial colours.
pub(crate) const TITLE_BAR_COLORS: [(&str, u32); 12] = [
    ("Background", 0xFFF2F6FA),
    ("Foreground", 0xFF1E2933),
    ("ButtonBackground", 0xFF3B82F6),
    ("ButtonForeground", 0xFFFFFFFF),
    ("ButtonHoverBackground", 0xFF2563EB),
    ("ButtonHoverForeground", 0xFFFFFFFF),
    ("InactiveBackground", 0xFFE5EAF0),
    ("InactiveForeground", 0xFF6B7280),
    ("ButtonInactiveBackground", 0xFFCBD5E1),
    ("ButtonInactiveForeground", 0xFF475569),
    ("ButtonPressedBackground", 0xFF1D4ED8),
    ("ButtonPressedForeground", 0xFFFFFFFF),
];

pub(crate) fn wire(page_id: &str, root: &Ref<Control>) {
    match page_id {
        "AppWindow" => wire_app_window(root),
        "AppWindowTitleBar" => wire_app_window_title_bar(root),
        "Windowing" => wire_windowing(root),
        _ => {}
    }
}

// --- Shared ----------------------------------------------------------------------------------

fn solid(color: Color) -> Rc<dyn IBrush> {
    Rc::new(ImmutableSolidColorBrush::new(color))
}

/// The window the page is in, if it is in a window of a windowing platform (`App.MainWindow`).
pub(crate) fn owner_window(root: &Ref<Control>) -> Option<Ref<Window>> {
    TopLevel::get_top_level(Some(root)).and_then(|top_level| top_level.cast::<Window>())
}

/// The render scaling of `owner`: physical pixels per device-independent pixel.
fn scaling(owner: &Window) -> f64 {
    let scaling = owner.render_scaling();
    if scaling > 0.0 {
        scaling
    } else {
        1.0
    }
}

/// `pixels` physical pixels in the device-independent pixels of `owner`.
pub(crate) fn dips(pixels: f64, owner: &Window) -> f64 {
    pixels / scaling(owner)
}

/// The value of a NumberBox as the source's `(int)` cast: NaN is 0.
fn int_value(root: &Ref<Control>, name: &str) -> i32 {
    root.get_control::<NumberBox>(name).value() as i32
}

fn is_on(root: &Ref<Control>, name: &str) -> bool {
    root.get_control::<ToggleSwitch>(name).is_checked() == Some(true)
}

fn set_error(root: &Ref<Control>, message: Option<&str>) {
    if let Some(error) = root
        .find_control::<Control>("WindowSampleError")
        .and_then(|error| error.cast::<TextBlock>())
    {
        error.set_text(message);
        error.set_is_visible(message.is_some());
    }
}

/// Wires the button `name` of the page: `open` runs with the page's window (the owner of the
/// sample windows); a page outside a window, or markup that fails to load, is reported in the
/// page's `WindowSampleError`.
fn on_open(
    root: &Ref<Control>,
    name: &str,
    open: impl Fn(&Ref<Control>, &Ref<Window>) -> Result<(), XamlLoadError> + 'static,
) {
    let weak = root.downgrade();
    root.get_control::<Button>(name).click(move |_, _| {
        let Some(root) = weak.upgrade() else { return };
        let Some(owner) = owner_window(&root) else {
            set_error(&root, Some(NO_WINDOWS));
            return;
        };
        match open(&root, &owner) {
            Ok(()) => set_error(&root, None),
            Err(reason) => set_error(
                &root,
                Some(&format!("The sample window failed to load: {reason}")),
            ),
        }
    });
}

/// A sample window with `content`, the theme of `owner` and the source's `MicaBackdrop`.
fn sample_window(owner: &Window, content: Ref<Control>) -> Ref<Window> {
    let window = Window::new();
    window.set_requested_theme_variant(owner.requested_theme_variant());
    window.set_system_backdrop(Some(MicaBackdrop::new().upcast::<SystemBackdrop>()));
    window.set_content(Some(Control::boxed(content)));
    window
}

fn parse(xaml: &str) -> Result<Ref<Control>, XamlLoadError> {
    FerroRuntimeXamlLoader::parse_as::<Ref<Control>>(xaml, None)
}

/// The content of a sample window that only shows a text.
fn text_content(text: &str) -> Ref<Control> {
    let block = TextBlock::new();
    block.set_text(Some(text));
    block.set_text_wrapping(TextWrapping::Wrap);
    block.set_text_alignment(TextAlignment::Center);
    block.set_horizontal_alignment(HorizontalAlignment::Center);
    block.set_vertical_alignment(VerticalAlignment::Center);
    block.upcast::<Control>()
}

/// `window` with its size given in physical pixels (`AppWindow.Resize`): FerroUI sizes the client
/// area, in device-independent pixels.
fn resize(window: &Window, owner: &Window, width: i32, height: i32) {
    window.set_width(dips(f64::from(width), owner));
    window.set_height(dips(f64::from(height), owner));
}

/// The working area of the screen of `owner` (`DisplayArea.GetFromWindowId(..., Nearest)`): the
/// screen of the window, else the primary screen.
pub(crate) fn work_area(owner: &Window) -> Option<PixelRect> {
    let screens = TopLevel::screens(owner)?;
    screens
        .screen_from_window(owner)
        .or_else(|| screens.primary())
        .map(|screen| screen.working_area())
}

fn click(root: &Ref<Control>, name: &str, handler: impl Fn() + 'static) {
    root.get_control::<Button>(name)
        .click(move |_, _| handler());
}

/// `this.Close()` of a sample window's button.
fn close_on_click(content: &Ref<Control>, name: &str, window: &Ref<Window>) {
    let weak = window.downgrade();
    click(content, name, move || {
        if let Some(window) = weak.upgrade() {
            window.close();
        }
    });
}

/// `action` with `window` after the source's three seconds.
fn after_delay(window: &Ref<Window>, action: impl Fn(&Ref<Window>) + 'static) {
    let weak = window.downgrade();
    DispatcherTimer::run_once(
        move || {
            if let Some(window) = weak.upgrade() {
                action(&window);
            }
        },
        RESTORE_DELAY,
        DispatcherPriority::NORMAL,
    )
    .detach();
}

/// The Minimize, minimize-and-restore and Close buttons of SampleWindow3 and SampleWindow4.
fn wire_presenter_buttons(content: &Ref<Control>, window: &Ref<Window>) {
    let weak = window.downgrade();
    click(content, "MinimizeBtn", move || {
        if let Some(window) = weak.upgrade() {
            window.set_window_state(WindowState::Minimized);
        }
    });
    let weak = window.downgrade();
    click(content, "RestoreBtn", move || {
        if let Some(window) = weak.upgrade() {
            window.set_window_state(WindowState::Minimized);
            after_delay(&window, |window| {
                window.set_window_state(WindowState::Normal)
            });
        }
    });
    close_on_click(content, "CloseBtn", window);
}

/// Calls `handler` when `window`'s state changes (`SizeChanged` of the source, whose handler reads
/// the presenter's state).
fn on_window_state(window: &Ref<Window>, handler: impl Fn(&Ref<Window>) + 'static) {
    let weak = window.downgrade();
    window.property_changed(move |change| {
        if change.property() != Window::window_state_property().as_property() {
            return;
        }
        if let Some(window) = weak.upgrade() {
            handler(&window);
        }
    });
}

fn on_checked(toggle: &Ref<ToggleButton>, handler: impl Fn(bool) + 'static) {
    let weak = toggle.downgrade();
    toggle.property_changed(move |change| {
        if change.property() != ToggleButton::is_checked_property().as_property() {
            return;
        }
        if let Some(toggle) = weak.upgrade() {
            handler(toggle.is_checked() == Some(true));
        }
    });
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

// --- AppWindow -------------------------------------------------------------------------------

fn wire_app_window(root: &Ref<Control>) {
    on_open(root, "ShowSampleWindow1", |root, owner| {
        open_sample_window1(
            owner,
            &root
                .get_control::<TextBox>("WindowTitle")
                .text()
                .unwrap_or_default(),
            (
                int_value(root, "WindowWidth"),
                int_value(root, "WindowHeight"),
            ),
            PixelPoint::new(int_value(root, "XPoint"), int_value(root, "YPoint")),
        )
        .map(drop)
    });
    on_open(root, "ShowSampleWindow2", |_, owner| {
        open_sample_window2(owner);
        Ok(())
    });
    on_open(root, "ShowSampleWindow3", |root, owner| {
        let options = PresenterOptions {
            is_always_on_top: is_on(root, "IsAlwaysOnTop"),
            is_maximizable: is_on(root, "IsMaximizable"),
            is_minimizable: is_on(root, "IsMinimizable"),
            is_resizable: is_on(root, "IsResizable"),
            has_border: is_on(root, "HasBorder"),
            has_title_bar: is_on(root, "HasTitleBar"),
        };
        open_sample_window3(owner, options).map(drop)
    });
    on_open(root, "ShowSampleWindow4", |root, owner| {
        open_sample_window4(
            owner,
            (
                int_value(root, "MinWidthBox"),
                int_value(root, "MinHeightBox"),
            ),
            (
                int_value(root, "MaxWidthBox"),
                int_value(root, "MaxHeightBox"),
            ),
        )
        .map(drop)
    });
    on_open(root, "ShowSampleWindow5", |_, owner| {
        open_modal_window(owner).map(drop)
    });
    on_open(root, "ShowSampleWindow6", |_, owner| {
        open_sample_window6(owner).map(drop)
    });
    on_open(root, "ShowSampleWindow7", |root, owner| {
        let index = root.get_control::<ComboBox>("InitialSize").selected_index();
        open_sample_window7(owner, usize::try_from(index).unwrap_or(0));
        Ok(())
    });

    // HasBorder_Toggled and HasTitleBar_Toggled: a title bar needs a border.
    let has_border = root.get_control::<ToggleSwitch>("HasBorder");
    let has_title_bar = root.get_control::<ToggleSwitch>("HasTitleBar");
    let weak_title_bar = has_title_bar.downgrade();
    on_checked(has_border.upcast_ref::<ToggleButton>(), move |on| {
        if let (false, Some(title_bar)) = (on, weak_title_bar.upgrade()) {
            title_bar.set_is_checked(Some(false));
        }
    });
    let weak_border = has_border.downgrade();
    on_checked(has_title_bar.upcast_ref::<ToggleButton>(), move |on| {
        if let (true, Some(border)) = (on, weak_border.upgrade()) {
            border.set_is_checked(Some(true));
        }
    });

    // InitialSize_SelectionChanged.
    let weak = root.downgrade();
    on_selection(
        root.get_control::<ComboBox>("InitialSize")
            .upcast_ref::<SelectingItemsControl>(),
        move |index| {
            let Some(root) = weak.upgrade() else { return };
            root.get_control::<TextBlock>("InitialSizeDescription")
                .set_text(Some(&initial_size_description(index)));
        },
    );
}

/// The text of `InitialSize_SelectionChanged` for the size at `index`.
pub(crate) fn initial_size_description(index: i32) -> String {
    let (size, percentage) = usize::try_from(index)
        .ok()
        .and_then(|index| COMPACT_OVERLAY_SIZES.get(index))
        .map_or(("Unknown", "Unknown"), |(size, percentage, _)| {
            (*size, *percentage)
        });
    format!("{size}: Window size is approximately {percentage} of the display's work area.")
}

/// `new SampleWindow1(title, width, height, x, y)` and `Activate()`: the title, the size, the
/// position, and the Hide, Show and Close buttons. `SetTaskbarIcon` and `SetTitleBarIcon` are the
/// window's `Icon`, which the gallery leaves to the application (it embeds no icon file).
pub(crate) fn open_sample_window1(
    owner: &Ref<Window>,
    title: &str,
    (width, height): (i32, i32),
    position: PixelPoint,
) -> Result<Ref<Window>, XamlLoadError> {
    let content = parse(include_str!("Pages/AppWindowSampleWindow1.xaml"))?;
    let window = sample_window(owner, content.clone());
    window.set_title(Some(title.to_string()));
    resize(&window, owner, width, height);
    window.set_position(position);
    // Hide_Click.
    let weak = window.downgrade();
    click(&content, "Hide", move || {
        if let Some(window) = weak.upgrade() {
            window.hide();
        }
    });
    // Show_Click: hidden, then shown and activated after three seconds.
    let weak = window.downgrade();
    click(&content, "Show", move || {
        if let Some(window) = weak.upgrade() {
            window.hide();
            after_delay(&window, |window| {
                window.show();
                window.activate();
            });
        }
    });
    close_on_click(&content, "Close", &window);
    crate::window_samples::show(&window);
    Ok(window)
}

/// The position that centres a window of `size` physical pixels in `area` (`CenterWindow`). The
/// source's formula leaves out the origin of the work area, which is not zero on a secondary
/// screen or below a menu bar; it is added here.
pub(crate) fn centered(area: PixelRect, (width, height): (i32, i32)) -> PixelPoint {
    PixelPoint::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
    )
}

/// `new SampleWindow2()` and `Activate()`: the window centred in the work area of the gallery's
/// screen.
pub(crate) fn open_sample_window2(owner: &Ref<Window>) -> Ref<Window> {
    let content = text_content("This is a centred sample window");
    if let Some(block) = content.cast::<TextBlock>() {
        block.set_font_size(28.0);
    }
    let window = sample_window(owner, content);
    if let Some(area) = work_area(owner) {
        let size = window.client_size();
        let factor = scaling(owner);
        let physical = (
            (size.width * factor).round() as i32,
            (size.height * factor).round() as i32,
        );
        window.set_position(centered(area, physical));
    }
    crate::window_samples::show(&window);
    window
}

/// The options of the OverlappedPresenter example.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PresenterOptions {
    pub(crate) is_always_on_top: bool,
    pub(crate) is_maximizable: bool,
    pub(crate) is_minimizable: bool,
    pub(crate) is_resizable: bool,
    pub(crate) has_border: bool,
    pub(crate) has_title_bar: bool,
}

/// `SetBorderAndTitleBar(hasBorder, hasTitleBar)`: the decorations of the window. A title bar
/// without a border, which WinUI refuses, is the full decorations.
pub(crate) fn decorations(has_border: bool, has_title_bar: bool) -> WindowDecorations {
    match (has_border, has_title_bar) {
        (_, true) => WindowDecorations::Full,
        (true, false) => WindowDecorations::BorderOnly,
        (false, false) => WindowDecorations::None,
    }
}

/// `new SampleWindow3(...)` and `Activate()`: the presenter's options, and the Maximize/Restore,
/// Minimize, minimize-and-restore and Close buttons.
pub(crate) fn open_sample_window3(
    owner: &Ref<Window>,
    options: PresenterOptions,
) -> Result<Ref<Window>, XamlLoadError> {
    let content = parse(include_str!("Pages/AppWindowSampleWindow3.xaml"))?;
    let window = sample_window(owner, content.clone());
    window.set_topmost(options.is_always_on_top);
    window.set_can_maximize(options.is_maximizable);
    window.set_can_minimize(options.is_minimizable);
    window.set_can_resize(options.is_resizable);
    window.set_window_decorations(decorations(options.has_border, options.has_title_bar));
    // MaximizeRestoreBtn_Click.
    let weak = window.downgrade();
    click(&content, "MaximizeRestoreBtn", move || {
        if let Some(window) = weak.upgrade() {
            window.set_window_state(if window.window_state() == WindowState::Maximized {
                WindowState::Normal
            } else {
                WindowState::Maximized
            });
        }
    });
    // SampleWindow3_SizeChanged.
    let weak_button = content
        .get_control::<Button>("MaximizeRestoreBtn")
        .downgrade();
    on_window_state(&window, move |window| {
        if let Some(button) = weak_button.upgrade() {
            let text = if window.window_state() == WindowState::Maximized {
                "Restore"
            } else {
                "Maximize"
            };
            button.set_content(Some(Rc::new(text.to_string()) as ferroui_base::BoxedValue));
        }
    });
    wire_presenter_buttons(&content, &window);
    crate::window_samples::show(&window);
    Ok(window)
}

/// `new SampleWindow4(minWidth, minHeight, maxWidth, maxHeight)` and `Activate()`: an 800 by 500
/// window that cannot be maximized, with the preferred minimum and maximum sizes, and the
/// Minimize, minimize-and-restore and Close buttons (SampleWindow4 has no maximize button).
pub(crate) fn open_sample_window4(
    owner: &Ref<Window>,
    (min_width, min_height): (i32, i32),
    (max_width, max_height): (i32, i32),
) -> Result<Ref<Window>, XamlLoadError> {
    let content = parse(include_str!("Pages/AppWindowSampleWindow3.xaml"))?;
    content
        .get_control::<Button>("MaximizeRestoreBtn")
        .set_is_visible(false);
    let window = sample_window(owner, content.clone());
    resize(&window, owner, 800, 500);
    window.set_min_width(dips(f64::from(min_width), owner));
    window.set_min_height(dips(f64::from(min_height), owner));
    window.set_max_width(dips(f64::from(max_width), owner));
    window.set_max_height(dips(f64::from(max_height), owner));
    window.set_can_maximize(false);
    wire_presenter_buttons(&content, &window);
    crate::window_samples::show(&window);
    Ok(window)
}

/// `new ModalWindow()` and `Activate()`: a 400 by 300 dialog (`OverlappedPresenter.CreateForDialog`:
/// no resizing, minimize or maximize) owned by the gallery's window and modal to it, which is
/// activated again when the dialog closes. OK and Cancel close it.
pub(crate) fn open_modal_window(owner: &Ref<Window>) -> Result<Ref<Window>, XamlLoadError> {
    let content = parse(include_str!("Pages/AppWindowModalWindow.xaml"))?;
    let window = sample_window(owner, content.clone());
    window.set_width(400.0);
    window.set_height(300.0);
    window.set_can_resize(false);
    window.set_can_minimize(false);
    window.set_can_maximize(false);
    close_on_click(&content, "OKButton", &window);
    close_on_click(&content, "CancelButton", &window);
    // ModalWindow_Closed.
    let weak_owner = owner.downgrade();
    window
        .closed(move || {
            if let Some(owner) = weak_owner.upgrade() {
                owner.activate();
            }
        })
        .detach();
    crate::window_samples::track(&window);
    if owner.is_visible() {
        drop(window.show_dialog(owner));
    } else {
        // A dialog needs a visible owner: the window opens on its own.
        window.show();
    }
    Ok(window)
}

/// `new SampleWindow6()` and `Activate()`: the window in full screen, with its Close button.
pub(crate) fn open_sample_window6(owner: &Ref<Window>) -> Result<Ref<Window>, XamlLoadError> {
    let content = parse(include_str!("Pages/AppWindowSampleWindow6.xaml"))?;
    let window = sample_window(owner, content.clone());
    window.set_window_state(WindowState::FullScreen);
    close_on_click(&content, "Close", &window);
    crate::window_samples::show(&window);
    Ok(window)
}

/// The client size, in physical pixels, of a compact overlay window that covers `share` of `area`
/// with its aspect.
pub(crate) fn compact_overlay_size(area: PixelRect, share: f64) -> (f64, f64) {
    let factor = share.sqrt();
    (
        f64::from(area.width) * factor,
        f64::from(area.height) * factor,
    )
}

/// `new SampleWindow7(initialSize)` and `Activate()`: FerroUI has no compact overlay presenter;
/// the window is topmost, without minimize and maximize, and covers the share of the working
/// area of the size at `index` (the platform's default size where no screen is known).
pub(crate) fn open_sample_window7(owner: &Ref<Window>, index: usize) -> Ref<Window> {
    let content = text_content("This window is set to CompactOverlay (Picture-in-Picture) mode.");
    let window = sample_window(owner, content);
    window.set_topmost(true);
    window.set_can_minimize(false);
    window.set_can_maximize(false);
    let share = COMPACT_OVERLAY_SIZES
        .get(index)
        .map_or(COMPACT_OVERLAY_SIZES[0].2, |size| size.2);
    if let Some(area) = work_area(owner) {
        let (width, height) = compact_overlay_size(area, share);
        window.set_width(dips(width, owner));
        window.set_height(dips(height, owner));
    }
    crate::window_samples::show(&window);
    window
}

// --- AppWindowTitleBar -----------------------------------------------------------------------

/// The state of the AppWindowTitleBar page (the fields of `AppWindowTitleBarPage`).
pub(crate) struct TitleBarPage {
    root: WeakRef<Control>,
    colors: [Cell<Color>; 12],
    window: RefCell<Option<WeakRef<Window>>>,
    extend_window: RefCell<Option<WeakRef<Window>>>,
    theme_window: RefCell<Option<WeakRef<Window>>>,
}

impl TitleBarPage {
    /// The colour of the option `name` (one of [`TITLE_BAR_COLORS`]).
    pub(crate) fn color(&self, name: &str) -> Color {
        TITLE_BAR_COLORS
            .iter()
            .position(|(option, _)| *option == name)
            .map_or(Color::default(), |index| self.colors[index].get())
    }

    /// The colour window, while it is open.
    pub(crate) fn window(&self) -> Option<Ref<Window>> {
        self.window.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    /// The window of the ExtendsContentIntoTitleBar example, while it is open.
    pub(crate) fn extend_window(&self) -> Option<Ref<Window>> {
        self.extend_window
            .borrow()
            .as_ref()
            .and_then(WeakRef::upgrade)
    }

    /// The window of the PreferredTheme example, while it is open.
    pub(crate) fn theme_window(&self) -> Option<Ref<Window>> {
        self.theme_window
            .borrow()
            .as_ref()
            .and_then(WeakRef::upgrade)
    }

    /// The colours of the title bar of the colour window for its activation: BackgroundColor and
    /// ForegroundColor while it is active, InactiveBackgroundColor and InactiveForegroundColor
    /// while it is not. The source sets a colour only when it is not the default (transparent
    /// black), which leaves the previous one.
    fn apply(&self) {
        if let Some(window) = self.window() {
            self.apply_for(window.is_active());
        }
    }

    /// [`apply`](Self::apply) for the activation `active` (the Activated and Deactivated handlers
    /// run before the window's `IsActive` changes).
    fn apply_for(&self, active: bool) {
        let Some(window) = self.window() else { return };
        let Some(content) = window.content().as_ref().and_then(Control::from_boxed) else {
            return;
        };
        let title_bar = content.get_control::<TitleBar>("titleBar");
        let (background, foreground) = if active {
            (self.color("Background"), self.color("Foreground"))
        } else {
            (
                self.color("InactiveBackground"),
                self.color("InactiveForeground"),
            )
        };
        if background != Color::default() {
            title_bar.set_background(Some(solid(background)));
        }
        if foreground != Color::default() {
            title_bar.set_foreground(Some(solid(foreground)));
        }
    }
}

ferroui_base::ferro_thread_local! {
    /// The state of the AppWindowTitleBar pages that are alive (for the tests).
    static TITLE_BAR_PAGES: RefCell<Vec<std::rc::Weak<TitleBarPage>>> = const { RefCell::new(Vec::new()) };
}

/// The state of the AppWindowTitleBar page `root`.
#[cfg(test)]
pub(crate) fn title_bar_page(root: &Ref<Control>) -> Option<Rc<TitleBarPage>> {
    TITLE_BAR_PAGES.with(|pages| {
        pages
            .borrow()
            .iter()
            .filter_map(std::rc::Weak::upgrade)
            .find(|page| page.root.upgrade().as_ref() == Some(root))
    })
}

fn wire_app_window_title_bar(root: &Ref<Control>) {
    let page = Rc::new(TitleBarPage {
        root: root.downgrade(),
        colors: TITLE_BAR_COLORS.map(|(_, color)| Cell::new(Color::from_uint32(color))),
        window: RefCell::new(None),
        extend_window: RefCell::new(None),
        theme_window: RefCell::new(None),
    });
    TITLE_BAR_PAGES.with(|pages| {
        let mut pages = pages.borrow_mut();
        pages.retain(|page| page.strong_count() > 0);
        pages.push(Rc::downgrade(&page));
    });
    for (index, (name, _)) in TITLE_BAR_COLORS.iter().enumerate() {
        wire_color_selector(root, &page, index, name);
    }
    root.get_control::<ComboBox>("HeightComboBox")
        .set_items_source(Some(ItemsSource::from_values(
            TITLE_BAR_HEIGHT_OPTIONS
                .iter()
                .map(|(name, _)| (*name).to_string()),
        )));
    root.get_control::<ComboBox>("HeightComboBox")
        .set_selected_index(0);
    root.get_control::<ComboBox>("ThemeComboBox")
        .set_items_source(Some(ItemsSource::from_values(
            TITLE_BAR_THEMES.iter().map(|name| (*name).to_string()),
        )));
    root.get_control::<ComboBox>("ThemeComboBox")
        .set_selected_index(1);

    // ShowWindowButton_Click and Window_Closed.
    let state = page.clone();
    on_open(root, "ShowWindowButton", move |root, owner| {
        let window = open_title_bar_color_window(owner, &state)?;
        track_button(root, "ShowWindowButton", &window);
        Ok(())
    });
    // ShowExtendButton_Click and ExtendWindow_Closed.
    let state = page.clone();
    on_open(root, "ShowExtendButton", move |root, owner| {
        let extend = root
            .get_control::<CheckBox>("ExtendContentCheckBox")
            .is_checked()
            == Some(true);
        let height = root
            .get_control::<ComboBox>("HeightComboBox")
            .selected_index();
        let window = open_title_bar_extend_window(owner, extend, height);
        *state.extend_window.borrow_mut() = Some(window.downgrade());
        track_button(root, "ShowExtendButton", &window);
        Ok(())
    });
    // ShowThemeButton_Click and ThemeHeightWindow_Closed.
    let state = page.clone();
    on_open(root, "ShowThemeHeightButton", move |root, owner| {
        let theme = root
            .get_control::<ComboBox>("ThemeComboBox")
            .selected_index();
        let window = open_title_bar_theme_window(owner, theme);
        *state.theme_window.borrow_mut() = Some(window.downgrade());
        track_button(root, "ShowThemeHeightButton", &window);
        Ok(())
    });

    // ExtendContentCheckBox_Checked and ExtendContentCheckBox_Unchecked.
    let state = page.clone();
    on_checked(
        root.get_control::<CheckBox>("ExtendContentCheckBox")
            .upcast_ref::<ToggleButton>(),
        move |checked| {
            if let Some(window) = state.extend_window() {
                window.set_extend_client_area_to_decorations_hint(checked);
            }
        },
    );
    // HeightComboBox_SelectionChanged: only while the content extends into the title bar.
    let state = page.clone();
    on_selection(
        root.get_control::<ComboBox>("HeightComboBox")
            .upcast_ref::<SelectingItemsControl>(),
        move |index| {
            if let Some(window) = state
                .extend_window()
                .filter(|window| window.extend_client_area_to_decorations_hint())
            {
                window.set_extend_client_area_title_bar_height_hint(title_bar_height(index));
            }
        },
    );
    // ThemeComboBox_SelectionChanged.
    let state = page;
    on_selection(
        root.get_control::<ComboBox>("ThemeComboBox")
            .upcast_ref::<SelectingItemsControl>(),
        move |index| {
            if let Some(window) = state.theme_window() {
                window.set_requested_theme_variant(title_bar_theme(index));
            }
        },
    );
}

/// The title bar height hint of the `TitleBarHeightOption` at `index`.
pub(crate) fn title_bar_height(index: i32) -> f64 {
    usize::try_from(index)
        .ok()
        .and_then(|index| TITLE_BAR_HEIGHT_OPTIONS.get(index))
        .map_or(TITLE_BAR_HEIGHT_OPTIONS[0].1, |option| option.1)
}

/// The source's `Button.IsEnabled = false` while the window of the button is open, and the
/// `Closed` handler that enables it again.
fn track_button(root: &Ref<Control>, name: &str, window: &Ref<Window>) {
    let button = root.get_control::<Button>(name);
    button.set_is_enabled(false);
    let weak = button.downgrade();
    window
        .closed(move || {
            if let Some(button) = weak.upgrade() {
                button.set_is_enabled(true);
            }
        })
        .detach();
}

/// The gallery's ColorSelector `name`: its preview and its colour picker (`ColorPicker_ColorChanged`
/// and `UpdateVisual`), and the page's `*_ColorChanged` handler, which sets the colour of the open
/// window.
fn wire_color_selector(
    root: &Ref<Control>,
    page: &Rc<TitleBarPage>,
    index: usize,
    name: &'static str,
) {
    let picker = root.get_control::<ColorView>(&format!("{name}Picker"));
    picker.set_color(page.colors[index].get());
    let weak_root = root.downgrade();
    let state = page.clone();
    picker.color_changed(move |args| {
        let color = args.new_color();
        if state.colors[index].get() == color {
            return;
        }
        state.colors[index].set(color);
        if let Some(root) = weak_root.upgrade() {
            root.get_control::<Border>(&format!("{name}Preview"))
                .set_background(Some(solid(color)));
        }
        state.apply();
    });
    if let Some(flyout) = root.get_control::<SplitButton>(name).flyout() {
        let weak_root = root.downgrade();
        let state = page.clone();
        flyout
            .opened(move || {
                if let Some(root) = weak_root.upgrade() {
                    root.get_control::<ColorView>(&format!("{name}Picker"))
                        .set_color(state.colors[index].get());
                }
            })
            .detach();
    }
}

/// A sample window of the AppWindowTitleBar page: an always on top window of 600 by 400 pixels
/// that cannot be resized (the source's overlapped presenter).
fn title_bar_sample_window(owner: &Ref<Window>, content: Ref<Control>) -> Ref<Window> {
    let window = sample_window(owner, content);
    window.set_topmost(true);
    window.set_can_resize(false);
    resize(&window, owner, 600, 400);
    window
}

/// `new AppWindowTitleBarWindow(colors)` and `Activate()`: the window extends its content into the
/// title bar and its TitleBar control has the colours, for the active and the inactive window.
pub(crate) fn open_title_bar_color_window(
    owner: &Ref<Window>,
    page: &Rc<TitleBarPage>,
) -> Result<Ref<Window>, XamlLoadError> {
    let content = parse(include_str!("Pages/AppWindowTitleBarWindow.xaml"))?;
    let window = title_bar_sample_window(owner, content);
    window.set_title(Some("AppWindowTitleBarWindow".to_string()));
    window.set_extend_client_area_to_decorations_hint(true);
    *page.window.borrow_mut() = Some(window.downgrade());
    for active in [true, false] {
        let state = Rc::downgrade(page);
        let handler = move || {
            if let Some(state) = state.upgrade() {
                state.apply_for(active);
            }
        };
        if active {
            window.activated(handler).detach();
        } else {
            window.deactivated(handler).detach();
        }
    }
    page.apply();
    crate::window_samples::show(&window);
    Ok(window)
}

/// `new AppWindowTitleBarExtendWindow(extends, heightOption)` and `Activate()`. The caption
/// buttons' transparent background of the source is the drawn decorations' own.
pub(crate) fn open_title_bar_extend_window(
    owner: &Ref<Window>,
    extends: bool,
    height_option: i32,
) -> Ref<Window> {
    let content = text_content(
        "This is a sample window to demonstrate content extending into the title bar area and \
         title bar height options.",
    );
    content.set_margin(Thickness::new(20.0, 20.0, 20.0, 20.0));
    if let Some(block) = content.cast::<TextBlock>() {
        block.set_font_size(16.0);
    }
    let window = title_bar_sample_window(owner, content);
    window.set_title(Some("AppWindowTitleBarExtendWindow".to_string()));
    window.set_extend_client_area_to_decorations_hint(extends);
    if extends {
        window.set_extend_client_area_title_bar_height_hint(title_bar_height(height_option));
    }
    crate::window_samples::show(&window);
    window
}

/// `new AppWindowTitleBarThemeWindow(theme)` and `Activate()`: the title bar's theme is the
/// window's, which its content follows too.
pub(crate) fn open_title_bar_theme_window(owner: &Ref<Window>, theme: i32) -> Ref<Window> {
    let content = text_content(
        "This is a sample window to demonstrate AppWindowTitleBar theme customization.",
    );
    content.set_margin(Thickness::new(20.0, 20.0, 20.0, 20.0));
    if let Some(block) = content.cast::<TextBlock>() {
        block.set_font_size(16.0);
    }
    let window = title_bar_sample_window(owner, content);
    window.set_title(Some("AppWindowTitleBarThemeHeightWindow".to_string()));
    window.set_requested_theme_variant(title_bar_theme(theme));
    crate::window_samples::show(&window);
    window
}

// --- Windowing -------------------------------------------------------------------------------

/// The message of `OpenConfiguredWindow_Click` for the dimensions, or `None` when they are valid.
pub(crate) fn validate_dimensions(
    (width, height): (f64, f64),
    (min_width, min_height): (f64, f64),
    (max_width, max_height): (f64, f64),
) -> Option<&'static str> {
    let all = [width, height, min_width, min_height, max_width, max_height];
    if !all.iter().all(|value| value.is_finite()) {
        return Some("Enter a value for every dimension.");
    }
    if min_width > width || width > max_width || min_height > height || height > max_height {
        return Some("Width and Height must be within their minimum and maximum limits.");
    }
    None
}

fn wire_windowing(root: &Ref<Control>) {
    // CreateNewWindow_Click.
    on_open(root, "CreateWindowButton", |root, owner| {
        open_child_window(owner, root.actual_theme_variant());
        Ok(())
    });
    // OpenConfiguredWindow_Click.
    on_open(root, "OpenConfiguredWindowButton", |root, owner| {
        let value = |name: &str| root.get_control::<NumberBox>(name).value();
        let size = (value("ClientWidthInput"), value("ClientHeightInput"));
        let min = (value("MinimumWidthInput"), value("MinimumHeightInput"));
        let max = (value("MaximumWidthInput"), value("MaximumHeightInput"));
        let info_bar = root.get_control::<InfoBar>("SizeValidationInfoBar");
        if let Some(message) = validate_dimensions(size, min, max) {
            info_bar.set_message(message);
            info_bar.set_is_open(true);
            return Ok(());
        }
        info_bar.set_is_open(false);
        open_configured_window(owner, size, min, max);
        Ok(())
    });
}

/// `CreateNewWindow_Click`: a window extended into the title bar with Mica and a centred text in
/// the page's theme, with a client area of 500 by 500 physical pixels (`AppWindow.ResizeClient`).
pub(crate) fn open_child_window(owner: &Ref<Window>, theme: Option<ThemeVariant>) -> Ref<Window> {
    let window = sample_window(owner, text_content("New child window!"));
    window.set_extend_client_area_to_decorations_hint(true);
    window.set_requested_theme_variant(theme);
    window.set_width(dips(500.0, owner));
    window.set_height(dips(500.0, owner));
    crate::window_samples::show(&window);
    window
}

/// `CreateSampleWindow` and the rest of `OpenConfiguredWindow_Click`: a window with the client
/// size and the constraints in device-independent pixels, which cannot be maximized.
pub(crate) fn open_configured_window(
    owner: &Ref<Window>,
    (width, height): (f64, f64),
    (min_width, min_height): (f64, f64),
    (max_width, max_height): (f64, f64),
) -> Ref<Window> {
    let message = format!(
        "Initial client area: {width} by {height} DIPs. Width is constrained to {min_width} to \
         {max_width} DIPs and height to {min_height} to {max_height} DIPs. Try resizing this window."
    );
    let block = TextBlock::new();
    block.set_text(Some(&message));
    block.set_text_wrapping(TextWrapping::Wrap);
    block.set_margin(Thickness::new(24.0, 24.0, 24.0, 24.0));
    let window = sample_window(owner, block.upcast::<Control>());
    window.set_title(Some(
        "Window client size and constraints (experimental)".to_string(),
    ));
    window.set_min_width(min_width);
    window.set_min_height(min_height);
    window.set_max_width(max_width);
    window.set_max_height(max_height);
    window.set_width(width);
    window.set_height(height);
    window.set_can_maximize(false);
    crate::window_samples::show(&window);
    window
}
