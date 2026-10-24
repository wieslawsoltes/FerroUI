//! Running an application whose user interface is an element: the main
//! window, and windows opened later.

use crate::element::IntoElement;
use crate::reactive::Owner;
use ferroui_base::Ref;
use ferroui_controls::{AppBuilder, Application, Window, WindowBaseImpl};
use ferroui_desktop::AppBuilderDesktopExtensions;
use ferroui_base::styling::IStyle;
use ferroui_themes_fluent::FluentTheme;
use std::rc::Rc;

/// Mounts `view` as a window under an owner that is disposed when the window
/// closes. `view` builds a `window()`; any other control becomes the content
/// of a new window.
pub fn mount_window(view: impl FnOnce() -> crate::Element) -> Ref<Window> {
    let owner = Owner::new_root();
    let window = owner.with(|| {
        let control = view().into_element().mount();
        match control.cast::<Window>() {
            Some(window) => window,
            None => {
                let window = Window::new();
                window.set_content(Some(ferroui_controls::Control::boxed(&control)));
                window
            }
        }
    });
    let _closed = window.closed(move || owner.dispose());
    window
}

/// Opens the window of `view` (see [`mount_window`]) and shows it.
pub fn open_window(view: impl FnOnce() -> crate::Element) -> Ref<Window> {
    let window = mount_window(view);
    WindowBaseImpl::show(&*window);
    window
}

/// Runs an application whose main window is the element `view` makes: the
/// platform of the operating system, the Fluent theme, and the main loop until
/// the main window closes.
///
/// ```ignore
/// fn main() {
///     ferroui_view::launch(|| window().title("Counter").child(counter()).into());
/// }
/// ```
pub fn launch(view: impl FnOnce() -> crate::Element + 'static) {
    launch_with(view, |_| {});
}

/// [`launch`], calling `started` with the main window before the main loop
/// runs (to drive the window from code: a demonstration, screenshots).
pub fn launch_with(view: impl FnOnce() -> crate::Element + 'static, started: impl FnOnce(&Ref<Window>) + 'static) {
    launch_themed(|| FluentTheme::new().as_style(), view, started);
}

/// [`launch_with`] with the theme `theme` makes (the WinUI theme of
/// `ferroui-themes-winui`, the simple theme, a theme of the application)
/// instead of the Fluent theme.
pub fn launch_themed(
    theme: impl FnOnce() -> Rc<dyn IStyle> + 'static,
    view: impl FnOnce() -> crate::Element + 'static,
    started: impl FnOnce(&Ref<Window>) + 'static,
) {
    let args: Vec<String> = std::env::args().skip(1).collect();
    AppBuilder::configure::<Application>().use_platform_detect().start(
        move |app, _| {
            app.styles().add(theme());
            let window = mount_window(view);
            crate::hot::watch();
            #[cfg(debug_assertions)]
            crate::inspect::attach_inspector(&window);
            smoke_exit(&window);
            started(&window);
            app.run_window(window);
        },
        &args,
    );
}

/// With `FERROUI_SMOKE_EXIT_MS=<n>`, the main window closes after `n`
/// milliseconds: the smoke runs of the samples.
fn smoke_exit(window: &Ref<Window>) {
    if let Some(ms) = std::env::var("FERROUI_SMOKE_EXIT_MS").ok().and_then(|value| value.parse::<u64>().ok()) {
        let window = window.clone();
        let _timer = ferroui_base::threading::DispatcherTimer::run_once(
            move || window.close(),
            std::time::Duration::from_millis(ms),
            ferroui_base::threading::DispatcherPriority::NORMAL,
        );
    }
}
