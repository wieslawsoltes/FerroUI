//! A measurement, not a check: the time the WinUI theme takes to load. The
//! theme is built by its compiled markup (`compiled_xaml.rs`): no document
//! is parsed, transformed or interpreted at start-up. The first load also
//! pays for what the process does once (the registration of the types);
//! the second load is the build alone.
//!
//! ```text
//! cargo test --release -p ferroui-themes-winui --lib tests::load_time -- --ignored --nocapture
//! ```

use std::time::Instant;

use super::support::start_application;
use crate::WinUITheme;

#[test]
#[ignore = "a measurement: prints the time the theme takes to load"]
fn measure_theme_load_time() {
    let _app = start_application();
    let start = Instant::now();
    let first = WinUITheme::new();
    let cold = start.elapsed();
    let start = Instant::now();
    let second = WinUITheme::new();
    let warm = start.elapsed();
    println!("WinUITheme: first load {cold:?}, second load {warm:?}");
    drop((first, second));
}

/// A measurement, not a check (#988): the start of the theme in an application and the first
/// templates of common controls, for the comparison of the compiled markup with the theme in Rust
/// (the feature `rust-theme`). Prints one line: the construction of the theme, the first window
/// with the controls (their control themes are built and their templates applied for the first
/// time) and a second window with the same controls.
///
/// ```text
/// cargo test --release -p ferroui-themes-winui --lib tests::load_time::measure_first_templates -- --ignored --nocapture
/// cargo test --release -p ferroui-themes-winui --features rust-theme --lib tests::load_time::measure_first_templates -- --ignored --nocapture
/// ```
#[test]
#[ignore = "a measurement: prints the start of the theme and the time of the first templates"]
fn measure_first_templates() {
    use ferroui_base::TypeInfo;
    use ferroui_controls::{Application, Control, StackPanel, Window};

    const CONTROLS: &[&str] = &[
        "Button", "CheckBox", "RadioButton", "ToggleSwitch", "TextBox", "ComboBox", "Slider", "ProgressBar", "ListBox",
        "Expander", "ToggleButton", "HyperlinkButton", "ScrollViewer", "TabControl", "CalendarDatePicker", "NumericUpDown",
    ];
    let _app = start_application();
    crate::register_types();
    let types: Vec<&'static TypeInfo> = CONTROLS
        .iter()
        .map(|name| {
            TypeInfo::registered_types()
                .into_iter()
                .find(|type_| type_.name() == *name && type_.module_path().starts_with("ferroui_controls"))
                .unwrap_or_else(|| panic!("{name}"))
        })
        .collect();
    let window_of_controls = || {
        let panel = StackPanel::new();
        for type_ in &types {
            let control = type_.create_instance().and_then(|instance| instance.cast::<Control>()).expect("a control");
            panel.children().add(control);
        }
        let window = Window::new();
        window.set_content(Some(Control::boxed(&panel.upcast::<Control>())));
        window
    };

    let start = Instant::now();
    let theme = WinUITheme::new();
    Application::current().expect("an application").styles().add(theme.as_style());
    let theme_time = start.elapsed();

    let first = window_of_controls();
    let start = Instant::now();
    first.show();
    let first_time = start.elapsed();

    let second = window_of_controls();
    let start = Instant::now();
    second.show();
    let second_time = start.elapsed();
    println!(
        "theme-start-us {} first-templates-us {} second-templates-us {}",
        theme_time.as_micros(),
        first_time.as_micros(),
        second_time.as_micros()
    );
}

/// A loop for a sampling profiler and for counted instructions (#994): the start of the theme, or
/// the start of the theme and the first templates of the controls of
/// [`measure_first_templates`], repeated with a new theme each time, so that every iteration
/// builds its control themes and applies its templates for the first time.
///
/// `FERROUI_PROFILE_PHASE` is `theme` (construct the theme and add it to the application) or
/// `templates` (the same, then show a window of the controls); `FERROUI_PROFILE_ITERATIONS` is
/// the number of iterations (200 by default). Prints the median of an iteration.
///
/// ```text
/// FERROUI_PROFILE_PHASE=theme cargo test --release -p ferroui-themes-winui --lib tests::load_time::profile_theme_start -- --ignored --nocapture
/// ```
#[test]
#[ignore = "a measurement: a loop of the start of the theme for a profiler"]
fn profile_theme_start() {
    use ferroui_base::TypeInfo;
    use ferroui_controls::{Application, Control, StackPanel, Window};

    const CONTROLS: &[&str] = &[
        "Button", "CheckBox", "RadioButton", "ToggleSwitch", "TextBox", "ComboBox", "Slider", "ProgressBar", "ListBox",
        "Expander", "ToggleButton", "HyperlinkButton", "ScrollViewer", "TabControl", "CalendarDatePicker", "NumericUpDown",
    ];
    let templates = std::env::var("FERROUI_PROFILE_PHASE").as_deref() == Ok("templates");
    let iterations: usize =
        std::env::var("FERROUI_PROFILE_ITERATIONS").ok().and_then(|text| text.parse().ok()).unwrap_or(200);
    let _app = start_application();
    crate::register_types();
    let types: Vec<&'static TypeInfo> = CONTROLS
        .iter()
        .map(|name| {
            TypeInfo::registered_types()
                .into_iter()
                .find(|type_| type_.name() == *name && type_.module_path().starts_with("ferroui_controls"))
                .unwrap_or_else(|| panic!("{name}"))
        })
        .collect();
    let application = Application::current().expect("an application");
    let mut times = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let start = Instant::now();
        let theme = WinUITheme::new();
        let style = theme.as_style();
        application.styles().add(style.clone());
        let window = templates.then(|| {
            let panel = StackPanel::new();
            for type_ in &types {
                let control = type_.create_instance().and_then(|instance| instance.cast::<Control>()).expect("a control");
                panel.children().add(control);
            }
            let window = Window::new();
            window.set_content(Some(Control::boxed(&panel.upcast::<Control>())));
            window.show();
            window
        });
        times.push(start.elapsed());
        if let Some(window) = window {
            window.close();
        }
        application.styles().remove(style);
        drop(theme);
    }
    times.sort();
    println!(
        "phase {} iterations {iterations} median-us {}",
        if templates { "templates" } else { "theme" },
        times[times.len() / 2].as_micros()
    );
}
