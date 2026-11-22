//! Thread-safe culture-data snapshot using the selected native formatters.
#[cfg(test)]
use ferroribbon_core::collections::RibbonFixedItems;
use ferroribbon_core::{RibbonString, localization::*};
use ferroui_base::data::converters::composite_format::{self, FormatArg};
use ferroui_base::utilities::{CultureInfo, DateTime, DateTimeOffset, Decimal};
use ferroui_base::utilities::{DateTimeFormatInfo, NumberFormatInfo};
use std::{
    rc::Rc,
    sync::{Arc, Mutex, PoisonError},
};
mod startup;
pub use startup::initialize_ui_culture_from_platform;
#[cfg(any(
    target_os = "macos",
    target_os = "windows",
    target_os = "emscripten",
    target_os = "linux"
))]
mod number_provider;
#[cfg(any(
    target_os = "macos",
    target_os = "windows",
    target_os = "emscripten",
    target_os = "linux"
))]
fn install_native_number_provider() {
    number_provider::install();
}
/// Captures explicit native data. This adapter owns a snapshot rather than a
/// thread-affine CultureInfo handle. Use set_number_format/set_date_time_format
/// for subsequent changes shared by all users of this culture.
pub struct RibbonFrameworkCulture {
    name: String,
    iso: String,
    data: Mutex<NativeData>,
}
#[derive(Clone)]
struct NativeData {
    number: Option<NumberFormatInfo>,
    date: Option<DateTimeFormatInfo>,
}
impl RibbonFrameworkCulture {
    pub fn new(culture: CultureInfo) -> Arc<Self> {
        #[cfg(any(
            target_os = "macos",
            target_os = "windows",
            target_os = "emscripten",
            target_os = "linux"
        ))]
        install_native_number_provider();
        let number = culture.number_format();
        let date = culture.date_time_format();
        let number = if !culture.is_invariant()
            && Rc::ptr_eq(&number, &NumberFormatInfo::invariant_info())
        {
            None
        } else {
            Some((*number).clone())
        };
        let date = if !culture.is_invariant()
            && Rc::ptr_eq(&date, &DateTimeFormatInfo::invariant_info())
        {
            None
        } else {
            Some((*date).clone())
        };
        Arc::new(Self {
            name: culture.name().to_owned(),
            iso: culture.two_letter_iso_language_name().to_owned(),
            data: Mutex::new(NativeData { number, date }),
        })
    }
    pub fn framework_culture(&self) -> CultureInfo {
        self.culture_from_snapshot(&self.snapshot())
    }
    fn snapshot(&self) -> NativeData {
        self.data
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
    fn culture_from_snapshot(&self, data: &NativeData) -> CultureInfo {
        // Native provider lookup can call application code. The adapter's
        // storage lock has already been released before entering that service.
        let mut culture = CultureInfo::get_culture_info(&self.name);
        if let Some(number) = &data.number {
            culture = culture.with_number_format(number.clone());
        }
        if let Some(date) = &data.date {
            culture = culture.with_date_time_format(date.clone());
        }
        culture
    }
    pub fn set_number_format(&self, number: NumberFormatInfo) {
        let old = self
            .data
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .number
            .replace(number);
        drop(old);
    }
    pub fn set_date_time_format(&self, date: DateTimeFormatInfo) {
        let old = self
            .data
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .date
            .replace(date);
        drop(old);
    }
}
impl RibbonCultureInfo for RibbonFrameworkCulture {
    fn name(&self) -> Option<RibbonString> {
        Some(self.name.as_str().into())
    }
    fn two_letter_iso_language_name(&self) -> Option<RibbonString> {
        Some(self.iso.as_str().into())
    }
    fn format_numeric(
        &self,
        arg: &RibbonFormatArgument,
        format: Option<&RibbonString>,
    ) -> Result<Option<RibbonString>, RibbonLocalizationError> {
        let data = self.snapshot();
        let culture = self.culture_from_snapshot(&data);
        let text = format
            .map(RibbonString::to_utf8)
            .transpose()
            .map_err(|_| {
                RibbonLocalizationError::new(
                    RibbonLocalizationErrorKind::Unavailable,
                    None,
                    "Native format specifiers require valid UTF16",
                )
            })?
            .unwrap_or_default();
        let payload = if let RibbonFormatArgument::Native(v) = arg {
            Some(v.payload())
        } else {
            None
        };
        // .NET renders default nonnegative integer values using ASCII digits.
        // A sign, a supplied format or a fractional type can need culture data.
        let needs_number_format = match arg {
            RibbonFormatArgument::I8(value) => *value < 0 || !text.is_empty(),
            RibbonFormatArgument::I16(value) => *value < 0 || !text.is_empty(),
            RibbonFormatArgument::I32(value) => *value < 0 || !text.is_empty(),
            RibbonFormatArgument::I64(value) => *value < 0 || !text.is_empty(),
            RibbonFormatArgument::U64(_) => !text.is_empty(),
            RibbonFormatArgument::F32(_) | RibbonFormatArgument::F64(_) => true,
            _ => payload.as_ref().is_some_and(|p| p.is::<Decimal>()),
        };
        let date = payload
            .as_ref()
            .is_some_and(|p| p.is::<DateTime>() || p.is::<DateTimeOffset>());
        if !culture.is_invariant() {
            let missing_number = needs_number_format && data.number.is_none();
            let missing_date = date && data.date.is_none();
            let unsupported_span = payload
                .as_ref()
                .is_some_and(|p| p.is::<ferroui_base::animation::SignedDuration>())
                && matches!(text.as_str(), "g" | "G");
            if missing_number || missing_date || unsupported_span {
                return Err(RibbonLocalizationError::new(
                    RibbonLocalizationErrorKind::Unavailable,
                    None,
                    "Faithful culture formatting data or this argument format is not supplied by the selected framework",
                ));
            }
        }
        let value = match arg {
            RibbonFormatArgument::I8(v) => FormatArg::I8(*v),
            RibbonFormatArgument::I16(v) => FormatArg::I16(*v),
            RibbonFormatArgument::I32(v) => FormatArg::I32(*v),
            RibbonFormatArgument::I64(v) => FormatArg::I64(*v),
            RibbonFormatArgument::U64(v) => FormatArg::U64(*v),
            RibbonFormatArgument::F32(v) => FormatArg::F32(*v),
            RibbonFormatArgument::F64(v) => FormatArg::F64(*v),
            RibbonFormatArgument::Native(v) => {
                let payload = v.payload();
                if let Some(v) = payload.downcast_ref::<Decimal>() {
                    FormatArg::Decimal(*v)
                } else if let Some(v) = payload.downcast_ref::<DateTime>() {
                    FormatArg::DateTime(*v)
                } else if let Some(v) = payload.downcast_ref::<DateTimeOffset>() {
                    FormatArg::DateTimeOffset(*v)
                } else if let Some(v) =
                    payload.downcast_ref::<ferroui_base::animation::SignedDuration>()
                {
                    FormatArg::TimeSpan(*v)
                } else {
                    return Err(RibbonLocalizationError::new(
                        RibbonLocalizationErrorKind::Unavailable,
                        None,
                        "Native argument requires explicit IFormattable/ToString dispatch",
                    ));
                }
            }
            _ => {
                return Err(RibbonLocalizationError::new(
                    RibbonLocalizationErrorKind::Unavailable,
                    None,
                    "Argument is not a native numeric/date value",
                ));
            }
        };
        composite_format::format_arg_with(&value, &text, &culture)
            .map(|v| Some(v.into()))
            .map_err(|e| {
                RibbonLocalizationError::new(
                    RibbonLocalizationErrorKind::Format,
                    None,
                    e.to_string(),
                )
            })
    }
}
pub(crate) fn install_culture_provider() {
    #[cfg(any(
        target_os = "macos",
        target_os = "windows",
        target_os = "emscripten",
        target_os = "linux"
    ))]
    install_native_number_provider();
    RibbonStrings::set_current_ui_culture_provider(Some(Rc::new(|| {
        RibbonFrameworkCulture::new(CultureInfo::from(
            &ferroui_base::utilities::current_ui_locale(),
        ))
    })));
}
#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::utilities::NumberFormatInfo;

    #[test]
    fn default_nonnegative_integers_format_without_a_locale_data_snapshot() {
        let culture = RibbonFrameworkCulture::new(CultureInfo::get_culture_info("ar-SA"));
        culture.data.lock().unwrap().number = None;

        for (argument, expected) in [
            (RibbonFormatArgument::I8(0), "0"),
            (RibbonFormatArgument::I16(12), "12"),
            (RibbonFormatArgument::I32(3), "3"),
            (RibbonFormatArgument::I64(i64::MAX), "9223372036854775807"),
            (RibbonFormatArgument::U64(u64::MAX), "18446744073709551615"),
        ] {
            assert_eq!(
                culture.format_numeric(&argument, None).unwrap(),
                Some(expected.into()),
            );
        }

        for (argument, format) in [
            (RibbonFormatArgument::I32(-1), None),
            (RibbonFormatArgument::I32(1234), Some("N0".into())),
            (RibbonFormatArgument::F64(1.5), None),
        ] {
            assert_eq!(
                culture
                    .format_numeric(&argument, format.as_ref())
                    .unwrap_err()
                    .kind,
                RibbonLocalizationErrorKind::Unavailable,
            );
        }
    }

    /// The recorded result with the locale values the operating system owns taken from the
    /// system: they differ between its releases. These are the group separator of `de-AT` in
    /// numbers and percentages and the currency symbol of `ar-SA`. Digits, grouping, decimal
    /// separator, sign, pattern and every other culture stay compared with the record.
    #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
    fn with_system_locale_values(
        name: &str,
        format: &str,
        recorded: &serde_json::Value,
    ) -> serde_json::Value {
        let first = format.chars().next();
        if !matches!((name, first), ("de-AT", Some('N' | 'P')) | ("ar-SA", Some('C'))) {
            return recorded.clone();
        }
        let units = recorded
            .as_array()
            .unwrap()
            .iter()
            .map(|unit| unit.as_u64().unwrap() as u16)
            .collect::<Vec<_>>();
        let text = String::from_utf16(&units).unwrap();
        let system = ferroribbon_platform::native_number_format(name).unwrap();
        let text = if name == "ar-SA" {
            text.replace('\u{20c1}', system.currency_symbol())
        } else {
            // Between digits only: the space before a percent sign belongs to the pattern.
            let chars = text.chars().collect::<Vec<_>>();
            chars
                .iter()
                .enumerate()
                .map(|(index, &c)| {
                    let grouped = c == '\u{a0}'
                        && index > 0
                        && chars[index - 1].is_ascii_digit()
                        && chars.get(index + 1).is_some_and(char::is_ascii_digit);
                    if grouped {
                        system.number_group_separator().to_owned()
                    } else {
                        c.to_string()
                    }
                })
                .collect::<String>()
        };
        serde_json::json!(text.encode_utf16().collect::<Vec<_>>())
    }

    #[test]
    #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
    #[cfg_attr(
        any(target_os = "linux", target_os = "windows"),
        ignore = "requires the source number corpus for this host"
    )]
    fn native_locale_provider_formats_the_whole_source_number_corpus() {
        #[cfg(target_os = "macos")]
        let cultures = serde_json::from_str::<serde_json::Value>(include_str!(
            "../../../tests/fixtures/native-numbers.json"
        ))
        .unwrap()["cultures"]
            .clone();
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        let cultures: serde_json::Value = serde_json::from_slice(
            &std::fs::read(
                std::env::var_os("FERRORIBBON_NATIVE_NUMBER_CORPUS")
                    .expect("path to the source oracle output"),
            )
            .unwrap(),
        )
        .unwrap();
        let mut failures = Vec::new();
        let mut compared = 0;
        for row in cultures.as_array().unwrap() {
            let name = row["name"].as_str().unwrap();
            let strings = RibbonStrings::for_culture(Some(RibbonFrameworkCulture::new(
                CultureInfo::get_culture_info(name),
            )))
            .unwrap();
            assert_eq!(
                serde_json::json!(
                    strings
                        .format(
                            Some(&"TablePickerFormat".into()),
                            Some(RibbonFixedItems::new(vec![2.into(), 1.into()]))
                        )
                        .unwrap()
                        .as_utf16()
                ),
                row["table"],
                "{name} table"
            );
            for sample in row["samples"].as_array().unwrap() {
                let format = sample["format"].as_str().unwrap();
                let value = match sample["value"].as_str().unwrap() {
                    "NaN" => f64::NAN,
                    "Infinity" => f64::INFINITY,
                    "-Infinity" => f64::NEG_INFINITY,
                    v => v.parse().unwrap(),
                };
                strings
                    .override_string(
                        Some("NativeFormat".into()),
                        Some(format!("{{0:{format}}}").into()),
                    )
                    .unwrap();
                let actual = strings.format(
                    Some(&"NativeFormat".into()),
                    Some(RibbonFixedItems::new(vec![RibbonFormatArgument::F64(
                        value,
                    )])),
                );
                if actual
                    .as_ref()
                    .map(|v| serde_json::json!(v.as_utf16()))
                    .ok()
                    != Some(with_system_locale_values(name, format, &sample["result"]))
                {
                    failures.push(serde_json::json!({"name":name,"format":format,"value":sample["value"],"source":sample["result"],"actual":actual.as_ref().ok().map(|s|s.to_utf8().unwrap()),"error":actual.as_ref().err().map(|e|e.to_string())}));
                }
                compared += 1;
            }
        }
        assert_eq!(compared, 1200);
        assert!(
            failures.is_empty(),
            "{}",
            serde_json::to_string_pretty(&failures).unwrap()
        );
    }
    #[test]
    #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
    fn native_provider_uses_scoped_shared_data_and_preserves_application_providers() {
        use ferroui_base::{FerroLocator, LocatorExtensions, utilities::ICultureDataProvider};
        let _scope = FerroLocator::enter_scope();
        install_native_number_provider();
        let provider = FerroLocator::current()
            .get_service::<dyn ICultureDataProvider>()
            .unwrap();
        assert!(Rc::ptr_eq(
            &provider
                .get_number_format_by_locale(&"de-DE".parse().unwrap())
                .unwrap(),
            &provider
                .get_number_format_by_locale(&"DE-de".parse().unwrap())
                .unwrap()
        ));
        struct AppProvider(Rc<NumberFormatInfo>);
        impl ICultureDataProvider for AppProvider {
            fn get_number_format_by_locale(
                &self,
                _: &ferroui_base::utilities::Locale,
            ) -> Option<Rc<NumberFormatInfo>> {
                Some(self.0.clone())
            }
        }
        let data = Rc::new(NumberFormatInfo::new().with_number_decimal_separator("custom"));
        let custom: Rc<dyn ICultureDataProvider> = Rc::new(AppProvider(data.clone()));
        FerroLocator::current_mutable()
            .bind::<dyn ICultureDataProvider>()
            .to_constant(custom.clone());
        install_native_number_provider();
        assert!(Rc::ptr_eq(
            &FerroLocator::current()
                .get_service::<dyn ICultureDataProvider>()
                .unwrap(),
            &custom
        ));
        assert!(Rc::ptr_eq(
            &CultureInfo::get_culture_info("de-DE").number_format(),
            &data
        ));
        assert_eq!(
            RibbonFrameworkCulture::new(CultureInfo::get_culture_info("de-DE"))
                .framework_culture()
                .number_format()
                .number_decimal_separator(),
            "custom"
        );
    }
    #[test]
    fn shared_native_culture_formats_on_workers_and_explicit_updates_reach_existing_strings() {
        fn thread_safe<T: Send + Sync>() {}
        thread_safe::<RibbonFrameworkCulture>();
        let native = CultureInfo::get_culture_info("de-DE").with_number_format(
            NumberFormatInfo::new()
                .with_number_decimal_separator(",")
                .with_number_group_separator("."),
        );
        let original = native.number_format();
        let culture = RibbonFrameworkCulture::new(native);
        let strings = RibbonStrings::for_culture(Some(culture.clone())).unwrap();
        strings
            .override_string(Some("NativeUpdate".into()), Some("{0:N}".into()))
            .unwrap();
        let format = |strings: &RibbonStrings| {
            strings
                .format(
                    Some(&"NativeUpdate".into()),
                    Some(RibbonFixedItems::new(vec![RibbonFormatArgument::F64(
                        1234.5,
                    )])),
                )
                .unwrap()
                .to_utf8()
                .unwrap()
        };
        assert_eq!(format(&strings), "1.234,50");
        // The thread-affine framework object is a constructor input snapshot.
        original.set_number_decimal_digits(3);
        assert_eq!(format(&strings), "1.234,50");
        let worker_strings = strings.clone();
        let worker_culture = culture.clone();
        let result = std::thread::spawn(move || {
            worker_culture.set_number_format(
                NumberFormatInfo::new()
                    .with_number_decimal_separator(";")
                    .with_number_group_separator("_")
                    .with_number_decimal_digits(1),
            );
            let result = format(&worker_strings);
            let mut date = DateTimeFormatInfo::new();
            date.set_short_date_pattern("yyyy/MM/dd");
            worker_culture.set_date_time_format(date);
            (
                result,
                worker_culture
                    .framework_culture()
                    .date_time_format()
                    .short_date_pattern()
                    .to_owned(),
            )
        })
        .join()
        .unwrap();
        assert_eq!(result, ("1_234;5".into(), "yyyy/MM/dd".into()));
        assert_eq!(format(&strings), "1_234;5");
        assert_eq!(
            culture
                .framework_culture()
                .date_time_format()
                .short_date_pattern(),
            "yyyy/MM/dd"
        );
    }

    #[test]
    fn missing_native_locale_data_is_explicit_until_supplied() {
        use ferroui_base::{FerroLocator, utilities::ICultureDataProvider};
        struct EmptyProvider;
        impl ICultureDataProvider for EmptyProvider {}
        let _scope = FerroLocator::enter_scope();
        FerroLocator::current_mutable()
            .bind::<dyn ICultureDataProvider>()
            .to_constant(Rc::new(EmptyProvider) as Rc<dyn ICultureDataProvider>);
        let culture = RibbonFrameworkCulture::new(CultureInfo::get_culture_info("de-DE"));
        let value = RibbonFormatArgument::F64(1234.5);
        assert_eq!(
            culture
                .format_numeric(&value, Some(&"N2".into()))
                .unwrap_err()
                .kind,
            RibbonLocalizationErrorKind::Unavailable
        );
        culture.set_number_format(
            NumberFormatInfo::new()
                .with_number_decimal_separator(",")
                .with_number_group_separator("."),
        );
        assert_eq!(
            culture
                .format_numeric(&value, Some(&"N2".into()))
                .unwrap()
                .unwrap()
                .to_utf8()
                .unwrap(),
            "1.234,50"
        );
    }
    #[test]
    fn resources_and_numeric_format_use_native_ui_and_instance_cultures_separately() {
        if crate::test_process::isolated_case(concat!(
            module_path!(),
            "::resources_and_numeric_format_use_native_ui_and_instance_cultures_separately"
        )) {
            return;
        }
        let old = ferroui_base::utilities::current_ui_locale();
        ferroui_base::utilities::set_current_ui_locale("de-AT".parse().unwrap());
        install_culture_provider();
        let current = RibbonStrings::current();
        assert_eq!(current.file().unwrap().to_utf8().unwrap(), "Datei");
        ferroui_base::utilities::set_current_ui_locale("pl-PL".parse().unwrap());
        assert!(Arc::ptr_eq(&current, &RibbonStrings::current()));
        let numbers = NumberFormatInfo::new()
            .with_number_decimal_separator(",")
            .with_number_group_separator(".")
            .with_negative_sign("~");
        let culture = RibbonFrameworkCulture::new(
            CultureInfo::get_culture_info("de-DE").with_number_format(numbers),
        );
        let strings = RibbonStrings::for_culture(Some(culture.clone())).unwrap();
        assert!(Arc::ptr_eq(
            &strings.culture(),
            &(culture as Arc<dyn RibbonCultureInfo>)
        ));
        strings
            .override_string(Some("Fmt".into()), Some("{0:N2}/{1:D4}".into()))
            .unwrap();
        assert_eq!(
            strings
                .format(
                    Some(&"Fmt".into()),
                    Some(RibbonFixedItems::new(vec![
                        RibbonFormatArgument::F64(1234.5),
                        RibbonFormatArgument::I32(-42)
                    ]))
                )
                .unwrap()
                .to_utf8()
                .unwrap(),
            "1.234,50/~0042"
        );
        ferroui_base::utilities::set_current_ui_locale(old);
    }
    #[test]
    fn source_numeric_format_corpus_uses_the_actual_native_argument_formatter() {
        let data: serde_json::Value =
            serde_json::from_str(include_str!("../../../tests/fixtures/localization.json"))
                .unwrap();
        for row in data["formatting"]["numeric"].as_array().unwrap() {
            let n = &row["number"];
            let numbers = NumberFormatInfo::new()
                .with_number_decimal_separator(n["NumberDecimalSeparator"].as_str().unwrap())
                .with_number_group_separator(n["NumberGroupSeparator"].as_str().unwrap())
                .with_negative_sign(n["NegativeSign"].as_str().unwrap())
                .with_positive_sign(n["PositiveSign"].as_str().unwrap())
                .with_number_decimal_digits(n["NumberDecimalDigits"].as_i64().unwrap() as i32)
                .with_number_group_sizes(
                    &n["NumberGroupSizes"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|v| v.as_i64().unwrap() as i32)
                        .collect::<Vec<_>>(),
                )
                .with_number_negative_pattern(n["NumberNegativePattern"].as_i64().unwrap() as i32);
            let culture = RibbonFrameworkCulture::new(
                CultureInfo::get_culture_info(row["culture"].as_str().unwrap())
                    .with_number_format(numbers),
            );
            let strings = RibbonStrings::for_culture(Some(culture)).unwrap();
            let pattern = RibbonString::from_utf16(
                row["template"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_u64().unwrap() as u16)
                    .collect::<Vec<_>>(),
            );
            strings
                .override_string(Some("Numeric".into()), Some(pattern))
                .unwrap();
            let result = strings.format(
                Some(&"Numeric".into()),
                Some(RibbonFixedItems::new(vec![
                    RibbonFormatArgument::F64(1234.5),
                    RibbonFormatArgument::I32(-42),
                ])),
            );
            assert_eq!(result.is_err(), !row["error"].is_null());
            if let Ok(result) = result {
                assert_eq!(serde_json::json!(result.as_utf16()), row["result"]);
            }
        }
    }
}
