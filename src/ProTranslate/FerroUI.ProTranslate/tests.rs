use super::*;
use ferroui_base::data::{BindingBase, BindingMode, IndexerBinding};
use ferroui_base::media::FlowDirection;
use ferroui_base::Ref;
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{Border, Button, Control, StackPanel, TextBlock, TextBox};
use ferroui_markup_xaml::FromXamlObject;
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use ferroui_protranslate_core::{
    Culture, CultureService, InMemoryTranslationProvider, TranslationService,
};
use std::cell::Cell;
use std::rc::Rc;

fn culture(name: &str) -> Culture {
    Culture::parse(name).unwrap()
}
fn source() -> TranslationBindingSource {
    let provider = Rc::new(InMemoryTranslationProvider::new("Test"));
    for (language, title) in [("en-US", "Hello"), ("pl-PL", "Cześć"), ("ar-SA", "مرحبا")] {
        provider.insert(culture(language), "Title", title);
        provider.insert(culture(language), "Price", "{0:N2}");
        provider.insert(culture(language), "Count", "Items: {0}");
    }
    TranslationBindingSource::new(TranslationService::new(
        provider,
        CultureService::new(culture("en-US")),
    ))
}
fn start() -> UnitTestApplicationScope {
    register_types();
    UnitTestApplication::start(TestServices::styled_window())
}

#[test]
fn native_binding_refreshes_and_disposes() {
    let _app = start();
    let source = source();
    let target = TextBlock::new();
    let binding = TranslateExtension::with_key("Title");
    binding.set_source(Some(source.clone()));
    let expression = target.bind_binding(TextBlock::text_property(), &binding);
    assert_eq!(target.text().as_deref(), Some("Hello"));
    source.set_culture(culture("pl-PL"));
    assert_eq!(target.text().as_deref(), Some("Cześć"));
    assert_eq!(source.subscriber_count(), 1);
    expression.dispose();
    assert_eq!(source.subscriber_count(), 0);
}

#[test]
fn binding_does_not_retain_target_or_leave_event_handlers() {
    let _app = start();
    let source = source();
    let target = TextBlock::new();
    let weak = target.downgrade();
    let binding = TranslateExtension::with_key("Title");
    binding.set_source(Some(source.clone()));
    target.bind_binding(TextBlock::text_property(), &binding);
    assert_eq!(source.subscriber_count(), 1);
    drop(target);
    assert!(weak.upgrade().is_none());
    assert_eq!(source.subscriber_count(), 0);
}

#[test]
fn attached_properties_refresh_text_and_flow_and_release_target() {
    let _app = start();
    let source = source();
    let target = TextBlock::new();
    let weak = target.downgrade();
    Translation::set_source(&target, Some(source.clone()));
    Translation::set_auto_flow_direction(&target, true);
    Translation::set_key(&target, Some("Title".into()));
    source.set_culture(culture("ar-SA"));
    assert_eq!(target.text().as_deref(), Some("مرحبا"));
    assert_eq!(target.flow_direction(), FlowDirection::RightToLeft);
    assert_eq!(source.subscriber_count(), 1);
    drop(target);
    assert!(weak.upgrade().is_none());
    assert_eq!(source.subscriber_count(), 0);
}

#[test]
fn attached_targets_include_editors_and_content_controls() {
    let _app = start();
    let source = source();
    let editor = TextBox::new();
    Translation::set_source(&editor, Some(source.clone()));
    Translation::set_key(&editor, Some("Title".into()));
    let button = Button::new();
    Translation::set_source(&button, Some(source.clone()));
    Translation::set_key(&button, Some("Title".into()));
    source.set_culture(culture("pl-PL"));
    assert_eq!(editor.text().as_deref(), Some("Cześć"));
    assert_eq!(
        button
            .content()
            .and_then(|value| value.downcast_ref::<String>().cloned())
            .as_deref(),
        Some("Cześć")
    );
}

#[test]
fn missing_key_fallback_and_string_format_update_in_place() {
    let _app = start();
    let source = source();
    let target = TextBlock::new();
    Translation::set_source(&target, Some(source));
    Translation::set_key(&target, Some("Missing".into()));
    Translation::set_fallback_text(&target, Some("Fallback".into()));
    Translation::set_string_format(&target, Some("[{0}]".into()));
    assert_eq!(target.text().as_deref(), Some("[Fallback]"));
    Translation::set_string_format(&target, Some("{5}".into()));
    // The core's default report-and-return-template policy is preserved.
    assert_eq!(target.text().as_deref(), Some("{5}"));
    Translation::set_string_format(&target, None);
    assert!(target.get_value(Translation::error_property()).is_none());
    assert_eq!(target.text().as_deref(), Some("Fallback"));
}

#[test]
fn reconfiguring_source_refreshes_existing_bindings_and_detaches_old_service() {
    let _app = start();
    let source = source();
    let old = source.service();
    let target = TextBlock::new();
    let binding = TranslateExtension::with_key("Title");
    binding.set_source(Some(source.clone()));
    target.bind_binding(TextBlock::text_property(), &binding);
    let new_source = self::source();
    new_source.set_culture(culture("pl-PL"));
    source.set_service(new_source.service());
    assert_eq!(target.text().as_deref(), Some("Cześć"));
    old.cultures().set_culture(culture("ar-SA"));
    assert_eq!(target.text().as_deref(), Some("Cześć"));
}

#[test]
fn formatted_binding_reacts_to_value_and_culture_with_identical_template() {
    let _app = start();
    let source = source();
    let input = Border::new();
    input.set_tag(Some(Rc::new(1234.5_f64)));
    let value_binding: Rc<dyn BindingBase> = Rc::new(IndexerBinding::new(
        input.clone().upcast(),
        Control::tag_property(),
        BindingMode::OneWay,
    ));
    let binding = FormatExtension::with_key("Price");
    binding.set_source(Some(source.clone()));
    binding.set_value(Some(Rc::new(value_binding)));
    let target = TextBlock::new();
    target.bind_binding(TextBlock::text_property(), &binding);
    assert_eq!(target.text().as_deref(), Some("1,234.50"));
    source.set_culture(culture("pl-PL"));
    let translated = target.text().unwrap();
    assert!(translated.ends_with("234,50"), "{translated}");
    input.set_tag(Some(Rc::new(12.25_f64)));
    assert_eq!(target.text().as_deref(), Some("12,25"));
}

#[test]
fn subscriptions_are_raii_and_allow_reentrancy() {
    let source = source();
    let count = Rc::new(Cell::new(0));
    let seen = count.clone();
    let reentrant = source.clone();
    let subscription = source.subscribe(move || {
        seen.set(seen.get() + 1);
        if seen.get() == 1 {
            reentrant.set_culture(culture("ar-SA"));
        }
    });
    source.set_culture(culture("pl-PL"));
    assert_eq!(count.get(), 2);
    drop(subscription);
    assert_eq!(source.subscriber_count(), 0);
}

fn load_text(xaml: &str) -> Ref<TextBlock> {
    let loaded = FerroRuntimeXamlLoader::load(xaml, None, None, None, false).unwrap();
    Ref::<TextBlock>::from_xaml_object(&loaded).unwrap()
}

#[test]
fn runtime_xaml_translation_extensions_and_aliases() {
    let _app = start();
    let source = source();
    Translation::set_service(source.service());
    for name in ["Translate", "T"] {
        let target = load_text(&format!(
            r#"<TextBlock xmlns="https://github.com/ferroui" xmlns:pt="https://github.com/protranslate/xaml" Text="{{pt:{name} Title}}" />"#
        ));
        assert_eq!(target.text().as_deref(), Some("Hello"));
        source.set_culture(culture("pl-PL"));
        assert_eq!(target.text().as_deref(), Some("Cześć"));
        source.set_culture(culture("en-US"));
    }
}

#[test]
fn runtime_xaml_format_extensions_and_aliases() {
    let _app = start();
    Translation::set_service(source().service());
    for name in ["Format", "F"] {
        let target = load_text(&format!(
            r#"<TextBlock xmlns="https://github.com/ferroui" xmlns:pt="https://github.com/protranslate/xaml" Text="{{pt:{name} Count, Value=3}}" />"#
        ));
        assert_eq!(target.text().as_deref(), Some("Items: 3"));
    }
}

#[test]
fn runtime_xaml_attached_culture_and_direction() {
    let _app = start();
    Translation::set_service(source().service());
    let target = load_text(
        r#"<TextBlock xmlns="https://github.com/ferroui" xmlns:pt="https://github.com/protranslate/xaml" pt:Translation.Key="Title" pt:Translation.Culture="ar-SA" pt:Translation.AutoFlowDirection="True" />"#,
    );
    assert_eq!(target.text().as_deref(), Some("مرحبا"));
    assert_eq!(target.flow_direction(), FlowDirection::RightToLeft);
}

#[test]
fn source_and_direction_inherit_through_native_tree() {
    let _app = start();
    let source = source();
    let parent = StackPanel::new();
    Translation::set_source(&parent, Some(source.clone()));
    Translation::set_auto_flow_direction(&parent, true);
    let target = TextBlock::new();
    Translation::set_key(&target, Some("Title".into()));
    parent.children().add(target.clone().upcast::<Control>());
    source.set_culture(culture("ar-SA"));
    assert_eq!(target.text().as_deref(), Some("مرحبا"));
    assert_eq!(target.flow_direction(), FlowDirection::RightToLeft);
}

#[test]
fn native_binding_reconnects_when_scoped_source_changes_after_binding() {
    let _app = start();
    let first = source();
    first.set_culture(culture("pl-PL"));
    let second = source();
    second.set_culture(culture("ar-SA"));
    let target = TextBlock::new();
    target.bind_binding(
        TextBlock::text_property(),
        &TranslateExtension::with_key("Title"),
    );
    let parent = StackPanel::new();
    Translation::set_source(&parent, Some(first.clone()));
    parent.children().add(target.clone().upcast::<Control>());
    assert_eq!(target.text().as_deref(), Some("Cześć"));
    assert_eq!(first.subscriber_count(), 1);
    Translation::set_source(&parent, Some(second.clone()));
    assert_eq!(target.text().as_deref(), Some("مرحبا"));
    assert_eq!(first.subscriber_count(), 0);
    assert_eq!(second.subscriber_count(), 1);
    first.set_culture(culture("en-US"));
    assert_eq!(target.text().as_deref(), Some("مرحبا"));
}

#[test]
fn native_bindings_convert_to_nullable_text_and_object_properties() {
    let _app = start();
    let source = source();
    let binding = TranslateExtension::with_key("Title");
    binding.set_source(Some(source.clone()));
    let editor = TextBox::new();
    editor.bind_binding(TextBox::text_property(), &binding);
    let button = Button::new();
    button.bind_binding(
        ferroui_controls::ContentControl::content_property(),
        &binding,
    );
    source.set_culture(culture("pl-PL"));
    assert_eq!(editor.text().as_deref(), Some("Cześć"));
    assert_eq!(
        button
            .content()
            .unwrap()
            .downcast_ref::<String>()
            .map(String::as_str),
        Some("Cześć")
    );
}

#[test]
fn native_binding_preserves_strict_format_diagnostics_and_typed_fallback() {
    use ferroui_protranslate_core::{ProTranslateDiagnosticKind, TranslationFormatFailureBehavior};
    let _app = start();
    let source = source();
    let service = source.service();
    let mut options = service.options();
    options.format_failure_behavior = TranslationFormatFailureBehavior::ReportAndReturnError;
    service.set_options(options);
    let count = Rc::new(Cell::new(0));
    let count_for_diagnostic = count.clone();
    let _diagnostics = service.subscribe_diagnostics(move |diagnostic| {
        if diagnostic.kind == ProTranslateDiagnosticKind::FormatFailure {
            count_for_diagnostic.set(count_for_diagnostic.get() + 1);
        }
    });
    let target = TextBlock::new();
    let binding = TranslateExtension::with_key("Title");
    binding.set_source(Some(source));
    binding.set_fallback_text(Some("Safe fallback".into()));
    binding.set_string_format(Some("{5}".into()));
    target.bind_binding(TextBlock::text_property(), &binding);
    assert_eq!(target.text().as_deref(), Some("Safe fallback"));
    assert_eq!(count.get(), 1);
}

#[test]
fn native_format_arguments_preserve_decimal_and_offset_date_types() {
    use crate::binding_source::boxed_argument;
    use ferroui_protranslate_core::{format_composite, FormatArgument};
    let value: ferroui_base::BoxedValue =
        Rc::new(rust_decimal::Decimal::from_str_exact("12345678901234567890.25").unwrap());
    let value = boxed_argument(Some(&value)).unwrap();
    assert_eq!(
        format_composite("{0:F2}", &[value], &culture("en-US")).unwrap(),
        "12345678901234567890.25"
    );
    let value: ferroui_base::BoxedValue =
        Rc::new(time::OffsetDateTime::from_unix_timestamp(0).unwrap());
    assert!(matches!(
        boxed_argument(Some(&value)).unwrap(),
        FormatArgument::DateTime(_)
    ));
}

#[test]
fn typed_observable_terminates_and_unsubscribes_on_strict_format_error() {
    use ferroui_base::reactive::AnonymousObserver;
    use ferroui_protranslate_core::TranslationFormatFailureBehavior;
    let provider = Rc::new(InMemoryTranslationProvider::new("Errors"));
    provider.insert(culture("en-US"), "Broken", "{3}");
    let service = TranslationService::new(provider, CultureService::new(culture("en-US")));
    let mut options = service.options();
    options.format_failure_behavior = TranslationFormatFailureBehavior::ReportAndReturnError;
    service.set_options(options);
    let source = TranslationBindingSource::new(service);
    let errors = Rc::new(Cell::new(0));
    let errors_seen = errors.clone();
    let _subscription = source
        .observe("Broken", vec!["value".into()])
        .subscribe(Rc::new(AnonymousObserver::new_with_error(
            |_| panic!("the format must fail"),
            move |_| errors_seen.set(errors_seen.get() + 1),
        )));
    assert_eq!(errors.get(), 1);
    assert_eq!(source.subscriber_count(), 0);
    source.refresh();
    assert_eq!(errors.get(), 1);
}

#[test]
fn object_fallback_preserves_content_identity_and_formats_numeric_text() {
    use ferroui_controls::ContentControl;
    let _app = start();
    let source = source();
    let fallback: ferroui_base::BoxedValue = Rc::new(vec![1_i32, 2, 3]);
    let attached = ContentControl::new();
    Translation::set_source(&attached, Some(source.clone()));
    Translation::set_fallback_value(&attached, Some(fallback.clone()));
    Translation::set_key(&attached, Some("Missing".into()));
    assert!(Rc::ptr_eq(&attached.content().unwrap(), &fallback));
    let bound = ContentControl::new();
    let binding = TranslateExtension::with_key("Missing");
    binding.set_source(Some(source.clone()));
    binding.set_fallback_value(Some(fallback.clone()));
    bound.bind_binding(ContentControl::content_property(), &binding);
    assert!(Rc::ptr_eq(&bound.content().unwrap(), &fallback));
    let text = TextBlock::new();
    Translation::set_source(&text, Some(source.clone()));
    Translation::set_fallback_value(&text, Some(Rc::new(42_i32)));
    Translation::set_key(&text, Some("Missing".into()));
    assert_eq!(text.text().as_deref(), Some("42"));
    Translation::set_string_format(&text, Some("{0:N1}".into()));
    source.set_culture(culture("pl-PL"));
    assert_eq!(text.text().as_deref(), Some("42,0"));
}
