//! The entry point: the ignored test `run_comparison`, which measures the benchmarks the
//! environment selects with the harness of the framework's benchmarks and prints one table.
//!
//! ```sh
//! cargo test --release --lib run_comparison -- --ignored --nocapture --test-threads=1
//! FERROUI_BENCH_FILTER=vertical_small_scrolls cargo test --release --lib run_comparison -- --ignored --nocapture --test-threads=1
//! ```

use crate::TargetKind;
use ferroui_benchmarks::harness::{self, Descriptor, Measurement, Options, Registry};
use std::fmt::Write as _;

/// Measures the benchmarks the environment selects (`FERROUI_BENCH_FILTER` and the other
/// variables of the harness) and prints the comparison table.
pub fn run() {
    let options = Options::from_environment();
    let mut registry = Registry::new();
    crate::register(&mut registry);
    let descriptors: Vec<Descriptor> = registry.descriptors().cloned().collect();
    let selected = descriptors.iter().filter(|descriptor| options.matches(descriptor)).count();

    if options.list {
        for descriptor in descriptors.iter().filter(|descriptor| options.matches(descriptor)) {
            println!("{}", descriptor.full_name());
        }
        println!("{selected} of {} benchmarks", descriptors.len());
        return;
    }

    println!(
        "{selected} of {} benchmarks, {} iterations after {} of warm-up{}",
        descriptors.len(),
        options.iterations,
        options.warmup,
        if cfg!(debug_assertions) { " (a build without optimisation: the times mean nothing)" } else { "" }
    );
    let results = harness::measure_selected(crate::register, &options);
    println!();
    print!("{}", format_comparison(&results));

    let failed = results.iter().filter(|(_, result)| result.is_err()).count();
    assert!(failed == 0, "{failed} benchmarks failed");
}

/// A row of the table: a scenario with its parameters, and what each target measured.
struct ComparisonRow<'a> {
    scenario: &'static str,
    parameters: &'a str,
    cells: [Option<&'a Result<Measurement, String>>; 4],
}

fn column_of(class: &str) -> Option<usize> {
    TargetKind::ALL.iter().position(|target| target.class() == class)
}

fn format_cell(cell: Option<&Result<Measurement, String>>) -> String {
    match cell {
        None => "n/a".to_string(),
        Some(Err(_)) => "FAILED".to_string(),
        Some(Ok(measurement)) => {
            let mut text = harness::format_time(measurement.mean());
            if let (Some(bytes), Some(blocks)) = (measurement.allocated_bytes, measurement.allocations) {
                let _ = write!(text, " ({}, {blocks:.1} blocks)", harness::format_bytes(bytes));
            }
            text
        }
    }
}

fn mean(cell: Option<&Result<Measurement, String>>) -> Option<f64> {
    match cell {
        Some(Ok(measurement)) if measurement.mean() > 0.0 => Some(measurement.mean()),
        _ => None,
    }
}

/// The table: one row per scenario and parameters, the mean time of an operation of each
/// target, and the time of each GPUI target over FerroUI's (its layout and render target when it
/// has one, its layout target otherwise). A ratio above 1 means FerroUI took less time.
fn format_comparison(results: &[(Descriptor, Result<Measurement, String>)]) -> String {
    let mut rows: Vec<ComparisonRow<'_>> = Vec::new();
    for (descriptor, result) in results {
        let Some(column) = column_of(descriptor.class) else { continue };
        let position = rows
            .iter()
            .position(|row| row.scenario == descriptor.name && row.parameters == descriptor.parameters);
        let index = match position {
            Some(index) => index,
            None => {
                rows.push(ComparisonRow { scenario: descriptor.name, parameters: &descriptor.parameters, cells: [None; 4] });
                rows.len() - 1
            }
        };
        rows[index].cells[column] = Some(result);
    }

    let mut header: Vec<String> = vec!["Scenario".into(), "Parameters".into()];
    header.extend(TargetKind::ALL.iter().map(|target| target.heading().to_string()));
    header.push("gpui-component / FerroUI".into());
    header.push("uniform_list / FerroUI".into());

    let mut lines: Vec<Vec<String>> = Vec::with_capacity(rows.len());
    for row in &rows {
        let mut line = vec![row.scenario.to_string(), row.parameters.to_string()];
        line.extend(row.cells.iter().map(|cell| format_cell(*cell)));
        let ferroui = mean(row.cells[1]).or_else(|| mean(row.cells[0]));
        for gpui in [row.cells[2], row.cells[3]] {
            line.push(match (mean(gpui), ferroui) {
                (Some(gpui), Some(ferroui)) => format!("{:.2}", gpui / ferroui),
                _ => "n/a".to_string(),
            });
        }
        lines.push(line);
    }

    let widths: Vec<usize> = (0..header.len())
        .map(|column| lines.iter().map(|line| line[column].len()).max().unwrap_or(0).max(header[column].len()))
        .collect();

    let mut out = String::new();
    let mut write_line = |cells: &[String]| {
        let mut line = String::new();
        for (column, cell) in cells.iter().enumerate() {
            let width = widths[column];
            if column < 2 {
                let _ = write!(line, "| {cell:<width$} ");
            } else {
                let _ = write!(line, "| {cell:>width$} ");
            }
        }
        let _ = writeln!(out, "{line}|");
    };
    write_line(&header);
    let rule: Vec<String> = widths
        .iter()
        .enumerate()
        .map(|(column, width)| if column < 2 { "-".repeat(*width) } else { format!("{}:", "-".repeat(width.saturating_sub(1))) })
        .collect();
    write_line(&rule);
    for line in &lines {
        write_line(line);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "measures every benchmark the environment selects; run it alone in an optimised build"]
    fn run_comparison() {
        super::run();
    }

    #[test]
    fn the_table_has_a_row_per_scenario_and_a_ratio_to_ferroui() {
        let descriptor = |class: &'static str| Descriptor {
            category: crate::CATEGORY,
            class,
            name: "vertical_small_scrolls",
            parameters: "Rows=10000, Columns=12, Text=Synthetic".into(),
            baseline: false,
            operations_per_invoke: 10_000,
            min_iteration_time: None,
            has_iteration_setup: true,
        };
        let measurement = |nanoseconds: f64| Measurement {
            invocations: 1,
            nanoseconds: vec![nanoseconds],
            allocated_bytes: None,
            allocations: None,
            instructions: None,
            cycles: None,
        };
        let results = vec![
            (descriptor("FerroUiLayout"), Ok(measurement(1_000.0))),
            (descriptor("FerroUiRender"), Ok(measurement(2_000.0))),
            (descriptor("GpuiComponent"), Ok(measurement(5_000.0))),
        ];
        let table = format_comparison(&results);
        let lines: Vec<&str> = table.lines().collect();
        assert_eq!(lines.len(), 3, "{table}");
        assert!(lines[2].contains("vertical_small_scrolls"), "{table}");
        assert!(lines[2].contains("2.50"), "the ratio is to the render target: {table}");
        assert!(lines[2].contains("n/a"), "the missing target: {table}");
    }
}
