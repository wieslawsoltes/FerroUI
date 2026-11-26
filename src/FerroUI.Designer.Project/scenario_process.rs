//! Run each saved scenario in its own native renderer process.
use crate::{require, OutputStream, ProjectError, Result, TaskHandle, TaskStatus};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const MAX_REPORT_BYTES: usize = 4 * 1024 * 1024;

#[derive(Clone, Debug, Default)]
pub struct Options {
    pub review: bool,
    pub reference: Option<String>,
    pub approval: Option<String>,
}

pub struct ScenarioProcess {
    child: TaskHandle,
    stdout: String,
    stderr: String,
}

impl ScenarioProcess {
    pub fn start(
        program: &Path,
        workspace: &Path,
        scenario: &Path,
        artifacts: &Path,
        options: &Options,
    ) -> Result<Self> {
        let workspace = workspace.canonicalize()?;
        let file = scenario.canonicalize()?;
        require(
            file.starts_with(&workspace) && is_scenario(&file) && file.is_file(),
            "Choose a saved FerroUI scenario in the selected workspace",
        )?;
        let mut command = Command::new(program);
        command
            .current_dir(&workspace)
            .arg("--scenario")
            .arg(&file)
            .arg("--workspace")
            .arg(&workspace)
            .arg("--artifacts")
            .arg(artifacts);
        if let Some(approval) = &options.approval {
            command.args(["--approve-baseline", approval]);
        } else if options.review {
            command.arg("--review-baselines");
        }
        if let Some(reference) = &options.reference {
            command.args(["--baseline-ref", reference]);
        }
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let child = TaskHandle::spawn_with_output_limit(
            &mut command,
            None,
            Default::default(),
            MAX_REPORT_BYTES + 16 * 1024,
        )?;
        Ok(Self {
            child,
            stdout: String::new(),
            stderr: String::new(),
        })
    }

    /// Failed assertions still produce a report. A process error without a
    /// report is returned separately so one broken case cannot stop a suite.
    pub fn poll(&mut self) -> Option<Result<String>> {
        let status = self.child.poll();
        for output in self.child.take_output() {
            match output.stream {
                OutputStream::Stdout => self.stdout.push_str(&output.text),
                OutputStream::Stderr if self.stderr.len() < 8192 => {
                    self.stderr.push_str(&output.text)
                }
                _ => {}
            }
        }
        if self.stdout.len() > MAX_REPORT_BYTES {
            let _ = self.child.cancel();
            return Some(Err(ProjectError(
                "The native scenario report exceeds 4 MiB".into(),
            )));
        }
        match status {
            TaskStatus::Running => None,
            TaskStatus::Exited { .. } if !self.stdout.trim().is_empty() => {
                Some(Ok(std::mem::take(&mut self.stdout)))
            }
            TaskStatus::Exited { code } => {
                Some(Err(ProjectError(if self.stderr.trim().is_empty() {
                    format!("The native scenario exited with {code:?}")
                } else {
                    self.stderr.trim().into()
                })))
            }
            TaskStatus::Failed { message } => Some(Err(ProjectError(message))),
            TaskStatus::Cancelled => Some(Err(ProjectError("Scenario cancelled".into()))),
        }
    }
}

/// Find the shipped helper from Studio, or use an explicitly configured build.
pub fn companion() -> Result<PathBuf> {
    if let Some(program) = std::env::var_os("FERROUI_DESIGNER_PREVIEW") {
        let program = PathBuf::from(program).canonicalize()?;
        require(
            program.is_file(),
            "FERROUI_DESIGNER_PREVIEW must point to the preview executable",
        )?;
        return Ok(program);
    }
    let name = if cfg!(windows) {
        "ferroui-designer-preview.exe"
    } else {
        "ferroui-designer-preview"
    };
    let sibling = std::env::current_exe()?.with_file_name(name);
    Ok(if sibling.is_file() {
        sibling
    } else {
        PathBuf::from(name)
    })
}

pub fn discover(directory: &Path) -> Result<Vec<PathBuf>> {
    let mut directories = vec![directory.to_owned()];
    let mut scenarios = Vec::new();
    while let Some(directory) = directories.pop() {
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            let name = entry.file_name();
            if kind.is_dir() && !matches!(name.to_str(), Some(".git" | "target" | "node_modules")) {
                directories.push(entry.path());
            } else if kind.is_file() && is_scenario(&entry.path()) {
                scenarios.push(entry.path());
            }
        }
    }
    scenarios.sort();
    Ok(scenarios)
}

fn is_scenario(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(".ferroui-scenario.json"))
}
