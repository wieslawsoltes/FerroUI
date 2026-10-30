// Copyright 2022-2026 The Android Open Source Project (reference algorithms).
// Copyright 2026 FerroUI contributors (Rust adaptation).
// SPDX-License-Identifier: Apache-2.0
//! Canonical date selection and locale-ordered numeric input for Material date pickers.
//! Reference: AndroidX DatePicker.kt, DateRangePicker.kt and DateInput.kt at
//! e4d933c98df7baa7ea2b83eea6e086915961e0ff. Dates are calendar days, without a time zone.

use ferroui_base::utilities::{DateTime, DateTimeFormatInfo, DayOfWeek};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MaterialDatePickerMode {
    #[default]
    Picker,
    Input,
}
ferroui_base::ferro_markup_enum!(MaterialDatePickerMode { Picker, Input },
    { namespace: "FerroUI.Themes.Material" });

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MaterialDateSelectionMode {
    #[default]
    Single,
    Range,
}
ferroui_base::ferro_markup_enum!(MaterialDateSelectionMode { Single, Range },
    { namespace: "FerroUI.Themes.Material" });

/// A restorable native selection snapshot. End dates are inclusive. An incomplete range
/// has a start and no end; an invalid or reversed programmatic range clears both ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MaterialDatePickerState {
    pub selected_start: Option<DateTime>,
    pub selected_end: Option<DateTime>,
    pub displayed_month: DateTime,
    pub mode: MaterialDatePickerMode,
    pub selection_mode: MaterialDateSelectionMode,
    pub first_year: i32,
    pub last_year: i32,
}
impl Default for MaterialDatePickerState {
    fn default() -> Self {
        Self {
            selected_start: None,
            selected_end: None,
            displayed_month: month_start(DateTime::today()),
            mode: MaterialDatePickerMode::Picker,
            selection_mode: MaterialDateSelectionMode::Single,
            first_year: 1900,
            last_year: 2100,
        }
    }
}
impl MaterialDatePickerState {
    pub fn canonical_date(&self, date: Option<DateTime>) -> Option<DateTime> {
        date.filter(|d| self.contains_year(d.year())).map(day_start)
    }
    pub fn contains_year(&self, year: i32) -> bool {
        (self.first_year..=self.last_year).contains(&year)
    }
    pub fn set_selection(&mut self, start: Option<DateTime>, end: Option<DateTime>) {
        let start = self.canonical_date(start);
        let end = if self.selection_mode == MaterialDateSelectionMode::Range {
            self.canonical_date(end)
        } else {
            None
        };
        if start.is_some() && (end.is_none() || start <= end) {
            self.selected_start = start;
            self.selected_end = end;
        } else {
            self.selected_start = None;
            self.selected_end = None;
        }
    }
    /// Call only after checking the date and year selection policy. Range selection does
    /// not require every intervening day to be selectable, matching the native picker.
    pub fn select_date(&mut self, date: DateTime) {
        let Some(date) = self.canonical_date(Some(date)) else {
            return;
        };
        if self.selection_mode == MaterialDateSelectionMode::Single {
            self.set_selection(Some(date), None);
        } else if self.selected_end.is_some()
            || self.selected_start.is_none()
            || self.selected_start.is_some_and(|start| date < start)
        {
            self.set_selection(Some(date), None);
        } else {
            self.set_selection(self.selected_start, Some(date));
        }
    }
    pub fn set_displayed_month(&mut self, date: DateTime) {
        if self.contains_year(date.year()) {
            self.displayed_month = month_start(date);
        }
    }
    pub fn set_mode(&mut self, mode: MaterialDatePickerMode) {
        if let Some(date) = self.selected_start {
            self.set_displayed_month(date);
        }
        self.mode = mode;
    }
    pub fn set_year_range(&mut self, first: i32, last: i32) {
        self.first_year = first.clamp(1, 9999);
        self.last_year = last.clamp(self.first_year, 9999);
        self.set_selection(self.selected_start, self.selected_end);
        if !self.contains_year(self.displayed_month.year()) {
            self.displayed_month = DateTime::new(
                self.displayed_month
                    .year()
                    .clamp(self.first_year, self.last_year),
                self.displayed_month.month(),
                1,
            );
        }
    }
    pub fn is_in_range(&self, date: DateTime) -> bool {
        self.selected_start
            .zip(self.selected_end)
            .is_some_and(|(start, end)| start <= date && date <= end)
    }
    pub fn is_complete(&self) -> bool {
        self.selected_start.is_some()
            && (self.selection_mode == MaterialDateSelectionMode::Single
                || self.selected_end.is_some())
    }
    pub fn month_index(&self, month: DateTime) -> i32 {
        (month.year() - self.first_year) * 12 + month.month() - 1
    }
    pub fn month_count(&self) -> i32 {
        (self.last_year - self.first_year + 1) * 12
    }
    pub fn month_at(&self, index: i32) -> DateTime {
        let index = index.clamp(0, self.month_count() - 1);
        DateTime::new(self.first_year + index / 12, index % 12 + 1, 1)
    }
}

pub(crate) fn day_start(date: DateTime) -> DateTime {
    DateTime::new(date.year(), date.month(), date.day())
}
pub(crate) fn month_start(date: DateTime) -> DateTime {
    DateTime::new(date.year(), date.month(), 1)
}
pub(crate) fn month_leading_days(month: DateTime, first_day: DayOfWeek) -> i32 {
    (month_start(month).day_of_week().to_i32() - first_day.to_i32() + 7) % 7
}

/// Native date input always uses two-digit month/day and four-digit year in the
/// culture's order. Eight underlying digits are displayed with two delimiters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaterialDateInputFormat {
    order: [char; 3],
    delimiter: char,
}
impl MaterialDateInputFormat {
    pub fn for_culture(info: &DateTimeFormatInfo) -> Self {
        let mut order = Vec::new();
        let mut delimiter = None;
        let mut quoted = false;
        for c in info.short_date_pattern().chars() {
            if c == '\'' || c == '"' {
                quoted = !quoted;
                continue;
            }
            if quoted {
                continue;
            }
            let part = match c {
                'M' => Some('M'),
                'd' => Some('d'),
                'y' => Some('y'),
                _ => None,
            };
            if let Some(part) = part {
                if !order.contains(&part) {
                    order.push(part);
                }
            } else if matches!(c, '/' | '.' | '-') && delimiter.is_none() {
                delimiter = Some(c);
            }
        }
        Self {
            order: order.try_into().unwrap_or(['M', 'd', 'y']),
            delimiter: delimiter.unwrap_or('/'),
        }
    }
    pub fn pattern(&self) -> String {
        self.order
            .iter()
            .map(|c| match c {
                'y' => "YYYY",
                'M' => "MM",
                _ => "DD",
            })
            .collect::<Vec<_>>()
            .join(&self.delimiter.to_string())
    }
    pub fn digits(&self, date: DateTime) -> String {
        self.order
            .iter()
            .map(|c| match c {
                'y' => format!("{:04}", date.year()),
                'M' => format!("{:02}", date.month()),
                _ => format!("{:02}", date.day()),
            })
            .collect()
    }
    pub fn format_digits(&self, digits: &str) -> String {
        let mut text = String::new();
        let first = if self.order[0] == 'y' { 4 } else { 2 };
        let second = first + if self.order[1] == 'y' { 4 } else { 2 };
        for (index, digit) in digits.chars().take(8).enumerate() {
            if index == first || index == second {
                text.push(self.delimiter);
            }
            text.push(digit);
        }
        // OutputTransformation also inserts a delimiter immediately after a completed part.
        if digits.len() == first || digits.len() == second {
            text.push(self.delimiter);
        }
        text
    }
    pub fn extract_digits(&self, text: &str) -> Option<String> {
        if text
            .chars()
            .any(|c| !c.is_ascii_digit() && c != self.delimiter)
        {
            return None;
        }
        let digits: String = text.chars().filter(char::is_ascii_digit).collect();
        (digits.len() <= 8).then_some(digits)
    }
    pub fn parse_digits(&self, digits: &str) -> Option<DateTime> {
        if digits.len() != 8 || !digits.bytes().all(|v| v.is_ascii_digit()) {
            return None;
        }
        let mut index = 0;
        let (mut year, mut month, mut day) = (0, 0, 0);
        for part in self.order {
            let length = if part == 'y' { 4 } else { 2 };
            let value: i32 = digits[index..index + length].parse().ok()?;
            match part {
                'y' => year = value,
                'M' => month = value,
                _ => day = value,
            }
            index += length;
        }
        DateTime::try_new(year, month, day)
    }
}
