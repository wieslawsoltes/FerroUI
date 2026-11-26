//! Studio owns the same isolated helper used by VS Code and the CLI.
use super::DesignerWorkspace;
use ferroui_designer_project::scenario_process::{self, Options, ScenarioProcess};
use ferroui_designer_tools::scenario::ScenarioReport;
use std::{
    fs, io,
    path::{Path, PathBuf},
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
};

#[derive(Clone)]
pub(crate) enum ScenarioProgress {
    Running,
    Complete(Rc<ScenarioReport>),
    Failed(String),
}

#[derive(Default)]
pub(super) struct ScenarioRuns {
    next: u64,
    artifacts: Option<PathBuf>,
    active: Option<Run>,
}

struct Run {
    id: u64,
    process: Option<ScenarioProcess>,
    progress: ScenarioProgress,
}

impl DesignerWorkspace {
    fn scenario_root(&self) -> Result<PathBuf, String> {
        self.solution()
            .ok_or("Open the project folder containing the saved scenario")?
            .root
            .canonicalize()
            .map_err(|error| error.to_string())
    }

    pub(crate) fn list_scenarios(&self) -> Result<Vec<String>, String> {
        let root = self.scenario_root()?;
        let files = scenario_process::discover(&root).map_err(|error| error.to_string())?;
        Ok(files
            .into_iter()
            .filter_map(|path| path.strip_prefix(&root).ok()?.to_str().map(str::to_owned))
            .collect())
    }

    pub(crate) fn start_scenario(&self, path: &str, options: &Options) -> Result<u64, String> {
        let root = self.scenario_root()?;
        if Path::new(path).is_absolute() {
            return Err("Use a scenario path relative to the project folder".into());
        }
        self.scenarios
            .borrow_mut()
            .start(&root, &root.join(path), options)
    }

    pub(crate) fn scenario_progress(&self, id: u64) -> Result<ScenarioProgress, String> {
        Ok(self.scenarios.borrow_mut().run(id)?.poll())
    }

    pub(crate) fn cancel_scenario(&self, id: u64) -> Result<(), String> {
        let mut runs = self.scenarios.borrow_mut();
        let run = runs.run(id)?;
        if run.process.take().is_some() {
            run.progress = ScenarioProgress::Failed("Scenario cancelled".into());
        }
        Ok(())
    }
}

impl ScenarioRuns {
    fn start(&mut self, root: &Path, scenario: &Path, options: &Options) -> Result<u64, String> {
        if self
            .active
            .as_mut()
            .is_some_and(|run| matches!(run.poll(), ScenarioProgress::Running))
        {
            return Err("A FerroUI scenario is already running in this Studio workspace".into());
        }
        let binary = scenario_process::companion().map_err(|error| error.to_string())?;
        if self.artifacts.is_none() {
            self.artifacts = Some(temporary_directory().map_err(|error| error.to_string())?);
        }
        let id = self.next + 1;
        let artifacts = self
            .artifacts
            .as_ref()
            .expect("scenario artifact root")
            .join(id.to_string());
        let process = ScenarioProcess::start(&binary, root, scenario, &artifacts, options)
            .map_err(|error| format!("Start the FerroUI scenario helper: {error}"))?;
        self.next = id;
        self.active = Some(Run {
            id,
            process: Some(process),
            progress: ScenarioProgress::Running,
        });
        Ok(id)
    }

    fn run(&mut self, id: u64) -> Result<&mut Run, String> {
        self.active
            .as_mut()
            .filter(|run| run.id == id)
            .ok_or_else(|| "Choose the current Studio scenario run".into())
    }
}

impl Run {
    fn poll(&mut self) -> ScenarioProgress {
        if let Some(result) = self.process.as_mut().and_then(ScenarioProcess::poll) {
            self.process = None;
            self.progress = match result.map_err(|error| error.to_string()).and_then(|json| {
                serde_json::from_str(&json)
                    .map_err(|error| format!("Invalid scenario report: {error}"))
            }) {
                Ok(report) => ScenarioProgress::Complete(Rc::new(report)),
                Err(error) => ScenarioProgress::Failed(error),
            };
        }
        self.progress.clone()
    }
}

impl Drop for ScenarioRuns {
    fn drop(&mut self) {
        self.active.take();
        if let Some(artifacts) = self.artifacts.take() {
            let _ = fs::remove_dir_all(artifacts);
        }
    }
}

fn temporary_directory() -> io::Result<PathBuf> {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    loop {
        let path = std::env::temp_dir().join(format!(
            "ferroui-studio-scenarios-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        match fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
}
