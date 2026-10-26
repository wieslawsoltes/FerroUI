use super::support::start_application;
use crate::MainWindow;
use ferroui_base::media::FlowDirection;
use ferroui_controls::{Slider, TextBlock};
use ferroui_protranslate::TranslationBindingSource;
use ferroui_protranslate_core::Culture;

#[test]
fn compiled_window_updates_language_value_region_and_direction() {
    let _app = start_application();
    let window = MainWindow::new();
    let heading = window.find_control::<TextBlock>("Heading").unwrap();
    let amount = window.find_control::<TextBlock>("FormattedAmount").unwrap();
    assert_eq!(
        heading.text().as_deref(),
        Some("One application. Every language.")
    );
    assert_eq!(amount.text().as_deref(), Some("Amount: 1,234.50"));
    let source = TranslationBindingSource::default_source();
    source.set_culture(Culture::parse("pl-PL").unwrap());
    assert_eq!(
        heading.text().as_deref(),
        Some("Jedna aplikacja. Każdy język.")
    );
    window.find_control::<Slider>("Amount").unwrap().set_value(
        ferroui_controls::primitives::RangeBase::value_property(),
        12.25,
    );
    assert_eq!(amount.text().as_deref(), Some("Kwota: 12,25"));
    assert!(window
        .find_control::<TextBlock>("RegionDetails")
        .unwrap()
        .text()
        .unwrap()
        .contains("PL"));
    source.set_culture(Culture::parse("ar-SA").unwrap());
    assert_eq!(heading.flow_direction(), FlowDirection::RightToLeft);
    assert_eq!(heading.text().as_deref(), Some("تطبيق واحد. كل اللغات."));
}

#[test]
fn closing_window_releases_translation_consumers() {
    let _app = start_application();
    let source = TranslationBindingSource::default_source();
    let before = source.subscriber_count();
    let window = MainWindow::new();
    let weak = window.downgrade();
    assert!(source.subscriber_count() > before);
    window.show();
    window.close();
    drop(window);
    // Loaded/unloaded dispatcher jobs temporarily retain the native tree, just
    // as in FerroUI's own control lifetime tests. Drain them after closing.
    ferroui_base::threading::Dispatcher::ui_thread()
        .run_jobs(Some(ferroui_base::threading::DispatcherPriority::LOADED));
    ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
    assert!(weak.upgrade().is_none());
    assert_eq!(source.subscriber_count(), before);
}

#[test]
fn generated_strings_use_typed_compiled_binding_and_release_the_service_subscription() {
    use crate::generated::{GeneratedTranslationProvider, ProTranslateStrings};
    use ferroui_base::data::{CompiledBinding, CompiledBindingPathBuilder};
    use ferroui_protranslate_core::{CultureService, TranslationService};
    use std::rc::Rc;

    let _app = start_application();
    let cultures = CultureService::new(Culture::parse("en-US").unwrap());
    let service = TranslationService::new(Rc::new(GeneratedTranslationProvider), cultures.clone());
    let before = service.subscriber_count();
    let strings = Rc::new(ProTranslateStrings::new(service.clone()));
    let weak = Rc::downgrade(&strings);
    let target = TextBlock::new();
    let path = CompiledBindingPathBuilder::new()
        .typed_property::<ProTranslateStrings, Option<String>>(
            "Title",
            |source| Some(source.value_title().expect("the test catalog is validated")),
            None,
        )
        .build();
    let binding = CompiledBinding::new(path);
    binding.set_source(Some(strings.clone()));
    target.bind_binding(TextBlock::text_property(), &binding);
    assert_eq!(
        target.text().as_deref(),
        Some("One application. Every language.")
    );
    cultures.set_culture(Culture::parse("pl-PL").unwrap());
    assert_eq!(
        target.text().as_deref(),
        Some("Jedna aplikacja. Każdy język.")
    );
    drop(binding);
    drop(strings);
    drop(target);
    assert!(weak.upgrade().is_none());
    assert_eq!(service.subscriber_count(), before);
}
