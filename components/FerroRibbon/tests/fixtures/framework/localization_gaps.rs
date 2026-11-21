//! Retained source-global and native-number regressions; normal consumer coverage.
use ferroribbon_core::{RibbonString, localization::*, model::RibbonItemsSource};
use ferroui_base::{
    data::converters::composite_format::{self, FormatArg},
    utilities::CultureInfo,
};
use std::sync::{Arc, Mutex};
fn isolated(name: &str) -> bool {
    if std::env::var("FERRORIBBON_LOCALE_REPRO").as_deref() == Ok(name) {
        return false;
    }
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name, "--nocapture"])
        .env("FERRORIBBON_LOCALE_REPRO", name)
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(
        text.contains("running 1 test\n") && output.status.success(),
        "{name}\n{text}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    true
}
#[test]
fn source_current_and_event_are_shared_between_threads() {
    if isolated("source_current_and_event_are_shared_between_threads") {
        return;
    }
    RibbonStrings::set_current(Some(
        RibbonStrings::for_culture(Some(RibbonCulture::new("en".into(), "en".into()))).unwrap(),
    ))
    .unwrap();
    let events = Arc::new(Mutex::new(Vec::new()));
    let seen = events.clone();
    let event = RibbonStrings::current_changed();
    let token = event.add(Arc::new(move |_| {
        seen.lock()
            .unwrap()
            .push(RibbonStrings::current().file().unwrap().as_utf16().to_vec())
    }));
    std::thread::spawn(|| {
        RibbonStrings::set_current(Some(
            RibbonStrings::for_culture(Some(RibbonCulture::new("fr".into(), "fr".into()))).unwrap(),
        ))
        .unwrap();
        RibbonStrings::current()
            .override_string(Some("File".into()), Some("Thread override".into()))
            .unwrap();
    })
    .join()
    .unwrap();
    event.remove(token);
    let source: serde_json::Value =
        serde_json::from_str(include_str!("../localization-registry.json")).unwrap();
    assert_eq!(
        serde_json::json!(RibbonStrings::current().file().unwrap().as_utf16()),
        source["cases"]["current"]["mainFile"],
        "Current must be shared process-wide"
    );
    assert_eq!(
        serde_json::json!(*events.lock().unwrap()),
        source["cases"]["current"]["events"]
    );
}
#[test]
fn source_process_wide_registration_is_visible_on_the_calling_thread() {
    if isolated("source_process_wide_registration_is_visible_on_the_calling_thread") {
        return;
    }
    let _ = RibbonStrings::current();
    std::thread::spawn(|| {
        RibbonStrings::register(
            Some("x-thread".into()),
            Some(RibbonItemsSource::from_values(vec![(
                Some("File".into()),
                Some("Background".into()),
            )])),
        )
        .unwrap()
    })
    .join()
    .unwrap();
    let strings =
        RibbonStrings::for_culture(Some(RibbonCulture::new("x-thread".into(), "x".into())))
            .unwrap();
    assert_eq!(
        strings.file(),
        Some(RibbonString::from("Background")),
        "Source registry is process-wide; background registration must be visible"
    );
}
#[test]
fn source_german_numeric_format_requires_actual_locale_data() {
    if isolated("source_german_numeric_format_requires_actual_locale_data") {
        return;
    }
    // Native library registration supplies the selected platform data service.
    ferroribbon::register_types();
    let result = composite_format::format_with(
        "{0} {1}",
        &[FormatArg::F64(1234.5), FormatArg::I32(-42)],
        &CultureInfo::get_culture_info("de-DE"),
    )
    .unwrap();
    assert_eq!(
        result, "1234,5 -42",
        "Source CultureInfo has locale data; native provider-less fallback is not parity"
    );
}

#[test]
fn source_language_selection_must_not_break_the_actual_table_picker() {
    if isolated("source_language_selection_must_not_break_the_actual_table_picker") {
        return;
    }
    use ferroui_controls::{AppBuilder, Application};
    use ferroui_headless::{
        FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, HeadlessUnitTestSession,
    };
    let session = HeadlessUnitTestSession::start_new(|| {
        AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
    });
    let result = session
        .dispatch(
            || {
                ferroribbon::register_types();
                RibbonStrings::set_current(Some(
                    RibbonStrings::for_culture(Some(
                        ferroribbon::localization::RibbonFrameworkCulture::new(
                            CultureInfo::get_culture_info("en"),
                        ),
                    ))
                    .unwrap(),
                ))
                .unwrap();
                let picker = ferroribbon::controls::RibbonGridPicker::new();
                // The actual source Settings language choice creates a named culture;
                // the actual picker formats columns/rows via Current.TablePickerFormat.
                picker.highlight(ferroribbon_core::model::RibbonGridSize::new(1, 2));
                assert_eq!(
                    picker.highlighted_size(),
                    ferroribbon_core::model::RibbonGridSize::new(1, 2)
                );
            },
            ferroui_base::threading::CancellationToken::none(),
        )
        .wait();
    session.dispose();
    result.unwrap();
}
