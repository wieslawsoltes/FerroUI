use super::CultureInfo;
use crate::data::converters::composite_format::{format_with, FormatArg};

const FORMAT_TEMPLATE_SEPARATED: &str = "{0}{1:0.#} {2}";
const FORMAT_TEMPLATE: &str = "{0}{1:0.#}{2}";

const PREFIXES: [&str; 9] = ["B", "KB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"];

/// Formats a size in bytes as text with a decimal unit prefix (`1.5 KB`).
pub struct ByteSizeHelper;

impl ByteSizeHelper {
    /// The text of `bytes` with the largest decimal unit that keeps the
    /// number at least one, with at most one fractional digit, formatted
    /// with the conventions of the current culture.
    ///
    /// `separate` puts a space between the number and the unit; as in the
    /// original, only a size of zero honours it.
    pub fn to_string(bytes: u64, separate: bool) -> String {
        let culture = CultureInfo::current_culture();

        if bytes == 0 {
            let template = if separate { FORMAT_TEMPLATE_SEPARATED } else { FORMAT_TEMPLATE };
            return format_with(template, &[FormatArg::Null, FormatArg::I32(0), FormatArg::Str(PREFIXES[0])], &culture)
                .expect("the size format is valid");
        }

        let abs_size = (bytes as f64).abs();
        let fp_power = abs_size.ln() / 1000f64.ln();
        let int_power = fp_power as i32;
        let i_unit = if int_power >= PREFIXES.len() as i32 { PREFIXES.len() - 1 } else { int_power as usize };
        let norm_size = abs_size / 1000f64.powi(i_unit as i32);

        // An unsigned size is never negative: the sign argument is always null.
        format_with(FORMAT_TEMPLATE, &[FormatArg::Null, FormatArg::F64(norm_size), FormatArg::Str(PREFIXES[i_unit])], &culture)
            .expect("the size format is valid")
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the upstream project has no tests of this helper.
    use super::*;

    #[test]
    fn zero_honours_the_separator() {
        assert_eq!("0 B", ByteSizeHelper::to_string(0, true));
        assert_eq!("0B", ByteSizeHelper::to_string(0, false));
    }

    #[test]
    fn non_zero_sizes_use_decimal_units_without_separator() {
        assert_eq!("999B", ByteSizeHelper::to_string(999, true));
        assert_eq!("1KB", ByteSizeHelper::to_string(1000, true));
        assert_eq!("1.5KB", ByteSizeHelper::to_string(1500, false));
        assert_eq!("2.5MB", ByteSizeHelper::to_string(2_500_000, true));
        assert_eq!("18.4EB", ByteSizeHelper::to_string(u64::MAX, true));
    }
}
