//! System ICU locale conventions for FerroUI's existing native number formatter.
//! ICU supplies data; no numeric formatter or general culture runtime lives here.
use ferroui_base::utilities::NumberFormatInfo;
use std::{
    ffi::{CString, c_char, c_void},
    ptr::NonNull,
};

#[derive(Debug)]
pub enum NativeNumberDataError {
    InvalidLocale,
    Icu {
        operation: &'static str,
        status: i32,
    },
    InvalidData(&'static str),
    UnsupportedPattern(String),
    InvalidUtf16,
    Allocation,
}
impl std::fmt::Display for NativeNumberDataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "native number conventions: {self:?}")
    }
}
impl std::error::Error for NativeNumberDataError {}
#[repr(C)]
struct UNumberFormat {
    _opaque: [u8; 0],
}
// Stable ICU C ABI. Declarations/enum values checked against ICU76.1 unum.h;
// the operating system supplies the actual version and locale data at runtime.
#[cfg_attr(target_os = "macos", link(name = "icucore"))]
#[cfg_attr(target_os = "windows", link(name = "icu", kind = "raw-dylib"))]
unsafe extern "C" {
    #[cfg_attr(target_os = "emscripten", link_name = "unum_open_68")]
    #[cfg_attr(target_os = "linux", link_name = "fr_icu_unum_open")]
    fn unum_open(
        style: i32,
        pattern: *const u16,
        length: i32,
        locale: *const c_char,
        parse_error: *mut c_void,
        status: *mut i32,
    ) -> *mut UNumberFormat;
    #[cfg_attr(target_os = "emscripten", link_name = "unum_close_68")]
    #[cfg_attr(target_os = "linux", link_name = "fr_icu_unum_close")]
    fn unum_close(format: *mut UNumberFormat);
    #[cfg_attr(target_os = "emscripten", link_name = "unum_getAttribute_68")]
    #[cfg_attr(target_os = "linux", link_name = "fr_icu_unum_getAttribute")]
    fn unum_getAttribute(format: *const UNumberFormat, attribute: i32) -> i32;
    #[cfg_attr(target_os = "emscripten", link_name = "unum_getSymbol_68")]
    #[cfg_attr(target_os = "linux", link_name = "fr_icu_unum_getSymbol")]
    fn unum_getSymbol(
        format: *const UNumberFormat,
        symbol: i32,
        buffer: *mut u16,
        capacity: i32,
        status: *mut i32,
    ) -> i32;
    #[cfg_attr(target_os = "emscripten", link_name = "unum_toPattern_68")]
    #[cfg_attr(target_os = "linux", link_name = "fr_icu_unum_toPattern")]
    fn unum_toPattern(
        format: *const UNumberFormat,
        localized: i8,
        buffer: *mut u16,
        capacity: i32,
        status: *mut i32,
    ) -> i32;
}
struct Format(NonNull<UNumberFormat>);
impl Drop for Format {
    fn drop(&mut self) {
        unsafe { unum_close(self.0.as_ptr()) };
    }
}
fn check(operation: &'static str, status: i32) -> Result<(), NativeNumberDataError> {
    if status > 0 {
        Err(NativeNumberDataError::Icu { operation, status })
    } else {
        Ok(())
    }
}
// Buffers own UTF-16 only for the duration of each native call. Retry a reported
// overflow using ICU's required length; never truncate or manufacture a value.
fn text(
    operation: &'static str,
    mut read: impl FnMut(*mut u16, i32, *mut i32) -> i32,
) -> Result<String, NativeNumberDataError> {
    let mut buffer = vec![0; 64];
    loop {
        let capacity = buffer
            .len()
            .try_into()
            .map_err(|_| NativeNumberDataError::Allocation)?;
        let mut status = 0;
        let length = read(buffer.as_mut_ptr(), capacity, &mut status);
        let length =
            usize::try_from(length).map_err(|_| NativeNumberDataError::InvalidData(operation))?;
        if status == 15 || length >= buffer.len() {
            let capacity = length
                .checked_add(1)
                .ok_or(NativeNumberDataError::Allocation)?;
            buffer
                .try_reserve_exact(capacity.saturating_sub(buffer.len()))
                .map_err(|_| NativeNumberDataError::Allocation)?;
            buffer.resize(capacity, 0);
            continue;
        }
        check(operation, status)?;
        return String::from_utf16(&buffer[..length])
            .map_err(|_| NativeNumberDataError::InvalidUtf16);
    }
}
impl Format {
    fn new(locale: &CString, style: i32) -> Result<Self, NativeNumberDataError> {
        let mut status = 0;
        // SAFETY: ICU copies locale/pattern input. Acquire any returned handle
        // before checking status so every exit path closes it exactly once.
        let raw = unsafe {
            unum_open(
                style,
                std::ptr::null(),
                0,
                locale.as_ptr(),
                std::ptr::null_mut(),
                &mut status,
            )
        };
        let handle = NonNull::new(raw).map(Self);
        check("open number conventions", status)?;
        handle.ok_or(NativeNumberDataError::Allocation)
    }
    fn symbol(&self, symbol: i32) -> Result<String, NativeNumberDataError> {
        // SAFETY: format and writable buffer live across this call; ICU receives
        // the exact capacity in UTF-16 units and reports the required length.
        text("number symbol", |b, c, s| unsafe {
            unum_getSymbol(self.0.as_ptr(), symbol, b, c, s)
        })
    }
    fn attribute(&self, attribute: i32) -> i32 {
        unsafe { unum_getAttribute(self.0.as_ptr(), attribute) }
    }
    fn groups(&self) -> Result<Vec<i32>, NativeNumberDataError> {
        let first = self.attribute(10);
        let next = self.attribute(15);
        if !(0..=9).contains(&first) || !(0..=9).contains(&next) {
            return Err(NativeNumberDataError::InvalidData("group sizes"));
        }
        Ok(if next == 0 || next == first {
            vec![first]
        } else {
            vec![first, next]
        })
    }
    fn digits(&self) -> Result<i32, NativeNumberDataError> {
        let n = self.attribute(6);
        if (0..=99).contains(&n) {
            Ok(n)
        } else {
            Err(NativeNumberDataError::InvalidData("decimal digits"))
        }
    }
    fn pattern(&self, negative: bool, patterns: &[&str]) -> Result<i32, NativeNumberDataError> {
        let raw = text("number pattern", |b, c, s| unsafe {
            unum_toPattern(self.0.as_ptr(), 0, b, c, s)
        })?;
        let (positive, explicit_negative) = raw
            .split_once(';')
            .map_or((raw.as_str(), None), |(p, n)| (p, Some(n)));
        let selected = if negative {
            explicit_negative.unwrap_or(positive)
        } else {
            positive
        };
        let mut pattern = String::new();
        let mut number = false;
        let mut currency = false;
        let mut space = false;
        for ch in selected.chars() {
            match ch {
                '#' | '0' if !number => {
                    pattern.push('n');
                    number = true;
                }
                '¤' if !currency => {
                    pattern.push('C');
                    currency = true;
                }
                ' ' | '\u{a0}' if !space => {
                    pattern.push(' ');
                    space = true;
                }
                '-' | '(' | ')' | '%' => pattern.push(ch),
                _ => {}
            }
        }
        // An omitted negative subpattern has ICU's automatic minus prefix.
        if negative && explicit_negative.is_none() && !pattern.contains(['-', '(', ')']) {
            pattern.insert(0, '-');
        }
        patterns
            .iter()
            .position(|p| *p == pattern)
            .map(|n| n as i32)
            .ok_or(NativeNumberDataError::UnsupportedPattern(raw))
    }
}
#[cfg(target_os = "windows")]
fn windows_fraction_digits(locale: &str) -> Option<i32> {
    const LOCALE_NAME_MAX_LENGTH: usize = 85;
    const LOCALE_IDIGITS: u32 = 0x11;
    const LOCALE_RETURN_NUMBER: u32 = 0x2000_0000;

    #[link(name = "kernel32", kind = "raw-dylib")]
    unsafe extern "system" {
        fn GetUserDefaultLocaleName(name: *mut u16, capacity: i32) -> i32;
        fn GetLocaleInfoEx(name: *const u16, kind: u32, result: *mut u16, capacity: i32) -> i32;
    }

    let mut name = [0_u16; LOCALE_NAME_MAX_LENGTH];
    // The array has the Win32 maximum locale-name capacity, including the NUL.
    let length = unsafe { GetUserDefaultLocaleName(name.as_mut_ptr(), name.len() as i32) };
    if length <= 0
        || !String::from_utf16(&name[..length as usize - 1])
            .ok()?
            .eq_ignore_ascii_case(locale)
    {
        return None;
    }

    let mut digits = 0_u32;
    // A null locale name selects the user default. With LOCALE_RETURN_NUMBER,
    // Win32 writes a DWORD and reports its size in UTF-16 units.
    let written = unsafe {
        GetLocaleInfoEx(
            std::ptr::null(),
            LOCALE_IDIGITS | LOCALE_RETURN_NUMBER,
            (&mut digits as *mut u32).cast(),
            2,
        )
    };
    (written == 2 && digits <= 99).then_some(digits as i32)
}

/// Reads live OS/ICU conventions into FerroUI's actual NumberFormatInfo. The
/// returned value is owned; no ICU pointer or process/thread locale is retained.
pub fn native_number_format(locale: &str) -> Result<NumberFormatInfo, NativeNumberDataError> {
    #[cfg(target_os = "emscripten")]
    check("load browser ICU data", crate::browser_icu::initialize())?;
    if locale.is_empty() {
        return Err(NativeNumberDataError::InvalidLocale);
    }
    #[cfg(target_os = "windows")]
    let windows_digits = windows_fraction_digits(locale);
    let locale = CString::new(locale).map_err(|_| NativeNumberDataError::InvalidLocale)?;
    let number = Format::new(&locale, 1)?;
    let currency = Format::new(&locale, 2)?;
    let percent = Format::new(&locale, 3)?;
    let negative = number.symbol(6)?;
    let infinity = number.symbol(14)?;
    let digits = number.digits()?;
    #[cfg(target_os = "windows")]
    let digits = windows_digits.unwrap_or(digits);
    // Percent precision/grouping use the number conventions in the source
    // product's culture format, not the ICU percent renderer's default precision.
    let result = NumberFormatInfo::new()
        .with_number_decimal_digits(digits)
        .with_number_decimal_separator(number.symbol(0)?)
        .with_number_group_separator(number.symbol(1)?)
        .with_number_group_sizes(&number.groups()?)
        .with_number_negative_pattern(number.pattern(true, &["(n)", "-n", "- n", "n-", "n -"])?)
        .with_currency_decimal_digits(currency.digits()?)
        .with_currency_decimal_separator(currency.symbol(10)?)
        .with_currency_group_separator(currency.symbol(17)?)
        .with_currency_group_sizes(&currency.groups()?)
        .with_currency_symbol(currency.symbol(8)?)
        .with_currency_positive_pattern(currency.pattern(false, &["Cn", "nC", "C n", "n C"])?)
        .with_currency_negative_pattern(currency.pattern(
            true,
            &[
                "(Cn)", "-Cn", "C-n", "Cn-", "(nC)", "-nC", "n-C", "nC-", "-n C", "-C n", "n C-",
                "C n-", "C -n", "n- C", "(C n)", "(n C)", "C- n",
            ],
        )?)
        .with_percent_decimal_digits(digits)
        .with_percent_decimal_separator(number.symbol(0)?)
        .with_percent_group_separator(number.symbol(1)?)
        .with_percent_group_sizes(&number.groups()?)
        .with_percent_symbol(number.symbol(3)?)
        .with_per_mille_symbol(number.symbol(12)?)
        .with_percent_positive_pattern(percent.pattern(false, &["n %", "n%", "%n", "% n"])?)
        .with_percent_negative_pattern(percent.pattern(
            true,
            &[
                "-n %", "-n%", "-%n", "%-n", "%n-", "n-%", "n%-", "-% n", "n %-", "% n-", "% -n",
                "n- %",
            ],
        )?)
        .with_nan_symbol(number.symbol(15)?)
        .with_negative_infinity_symbol(format!("{negative}{infinity}"))
        .with_positive_infinity_symbol(infinity)
        .with_negative_sign(negative)
        .with_positive_sign(number.symbol(7)?);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(target_os = "emscripten"))]
    use serde_json::json;
    #[cfg(not(target_os = "emscripten"))]
    fn values(n: &NumberFormatInfo) -> serde_json::Value {
        json!({"CurrencyDecimalDigits":n.currency_decimal_digits(),"CurrencyDecimalSeparator":n.currency_decimal_separator(),"CurrencyGroupSeparator":n.currency_group_separator(),"CurrencyGroupSizes":n.currency_group_sizes(),"CurrencyNegativePattern":n.currency_negative_pattern(),"CurrencyPositivePattern":n.currency_positive_pattern(),"CurrencySymbol":n.currency_symbol(),"NaNSymbol":n.nan_symbol(),"NegativeInfinitySymbol":n.negative_infinity_symbol(),"NegativeSign":n.negative_sign(),"NumberDecimalDigits":n.number_decimal_digits(),"NumberDecimalSeparator":n.number_decimal_separator(),"NumberGroupSeparator":n.number_group_separator(),"NumberGroupSizes":n.number_group_sizes(),"NumberNegativePattern":n.number_negative_pattern(),"PercentDecimalDigits":n.percent_decimal_digits(),"PercentDecimalSeparator":n.percent_decimal_separator(),"PercentGroupSeparator":n.percent_group_separator(),"PercentGroupSizes":n.percent_group_sizes(),"PercentNegativePattern":n.percent_negative_pattern(),"PercentPositivePattern":n.percent_positive_pattern(),"PercentSymbol":n.percent_symbol(),"PerMilleSymbol":n.per_mille_symbol(),"PositiveInfinitySymbol":n.positive_infinity_symbol(),"PositiveSign":n.positive_sign()})
    }
    #[test]
    #[cfg(not(target_os = "emscripten"))]
    #[cfg_attr(
        any(target_os = "linux", target_os = "windows"),
        ignore = "requires the source number corpus for this host"
    )]
    fn native_conventions_match_source_culture_data_without_runtime_tables() {
        #[cfg(target_os = "macos")]
        let cultures = serde_json::from_str::<serde_json::Value>(include_str!(
            "../../tests/fixtures/native-numbers.json"
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
        let mut differences = Vec::new();
        for row in cultures.as_array().unwrap() {
            let name = row["name"].as_str().unwrap();
            let data = native_number_format(name).unwrap();
            let actual = values(&data);
            if actual != row["number"] {
                differences.push(json!({"name":name,"actual":actual,"source":row["number"]}));
            }
        }
        assert!(
            differences.is_empty(),
            "{}",
            serde_json::to_string_pretty(&differences).unwrap()
        );
    }
    #[test]
    fn invalid_inputs_and_owned_mutations_do_not_change_another_read() {
        assert!(matches!(
            native_number_format(""),
            Err(NativeNumberDataError::InvalidLocale)
        ));
        assert!(matches!(
            native_number_format("de\0-DE"),
            Err(NativeNumberDataError::InvalidLocale)
        ));
        let mut first = native_number_format("de-DE").unwrap();
        first.set_negative_sign("custom");
        assert_eq!(native_number_format("de-DE").unwrap().negative_sign(), "-");
    }
    #[test]
    fn native_buffer_growth_preserves_utf16_and_reports_errors() {
        let expected = "😀e\u{301}".repeat(50);
        let units = expected.encode_utf16().collect::<Vec<_>>();
        let mut calls = 0;
        let actual = text("buffer probe", |buffer, capacity, status| {
            calls += 1;
            // SAFETY: text supplies a live status and exactly capacity writable
            // units. The simulated native writer obeys that same ABI contract.
            unsafe {
                if capacity as usize <= units.len() {
                    *status = 15;
                } else {
                    std::ptr::copy_nonoverlapping(units.as_ptr(), buffer, units.len());
                }
            }
            units.len() as i32
        })
        .unwrap();
        assert_eq!(actual, expected);
        assert_eq!(calls, 2);
        assert!(matches!(
            text("failure", |_, _, status| {
                unsafe {
                    *status = 1;
                }
                0
            }),
            Err(NativeNumberDataError::Icu {
                operation: "failure",
                status: 1
            })
        ));
        assert!(matches!(
            text("encoding", |buffer, _, _| {
                unsafe {
                    *buffer = 0xd800;
                }
                1
            }),
            Err(NativeNumberDataError::InvalidUtf16)
        ));
    }
}
