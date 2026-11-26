//! Saved scenarios use the same command and result format in editors and CI.
use crate::scenario;
use crate::scenario_review::{Action, Options};
use ferroui_designer_project::scenario_process::{self, ScenarioProcess};
use ferroui_designer_tools::scenario::{ScenarioSuiteEntry, ScenarioSuiteReport};
use std::ffi::OsString;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub(crate) fn command(suite: bool) -> io::Result<()> {
    let mut args = std::env::args_os().skip(2);
    let input = PathBuf::from(next(
        &mut args,
        "a FerroUI scenario file or suite directory",
    )?)
    .canonicalize()?;
    let mut workspace = if suite {
        input.clone()
    } else {
        input
            .parent()
            .ok_or_else(|| io::Error::other("The scenario needs a containing folder"))?
            .into()
    };
    let mut artifacts = std::env::temp_dir()
        .join("ferroui-scenario-results")
        .join(std::process::id().to_string());
    let mut action = Action::Verify;
    let mut reference = None;
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--artifacts") => artifacts = next(&mut args, "an artifact directory")?.into(),
            Some("--workspace") => workspace = PathBuf::from(next(&mut args, "the workspace directory")?).canonicalize()?,
            Some("--baseline-ref") => reference = Some(string(next(&mut args, "a Git reference")?)?),
            Some("--accept-baselines") if matches!(action, Action::Verify) => action = Action::Accept,
            Some("--review-baselines") if matches!(action, Action::Verify) => action = Action::Review,
            Some("--approve-baseline") if matches!(action, Action::Verify) => {
                let receipt = string(next(&mut args, "the reviewed baseline identifier")?)?;
                if receipt.len() != 16 || !receipt.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                    return Err(invalid("Pass the approval identifier from the current scenario review"));
                }
                action = Action::Approve(receipt);
            }
            _ => return Err(invalid("Use --workspace, --artifacts, --baseline-ref, or one baseline review or acceptance option")),
        }
    }
    if !input.starts_with(&workspace) || !workspace.is_dir() {
        return Err(invalid(
            "The scenario must belong to the selected workspace",
        ));
    }
    if (suite && matches!(action, Action::Accept | Action::Approve(_)))
        || (reference.is_some() && matches!(action, Action::Accept))
    {
        return Err(invalid(
            "Approve a reviewed baseline from a single scenario",
        ));
    }
    if reference.is_some() && matches!(action, Action::Verify) {
        action = Action::Review;
    }
    let artifacts = std::path::absolute(artifacts)?;
    let options = Options { action, reference };
    let mut output = ferroui_mcp::reserve_stdout()?;
    let passed = if suite {
        let report = run_suite(&input, &workspace, &artifacts, &options)?;
        crate::scenario_suite_report::write(&artifacts, &workspace, &report)?;
        serde_json::to_writer(&mut output, &report)?;
        report.passed
    } else {
        let report =
            scenario::run(&input, &workspace, &artifacts, &options).map_err(io::Error::other)?;
        serde_json::to_writer(&mut output, &report)?;
        report.approved.is_some() || report.passed
    };
    output.write_all(b"\n")?;
    output.flush()?;
    if passed {
        Ok(())
    } else {
        Err(io::Error::other("Scenario assertions failed"))
    }
}

fn run_suite(
    input: &Path,
    workspace: &Path,
    artifacts: &Path,
    options: &Options,
) -> io::Result<ScenarioSuiteReport> {
    let scenarios = scenario_process::discover(input).map_err(io::Error::other)?;
    if scenarios.is_empty() {
        return Err(invalid("The suite contains no saved FerroUI scenarios"));
    }
    let executable = std::env::current_exe()?;
    let launch = scenario_process::Options {
        review: matches!(options.action, Action::Review),
        reference: options.reference.clone(),
        approval: None,
    };
    let mut passed = true;
    let mut results = Vec::with_capacity(scenarios.len());
    for (index, path) in scenarios.into_iter().enumerate() {
        let directory = artifacts.join(format!("{:03}", index + 1));
        let result = ScenarioProcess::start(&executable, workspace, &path, &directory, &launch)
            .and_then(|mut process| loop {
                if let Some(result) = process.poll() {
                    break result;
                }
                std::thread::sleep(Duration::from_millis(15));
            });
        let (report, error) = match result
            .map_err(|error| error.to_string())
            .and_then(|result| {
                serde_json::from_str::<ferroui_designer_tools::scenario::ScenarioReport>(&result)
                    .map_err(|error| format!("Invalid scenario report: {error}"))
            }) {
            Ok(report) => {
                passed &= report.passed;
                (Some(report), None)
            }
            Err(error) => {
                passed = false;
                (None, Some(error))
            }
        };
        results.push(ScenarioSuiteEntry {
            path,
            report,
            error,
        });
    }
    Ok(ScenarioSuiteReport {
        passed,
        scenarios: results,
        review_report: artifacts.join("review.html"),
    })
}

fn next(args: &mut impl Iterator<Item = OsString>, name: &str) -> io::Result<OsString> {
    args.next().ok_or_else(|| invalid(format!("Pass {name}")))
}

fn string(value: OsString) -> io::Result<String> {
    value
        .into_string()
        .map_err(|_| invalid("Use UTF-8 for Git references and review identifiers"))
}

fn invalid(message: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}
