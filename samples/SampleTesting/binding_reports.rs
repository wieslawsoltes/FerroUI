//! The bindings of a sample that report an error: the events the bindings log (the area
//! `Binding`, from the level the desktop hosts log at) are collected, and the guard of a
//! sample is its list of accepted reports (as `samples/ControlCatalog/tests/binding_reports.rs`).

use ferroui_base::logging::{LogArea, LogEventLevel};
use ferroui_base::reactive::IDisposable;
use ferroui_base::FerroObject;
use ferroui_controls::testing::TestLogSink;
use std::any::Any;
use std::fmt::Display;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

/// A report of a binding: the class of the target, the property, the expression with the
/// point of the error, and the message.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Report {
    pub target: String,
    pub property: String,
    pub expression: String,
    pub message: String,
}

impl Report {
    /// The report of a log event of the area `Binding`.
    pub fn of(source: Option<&dyn Any>, template: &str, values: &[&dyn Display]) -> Report {
        let target = source
            .and_then(|source| source.downcast_ref::<FerroObject>())
            .map_or_else(|| String::from("(unknown)"), |object| object.get_type().name().to_string());
        let values: Vec<String> = values.iter().map(|value| value.to_string()).collect();
        // "An error occurred binding {Property} to {Expression}: {Message}", with
        // "at {ExpressionErrorPoint}" before the message when the error has a point.
        match values.as_slice() {
            [property, expression, message] => {
                Report { target, property: property.clone(), expression: expression.clone(), message: message.clone() }
            }
            [property, expression, point, message] => Report {
                target,
                property: property.clone(),
                expression: format!("{expression} at {point}"),
                message: message.clone(),
            },
            _ => Report { target, property: String::new(), expression: template.to_string(), message: values.join(", ") },
        }
    }

    /// `Target.Property <- expression: message`.
    pub fn line(&self) -> String {
        format!("{}.{} <- {}: {}", self.target, self.property, self.expression, self.message)
    }
}

/// Collects the events of the area `Binding` logged on this thread at the level `Warning`
/// and above until it is dropped. Started before the application of the test, it replaces
/// the log sink of the thread; the sink before it is restored when it is dropped.
pub struct BindingReports {
    reports: Rc<RefCell<Vec<Report>>>,
    sink: Rc<dyn IDisposable>,
}

impl BindingReports {
    pub fn start() -> BindingReports {
        let reports = Rc::new(RefCell::new(Vec::new()));
        let sink = {
            let reports = reports.clone();
            TestLogSink::start(move |level, area, source, template, values| {
                if area != LogArea::BINDING || level < LogEventLevel::Warning {
                    return;
                }
                let report = Report::of(source, template, values);
                reports.borrow_mut().push(report);
            })
        };
        BindingReports { reports, sink }
    }

    /// The reports since the last call, each with its count.
    pub fn take(&self) -> BTreeMap<Report, usize> {
        let mut counted = BTreeMap::new();
        for report in self.reports.borrow_mut().drain(..) {
            *counted.entry(report).or_insert(0) += 1;
        }
        counted
    }
}

impl Drop for BindingReports {
    fn drop(&mut self) {
        self.sink.dispose();
    }
}

/// Fails when the reports `found` (`(where, report line, count)`) differ from the accepted
/// ones: a report that is not accepted, one that is made another number of times, and an
/// accepted one that is no longer made.
#[track_caller]
pub fn assert_accepted(found: &[(String, String, usize)], accepted: &[(&str, &str, usize)]) {
    let mut by_place: BTreeMap<(String, String), usize> = BTreeMap::new();
    for (place, line, count) in found {
        let entry = by_place.entry((place.clone(), line.clone())).or_insert(0);
        *entry = (*entry).max(*count);
    }
    let accepted: BTreeMap<(String, String), usize> =
        accepted.iter().map(|(place, line, count)| ((place.to_string(), line.to_string()), *count)).collect();

    let mut differences = Vec::new();
    for (key, count) in &by_place {
        match accepted.get(key) {
            Some(expected) if expected == count => {}
            Some(expected) => differences.push(format!("{} times and not {expected}: [{}] {}", count, key.0, key.1)),
            None => differences.push(format!("not accepted ({count}): [{}] {}", key.0, key.1)),
        }
    }
    for (key, count) in &accepted {
        if !by_place.contains_key(key) {
            differences.push(format!("no longer reported ({count}): [{}] {}", key.0, key.1));
        }
    }
    assert!(differences.is_empty(), "the binding reports differ from the accepted ones:\n{}", differences.join("\n"));
}
