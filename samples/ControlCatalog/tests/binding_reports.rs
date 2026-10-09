//! The bindings of the catalog that report an error: every page is visited as
//! the drawer selects it (the tour of `catalog_tour.rs`: the main view in a
//! shown window under the Fluent theme, with the layout, the transition of
//! the navigation page and rendered frames after each selection), and the
//! events the bindings log (the area `Binding`, from the level the desktop
//! host logs at) are collected by page.
//!
//! Not a port: the upstream sample has no tests. The guard is the list of
//! [`ACCEPTED`]: the reports the upstream sample makes too, each with its
//! reason. A report that is not in the list fails the test, and so does an
//! entry of the list that is no longer reported.

use super::catalog_tour::Tour;
use ferroui_base::logging::{LogArea, LogEventLevel};
use ferroui_base::reactive::IDisposable;
use ferroui_base::FerroObject;
use ferroui_controls::testing::TestLogSink;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

/// A report of a binding: the class of the target, the property, the
/// expression with the point of the error, and the message.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Report {
    target: String,
    property: String,
    expression: String,
    message: String,
}

impl Report {
    fn line(&self) -> String {
        format!("{}.{} <- {}: {}", self.target, self.property, self.expression, self.message)
    }
}

/// Collects the events of the area `Binding` logged on this thread at the
/// level `Warning` and above until it is dropped.
struct Reports {
    reports: Rc<RefCell<Vec<Report>>>,
    sink: Rc<dyn IDisposable>,
}

impl Reports {
    fn start() -> Reports {
        let reports = Rc::new(RefCell::new(Vec::new()));
        let sink = {
            let reports = reports.clone();
            TestLogSink::start(move |level, area, source, template, values| {
                if area != LogArea::BINDING || level < LogEventLevel::Warning {
                    return;
                }
                let target = source
                    .and_then(|source| source.downcast_ref::<FerroObject>())
                    .map_or_else(|| String::from("(unknown)"), |object| object.get_type().name().to_string());
                let values: Vec<String> = values.iter().map(|value| value.to_string()).collect();
                // "An error occurred binding {Property} to {Expression}: {Message}", with
                // "at {ExpressionErrorPoint}" before the message when the error has a point.
                let report = match values.as_slice() {
                    [property, expression, message] => Report {
                        target,
                        property: property.clone(),
                        expression: expression.clone(),
                        message: message.clone(),
                    },
                    [property, expression, point, message] => Report {
                        target,
                        property: property.clone(),
                        expression: format!("{expression} at {point}"),
                        message: message.clone(),
                    },
                    _ => Report {
                        target,
                        property: String::new(),
                        expression: template.to_string(),
                        message: values.join(", "),
                    },
                };
                reports.borrow_mut().push(report);
            })
        };
        Reports { reports, sink }
    }

    /// The reports since the last call, each with its count.
    fn take(&self) -> BTreeMap<Report, usize> {
        let mut counted = BTreeMap::new();
        for report in self.reports.borrow_mut().drain(..) {
            *counted.entry(report).or_insert(0) += 1;
        }
        counted
    }
}

impl Drop for Reports {
    fn drop(&mut self) {
        self.sink.dispose();
    }
}

/// The reports the catalog is known to make, by page: the header of the page
/// (`(start)` for the main view before a page is selected), the report as
/// [`Report::line`] writes it, and how often it is made in a visit. Each is a
/// report the upstream sample makes too (`GAPS.md`, "Bindings that report an
/// error"): the document of the page binds a member of the selected item of
/// its view model, nothing is selected when the page is shown, and a null in
/// the middle of a path is an error of the binding upstream too
/// (`ExpressionNode.ValidateNonNullSource`, logged by
/// `BindingExpression.OnNodeError` at the level `Warning`).
const ACCEPTED: &[(&str, &str, usize)] = &[
    // Pages/ComboBoxPage.xaml: `{Binding SelectedItem.Name, StringFormat=Selected Item: {0}}`.
    ("ComboBox", "TextBlock.Text <- SelectedItem.Name at SelectedItem: Value is null.", 1),
    // Pages/FlexPage.xaml: the editors of the selected item of the panel.
    ("Flex Panel", "CheckBox.IsChecked <- SelectedItem.IsVisible at SelectedItem: Value is null.", 1),
    ("Flex Panel", "ComboBox.SelectedItem <- SelectedItem.AlignSelfItem at SelectedItem: Value is null.", 1),
    ("Flex Panel", "ComboBox.SelectedItem <- SelectedItem.BasisKind at SelectedItem: Value is null.", 1),
    ("Flex Panel", "ComboBox.SelectedItem <- SelectedItem.HorizontalAlignment at SelectedItem: Value is null.", 1),
    ("Flex Panel", "ComboBox.SelectedItem <- SelectedItem.VerticalAlignment at SelectedItem: Value is null.", 1),
    ("Flex Panel", "NumericUpDown.Value <- SelectedItem.BasisValue at SelectedItem: Value is null.", 1),
    ("Flex Panel", "NumericUpDown.Value <- SelectedItem.Grow at SelectedItem: Value is null.", 1),
    ("Flex Panel", "NumericUpDown.Value <- SelectedItem.Order at SelectedItem: Value is null.", 1),
    ("Flex Panel", "NumericUpDown.Value <- SelectedItem.Shrink at SelectedItem: Value is null.", 1),
];

/// Visits every page of the catalog and returns the reports by page.
fn reports_of_the_tour() -> Vec<(String, String, usize)> {
    let reports = Reports::start();
    let tour = Tour::start();
    let mut found = Vec::new();
    let mut record = |page: &str, reports: BTreeMap<Report, usize>| {
        for (report, count) in reports {
            found.push((page.to_string(), report.line(), count));
        }
    };
    record("(start)", reports.take());
    let home = tour.view_model().home_item();
    for page in tour.pages() {
        let header = page.header();
        // The home page is the page the catalog starts on: its visit is the one after another
        // page, as the visits of the other pages are.
        if Rc::ptr_eq(&page, &home) {
            continue;
        }
        assert!(tour.show(&page), "{header} is not shown");
        record(&header, reports.take());
        assert!(tour.show(&home), "the home page is not shown after {header}");
        record("Home", reports.take());
    }
    drop(tour);
    record("(end)", reports.take());
    found
}

#[test]
fn the_bindings_of_the_catalog_report_only_the_accepted_errors() {
    let found = reports_of_the_tour();
    let mut by_page: BTreeMap<(String, String), usize> = BTreeMap::new();
    for (page, line, count) in &found {
        let entry = by_page.entry((page.clone(), line.clone())).or_insert(0);
        // The home page is visited after every page: the most a visit reports.
        *entry = (*entry).max(*count);
    }
    let accepted: BTreeMap<(String, String), usize> =
        ACCEPTED.iter().map(|(page, line, count)| ((page.to_string(), line.to_string()), *count)).collect();

    let mut differences = Vec::new();
    for (key, count) in &by_page {
        match accepted.get(key) {
            Some(expected) if expected == count => {}
            Some(expected) => differences.push(format!("{} times and not {expected}: [{}] {}", count, key.0, key.1)),
            None => differences.push(format!("not accepted ({count}): [{}] {}", key.0, key.1)),
        }
    }
    for (key, count) in &accepted {
        if !by_page.contains_key(key) {
            differences.push(format!("no longer reported ({count}): [{}] {}", key.0, key.1));
        }
    }
    assert!(
        differences.is_empty(),
        "the binding reports of the tour differ from the accepted ones:\n{}",
        differences.join("\n")
    );
}
