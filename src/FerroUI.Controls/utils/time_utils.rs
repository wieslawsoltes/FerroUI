use ferroui_base::utilities::CultureInfo;

/// Helpers for the time designators of the current culture.
pub(crate) struct TimeUtils;

impl TimeUtils {
    /// The PM designator of the current culture, or the one of the
    /// invariant culture when the current culture has none.
    pub fn get_pm_designator() -> String {
        let current = CultureInfo::current_culture().date_time_format();
        if !current.pm_designator().is_empty() {
            current.pm_designator().to_string()
        } else {
            CultureInfo::invariant_culture().date_time_format().pm_designator().to_string()
        }
    }

    /// The AM designator of the current culture, or the one of the
    /// invariant culture when the current culture has none.
    pub fn get_am_designator() -> String {
        let current = CultureInfo::current_culture().date_time_format();
        if !current.am_designator().is_empty() {
            current.am_designator().to_string()
        } else {
            CultureInfo::invariant_culture().date_time_format().am_designator().to_string()
        }
    }
}
