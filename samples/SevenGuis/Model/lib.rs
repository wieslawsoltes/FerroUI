//! seven-guis-model
//!
//! What the ports of the 7GUIs tasks (<https://eugenkiss.github.io/7guis/tasks>)
//! share apart from their user interface: the conversions, the parsing of
//! dates, the filter of the CRUD list, the history of the circle drawer, the
//! formulas of the cells, the reactive state of the circle drawer and of the
//! cells, the scenarios the tests run (`seven-guis-tests`), and the window a
//! task runs in ([`run`]). Each task is a crate twice: `Builders/<Task>` writes
//! its user interface with the builders of `ferroui-view`, `Macros/<Task>`
//! with its `rsx!` macro, so the two differ only in how the user interface is
//! written.

pub mod cells;
#[cfg(feature = "scenarios")]
pub mod scenarios;
mod shots;
pub mod state;

pub use shots::{capture_if_asked, run};

/// Temperature converter: degrees Celsius to Fahrenheit.
pub fn celsius_to_fahrenheit(celsius: f64) -> f64 {
    celsius * 9.0 / 5.0 + 32.0
}

/// Temperature converter: degrees Fahrenheit to Celsius.
pub fn fahrenheit_to_celsius(fahrenheit: f64) -> f64 {
    (fahrenheit - 32.0) * 5.0 / 9.0
}

/// A temperature as the converter shows it: rounded to whole degrees, as the
/// task asks for in its example.
pub fn format_degrees(value: f64) -> String {
    format!("{}", value.round() as i64)
}

/// A temperature typed by the user, or `None` when it is not a number.
pub fn parse_degrees(text: &str) -> Option<f64> {
    text.trim().parse::<f64>().ok().filter(|value| value.is_finite())
}

/// A date of the flight booker: day, month and year.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date {
    /// The year (compared first).
    pub year: i32,
    /// The month, 1 to 12.
    pub month: u32,
    /// The day, 1 to the length of the month.
    pub day: u32,
}

impl Date {
    /// Parses `dd.mm.yyyy`; `None` when the text is not a valid date.
    pub fn parse(text: &str) -> Option<Date> {
        let mut parts = text.trim().split('.');
        let day: u32 = parts.next()?.parse().ok()?;
        let month: u32 = parts.next()?.parse().ok()?;
        let year: i32 = parts.next()?.parse().ok()?;
        if parts.next().is_some() || !(1..=12).contains(&month) || day == 0 || day > days_in_month(year, month) {
            return None;
        }
        Some(Date { year, month, day })
    }
}

impl std::fmt::Display for Date {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:02}.{:02}.{:04}", self.day, self.month, self.year)
    }
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        2 if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// The kind of flight of the flight booker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlightKind {
    /// One way: the return date is not used.
    OneWay,
    /// A return flight: the return date is not before the start date.
    Return,
}

impl FlightKind {
    /// The kinds, in the order of the combo box.
    pub const ALL: [FlightKind; 2] = [FlightKind::OneWay, FlightKind::Return];

    /// The text of the combo box.
    pub fn label(self) -> &'static str {
        match self {
            FlightKind::OneWay => "one-way flight",
            FlightKind::Return => "return flight",
        }
    }
}

/// Whether the flight booker can book: the dates the kind uses are valid and
/// in order.
pub fn can_book(kind: FlightKind, start: Option<Date>, back: Option<Date>) -> bool {
    match (kind, start, back) {
        (FlightKind::OneWay, Some(_), _) => true,
        (FlightKind::Return, Some(start), Some(back)) => back >= start,
        _ => false,
    }
}

/// The message of a booking.
pub fn booking_message(kind: FlightKind, start: Date, back: Option<Date>) -> String {
    match (kind, back) {
        (FlightKind::Return, Some(back)) => format!("You have booked a return flight on {start}, returning on {back}."),
        _ => format!("You have booked a one-way flight on {start}."),
    }
}

/// A person of the CRUD task.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Person {
    /// The identity of the person in the list.
    pub id: u32,
    /// The first name.
    pub name: String,
    /// The surname, which the filter matches.
    pub surname: String,
}

impl Person {
    /// The text of the person in the list: `Surname, Name`.
    pub fn label(&self) -> String {
        format!("{}, {}", self.surname, self.name)
    }
}

/// The people whose surname starts with `prefix` (ignoring case), in order.
pub fn filter_people(people: &[Person], prefix: &str) -> Vec<Person> {
    let prefix = prefix.to_lowercase();
    people.iter().filter(|person| person.surname.to_lowercase().starts_with(&prefix)).cloned().collect()
}

/// The people of the CRUD task when it starts.
pub fn initial_people() -> Vec<Person> {
    vec![
        Person { id: 0, name: "Hans".into(), surname: "Emil".into() },
        Person { id: 1, name: "Max".into(), surname: "Mustermann".into() },
        Person { id: 2, name: "Roman".into(), surname: "Tisch".into() },
    ]
}

/// A circle of the circle drawer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Circle {
    /// The identity of the circle.
    pub id: u32,
    /// The center, horizontally.
    pub x: f64,
    /// The center, vertically.
    pub y: f64,
    /// The diameter.
    pub diameter: f64,
}

/// The circle nearest to a point among those that contain it.
pub fn circle_at(circles: &[Circle], x: f64, y: f64) -> Option<u32> {
    circles
        .iter()
        .map(|circle| (circle.id, (circle.x - x).hypot(circle.y - y), circle.diameter / 2.0))
        .filter(|(_, distance, radius)| distance <= radius)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(id, _, _)| id)
}

/// The states of a value with undo and redo: every change is a new state, an
/// undo goes back one, a redo forward one, and a change after an undo drops
/// the states it undid.
#[derive(Clone, Debug, PartialEq)]
pub struct History<T> {
    states: Vec<T>,
    current: usize,
}

impl<T: Clone> History<T> {
    /// A history whose only state is `initial`.
    pub fn new(initial: T) -> Self {
        Self { states: vec![initial], current: 0 }
    }

    /// The current state.
    pub fn current(&self) -> &T {
        &self.states[self.current]
    }

    /// Makes `state` the current state.
    pub fn push(&mut self, state: T) {
        self.states.truncate(self.current + 1);
        self.states.push(state);
        self.current += 1;
    }

    /// Whether there is a state to go back to.
    pub fn can_undo(&self) -> bool {
        self.current > 0
    }

    /// Whether there is an undone state to go forward to.
    pub fn can_redo(&self) -> bool {
        self.current + 1 < self.states.len()
    }

    /// Goes back one state.
    pub fn undo(&mut self) {
        if self.can_undo() {
            self.current -= 1;
        }
    }

    /// Goes forward one state.
    pub fn redo(&mut self) {
        if self.can_redo() {
            self.current += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temperatures_convert_both_ways() {
        assert_eq!(celsius_to_fahrenheit(100.0), 212.0);
        assert_eq!(fahrenheit_to_celsius(212.0), 100.0);
        assert_eq!(format_degrees(fahrenheit_to_celsius(5.0)), "-15");
        assert_eq!(parse_degrees(" 12.5 "), Some(12.5));
        assert_eq!(parse_degrees("12a"), None);
    }

    #[test]
    fn dates_are_parsed_and_checked() {
        assert_eq!(Date::parse("27.03.2014"), Some(Date { year: 2014, month: 3, day: 27 }));
        assert_eq!(Date::parse("29.02.2015"), None);
        assert_eq!(Date::parse("29.02.2016").map(|d| d.to_string()).as_deref(), Some("29.02.2016"));
        assert_eq!(Date::parse("1.13.2014"), None);
        let (start, back) = (Date::parse("27.03.2014"), Date::parse("26.03.2014"));
        assert!(can_book(FlightKind::OneWay, start, back));
        assert!(!can_book(FlightKind::Return, start, back));
        assert!(can_book(FlightKind::Return, start, start));
    }

    #[test]
    fn people_are_filtered_by_surname() {
        let people = initial_people();
        assert_eq!(filter_people(&people, "m").len(), 1);
        assert_eq!(filter_people(&people, "").len(), 3);
    }

    #[test]
    fn the_nearest_circle_containing_a_point_is_found() {
        let circles = [
            Circle { id: 1, x: 10.0, y: 10.0, diameter: 30.0 },
            Circle { id: 2, x: 20.0, y: 10.0, diameter: 30.0 },
        ];
        assert_eq!(circle_at(&circles, 18.0, 10.0), Some(2));
        assert_eq!(circle_at(&circles, 100.0, 100.0), None);
    }

    #[test]
    fn history_undoes_and_redoes() {
        let mut history = History::new(0);
        history.push(1);
        history.push(2);
        history.undo();
        assert_eq!(*history.current(), 1);
        history.redo();
        assert_eq!(*history.current(), 2);
        history.undo();
        history.push(5);
        assert!(!history.can_redo());
        assert_eq!(*history.current(), 5);
    }
}
