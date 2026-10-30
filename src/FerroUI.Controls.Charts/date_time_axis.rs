use crate::{CartesianAxis, ChartError, DateTimeIntervalType};
use ferroui_base::animation::TimeSpan;
use ferroui_base::utilities::{CultureInfo, DateTime, DateTimeOffset};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, FerroObject,
    FerroObjectImpl, FerroProperty, Ref, StyledProperty, StyledPropertyOptions,
};

/// A date axis whose bounds compare UTC instants and whose ticks follow
/// calendar boundaries. Dates before the Unix epoch remain valid.
#[repr(C)]
pub struct DateTimeAxis {
    base: CartesianAxis,
}

ferro_class!(DateTimeAxis: CartesianAxis);
ferro_class_info!(DateTimeAxis {
    new: DateTimeAxis::new
});
ferro_impl_classes!(DateTimeAxis: FerroObjectImpl);

ferro_properties! { impl DateTimeAxis {
    pub fn minimum_property() -> StyledProperty<Option<DateTimeOffset>> {
        FerroProperty::register_with::<DateTimeAxis, _>("Minimum",
            StyledPropertyOptions::new(None).coerce(Self::coerce_minimum))
    }
    pub fn maximum_property() -> StyledProperty<Option<DateTimeOffset>> {
        FerroProperty::register_with::<DateTimeAxis, _>("Maximum",
            StyledPropertyOptions::new(None).coerce(Self::coerce_maximum))
    }
    pub fn interval_type_property() -> StyledProperty<DateTimeIntervalType> {
        FerroProperty::register::<DateTimeAxis, _>("IntervalType", DateTimeIntervalType::Auto)
    }
    pub fn label_format_property() -> StyledProperty<String> {
        FerroProperty::register::<DateTimeAxis, _>("LabelFormat", String::new())
    }
} }

impl DateTimeAxis {
    pub fn construct() -> Self {
        Self {
            base: CartesianAxis::construct(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn minimum(&self) -> Option<DateTimeOffset> {
        self.get_value(Self::minimum_property())
    }

    pub fn set_minimum(&self, value: Option<DateTimeOffset>) -> Result<(), ChartError> {
        validate_bounds(value, self.maximum())?;
        self.set_value(Self::minimum_property(), value);
        Ok(())
    }

    pub fn maximum(&self) -> Option<DateTimeOffset> {
        self.get_value(Self::maximum_property())
    }

    pub fn set_maximum(&self, value: Option<DateTimeOffset>) -> Result<(), ChartError> {
        validate_bounds(self.minimum(), value)?;
        self.set_value(Self::maximum_property(), value);
        Ok(())
    }

    pub fn interval_type(&self) -> DateTimeIntervalType {
        self.get_value(Self::interval_type_property())
    }

    pub fn set_interval_type(&self, value: DateTimeIntervalType) {
        self.set_value(Self::interval_type_property(), value);
    }

    pub fn label_format(&self) -> String {
        self.get_value(Self::label_format_property())
    }

    pub fn set_label_format(&self, value: impl Into<String>) {
        self.set_value(Self::label_format_property(), value.into());
    }

    pub fn set_optional_label_format(&self, value: Option<&str>) {
        self.set_label_format(value.unwrap_or_default());
    }

    // Deviation (DEVIATIONS.md, WinUI Charts): upstream rejects an invalid bound with an
    // exception; a write through the property system keeps the old value instead
    // (winui-charts.md section 4, item 2). The typed setters return `ChartError`.
    fn coerce_minimum(
        sender: &FerroObject,
        value: Option<DateTimeOffset>,
    ) -> Option<DateTimeOffset> {
        if validate_bounds(value, sender.get_value(Self::maximum_property())).is_ok() {
            value
        } else {
            sender.get_value(Self::minimum_property())
        }
    }

    fn coerce_maximum(
        sender: &FerroObject,
        value: Option<DateTimeOffset>,
    ) -> Option<DateTimeOffset> {
        if validate_bounds(sender.get_value(Self::minimum_property()), value).is_ok() {
            value
        } else {
            sender.get_value(Self::maximum_property())
        }
    }

    /// Automatic bounds use the sample instants; an empty source has a
    /// deterministic one-day range beginning at the Unix epoch.
    pub fn resolve_range(&self, values: &[DateTimeOffset]) -> (DateTimeOffset, DateTimeOffset) {
        let fallback = DateTimeOffset::UNIX_EPOCH;
        let mut minimum = self
            .minimum()
            .or_else(|| values.iter().copied().min())
            .unwrap_or(fallback);
        let mut maximum = self
            .maximum()
            .or_else(|| values.iter().copied().max())
            .unwrap_or_else(|| from_utc(fallback.utc_date_time().add_days(1.0)));
        if minimum >= maximum {
            if self.maximum().is_none() {
                maximum = minimum
                    .utc_date_time()
                    .try_add_days(1.0)
                    .map(from_utc)
                    .unwrap_or(DateTimeOffset::MAX_VALUE);
            }
            if minimum >= maximum && self.minimum().is_none() {
                minimum = maximum
                    .utc_date_time()
                    .try_add_days(-1.0)
                    .map(from_utc)
                    .unwrap_or(DateTimeOffset::MIN_VALUE);
            }
            if minimum >= maximum {
                if minimum == DateTimeOffset::MAX_VALUE {
                    minimum = from_utc(DateTime::MAX_VALUE.add_days(-1.0));
                } else {
                    maximum = from_utc(minimum.utc_date_time().add_days(1.0));
                }
            }
        }
        (minimum, maximum)
    }

    /// The interval of the ticks between `minimum` and `maximum`: the `IntervalType`, or
    /// for `Auto` the calendar unit that gives about `target_tick_count` ticks.
    pub fn resolved_interval(
        &self,
        minimum: DateTimeOffset,
        maximum: DateTimeOffset,
        target_tick_count: usize,
    ) -> DateTimeIntervalType {
        let interval = self.interval_type();
        if interval != DateTimeIntervalType::Auto {
            return interval;
        }
        let days = span_in_days(minimum, maximum);
        let span_per_tick = days / target_tick_count.clamp(2, 4096) as f64;
        if span_per_tick <= 1.0 {
            DateTimeIntervalType::Day
        } else if span_per_tick <= 7.0 {
            DateTimeIntervalType::Week
        } else if span_per_tick <= 31.0 {
            DateTimeIntervalType::Month
        } else {
            DateTimeIntervalType::Year
        }
    }

    /// Calendar-aligned ticks in UTC. Month/year ticks use calendar
    /// arithmetic, so leap days and variable-length months are preserved.
    /// Explicit daily/weekly intervals may be sampled for very large ranges;
    /// no call produces more than 4096 ticks. A daily range with fewer than two
    /// midnights in it is divided into whole minutes or hours instead.
    pub fn ticks(
        &self,
        minimum: DateTimeOffset,
        maximum: DateTimeOffset,
        target_tick_count: usize,
    ) -> Vec<DateTimeOffset> {
        self.tick_plan(minimum, maximum, target_tick_count).0
    }

    /// The ticks with their labels, in the format `LabelFormat` names or, when it is empty,
    /// the default format of the interval the ticks were actually made with.
    pub(crate) fn labeled_ticks(
        &self,
        minimum: DateTimeOffset,
        maximum: DateTimeOffset,
        target_tick_count: usize,
    ) -> Vec<(DateTimeOffset, String)> {
        let (ticks, unit) = self.tick_plan(minimum, maximum, target_tick_count);
        let default = match unit {
            TickUnit::Calendar(interval) => default_format(interval),
            TickUnit::SubDay => {
                let first = ticks.first().map(|tick| tick.utc_date_time().date());
                if ticks.iter().all(|tick| Some(tick.utc_date_time().date()) == first) {
                    "HH:mm"
                } else {
                    "MMM d HH:mm"
                }
            }
        };
        ticks
            .into_iter()
            .map(|tick| (tick, self.format_with_default(tick, default)))
            .collect()
    }

    fn tick_plan(
        &self,
        minimum: DateTimeOffset,
        maximum: DateTimeOffset,
        target_tick_count: usize,
    ) -> (Vec<DateTimeOffset>, TickUnit) {
        if minimum >= maximum {
            return (Vec::new(), TickUnit::Calendar(DateTimeIntervalType::Day));
        }
        let min = minimum.utc_date_time();
        let max = maximum.utc_date_time();
        let days = span_in_days(minimum, maximum);
        let target = target_tick_count.clamp(2, 4096) as f64;
        let automatic = self.interval_type() == DateTimeIntervalType::Auto;
        let interval = self.resolved_interval(minimum, maximum, target_tick_count);
        let units = match interval {
            DateTimeIntervalType::Auto | DateTimeIntervalType::Day => days,
            DateTimeIntervalType::Week => days / 7.0,
            DateTimeIntervalType::Month => {
                (max.year() - min.year()) as f64 * 12.0 + (max.month() - min.month()) as f64
            }
            DateTimeIntervalType::Year => (max.year() - min.year()) as f64,
        };
        let limit = if automatic { target } else { 4095.0 };
        let step = (units / limit).ceil().max(1.0) as i32;
        let start = match interval {
            DateTimeIntervalType::Month => DateTime::new(min.year(), min.month(), 1),
            DateTimeIntervalType::Year => DateTime::new(min.year(), 1, 1),
            DateTimeIntervalType::Week => {
                let since_monday = (min.day_of_week().to_i32() + 6) % 7;
                min.date()
                    .try_add_days(-(since_monday as f64))
                    .unwrap_or(DateTime::MIN_VALUE)
            }
            _ => min.date(),
        };
        let mut current = Some(start);
        let mut ticks = Vec::new();
        // At most one aligned tick precedes the lower bound.
        for _ in 0..=4096 {
            let Some(date) = current else { break };
            if date > max {
                break;
            }
            if date >= min {
                ticks.push(from_utc(date));
                if ticks.len() == 4096 {
                    break;
                }
            }
            current = match interval {
                DateTimeIntervalType::Month => date.try_add_months(step),
                DateTimeIntervalType::Year => date.try_add_years(step),
                DateTimeIntervalType::Week => date.try_add_days(step as f64 * 7.0),
                _ => date.try_add_days(step as f64),
            };
        }
        // Deviation (DEVIATIONS.md, WinUI Charts): the published interval types stop at a
        // day, so a daily range without two midnights in it is divided into whole minutes
        // or hours rather than left without ticks.
        if ticks.len() < 2 && matches!(interval, DateTimeIntervalType::Day) {
            return (sub_day_ticks(min, max, target as i64), TickUnit::SubDay);
        }
        (ticks, TickUnit::Calendar(interval))
    }

    /// Formats an axis label using the framework's culture-aware date
    /// formatter. The public WinRT templates used by the source samples are
    /// translated to their framework format equivalents.
    pub fn format_tick(&self, value: DateTimeOffset) -> String {
        self.format_with_default(value, default_format(self.interval_type()))
    }

    fn format_with_default(&self, value: DateTimeOffset, default: &str) -> String {
        let requested = self.label_format();
        let format = match requested.as_str() {
            "" => default,
            "shortdate" => "d",
            "longdate" => "D",
            "month year" => "MMMM yyyy",
            "month" => "MMMM",
            "year" => "yyyy",
            other => other,
        };
        value
            .try_to_string_format(format, &CultureInfo::current_culture())
            .unwrap_or_else(|_| {
                format!(
                    "{:04}-{:02}-{:02}",
                    value.year(),
                    value.month(),
                    value.day()
                )
            })
    }
}

/// The unit the ticks of a range were made with.
#[derive(Clone, Copy)]
enum TickUnit {
    Calendar(DateTimeIntervalType),
    SubDay,
}

/// The default label format of the ticks of a calendar interval.
fn default_format(interval: DateTimeIntervalType) -> &'static str {
    match interval {
        DateTimeIntervalType::Year => "yyyy",
        DateTimeIntervalType::Month => "MMM yyyy",
        _ => "yyyy-MM-dd",
    }
}

fn span_in_days(minimum: DateTimeOffset, maximum: DateTimeOffset) -> f64 {
    (maximum.utc_date_time().ticks() - minimum.utc_date_time().ticks()) as f64
        / TimeSpan::TICKS_PER_DAY as f64
}

/// Ticks at whole minutes or hours (the smallest step of 1, 5, 15 or 30 minutes or 1, 2, 3,
/// 6 or 12 hours that gives at most `target` ticks), or the two bounds when the range is
/// shorter than a minute.
fn sub_day_ticks(min: DateTime, max: DateTime, target: i64) -> Vec<DateTimeOffset> {
    const MINUTE: i64 = TimeSpan::TICKS_PER_MINUTE;
    const HOUR: i64 = TimeSpan::TICKS_PER_HOUR;
    let (low, high) = (min.ticks(), max.ticks());
    let span = high - low;
    let step = [
        MINUTE,
        5 * MINUTE,
        15 * MINUTE,
        30 * MINUTE,
        HOUR,
        2 * HOUR,
        3 * HOUR,
        6 * HOUR,
        12 * HOUR,
    ]
    .into_iter()
    .find(|step| span / step <= target)
    .unwrap_or(12 * HOUR);
    let mut ticks = Vec::new();
    let mut tick = low.div_euclid(step) * step;
    if tick < low {
        tick += step;
    }
    while tick <= high {
        ticks.push(DateTimeOffset::from_ticks(tick, TimeSpan::ZERO));
        tick += step;
    }
    if ticks.len() < 2 {
        ticks = vec![from_utc(min), from_utc(max)];
    }
    ticks
}

fn from_utc(value: DateTime) -> DateTimeOffset {
    DateTimeOffset::from_ticks(value.ticks(), TimeSpan::ZERO)
}

fn validate_bounds(
    minimum: Option<DateTimeOffset>,
    maximum: Option<DateTimeOffset>,
) -> Result<(), ChartError> {
    if matches!((minimum, maximum), (Some(minimum), Some(maximum)) if minimum >= maximum) {
        Err(ChartError::InvalidValue(
            "Minimum must be earlier than Maximum.",
        ))
    } else {
        Ok(())
    }
}
