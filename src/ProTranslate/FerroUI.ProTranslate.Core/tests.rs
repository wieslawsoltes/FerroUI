use super::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
fn culture(name: &str) -> Culture {
    Culture::parse(name).unwrap()
}
fn setup() -> (
    Rc<InMemoryTranslationProvider>,
    CultureService,
    TranslationService,
) {
    let provider = Rc::new(InMemoryTranslationProvider::new("Memory"));
    let cultures = CultureService::new(culture("en-US"));
    let service = TranslationService::new(provider.clone(), cultures.clone());
    (provider, cultures, service)
}
#[test]
fn cultures_are_canonical_and_explicit() {
    assert_eq!(culture("EN-us").name(), "en-US");
    assert!(Culture::parse(" bad ").is_err());
    assert!(Culture::parse("en_US").is_err());
    assert!(Culture::parse("en--US").is_err());
    assert!(Culture::parse("").unwrap().is_invariant());
}
#[test]
fn cldr_resolves_likely_regions_and_script_direction() {
    assert_eq!(culture("pl").region_code().as_deref(), Some("PL"));
    assert_eq!(culture("zh-Hant").region_code().as_deref(), Some("TW"));
    assert_eq!(
        culture("ar").flow_direction(),
        TextFlowDirection::RightToLeft
    );
    assert_eq!(
        culture("az-Arab").flow_direction(),
        TextFlowDirection::RightToLeft
    );
    assert_eq!(
        culture("ar-Latn").flow_direction(),
        TextFlowDirection::LeftToRight
    );
    assert_eq!(Culture::invariant().region_code().as_deref(), Some("US"));
}
#[test]
fn parent_fallback_preserves_script_and_removes_extensions_first() {
    let (_, _, service) = setup();
    assert_eq!(
        service
            .fallback_cultures(&culture("zh-Hant-TW-u-nu-hanidec"))
            .iter()
            .map(Culture::name)
            .collect::<Vec<_>>(),
        ["zh-Hant-TW-u-nu-hanidec", "zh-Hant-TW", "zh-Hant", "zh", ""]
    );
}
#[test]
fn culture_switch_reports_both_snapshots_and_same_switch_is_noop() {
    let (_, cultures, _) = setup();
    let events = Rc::new(RefCell::new(Vec::new()));
    let seen = events.clone();
    let _subscription = cultures.subscribe(move |event| seen.borrow_mut().push(event.clone()));
    assert!(!cultures.set_culture(culture("en-US")));
    assert!(cultures.set_cultures(culture("pl-PL"), culture("ar-EG")));
    let events = events.borrow();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].old.culture.name(), "en-US");
    assert_eq!(events[0].new.culture.name(), "pl-PL");
    assert_eq!(events[0].new.ui_culture.name(), "ar-EG");
}
#[test]
fn event_delivery_is_fifo_under_reentrant_changes_and_disconnects() {
    let (_, cultures, _) = setup();
    let events = Rc::new(RefCell::new(Vec::new()));
    let inner = cultures.clone();
    let _first = cultures.subscribe(move |event| {
        if event.new.culture.name() == "pl" {
            inner.set_culture(culture("de"));
        }
    });
    let seen = events.clone();
    let second = cultures
        .subscribe(move |event| seen.borrow_mut().push(event.new.culture.name().to_string()));
    cultures.set_culture(culture("pl"));
    assert_eq!(*events.borrow(), ["pl", "de"]);
    drop(second);
    cultures.set_culture(culture("en"));
    assert_eq!(events.borrow().len(), 2);
}
#[test]
fn service_and_observable_subscriptions_do_not_leak() {
    let (provider, cultures, service) = setup();
    provider.insert(culture("en"), "Title", "Hello");
    assert_eq!(cultures.subscriber_count(), 1);
    {
        let _observer = service.observe("Title", vec![]);
        assert_eq!(service.subscriber_count(), 1);
    }
    assert_eq!(service.subscriber_count(), 0);
    drop(service);
    assert_eq!(cultures.subscriber_count(), 0);
}
#[test]
fn dropping_subscription_releases_captured_state_even_when_service_lives() {
    let (_, cultures, _) = setup();
    let captured = Rc::new(());
    let weak = Rc::downgrade(&captured);
    let subscription = cultures.subscribe(move |_| {
        let _ = &captured;
    });
    assert!(weak.upgrade().is_some());
    drop(subscription);
    assert!(weak.upgrade().is_none());
}
#[test]
fn ordered_fallback_is_exact_parent_explicit_default_and_deduplicated() {
    let (provider, _, service) = setup();
    let options = TranslationFallbackOptions {
        fallback_cultures: vec![culture("fr-CA"), culture("fr")],
        default_culture: culture("de-DE"),
        ..Default::default()
    };
    service.set_options(options);
    assert_eq!(
        service
            .fallback_cultures(&culture("pl-PL"))
            .iter()
            .map(Culture::name)
            .collect::<Vec<_>>(),
        ["pl-PL", "pl", "", "fr-CA", "fr", "de-DE", "de"]
    );
    provider.insert(culture("fr"), "Title", "Français");
    provider.insert(culture("de-DE"), "Title", "Deutsch");
    assert_eq!(
        service
            .get_string_for("Title", &culture("pl-PL"))
            .unwrap()
            .value,
        "Français"
    );
}
#[test]
fn empty_translation_wins_and_missing_is_structured() {
    let (provider, _, service) = setup();
    provider.insert(culture("en-US"), "Empty", "");
    provider.insert(culture("en"), "Empty", "wrong");
    let empty = service.get_string("Empty").unwrap();
    assert_eq!(empty.value, "");
    assert!(!empty.resource_not_found);
    let missing = service.get_string("Missing").unwrap();
    assert_eq!(missing.value, "Missing");
    assert!(missing.resource_not_found);
    assert_eq!(
        missing.diagnostics[0].kind,
        ProTranslateDiagnosticKind::MissingTranslation
    );
    let mut options = service.options();
    options.return_key_when_missing = false;
    service.set_options(options);
    assert_eq!(service.get_string("Missing").unwrap().value, "");
    assert!(service.get_string("  ").is_err());
}
#[test]
fn composite_preserves_failures_and_provider_priority() {
    let failed = Rc::new(CallbackTranslationProvider::new(
        "Failing",
        |_: &str, _: &Culture| Err(TranslationError::Provider("offline".into())),
    ));
    let first = Rc::new(InMemoryTranslationProvider::new("First"));
    first.insert(culture("en-US"), "Title", "First");
    let last = Rc::new(InMemoryTranslationProvider::new("Last"));
    last.insert(culture("en-US"), "Title", "Last");
    let composite = CompositeTranslationProvider::new(vec![failed, first, last]);
    let value = composite.get_string("Title", &culture("en-US")).unwrap();
    assert_eq!(value.value, "First");
    assert_eq!(value.provider_name, "First");
    assert_eq!(value.diagnostics.len(), 1);
    assert_eq!(
        value.diagnostics[0].provider_name.as_deref(),
        Some("Failing")
    );
}
#[test]
fn provider_error_policy_emits_before_returning_error() {
    let provider = Rc::new(CallbackTranslationProvider::new(
        "Failing",
        |_: &str, _: &Culture| Err(TranslationError::Provider("offline".into())),
    ));
    let service = TranslationService::with_options(
        provider,
        CultureService::new(culture("en")),
        TranslationFallbackOptions {
            provider_failure_behavior: TranslationProviderFailureBehavior::ReportAndReturnError,
            ..Default::default()
        },
        Default::default(),
    );
    let count = Rc::new(Cell::new(0));
    let seen = count.clone();
    let _subscription = service.subscribe_diagnostics(move |_| seen.set(seen.get() + 1));
    assert!(service.get_string("Title").is_err());
    assert_eq!(count.get(), 1);
}
#[test]
fn bounded_cache_mutation_policy_and_key_invalidation() {
    let (provider, cultures, service) = setup();
    service.set_cache_options(TranslationCacheOptions {
        maximum_entries: 2,
        ..Default::default()
    });
    for key in ["A", "B", "C"] {
        service.get_string(key).unwrap();
    }
    assert_eq!(service.cached_lookup_count(), 2);
    assert!(!service.remove_cached_string("A", &culture("en-US")));
    provider.insert(culture("en-US"), "B", "Found");
    assert_eq!(service.get_string("B").unwrap().value, "Found");
    assert_eq!(service.cached_lookup_count(), 1);
    assert!(service.remove_cached_string("B", &culture("en-US")));
    assert_eq!(service.cached_lookup_count(), 0);
    service.get_string("C").unwrap();
    cultures.set_culture(culture("pl"));
    assert_eq!(service.cached_lookup_count(), 0);
}
#[test]
fn missing_cache_can_be_disabled() {
    let (_, _, service) = setup();
    service.set_cache_options(TranslationCacheOptions {
        cache_missing_translations: false,
        ..Default::default()
    });
    service.get_string("Missing").unwrap();
    assert_eq!(service.cached_lookup_count(), 0);
}
#[test]
fn changing_policy_invalidates_cached_fallback() {
    let (provider, _, service) = setup();
    provider.insert(culture("en"), "A", "Parent");
    assert_eq!(service.get_string("A").unwrap().value, "Parent");
    let mut options = service.options();
    options.use_parent_cultures = false;
    options.use_default_culture = false;
    service.set_options(options);
    assert!(service.get_string("A").unwrap().resource_not_found);
}
#[test]
fn observations_refresh_on_culture_arguments_and_catalog_refresh() {
    let (provider, cultures, service) = setup();
    provider.insert(culture("en"), "Greeting", "Hello {0}");
    provider.insert(culture("pl"), "Greeting", "Cześć {0}");
    let observer = service.observe("Greeting", vec!["Ada".into()]);
    assert_eq!(observer.value().unwrap(), "Hello Ada");
    cultures.set_culture(culture("pl"));
    assert_eq!(observer.value().unwrap(), "Cześć Ada");
    observer.set_arguments(vec!["Ewa".into()]);
    assert_eq!(observer.value().unwrap(), "Cześć Ewa");
    provider.insert(culture("pl"), "Greeting", "Witaj {0}");
    service.refresh();
    assert_eq!(observer.value().unwrap(), "Witaj Ewa");
}
#[test]
fn format_uses_format_culture_and_ui_template_separately() {
    let (provider, cultures, service) = setup();
    provider.insert(culture("en"), "Amount", "Amount: {0:N2}");
    cultures.set_cultures(culture("pl-PL"), culture("en"));
    assert_eq!(
        service.format("Amount", &[1234.5.into()]).unwrap(),
        "Amount: 1234,50"
    );
}
#[test]
fn format_errors_are_diagnosed_and_policy_controlled() {
    let (provider, _, service) = setup();
    provider.insert(culture("en"), "Bad", "{2}");
    let diagnostics = Rc::new(RefCell::new(Vec::new()));
    let seen = diagnostics.clone();
    let _subscription = service.subscribe_diagnostics(move |d| seen.borrow_mut().push(d.kind));
    assert_eq!(service.format("Bad", &[1.into()]).unwrap(), "{2}");
    assert_eq!(
        *diagnostics.borrow(),
        [ProTranslateDiagnosticKind::FormatFailure]
    );
    let mut options = service.options();
    options.format_failure_behavior = TranslationFormatFailureBehavior::ReportAndReturnError;
    service.set_options(options);
    assert!(service.format("Bad", &[1.into()]).is_err());
    assert!(service
        .format_value("Outer", "{1}", &[1.into()], &culture("en"))
        .is_err());
}
#[test]
fn placeholder_parser_supports_unicode_escapes_alignment_and_exact_indexes() {
    assert_eq!(
        placeholder_indexes("你好 {{literal}} {3,-9:N2} {0} {3}").unwrap(),
        [0, 3]
    );
    assert_eq!(required_argument_count("{3}").unwrap(), 4);
    assert_eq!(
        format_composite(
            "{{{0,4}}} {1,-3}!",
            &["🙂".into(), "ą".into()],
            &culture("en")
        )
        .unwrap(),
        "{  🙂} ą  !"
    );
    for invalid in [
        "{",
        "}",
        "{x}",
        "{-1}",
        "{0,+2}",
        "{0:{1}}",
        "{0,999999999999}",
        "{999999999999999999999999}",
    ] {
        assert!(placeholder_indexes(invalid).is_err(), "{invalid}");
    }
}
#[test]
fn numeric_formats_preserve_large_integers_exact_decimals_and_rounding() {
    let en = culture("en-US");
    assert_eq!(
        FormatArgument::Unsigned(u64::MAX).format("G", &en).unwrap(),
        "18446744073709551615"
    );
    assert_eq!(
        FormatArgument::Integer(-23).format("D4", &en).unwrap(),
        "-0023"
    );
    assert_eq!(255u32.into_format("X4", &en), "00FF");
    assert_eq!(255u32.into_format("B", &en), "11111111");
    assert_eq!(
        FormatArgument::decimal("12345678901234567890.125")
            .unwrap()
            .format("N2", &en)
            .unwrap(),
        "12,345,678,901,234,567,890.13"
    );
    assert_eq!(1.235f64.into_format("F2", &en), "1.24");
    assert_eq!(1234.5f64.into_format("E2", &en), "1.23E+003");
}
trait IntoFormat {
    fn into_format(self, format: &str, culture: &Culture) -> String;
}
impl<T: Into<FormatArgument>> IntoFormat for T {
    fn into_format(self, format: &str, culture: &Culture) -> String {
        self.into().format(format, culture).unwrap()
    }
}
#[test]
fn icu_formats_nonlatin_digits_and_indian_grouping() {
    assert_eq!(1234567u32.into_format("N0", &culture("en-IN")), "12,34,567");
    assert_eq!(1000007u32.into_format("N0", &culture("bn")), "১০,০০,০০৭");
    assert_eq!(1234u32.into_format("N0", &culture("th-u-nu-thai")), "๑,๒๓๔");
}
#[test]
fn currency_percent_and_custom_fixed_patterns_use_cldr_affixes() {
    assert_eq!(12.5f64.into_format("C2", &culture("en-US")), "$12.50");
    assert_eq!((-12.5f64).into_format("C2", &culture("en-US")), "-$12.50");
    assert_eq!(12.5f64.into_format("C", &culture("ja-JP")), "￥12");
    assert_eq!(0.125f64.into_format("P1", &culture("de-DE")), "12,5 %");
    assert_eq!(
        1234.5f64.into_format("#,##0.00", &culture("en-US")),
        "1,234.50"
    );
    assert_eq!(
        (-12.5f64).into_format("0.00;(0.00)", &culture("en-US")),
        "(12.50)"
    );
}
#[test]
fn unsupported_numeric_patterns_return_errors() {
    for invalid in ["Q2", "N999999999", "F1025"] {
        assert!(
            FormatArgument::from(1.0f64)
                .format(invalid, &culture("en"))
                .is_err(),
            "{invalid}"
        );
    }
    assert!(FormatArgument::Number(1.0)
        .format("X", &culture("en"))
        .is_err());
}
#[test]
fn custom_typed_formatter_is_not_coerced_to_a_string() {
    struct Money;
    impl LocalizedFormat for Money {
        fn format(&self, format: &str, culture: &Culture) -> Result<String, TranslationError> {
            Ok(format!("{format}@{culture}"))
        }
    }
    let custom = FormatArgument::Custom(Rc::new(Money));
    assert_eq!(
        format_composite("{0:domain}", &[custom], &culture("pl")).unwrap(),
        "domain@pl"
    );
}
#[test]
fn region_profiles_include_native_names_currencies_and_overrides() {
    let pl = RegionProfile::for_culture(&culture("pl-PL")).unwrap();
    assert_eq!(pl.native_name, "Polska");
    assert_eq!(pl.iso_currency_symbol, "PLN");
    assert_eq!(pl.currency_symbol, "zł");
    assert!(pl.is_metric);
    let jp = RegionProfile::for_culture(&culture("ja")).unwrap();
    assert_eq!(jp.native_name, "日本");
    assert_eq!(jp.currency_decimal_digits, 0);
    let us = RegionProfile::for_region("US", &culture("en")).unwrap();
    assert!(!us.is_metric);
    assert_eq!(us.english_name, "United States");
}
#[test]
fn every_unit_roundtrips_within_its_category() {
    let converter = DefaultUnitConversionService;
    assert_eq!(MeasurementUnit::ALL.len(), 31);
    for &from in MeasurementUnit::ALL {
        for &to in MeasurementUnit::ALL {
            if from.category() == to.category() {
                let converted = converter.convert(12.345, from, to).unwrap();
                let back = converter.convert(converted, to, from).unwrap();
                assert!((back - 12.345).abs() < 1e-9, "{from:?} -> {to:?}");
            } else {
                assert_eq!(
                    converter.convert(1.0, from, to),
                    Err(TranslationError::IncompatibleUnits)
                );
            }
        }
    }
}
#[test]
fn conversions_preserve_us_imperial_and_temperature_distinctions() {
    let c = DefaultUnitConversionService;
    assert!(
        (c.convert(1.0, MeasurementUnit::USGallon, MeasurementUnit::Liter)
            .unwrap()
            - 3.785411784)
            .abs()
            < 1e-10
    );
    assert!(
        (c.convert(1.0, MeasurementUnit::ImperialGallon, MeasurementUnit::Liter)
            .unwrap()
            - 4.54609)
            .abs()
            < 1e-10
    );
    assert!(
        (c.convert(32.0, MeasurementUnit::Fahrenheit, MeasurementUnit::Kelvin)
            .unwrap()
            - 273.15)
            .abs()
            < 1e-10
    );
}
#[test]
fn measurement_profiles_follow_source_policy() {
    let resolver = DefaultMeasurementSystemResolver;
    for (code, expected) in [
        ("en-US", MeasurementSystem::USCustomary),
        ("en-GB", MeasurementSystem::Imperial),
        ("my-MM", MeasurementSystem::USCustomary),
        ("en-LR", MeasurementSystem::USCustomary),
        ("pl-PL", MeasurementSystem::Metric),
    ] {
        assert_eq!(
            resolver
                .resolve(&RegionProfile::for_culture(&culture(code)).unwrap(), None)
                .system,
            expected
        );
    }
    let custom = MeasurementSystemProfile::for_system(MeasurementSystem::Custom);
    assert!(DefaultUnitConversionService
        .convert_to_profile(1.0, MeasurementUnit::Meter, &custom)
        .is_err());
}
#[test]
fn globalization_metadata_changes_refresh_without_changing_culture() {
    let (_, cultures, translations) = setup();
    let globalization = GlobalizationService::new(translations);
    let count = Rc::new(Cell::new(0));
    let seen = count.clone();
    let _sub = globalization.subscribe(move |_| seen.set(seen.get() + 1));
    assert_eq!(
        globalization.measurement_system().unwrap(),
        MeasurementSystem::USCustomary
    );
    assert!(
        globalization.set_region_override(RegionProfile::for_culture(&culture("en-GB")).unwrap())
    );
    assert_eq!(
        globalization.measurement_system().unwrap(),
        MeasurementSystem::Imperial
    );
    assert_eq!(cultures.current_culture().name(), "en-US");
    assert!(globalization.set_measurement_system_override(MeasurementSystem::Metric));
    assert!(!globalization.set_measurement_system_override(MeasurementSystem::Metric));
    assert_eq!(
        globalization
            .format_measurement_for_profile(21.0, MeasurementUnit::Celsius, Some("F1"))
            .unwrap(),
        "21.0 °C"
    );
    globalization.clear_measurement_system_override();
    globalization.clear_region_override();
    assert_eq!(count.get(), 4);
    assert_eq!(
        globalization
            .format_measurement_for_profile(21.0, MeasurementUnit::Celsius, Some("F1"))
            .unwrap(),
        "69.8 °F"
    );
}
#[test]
fn diagnostic_reentrancy_does_not_cache_a_stale_resolution() {
    let (provider, _, service) = setup();
    let changed = Rc::new(Cell::new(false));
    let once = changed.clone();
    let svc = service.clone();
    let catalog = provider.clone();
    let _sub = service.subscribe_diagnostics(move |_| {
        if !once.replace(true) {
            catalog.insert(culture("en-US"), "Title", "Loaded");
            svc.refresh();
        }
    });
    service.get_string("Title").unwrap();
    assert_eq!(service.get_string("Title").unwrap().value, "Loaded");
}

fn date_value() -> LocalizedDateTime {
    let date = time::Date::from_calendar_date(2026, time::Month::October, 26).unwrap();
    let time = time::Time::from_hms_nano(15, 30, 45, 123_456_789).unwrap();
    LocalizedDateTime::new(
        date.with_time(time)
            .assume_offset(time::UtcOffset::from_hms(2, 0, 0).unwrap()),
    )
}
#[test]
fn typed_dates_use_localized_gregorian_names_and_patterns() {
    let value = date_value();
    assert_eq!(
        value.format("D", &culture("pl-PL")).unwrap(),
        "poniedziałek, 26 października 2026"
    );
    assert_eq!(
        value.format("D", &culture("en-US")).unwrap(),
        "Monday, October 26, 2026"
    );
    assert_eq!(value.format("d", &culture("de-DE")).unwrap(), "26.10.2026");
    assert_eq!(
        value.format("HH:mm:ss", &culture("en")).unwrap(),
        "15:30:45"
    );
    assert_eq!(
        value.format("dd MMMM yyyy", &culture("pl")).unwrap(),
        "26 października 2026"
    );
    assert_eq!(value.format("MMMM", &culture("pl")).unwrap(), "październik");
    assert_eq!(value.format("d", &culture("haw")).unwrap(), "26/10/2026");
}
#[test]
fn typed_date_standard_machine_formats_preserve_offset_or_explicitly_use_utc() {
    let value = date_value();
    assert_eq!(
        value.format("O", &culture("ar")).unwrap(),
        "2026-10-26T15:30:45.1234567+02:00"
    );
    assert_eq!(
        value.format("s", &culture("pl")).unwrap(),
        "2026-10-26T15:30:45"
    );
    assert_eq!(
        value.format("u", &culture("pl")).unwrap(),
        "2026-10-26 13:30:45Z"
    );
    assert_eq!(
        value.format("R", &culture("pl")).unwrap(),
        "Mon, 26 Oct 2026 13:30:45 GMT"
    );
    assert_eq!(
        value
            .format("yyyy-MM-dd 'at' hh:mm tt K fffffffff", &culture("en"))
            .unwrap(),
        "2026-10-26 at 03:30 PM +02:00 123456789"
    );
}
#[test]
fn date_values_flow_through_translation_and_observation() {
    let (provider, cultures, service) = setup();
    provider.insert(culture("en"), "Date", "Date: {0:D}");
    provider.insert(culture("pl"), "Date", "Data: {0:D}");
    let observed = service.observe("Date", vec![date_value().into()]);
    assert_eq!(observed.value().unwrap(), "Date: Monday, October 26, 2026");
    cultures.set_culture(culture("pl-PL"));
    assert_eq!(
        observed.value().unwrap(),
        "Data: poniedziałek, 26 października 2026"
    );
}
#[test]
fn every_bundled_gregorian_pattern_is_supported() {
    let date = date_value();
    for row in crate::date_data::DATES {
        let culture = culture(row.locale);
        for specifier in ["d", "D", "t", "T", "f", "F", "g", "G", "M", "Y"] {
            assert!(
                date.format(specifier, &culture).is_ok(),
                "{} {}: {:?}",
                row.locale,
                specifier,
                date.format(specifier, &culture)
            );
        }
    }
}
#[test]
fn invalid_date_formats_and_timestamps_are_recoverable_errors() {
    let date = date_value();
    for invalid in ["Q", "yyyy'", "HH\\", "fffffffffff", "%%", "%"] {
        assert!(date.format(invalid, &culture("en")).is_err(), "{invalid}");
    }
    assert!(LocalizedDateTime::from_unix_timestamp(i64::MAX, 0).is_err());
    assert!(LocalizedDateTime::from_unix_timestamp(0, i32::MAX).is_err());
}

#[test]
fn invariant_formats_are_deterministic_and_preserve_invariant_symbols() {
    let invariant = Culture::invariant();
    assert_eq!(12.5f64.into_format("C2", &invariant), "¤12.50");
    assert_eq!((-12.5f64).into_format("C2", &invariant), "(¤12.50)");
    assert_eq!(0.125f64.into_format("P2", &invariant), "12.50 %");
    assert_eq!(date_value().format("d", &invariant).unwrap(), "10/26/2026");
    assert_eq!(
        date_value().format("G", &invariant).unwrap(),
        "10/26/2026 15:30:45"
    );
}
#[test]
fn general_scientific_formats_trim_optional_zeros() {
    let culture = culture("en");
    assert_eq!(100000.0f64.into_format("G3", &culture), "1E+05");
    assert_eq!(0.000000123f64.into_format("G3", &culture), "1.23E-07");
    assert_eq!(1e100f64.into_format("G", &culture), "1E+100");
    assert_eq!(1e-100f64.into_format("G", &culture), "1E-100");
    assert_eq!(123.4f64.into_format("E3", &culture), "1.234E+002");
}
#[test]
fn optional_date_fraction_removes_the_decimal_separator() {
    let value = LocalizedDateTime::new(date_value().0.replace_nanosecond(0).unwrap());
    assert_eq!(
        value.format("HH:mm:ss.FFF", &culture("en")).unwrap(),
        "15:30:45"
    );
    assert_eq!(
        value.format("yyyy-MM-ddTHH:mm:ss", &culture("en")).unwrap(),
        "2026-10-26T15:30:45"
    );
}

#[test]
fn custom_numeric_quotes_escapes_percent_and_scaling_are_semantic() {
    let culture = culture("en");
    assert_eq!(12.5f64.into_format("0.0'%'", &culture), "12.5%");
    assert_eq!(12.5f64.into_format("0.0\\%", &culture), "12.5%");
    assert_eq!(0.125f64.into_format("0.0%", &culture), "12.5%");
    assert_eq!(0.125f64.into_format("0.0%%", &culture), "1250.0%%");
    assert_eq!(0.125f64.into_format("0.0‰", &culture), "125.0‰");
    assert_eq!(123456789.0f64.into_format("#,##0,,", &culture), "123");
    assert_eq!(123456789.0f64.into_format("#,##0,,.00", &culture), "123.46");
    assert_eq!(0.0f64.into_format("#;(#);zero", &culture), "zero");
    assert_eq!(
        (-0.0001f64).into_format("0.00;(0.00);zero", &culture),
        "zero"
    );
    assert_eq!(12.5f64.into_format("'USD;'0.0", &culture), "USD;12.5");
    assert_eq!(0.0f64.into_format("#.##", &culture), "");
    assert_eq!(0.125f64.into_format("#.##", &culture), ".13");
    for pattern in ["0'", "0\\"] {
        assert!(
            FormatArgument::Number(12.5)
                .format(pattern, &culture)
                .is_err(),
            "{pattern}"
        );
    }
}

#[test]
fn numeric_masks_and_rounding_match_reference_runtime_vectors() {
    // Generated independently with Microsoft.NETCore.App 10.0.14. These cover
    // source-runtime behaviors, not expectations derived from this formatter.
    for line in include_str!("test_data/numeric-runtime.tsv")
        .lines()
        .filter(|line| !line.starts_with('#'))
    {
        let fields: Vec<_> = line.split('\t').collect();
        assert_eq!(fields.len(), 4, "{line}");
        let value = match fields[0] {
            "double" => FormatArgument::from(fields[1].parse::<f64>().unwrap()),
            "float" => FormatArgument::from(fields[1].parse::<f32>().unwrap()),
            "decimal" => FormatArgument::decimal(fields[1]).unwrap(),
            _ => panic!("unrecognized oracle type"),
        };
        assert_eq!(
            value.format(fields[2], &Culture::invariant()).unwrap(),
            fields[3].strip_prefix('=').unwrap(),
            "type={} value={} format={}",
            fields[0],
            fields[1],
            fields[2]
        );
    }
}

#[test]
fn numeric_type_width_and_binary_precision_are_preserved() {
    let invariant = Culture::invariant();
    assert_eq!((-1i8).into_format("X", &invariant), "FF");
    assert_eq!((-1i16).into_format("X", &invariant), "FFFF");
    assert_eq!((-1i32).into_format("X", &invariant), "FFFFFFFF");
    assert_eq!((-1i64).into_format("X", &invariant), "FFFFFFFFFFFFFFFF");
    assert_eq!(
        (-1i128).into_format("X", &invariant),
        "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF"
    );
    assert_eq!((-1i8).into_format("B", &invariant), "11111111");
    assert_eq!(
        u128::MAX.into_format("G", &invariant),
        u128::MAX.to_string()
    );
    assert_eq!(0.1f32.into_format("G", &invariant), "0.1");
    assert_eq!(0.1f32.into_format("F10", &invariant), "0.1000000015");
    assert_eq!(
        0.1f64.into_format("F20", &invariant),
        "0.10000000000000000555"
    );
    assert_eq!(2.675f64.into_format("F2", &invariant), "2.67");
    assert_eq!(2.675f64.into_format("0.00", &invariant), "2.68");
    assert_eq!(1e9f32.into_format("G", &invariant), "1E+09");
    assert_eq!(1e9f64.into_format("G", &invariant), "1000000000");
}

#[test]
fn localized_numeric_symbols_and_default_precision_follow_locale_data() {
    assert_eq!(
        f64::INFINITY.into_format("G", &Culture::invariant()),
        "Infinity"
    );
    assert_eq!(
        f64::NEG_INFINITY.into_format("G", &Culture::invariant()),
        "-Infinity"
    );
    assert_eq!(f64::NAN.into_format("G", &culture("ru")), "не число");
    assert_eq!(12.5f64.into_format("F", &culture("en-US")), "12.500");
    assert_eq!(12.5f64.into_format("F", &Culture::invariant()), "12.50");
    assert_eq!(0.001f64.into_format("0.0‰", &culture("ar-EG")), "١٫٠؉");
    assert_eq!(
        0.001f64.into_format("0.0‰", &culture("ar-u-nu-arab")),
        "١٫٠؉"
    );
    assert_eq!(12f64.into_format("0E+0", &culture("sv")), "1E+1");
    assert_eq!(0.12f64.into_format("0E+0", &culture("sv")), "1E−1");
    assert_eq!(
        1234.5f64.into_format("00-00.0", &culture("ar-EG")),
        "١٢-٣٤٫٥"
    );
}

#[test]
fn typed_custom_format_arguments_compare_by_shared_identity() {
    struct Value;
    impl LocalizedFormat for Value {
        fn format(&self, _: &str, _: &Culture) -> Result<String, TranslationError> {
            Ok("value".into())
        }
    }
    let value = FormatArgument::Custom(Rc::new(Value));
    assert_eq!(value, value.clone());
    assert_ne!(value, FormatArgument::Custom(Rc::new(Value)));
    assert_ne!(FormatArgument::from(1i8), FormatArgument::from(1i64));
}
#[test]
fn initial_observer_does_not_miss_reentrant_catalog_or_culture_refresh() {
    let (provider, cultures, service) = setup();
    let catalog = provider.clone();
    let state = cultures.clone();
    let once = Rc::new(Cell::new(false));
    let changed = once.clone();
    let _sub = service.subscribe_diagnostics(move |_| {
        if !changed.replace(true) {
            catalog.insert(culture("pl"), "Title", "Wczytano");
            state.set_culture(culture("pl"));
        }
    });
    let observer = service.observe("Title", vec![]);
    assert_eq!(observer.value().unwrap(), "Wczytano");
}

#[test]
fn cldr_flexible_day_periods_follow_locale_time_boundaries() {
    let evening = LocalizedDateTime::new(date_value().0.replace_hour(21).unwrap());
    let morning = LocalizedDateTime::new(date_value().0.replace_hour(6).unwrap());
    assert_eq!(
        evening.format("t", &culture("zh-Hant")).unwrap(),
        "晚上9:30"
    );
    assert_eq!(
        morning.format("t", &culture("zh-Hant")).unwrap(),
        "清晨6:30"
    );
}
#[test]
fn region_failures_have_structured_diagnostics() {
    let (_, cultures, translations) = setup();
    let diagnostics = Rc::new(RefCell::new(Vec::new()));
    let seen = diagnostics.clone();
    let _sub = translations
        .subscribe_diagnostics(move |diagnostic| seen.borrow_mut().push(diagnostic.kind));
    let globalization = GlobalizationService::new(translations);
    cultures.set_culture(culture("en-XX"));
    assert!(globalization.region_profile().is_err());
    assert_eq!(
        *diagnostics.borrow(),
        [ProTranslateDiagnosticKind::RegionProfileFailure]
    );
}

#[test]
fn nested_argument_refresh_cannot_overwrite_a_newer_observed_value() {
    let (provider, _, service) = setup();
    provider.insert(culture("en"), "Title", "{0}");
    let observer = service.observe("Title", vec!["initial".into()]);
    struct UpdatingFormat {
        observer: ObservableLocalizedString,
        once: Cell<bool>,
    }
    impl LocalizedFormat for UpdatingFormat {
        fn format(&self, _: &str, _: &Culture) -> Result<String, TranslationError> {
            if !self.once.replace(true) {
                self.observer.set_arguments(vec!["new".into()]);
            }
            Ok("stale".into())
        }
    }
    observer.set_arguments(vec![FormatArgument::Custom(Rc::new(UpdatingFormat {
        observer: observer.clone(),
        once: Cell::new(false),
    }))]);
    assert_eq!(observer.value().unwrap(), "new");
}

#[test]
fn culture_region_metadata_follows_formatting_culture_and_name_setter_validates() {
    let cultures = CultureService::new(culture("en-US"));
    cultures.set_cultures(culture("pl"), culture("ar-SA"));
    assert_eq!(
        cultures
            .current_region()
            .unwrap()
            .two_letter_iso_region_name,
        "PL"
    );
    assert!(cultures.is_metric().unwrap());
    assert_eq!(cultures.flow_direction(), TextFlowDirection::RightToLeft);
    assert!(cultures.set_culture_name("en-US").unwrap());
    assert!(!cultures.is_metric().unwrap());
    assert!(!cultures.set_culture_name("EN-us").unwrap());
    for invalid in ["", " ", "bad_name"] {
        assert!(cultures.set_culture_name(invalid).is_err());
        assert_eq!(cultures.current_culture(), culture("en-US"));
    }
}

#[test]
fn provider_boundaries_validate_keys_names_and_atomic_catalog_batches() {
    let provider = InMemoryTranslationProvider::default();
    assert_eq!(provider.name(), "InMemory");
    assert!(matches!(
        InMemoryTranslationProvider::try_new(" "),
        Err(TranslationError::InvalidProviderName)
    ));
    assert_eq!(
        provider.try_insert(culture("en"), " ", "bad"),
        Err(TranslationError::InvalidKey)
    );
    assert!(provider.is_empty());
    let revision = provider.revision();
    assert_eq!(
        provider.insert_many(culture("en"), [("valid", "value"), ("", "bad")]),
        Err(TranslationError::InvalidKey)
    );
    assert_eq!(provider.revision(), revision);
    assert!(provider.is_empty());
    provider
        .insert_many(culture("en"), [("Title", "hello"), ("Empty", "")])
        .unwrap();
    assert_eq!(provider.len(), 2);
    assert!(
        !provider
            .get_string("Empty", &culture("en"))
            .unwrap()
            .resource_not_found
    );
    assert_eq!(
        provider.get_string(" \t", &culture("en")),
        Err(TranslationError::InvalidKey)
    );
    let callback = CallbackTranslationProvider::new("Host", |_: &str, _: &Culture| {
        panic!("invalid key must not reach a host callback")
    });
    assert_eq!(
        callback.get_string("", &culture("en")),
        Err(TranslationError::InvalidKey)
    );
    let composite = CompositeTranslationProvider::new(vec![]);
    assert_eq!(
        composite.get_string("", &culture("en")),
        Err(TranslationError::InvalidKey)
    );
    let (_, _, service) = setup();
    assert!(matches!(
        service.try_observe(" ", vec![]),
        Err(TranslationError::InvalidKey)
    ));
    assert_eq!(service.subscriber_count(), 0);
}

#[test]
fn composite_revision_changes_even_for_previously_colliding_child_tokens() {
    struct VersionedProvider {
        revision: Cell<u64>,
        value: RefCell<String>,
    }
    impl TranslationProvider for VersionedProvider {
        fn name(&self) -> &str {
            "Versioned"
        }
        fn revision(&self) -> u64 {
            self.revision.get()
        }
        fn get_string(
            &self,
            key: &str,
            culture: &Culture,
        ) -> Result<LocalizedString, TranslationError> {
            Ok(LocalizedString::found(
                key,
                self.value.borrow().as_str(),
                culture.clone(),
                self.name(),
            ))
        }
    }
    let first = Rc::new(VersionedProvider {
        revision: Cell::new(0),
        value: RefCell::new("old".into()),
    });
    let second = Rc::new(VersionedProvider {
        revision: Cell::new(0),
        value: RefCell::new("unused".into()),
    });
    let provider = Rc::new(CompositeTranslationProvider::new(vec![
        first.clone(),
        second.clone(),
    ]));
    let service = TranslationService::new(provider.clone(), CultureService::new(culture("en")));
    assert_eq!(service.get_string("Title").unwrap().value, "old");
    let before = provider.revision();
    let hash_seed = 0xcbf29ce484222325u64;
    let prime = 0x100000001b3u64;
    first.revision.set(1);
    second
        .revision
        .set((hash_seed ^ 1).wrapping_mul(prime) ^ hash_seed.wrapping_mul(prime));
    *first.value.borrow_mut() = "new".into();
    assert_ne!(provider.revision(), before);
    assert_eq!(service.get_string("Title").unwrap().value, "new");
}

#[test]
fn explicit_cache_invalidation_during_lookup_prevents_stale_publication() {
    for remove_one in [false, true] {
        let holder = Rc::new(RefCell::new(None::<TranslationService>));
        let handle = holder.clone();
        let value = Rc::new(RefCell::new("old".to_string()));
        let data = value.clone();
        let once = Cell::new(false);
        let provider = Rc::new(CallbackTranslationProvider::new(
            "Host",
            move |key: &str, culture: &Culture| {
                let resolved = data.borrow().clone();
                if !once.replace(true) {
                    *data.borrow_mut() = "new".into();
                    let service = handle.borrow().clone().unwrap();
                    if remove_one {
                        assert!(!service.remove_cached_string(key, culture));
                    } else {
                        service.clear_cache();
                    }
                }
                Ok(Some(resolved))
            },
        ));
        let service = TranslationService::new(provider, CultureService::new(culture("en")));
        *holder.borrow_mut() = Some(service.clone());
        assert_eq!(service.get_string("Title").unwrap().value, "old");
        assert_eq!(service.cached_lookup_count(), 0);
        assert_eq!(service.get_string("Title").unwrap().value, "new");
        assert_eq!(service.cached_lookup_count(), 1);
        holder.borrow_mut().take();
    }
}

#[test]
fn observable_exposes_resolved_metadata_and_notifies_for_equal_values() {
    let (provider, cultures, service) = setup();
    provider.insert(culture("en"), "Title", "same");
    provider.insert(culture("fr"), "Title", "same");
    let observer = service.observe("Title", vec![]);
    assert_eq!(observer.culture().unwrap(), culture("en"));
    assert_eq!(observer.localized_string().unwrap().provider_name, "Memory");
    let snapshots = Rc::new(RefCell::new(Vec::new()));
    let seen = snapshots.clone();
    let _sub =
        observer.subscribe_snapshot(move |snapshot| seen.borrow_mut().push(snapshot.clone()));
    let value_notifications = Rc::new(Cell::new(0));
    let count = value_notifications.clone();
    let _values = observer.subscribe(move |_| count.set(count.get() + 1));
    cultures.set_cultures(culture("pl-PL"), culture("fr-CA"));
    assert_eq!(observer.value().unwrap(), "same");
    assert_eq!(observer.culture().unwrap(), culture("fr"));
    let snapshot = observer.snapshot();
    assert_eq!(snapshot.cultures.culture, culture("pl-PL"));
    assert_eq!(snapshot.cultures.ui_culture, culture("fr-CA"));
    assert_eq!(snapshots.borrow().as_slice(), &[snapshot]);
    observer.update_arguments(vec![]).unwrap();
    assert_eq!(value_notifications.get(), 2);
    assert_eq!(snapshots.borrow().len(), 2);
}

#[test]
fn observable_format_errors_preserve_resolved_metadata_and_dispose_disconnects() {
    let (provider, cultures, service) = setup();
    provider.insert(culture("en"), "Title", "{2}");
    service.set_options(TranslationFallbackOptions {
        format_failure_behavior: TranslationFormatFailureBehavior::ReportAndReturnError,
        ..Default::default()
    });
    let observer = service.observe("Title", vec![1.into()]);
    assert!(observer.value().is_err());
    assert_eq!(observer.localized_string().unwrap().value, "{2}");
    assert_eq!(observer.culture().unwrap(), culture("en"));
    assert_eq!(service.subscriber_count(), 1);
    observer.dispose();
    observer.dispose();
    assert!(observer.clone().is_disposed());
    assert_eq!(service.subscriber_count(), 0);
    assert_eq!(
        observer.update_arguments(vec![]),
        Err(TranslationError::Disposed)
    );
    let frozen = observer.snapshot();
    cultures.set_culture(culture("fr"));
    observer.refresh();
    assert_eq!(observer.snapshot(), frozen);
}

#[test]
fn locale_calendars_match_source_runtime_date_components() {
    let date = date_value();
    // Microsoft.NETCore.App 10.0.14 oracle: 2026-10-26 in each default calendar.
    // Explicit latn only removes ICU's localized digit substitution from this
    // calendar-component comparison; the reference runtime emits Latin digits.
    for (locale, expected, short_year) in [
        ("ar-SA-u-nu-latn", "1448-05-15", "48"),
        ("th-TH", "2569-10-26", "69"),
        ("fa-IR-u-nu-latn", "1405-08-04", "5"),
    ] {
        assert_eq!(
            date.format("yyyy-MM-dd", &culture(locale)).unwrap(),
            expected,
            "{locale}"
        );
        assert_eq!(
            date.format("%y", &culture(locale)).unwrap(),
            short_year,
            "{locale}"
        );
        assert!(
            date.format("D", &culture(locale))
                .unwrap()
                .contains(&expected[..4]),
            "{locale}"
        );
        assert_eq!(
            date.format("O", &culture(locale)).unwrap(),
            "2026-10-26T15:30:45.1234567+02:00"
        );
    }
    assert_eq!(
        date.format("yyyy-MM-dd", &culture("ar-SA-u-ca-gregory-nu-latn"))
            .unwrap(),
        "2026-10-26"
    );
}

#[test]
fn explicit_calendar_era_transitions_and_leap_months_are_preserved() {
    let iso = |year, month, day| {
        LocalizedDateTime::new(
            time::Date::from_calendar_date(year, month, day)
                .unwrap()
                .midnight()
                .assume_utc(),
        )
    };
    let japanese = culture("ja-JP-u-ca-japanese");
    assert_eq!(
        iso(2019, time::Month::April, 30)
            .format("gg yy-MM-dd", &japanese)
            .unwrap(),
        "平成 31-04-30"
    );
    assert_eq!(
        iso(2019, time::Month::May, 1)
            .format("gg yy-MM-dd", &japanese)
            .unwrap(),
        "令和 01-05-01"
    );
    let hebrew = culture("en-u-ca-hebrew");
    assert_eq!(
        iso(2024, time::Month::March, 24)
            .format("yyyy-MM-dd", &hebrew)
            .unwrap(),
        "5784-07-14"
    );
    assert_eq!(
        iso(2024, time::Month::March, 24)
            .format("MMMM", &hebrew)
            .unwrap(),
        "Adar II"
    );
    assert_eq!(
        iso(2024, time::Month::February, 29)
            .format("yyyy-MM-dd", &culture("th-TH"))
            .unwrap(),
        "2567-02-29"
    );
    assert_eq!(
        iso(2025, time::Month::March, 21)
            .format("yyyy-MM-dd", &culture("fa-IR-u-nu-latn"))
            .unwrap(),
        "1404-01-01"
    );
    assert_eq!(
        iso(2024, time::Month::February, 29)
            .format("yyyy-MM-dd", &culture("en-u-ca-roc"))
            .unwrap(),
        "0113-02-29"
    );
}

#[test]
fn every_supported_calendar_has_localized_standard_and_custom_date_fields() {
    for calendar in [
        "buddhist",
        "chinese",
        "coptic",
        "dangi",
        "ethiopic",
        "ethioaa",
        "gregory",
        "hebrew",
        "indian",
        "islamic-civil",
        "islamic-tbla",
        "islamic-umalqura",
        "iso8601",
        "japanese",
        "persian",
        "roc",
    ] {
        let locale = culture(&format!("en-u-ca-{calendar}"));
        for specifier in ["d", "D", "M", "Y", "yyyy-MM-dd", "dd MMMM yyyy", "MMMM"] {
            assert!(
                !date_value().format(specifier, &locale).unwrap().is_empty(),
                "{calendar} {specifier}"
            );
        }
    }
    assert!(date_value()
        .format("yyyy", &culture("en-u-ca-unknown"))
        .is_err());
}

#[test]
fn offset_and_naive_date_values_preserve_their_zone_semantics() {
    let offset = date_value();
    let naive_value = time::PrimitiveDateTime::new(offset.0.date(), offset.0.time());
    let naive = LocalizedNaiveDateTime::new(naive_value);
    let utc = LocalizedNaiveDateTime::utc(naive_value);
    let invariant = Culture::invariant();
    assert_eq!(
        offset.format("", &invariant).unwrap(),
        "10/26/2026 15:30:45 +02:00"
    );
    assert_eq!(
        offset.format("G", &invariant).unwrap(),
        "10/26/2026 15:30:45"
    );
    assert_eq!(naive.format("", &invariant).unwrap(), "10/26/2026 15:30:45");
    assert_eq!(
        naive.format("O", &invariant).unwrap(),
        "2026-10-26T15:30:45.1234567"
    );
    assert_eq!(
        utc.format("O", &invariant).unwrap(),
        "2026-10-26T15:30:45.1234567Z"
    );
    assert_eq!(offset.format("%K", &invariant).unwrap(), "+02:00");
    assert_eq!(
        LocalizedDateTime(offset.0.to_offset(time::UtcOffset::UTC))
            .format("%K", &invariant)
            .unwrap(),
        "+00:00"
    );
    assert_eq!(naive.format("%K", &invariant).unwrap(), "");
    assert_eq!(utc.format("%K", &invariant).unwrap(), "Z");
    assert_eq!(utc.format("zzz", &invariant).unwrap(), "+00:00");
    assert!(naive.format("zzz", &invariant).is_err());
    assert!(naive.format("U", &invariant).is_err());
    assert!(offset.format("U", &invariant).is_err());
    assert_eq!(
        utc.format("U", &invariant).unwrap(),
        "Monday, 26 October 2026 15:30:45"
    );
    assert_eq!(
        FormatArgument::from(naive_value)
            .format("O", &invariant)
            .unwrap(),
        naive.format("O", &invariant).unwrap()
    );
}

#[test]
fn custom_date_tokens_and_standard_joins_follow_source_rules() {
    let date = date_value();
    let en = culture("en-US");
    assert_eq!(
        date.format("f", &en).unwrap(),
        "Monday, October 26, 2026 3:30\u{202f}PM"
    );
    assert_eq!(date.format("g", &en).unwrap(), "10/26/2026 3:30\u{202f}PM");
    assert_eq!(date.format("%y", &en).unwrap(), "26");
    assert_eq!(date.format("yyyy''MM", &en).unwrap(), "202610");
    assert_eq!(date.format("'a''b'yyyy", &en).unwrap(), "ab2026");
    assert_eq!(
        date.format("HHHH:mmmm:ssss hhhhh", &en).unwrap(),
        "15:30:45 03"
    );
}
