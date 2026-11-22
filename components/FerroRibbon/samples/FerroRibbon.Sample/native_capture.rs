// Copyright (c) 2026 Wiesław Šoltés. MIT. Native services for original Capture.cs.
use crate::{
    capture::{CadCaptureAction, CaptureAction, CaptureError, CaptureRequest, CaptureSurface},
    pages::CadPage,
    shell_page::ShellPage,
};
use ferroribbon::{controls::*, theming::RibbonTheme};
use ferroribbon_core::{
    RibbonDensity, RibbonDisplayMode, RibbonGroupState, RibbonQuickAccessPosition,
    RibbonVisibilityMode,
    theming::{RibbonChromeStyle, RibbonThemeStyle},
};
use ferroui_base::{
    PixelSize, Point, Ref, Size, Vector, Visual,
    animation::SignedDuration,
    media::imaging::{Bitmap, BitmapEncoderOptions, RenderTargetBitmap, WriteableBitmap},
    platform::{AlphaFormat, PixelFormat},
    styling::ThemeVariant,
    threading::{Dispatcher, DispatcherPriority, DispatcherTimer},
};
use ferroui_controls::{ContentControl, Control, TextBox, TopLevel, Window, WindowResizeReason};
use std::{
    cell::RefCell,
    future::{Future, poll_fn},
    rc::Rc,
    sync::{Arc, Mutex},
    task::{Poll, Waker},
    time::Duration,
};
pub type CaptureEnvironment = Rc<dyn Fn(&str) -> Result<Option<String>, CaptureError>>;
pub struct NativeCaptureSurface {
    pub window: Ref<Window>,
    pub shell: Ref<ShellPage>,
    environment: CaptureEnvironment,
}
impl NativeCaptureSurface {
    pub fn new(window: Ref<Window>, shell: Ref<ShellPage>) -> Self {
        Self::with_environment(
            window,
            shell,
            Rc::new(|key| match std::env::var(key) {
                Ok(value) => Ok(Some(value)),
                Err(std::env::VarError::NotPresent) => Ok(None),
                Err(std::env::VarError::NotUnicode(_)) => {
                    Err(CaptureError(format!("{key} is not valid Unicode")))
                }
            }),
        )
    }
    pub fn with_environment(
        window: Ref<Window>,
        shell: Ref<ShellPage>,
        environment: CaptureEnvironment,
    ) -> Self {
        Self {
            window,
            shell,
            environment,
        }
    }
    fn content(&self) -> Result<Option<Ref<Control>>, CaptureError> {
        self.shell.current_content().map_err(error)
    }
}
fn error(e: impl std::fmt::Display) -> CaptureError {
    CaptureError(e.to_string())
}
fn native_call<T>(f: impl FnOnce() -> Result<T, CaptureError>) -> Result<T, CaptureError> {
    // The reference catches action/popup exceptions. Bound panicking native API
    // calls here and report them; never turn an unavailable action into success.
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).unwrap_or_else(|p| {
        Err(CaptureError(
            p.downcast_ref::<String>()
                .cloned()
                .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_else(|| "Native capture operation panicked".into()),
        ))
    })
}
/// Capture.cs fixes the child count once per recursive visit (ShellPage differs).
fn find<T: ferroui_base::ObjectType>(root: Option<Ref<Visual>>) -> Option<Ref<T>> {
    let root = root?;
    if let Some(value) = root.cast::<T>() {
        return Some(value);
    }
    let count = root.visual_children().count();
    for i in 0..count {
        if let Some(value) = find(Some(root.visual_children().get(i))) {
            return Some(value);
        }
    }
    None
}
fn search_box(
    surface: &NativeCaptureSurface,
) -> Result<Option<Ref<RibbonSearchBox>>, CaptureError> {
    Ok(find(surface.content()?.map(|c| c.upcast())))
}
pub(crate) fn capture_integer_with_format(
    value: Option<&str>,
    format: &ferroui_base::utilities::NumberFormatInfo,
) -> Option<i32> {
    use ferroui_system::invariant_numbers::{NumberParseFlags, try_parse_int};
    // Capture's Int32 grammar matches culture signs ordinally, unlike the
    // decimal service's NBSP/space aliases and negative-pattern spacing.
    let value = value?.trim_start_matches(|c| c == ' ' || ('\t'..='\r').contains(&c));
    let positive = format.positive_sign();
    let negative = format.negative_sign();
    let (sign, digits) = if let Some(digits) = (!positive.is_empty())
        .then(|| value.strip_prefix(positive))
        .flatten()
    {
        ('+', digits)
    } else if let Some(digits) = (!negative.is_empty())
        .then(|| value.strip_prefix(negative))
        .flatten()
    {
        ('-', digits)
    } else if matches!(negative, "‒" | "⁻" | "₋" | "−" | "➖" | "﹣" | "－")
        && let Some(digits) = value.strip_prefix('-')
    {
        ('-', digits)
    } else {
        ('+', value)
    };
    let mut normalized = String::with_capacity(digits.len() + 1);
    normalized.push(sign);
    normalized.push_str(digits);
    try_parse_int(&normalized, NumberParseFlags::INTEGER)
}
pub(crate) fn capture_integer(value: Option<&str>) -> Option<i32> {
    let value = value?;
    capture_integer_with_format(
        Some(value),
        &ferroui_base::utilities::CultureInfo::current_formatting().number_format(),
    )
}
impl CaptureSurface for NativeCaptureSurface {
    type Ribbon = Ref<Ribbon>;
    type Cad = Ref<CadPage>;
    fn environment(&self, key: &str) -> Result<Option<String>, CaptureError> {
        (self.environment)(key)
    }
    fn wait(&self, d: Duration) -> impl Future<Output = ()> {
        wait_for(d)
    }
    fn parse_integer(&self, value: Option<&str>) -> Option<i32> {
        capture_integer(value)
    }
    fn resize(&self, width: i32, height: i32) -> Result<(), CaptureError> {
        native_call(|| {
            let platform = self
                .window
                .platform_impl()
                .ok_or_else(|| CaptureError("Capture window is closed".into()))?;
            let scale = self.window.render_scaling();
            let client = platform.client_size();
            let insets = platform.frame_size().unwrap_or(client) - client;
            // Source AppWindow.Resize accepts physical outer-window dimensions.
            let physical = (
                (f64::from(width) * scale) as i32,
                (f64::from(height) * scale) as i32,
            );
            // Let the platform report the accepted client size to layout.
            platform.resize(
                Size::new(
                    f64::from(physical.0) / scale - insets.width,
                    f64::from(physical.1) / scale - insets.height,
                ),
                WindowResizeReason::Application,
            );
            Ok(())
        })
    }
    fn dark(&self) -> Result<(), CaptureError> {
        self.shell.set_value(
            ThemeVariant::requested_theme_variant_property(),
            Some(ThemeVariant::dark()),
        );
        Ok(())
    }
    fn show(&self, key: &str) -> Result<(), CaptureError> {
        self.shell.show(key).map_err(error)
    }
    fn find_ribbon(&self) -> Result<Option<Ref<Ribbon>>, CaptureError> {
        Ok(find(self.content()?.map(|c| c.upcast())))
    }
    fn current_cad(&self) -> Result<Option<Ref<CadPage>>, CaptureError> {
        Ok(self.content()?.and_then(|c| c.cast()))
    }
    fn action(&self, r: &Ref<Ribbon>, action: CaptureAction) -> Result<(), CaptureError> {
        native_call(|| {
            match action {
                CaptureAction::ShowKeyTips => r.show_key_tips(),
                CaptureAction::HideKeyTips => r.hide_key_tips(),
                CaptureAction::SelectTab(key) => {
                    r.select_tab_id(&key);
                }
                CaptureAction::ActivateHomeHeader => {
                    if let Some(header) = r.tab_headers().into_iter().find(|h| {
                        h.tab().id().or_else(|| h.tab().header()).as_deref() == Some("home")
                    }) {
                        r.activate_key_tip(&header);
                    }
                }
                CaptureAction::Backstage => r.set_is_backstage_open(true),
                CaptureAction::Simplified => r.set_display_mode(RibbonDisplayMode::Simplified),
                CaptureAction::Classic => r.set_display_mode(RibbonDisplayMode::Classic),
                CaptureAction::Overflow => {
                    if let Some(tab) = r.selected_tab() {
                        tab.show_overflow();
                    }
                }
                CaptureAction::TabsOnly => r.set_visibility_mode(RibbonVisibilityMode::TabsOnly),
                CaptureAction::OpenMinimized => r.open_minimized_popup(),
                CaptureAction::FullScreen => {
                    r.set_visibility_mode(RibbonVisibilityMode::FullScreen)
                }
                CaptureAction::Contextual => {
                    r.set_active_contextual_groups(&[Some("table"), Some("chart")])
                }
                CaptureAction::Gallery => {
                    let gallery = r
                        .find_item(Some("stylesGallery".into()))
                        .and_then(|c| c.cast::<RibbonGallery>())
                        .or_else(|| r.get_all_items().iter().find_map(|c| c.cast()));
                    if let Some(g) = gallery {
                        g.open_drop_down();
                    }
                }
                CaptureAction::ColorPicker => {
                    if let Some(c) = r
                        .find_item(Some("fontColor".into()))
                        .and_then(|c| c.cast::<RibbonColorPicker>())
                    {
                        c.open_drop_down();
                    }
                }
                CaptureAction::Paste => {
                    if let Some(c) = r
                        .find_item(Some("paste".into()))
                        .and_then(|c| c.cast::<RibbonSplitButton>())
                    {
                        c.open_drop_down();
                    }
                }
                CaptureAction::QatBelow => {
                    r.set_quick_access_position(RibbonQuickAccessPosition::BelowRibbon)
                }
                CaptureAction::SearchFocus => {
                    if let Some(c) = search_box(self)? {
                        c.focus_search();
                    }
                }
                CaptureAction::SearchText => {
                    if let Some(c) = search_box(self)?
                        && let Some(text) = find::<TextBox>(Some(c.upcast()))
                    {
                        text.set_text(Some("font col"));
                    }
                }
                CaptureAction::Palette => {
                    RibbonCommandPalette::show(r);
                }
                CaptureAction::CustomizeRibbon => {
                    r.show_customize_dialog(RibbonCustomizePage::Ribbon)
                        .map_err(error)?;
                }
                CaptureAction::CustomizeQat => {
                    r.show_customize_dialog(RibbonCustomizePage::QuickAccessToolbar)
                        .map_err(error)?;
                }
                CaptureAction::AddQat(id) => {
                    r.add_to_quick_access_id(Some(id.into()));
                }
                CaptureAction::CollapsedGroup => {
                    if let Some(tab) = r.selected_tab()
                        && let Some(group) = tab
                            .groups()
                            .iter()
                            .flatten()
                            .find(|g| g.state() == RibbonGroupState::Collapsed)
                    {
                        group.open_popup();
                    }
                }
                CaptureAction::Touch => r.set_density(RibbonDensity::Touch),
                CaptureAction::Compact => r.set_density(RibbonDensity::Compact),
                CaptureAction::Colorful => {
                    RibbonTheme::apply_chrome_style(RibbonChromeStyle::Colorful).map_err(error)?
                }
                CaptureAction::SurfaceCad => self
                    .shell
                    .settings()
                    .set_surface_style(RibbonThemeStyle::Cad)
                    .map_err(error)?,
            };
            Ok(())
        })
    }
    fn cad_action(
        &self,
        cad: &Ref<CadPage>,
        r: &Ref<Ribbon>,
        action: CadCaptureAction,
    ) -> Result<(), CaptureError> {
        native_call(|| {
            match action {
                CadCaptureAction::SlideOut => {
                    cad.draw_panel().set_is_slide_out_pinned(true);
                    cad.draw_panel().open_slide_out();
                }
                CadCaptureAction::ApplicationMenu => cad.show_application_menu(),
                CadCaptureAction::InvokeSaveAs => {
                    cad.application_menu().invoke(cad.save_as_menu_item())
                }
                CadCaptureAction::PanelButtons => {
                    r.set_visibility_mode(RibbonVisibilityMode::PanelButtons)
                }
                CadCaptureAction::DrawPopup => cad.draw_panel().open_popup(),
                CadCaptureAction::PanelTitles => {
                    r.set_visibility_mode(RibbonVisibilityMode::PanelTitles)
                }
                CadCaptureAction::LayersPopup => cad.layers_panel().open_popup(),
                CadCaptureAction::Float => cad.float_layers_panel(),
                CadCaptureAction::Tooltip => {
                    RibbonScreenTipService::set_extended_delay(SignedDuration::from_ticks(0));
                    cad.show_tool_tip(Some("line")).map_err(error)?;
                }
                CadCaptureAction::Layers => cad.layer_drop_down().open_drop_down(),
                CadCaptureAction::Contextual => cad.show_text_editor(),
                CadCaptureAction::Hatch => cad.show_hatch_creation(),
                CadCaptureAction::Light => cad.set_light_theme(true),
                CadCaptureAction::Command(command) => cad.run_command(&command),
                CadCaptureAction::Circle => {
                    if let Some(c) = r
                        .find_item(Some("circle".into()))
                        .and_then(|c| c.cast::<RibbonSplitButton>())
                    {
                        c.open_drop_down();
                    }
                }
                CadCaptureAction::Blocks => {
                    if let Some(c) = r
                        .find_item(Some("insertBlock".into()))
                        .and_then(|c| c.cast::<RibbonGallery>())
                    {
                        c.open_drop_down();
                    }
                }
                CadCaptureAction::View => {
                    r.select_tab_id("view");
                }
                CadCaptureAction::Workspace3d => cad.apply_workspace(Some("3D Modeling".into())),
            };
            Ok(())
        })
    }
    fn action_failed(&self, e: &CaptureError) {
        eprintln!("[capture] action failed: {e}");
    }
    fn save(&self, path: &str) -> impl Future<Output = Result<(), CaptureError>> {
        let pixels = capture_current(&self.window);
        let path = path.to_owned();
        async move {
            let frame = pixels?;
            let bitmap = Bitmap::from_pixels(
                PixelFormat::BGRA8888,
                AlphaFormat::Premul,
                &frame.pixels,
                frame.size,
                Vector::new(96., 96.),
                frame.size.width * 4,
            );
            let mut png = Vec::new();
            let encoded = bitmap
                .save(
                    &mut png,
                    &BitmapEncoderOptions::Png(
                        ferroui_base::media::imaging::PngBitmapEncoderOptions::default(),
                    ),
                )
                .map_err(error);
            bitmap.dispose();
            encoded?;
            write_file(path, png).await
        }
    }
}
/// Native dispatcher timer + Rust Future. Dropping it cancels the actual timer.
pub async fn wait_for(duration: Duration) {
    let state = Rc::new(RefCell::new((false, None::<Waker>)));
    let signal = state.clone();
    let _timer = DispatcherTimer::run_once(
        move || {
            let waker = {
                let mut s = signal.borrow_mut();
                s.0 = true;
                s.1.take()
            };
            if let Some(w) = waker {
                w.wake();
            }
        },
        duration,
        DispatcherPriority::NORMAL,
    );
    poll_fn(move |cx| {
        let mut s = state.borrow_mut();
        if s.0 {
            Poll::Ready(())
        } else {
            s.1 = Some(cx.waker().clone());
            Poll::Pending
        }
    })
    .await
}
async fn write_file(path: String, pixels: Vec<u8>) -> Result<(), CaptureError> {
    let state = Arc::new(Mutex::new((None, None::<Waker>)));
    let signal = state.clone();
    std::thread::spawn(move || {
        let result = std::fs::write(path, pixels);
        let waker = {
            let mut s = signal.lock().expect("capture IO state");
            s.0 = Some(result);
            s.1.take()
        };
        if let Some(w) = waker {
            w.wake();
        }
    });
    poll_fn(move |cx| {
        let mut s = state.lock().expect("capture IO state");
        if let Some(result) = s.0.take() {
            Poll::Ready(result.map_err(error))
        } else {
            s.1 = Some(cx.waker().clone());
            Poll::Pending
        }
    })
    .await
}
pub struct CaptureFrame {
    pub pixels: Vec<u8>,
    pub size: PixelSize,
}
struct RenderBitmap(RenderTargetBitmap);
impl Drop for RenderBitmap {
    fn drop(&mut self) {
        self.0.dispose();
    }
}
fn render_node(node: &Ref<Control>, scale: f64) -> Result<CaptureFrame, CaptureError> {
    native_call(|| {
        let size = PixelSize::from_size(node.bounds().size(), scale);
        if size.width < 1 || size.height < 1 {
            return Err(CaptureError("Visual has no positive capture area".into()));
        }
        let bitmap = RenderBitmap(RenderTargetBitmap::with_dpi(
            size,
            Vector::new(96. * scale, 96. * scale),
        ));
        bitmap.0.render(&node.clone().upcast());
        let buffer = WriteableBitmap::new(
            size,
            Vector::new(96. * scale, 96. * scale),
            Some(PixelFormat::BGRA8888),
            Some(AlphaFormat::Premul),
        );
        let framebuffer = buffer.lock();
        bitmap.0.copy_pixels_to_framebuffer(framebuffer.as_ref());
        framebuffer.dispose();
        let mut pixels = vec![0; (size.width as usize) * (size.height as usize) * 4];
        buffer.copy_pixels(
            ferroui_base::PixelRect::from_size(size),
            &mut pixels,
            size.width * 4,
        );
        buffer.dispose();
        Ok(CaptureFrame { pixels, size })
    })
}
fn opened_popups(window: &TopLevel) -> Vec<Ref<ferroui_controls::primitives::Popup>> {
    fn add(
        p: Ref<ferroui_controls::primitives::Popup>,
        out: &mut Vec<Ref<ferroui_controls::primitives::Popup>>,
    ) {
        if out.iter().any(|existing| existing == &p) {
            return;
        }
        out.push(p.clone());
        for child in p.opened_popups() {
            add(child, out);
        }
        // A separately hosted popup is itself a TopLevel. Popups anchored in
        // its content can be recorded there instead of on the parent Popup.
        if let Some(root) = p
            .host()
            .and_then(|h| h.hosted_visual_tree_root())
            .and_then(|r| r.cast::<TopLevel>())
        {
            for child in root.opened_popups() {
                add(child, out);
            }
        }
    }
    let mut result = Vec::new();
    for popup in window.opened_popups() {
        add(popup, &mut result);
    }
    // Per-parent lists cannot recover interleaved root/nested chronology.
    // Use the actual hosted opening order, preserving Uno's newest-first query.
    result.sort_by_key(|popup| std::cmp::Reverse(popup.opening_sequence()));
    result
}
pub fn capture_current(window: &Ref<Window>) -> Result<CaptureFrame, CaptureError> {
    let frame = crate::capture::compose(&NativeRenderer(window.clone()))?;
    Ok(CaptureFrame {
        pixels: frame.pixels,
        size: PixelSize::new(frame.width, frame.height),
    })
}
struct NativeRenderer(Ref<Window>);
impl crate::capture::CaptureRenderer for NativeRenderer {
    type Node = Ref<Control>;
    type Popup = Ref<ferroui_controls::primitives::Popup>;
    fn root(&self) -> Result<Self::Node, CaptureError> {
        self.0
            .content()
            .as_ref()
            .and_then(Control::from_boxed)
            .ok_or_else(|| CaptureError("Window content is not a visual".into()))
    }
    fn render(&self, node: &Self::Node) -> Result<crate::capture::BgraFrame, CaptureError> {
        let image = render_node(node, self.0.render_scaling())?;
        Ok(crate::capture::BgraFrame {
            pixels: image.pixels,
            width: image.size.width,
            height: image.size.height,
        })
    }
    fn actual_width(&self, node: &Self::Node) -> f64 {
        node.bounds().width
    }
    fn popups(&self) -> Vec<Self::Popup> {
        opened_popups(&self.0)
    }
    fn child(&self, p: &Self::Popup) -> Option<Self::Node> {
        p.child()
    }
    fn candidates(&self, child: &Self::Node) -> [Option<Self::Node>; 3] {
        [
            Some(child.clone()),
            (child.visual_children().count() > 0)
                .then(|| child.visual_children().get(0).cast::<Control>())
                .flatten(),
            child
                .cast::<ContentControl>()
                .and_then(|c| c.content().as_ref().and_then(Control::from_boxed)),
        ]
    }
    fn origin(&self, target: &Self::Node, root: &Self::Node) -> Result<(f64, f64), CaptureError> {
        crate::pages::point_in_page_root(target, root, Point::default())
            .map(|p| (p.x, p.y))
            .ok_or_else(|| CaptureError("Popup has no transform to capture root".into()))
    }
    fn rendered(&self, child: &Self::Node, w: i32, h: i32) {
        eprintln!(
            "[capture] popup {} rendered {w}x{h}",
            child.get_type().name()
        );
    }
    fn popup_failed(&self, error: &CaptureError) {
        eprintln!("[capture] popup render failed: {error}");
    }
}
/// Called only for the original shell entry point. The dispatcher owns the
/// future, which uses native timers/IO; no GUI is initialized by the query path.
pub fn start(window: Ref<Window>, shell: Ref<ShellPage>) -> Result<bool, CaptureError> {
    let surface = NativeCaptureSurface::new(window, shell);
    let Some(request) = CaptureRequest::read(&surface)? else {
        return Ok(false);
    };
    drop(Dispatcher::ui_thread().start_local_with_priority(
        async move {
            match request.run(&surface).await {
                Ok(()) => std::process::exit(0),
                Err(e) => eprintln!("[capture] capture failed: {e}"),
            }
        },
        DispatcherPriority::DEFAULT,
    ));
    Ok(true)
}

#[cfg(all(test, feature = "desktop"))]
#[path = "native_capture_tests.rs"]
mod tests;
