//! Process restart is a two-phase operation: successful build, then acknowledged startup.
//! Pump this supervisor off the UI thread. Capture application state on the UI thread only
//! after `poll_build` returns a ticket, and pass that snapshot to `launch`.
use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    process::{Child, Command},
    time::{Duration, Instant},
};
use tempfile::TempDir;

const STATE_ENV: &str = "FERROUI_RESTART_STATE";
const READY_ENV: &str = "FERROUI_RESTART_READY";
const FORMAT_VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WindowPlacement {
    pub key: String,
    pub position: [i32; 2],
    pub size: [f64; 2],
    pub maximized: bool,
}
/// Application data is explicitly provided and versioned, never inferred from view models.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RestartState {
    pub format_version: u32,
    pub application: String,
    pub application_version: u32,
    pub windows: Vec<WindowPlacement>,
    pub navigation: Option<String>,
    pub application_state: serde_json::Value,
}
impl RestartState {
    pub fn new(
        application: String,
        application_version: u32,
        application_state: serde_json::Value,
    ) -> Self {
        Self {
            format_version: FORMAT_VERSION,
            application,
            application_version,
            windows: Vec::new(),
            navigation: None,
            application_state,
        }
    }
    fn validate(&self) -> io::Result<()> {
        if self.format_version != FORMAT_VERSION || self.application.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Unsupported restart state envelope",
            ));
        }
        let mut keys = std::collections::BTreeSet::new();
        for window in &self.windows {
            if window.key.is_empty()
                || !keys.insert(&window.key)
                || window
                    .size
                    .iter()
                    .any(|value| !value.is_finite() || *value <= 0.0)
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Invalid or ambiguous window placement",
                ));
            }
        }
        Ok(())
    }
}

/// Startup acknowledgement is explicit: call only after restoring state and initializing the UI.
pub struct RestartStartup {
    pub state: RestartState,
    ready: std::path::PathBuf,
}
impl RestartStartup {
    pub fn from_environment(application: &str, version: u32) -> io::Result<Option<Self>> {
        let (state, ready) = (std::env::var_os(STATE_ENV), std::env::var_os(READY_ENV));
        match (state, ready) {
            (None, None) => Ok(None),
            (Some(state), Some(ready)) => {
                let state: RestartState = serde_json::from_slice(&fs::read(state)?)?;
                state.validate()?;
                if state.application != application || state.application_version != version {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Application restart state version mismatch",
                    ));
                }
                Ok(Some(Self {
                    state,
                    ready: ready.into(),
                }))
            }
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Incomplete restart environment",
            )),
        }
    }
    pub fn acknowledge(self) -> io::Result<()> {
        let staged = self.ready.with_extension("staged");
        fs::write(&staged, b"ferroui-ready-v1")?;
        fs::rename(staged, self.ready)
    }
}

struct Build {
    revision: u64,
    child: Child,
}
struct Candidate {
    revision: u64,
    child: Child,
    directory: TempDir,
    deadline: Instant,
}
/// Opaque proof that this supervisor observed a successful build for the current revision.
#[derive(Debug)]
pub struct BuiltRevision {
    revision: u64,
    owner: u64,
}
#[derive(Debug, PartialEq, Eq)]
pub enum ReplacementStatus {
    Waiting,
    Ready,
    Failed(String),
    Superseded,
}

pub struct RestartSupervisor {
    current: Child,
    latest_revision: u64,
    owner: u64,
    build: Option<Build>,
    candidate: Option<Candidate>,
}
impl RestartSupervisor {
    pub fn new(current: Child) -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        Self {
            current,
            latest_revision: 0,
            owner: NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            build: None,
            candidate: None,
        }
    }
    pub fn current_id(&self) -> u32 {
        self.current.id()
    }
    pub fn current_is_running(&mut self) -> io::Result<bool> {
        Ok(self.current.try_wait()?.is_none())
    }
    /// Marks all earlier build/startup work stale without interrupting the running application.
    pub fn supersede(&mut self, revision: u64) -> io::Result<()> {
        if revision <= self.latest_revision {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Revision must increase",
            ));
        }
        self.latest_revision = revision;
        Ok(())
    }
    /// Command must build into an isolated output path; never overwrite the running executable.
    /// Configure stdout/stderr as files or inherited streams, not unconsumed pipes.
    pub fn build(&mut self, revision: u64, command: &mut Command) -> io::Result<()> {
        if self.build.is_some() || self.candidate.is_some() {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "Restart work is still pending",
            ));
        }
        self.supersede(revision)?;
        self.build = Some(Build {
            revision,
            child: command.spawn()?,
        });
        Ok(())
    }
    /// None means the build is still running. Failure and supersession never retire the old process.
    pub fn poll_build(&mut self) -> io::Result<Option<BuiltRevision>> {
        let build = self
            .build
            .as_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No restart build"))?;
        let Some(status) = build.child.try_wait()? else {
            return Ok(None);
        };
        let build = self.build.take().unwrap();
        if build.revision != self.latest_revision {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "Restart build superseded",
            ));
        }
        if !status.success() {
            return Err(io::Error::other(format!(
                "Replacement build failed: {status}"
            )));
        }
        Ok(Some(BuiltRevision {
            revision: build.revision,
            owner: self.owner,
        }))
    }
    /// The supplied command must run the artifact produced by the ticket's build.
    /// State is captured after building, so edits made while the compiler runs are retained.
    pub fn launch(
        &mut self,
        ticket: BuiltRevision,
        command: &mut Command,
        state: &RestartState,
        timeout: Duration,
    ) -> io::Result<()> {
        if ticket.owner != self.owner || ticket.revision != self.latest_revision {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "Stale or foreign restart build",
            ));
        }
        if self.candidate.is_some() {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "Replacement already starting",
            ));
        }
        state.validate()?;
        let directory = tempfile::tempdir()?;
        let state_file = directory.path().join("state.json");
        fs::write(&state_file, serde_json::to_vec(state)?)?;
        let child = command
            .env(STATE_ENV, state_file)
            .env(READY_ENV, directory.path().join("ready"))
            .spawn()?;
        self.candidate = Some(Candidate {
            revision: ticket.revision,
            child,
            directory,
            deadline: Instant::now() + timeout,
        });
        Ok(())
    }
    pub fn poll_replacement(&mut self) -> io::Result<ReplacementStatus> {
        let candidate = self
            .candidate
            .as_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No replacement starting"))?;
        let status = if candidate.revision != self.latest_revision {
            ReplacementStatus::Superseded
        } else if let Some(status) = candidate.child.try_wait()? {
            ReplacementStatus::Failed(format!("Replacement exited during startup: {status}"))
        } else if fs::read(candidate.directory.path().join("ready"))
            .is_ok_and(|bytes| bytes == b"ferroui-ready-v1")
        {
            return Ok(ReplacementStatus::Ready);
        } else if Instant::now() >= candidate.deadline {
            ReplacementStatus::Failed("Replacement startup timed out".into())
        } else {
            return Ok(ReplacementStatus::Waiting);
        };
        let mut candidate = self.candidate.take().unwrap();
        stop(&mut candidate.child)?;
        Ok(status)
    }
    /// Retires the old process only after an explicit successful startup acknowledgement.
    pub fn commit(&mut self) -> io::Result<()> {
        if self.poll_replacement()? != ReplacementStatus::Ready {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "Replacement is not ready",
            ));
        }
        stop(&mut self.current)?;
        self.current = self.candidate.take().unwrap().child;
        Ok(())
    }
}
fn stop(child: &mut Child) -> io::Result<()> {
    if child.try_wait()?.is_none() {
        child.kill()?;
    }
    child.wait()?;
    Ok(())
}
impl Drop for RestartSupervisor {
    fn drop(&mut self) {
        if let Some(build) = &mut self.build {
            let _ = stop(&mut build.child);
        }
        if let Some(candidate) = &mut self.candidate {
            let _ = stop(&mut candidate.child);
        }
        let _ = stop(&mut self.current);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Stdio;
    fn command(mode: &str) -> Command {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args(["--exact", "restart::tests::child_fixture", "--nocapture"])
            .env("FERROUI_RESTART_TEST_CHILD", mode)
            .env_remove(STATE_ENV)
            .env_remove(READY_ENV)
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        command
    }
    #[test]
    fn child_fixture() {
        let Ok(mode) = std::env::var("FERROUI_RESTART_TEST_CHILD") else {
            return;
        };
        match mode.as_str() {
            "build-ok" => return,
            "build-failed" => std::process::exit(7),
            "restore" => {
                let startup = RestartStartup::from_environment("fixture", 2)
                    .unwrap()
                    .unwrap();
                assert_eq!(startup.state.application_state["counter"], 41);
                assert_eq!(startup.state.navigation.as_deref(), Some("settings"));
                assert_eq!(startup.state.windows[0].position, [40, 50]);
                startup.acknowledge().unwrap();
            }
            "wrong-version" => {
                assert!(RestartStartup::from_environment("fixture", 3).is_err());
                std::process::exit(9);
            }
            "wait" => (),
            _ => panic!("unknown child mode"),
        }
        loop {
            std::thread::sleep(Duration::from_millis(50));
        }
    }
    fn supervisor() -> RestartSupervisor {
        RestartSupervisor::new(command("wait").spawn().unwrap())
    }
    fn state() -> RestartState {
        let mut state =
            RestartState::new("fixture".into(), 2, serde_json::json!({ "counter": 41 }));
        state.navigation = Some("settings".into());
        state.windows.push(WindowPlacement {
            key: "main".into(),
            position: [40, 50],
            size: [640.0, 480.0],
            maximized: false,
        });
        state
    }
    fn built(supervisor: &mut RestartSupervisor) -> io::Result<BuiltRevision> {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(ticket) = supervisor.poll_build()? {
                return Ok(ticket);
            }
            assert!(Instant::now() < deadline, "build timed out");
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    fn settled(supervisor: &mut RestartSupervisor) -> ReplacementStatus {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let status = supervisor.poll_replacement().unwrap();
            if status != ReplacementStatus::Waiting {
                return status;
            }
            assert!(Instant::now() < deadline, "startup timed out");
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    #[test]
    fn failed_build_retains_original_process() {
        let mut supervisor = supervisor();
        let old = supervisor.current_id();
        supervisor.build(1, &mut command("build-failed")).unwrap();
        assert!(built(&mut supervisor).is_err());
        assert_eq!(supervisor.current_id(), old);
        assert!(supervisor.current_is_running().unwrap());
    }
    #[test]
    fn acknowledged_state_restore_precedes_retiring_old_process() {
        let mut supervisor = supervisor();
        let old = supervisor.current_id();
        supervisor.build(1, &mut command("build-ok")).unwrap();
        let ticket = built(&mut supervisor).unwrap();
        supervisor
            .launch(
                ticket,
                &mut command("restore"),
                &state(),
                Duration::from_secs(5),
            )
            .unwrap();
        assert!(supervisor.current_is_running().unwrap());
        assert_eq!(settled(&mut supervisor), ReplacementStatus::Ready);
        assert_eq!(supervisor.current_id(), old);
        supervisor.commit().unwrap();
        assert_ne!(supervisor.current_id(), old);
        assert!(supervisor.current_is_running().unwrap());
    }
    #[test]
    fn incompatible_restore_and_startup_timeout_retain_old_process() {
        let mut supervisor = supervisor();
        let old = supervisor.current_id();
        for (revision, mode) in [(1, "wrong-version"), (2, "wait")] {
            supervisor
                .build(revision, &mut command("build-ok"))
                .unwrap();
            let ticket = built(&mut supervisor).unwrap();
            supervisor
                .launch(
                    ticket,
                    &mut command(mode),
                    &state(),
                    Duration::from_millis(200),
                )
                .unwrap();
            assert!(matches!(
                settled(&mut supervisor),
                ReplacementStatus::Failed(_)
            ));
            assert_eq!(supervisor.current_id(), old);
            assert!(supervisor.current_is_running().unwrap());
        }
    }
    #[test]
    fn superseded_build_and_startup_cannot_replace_current_process() {
        let mut supervisor = supervisor();
        supervisor.build(1, &mut command("build-ok")).unwrap();
        supervisor.supersede(2).unwrap();
        assert_eq!(
            built(&mut supervisor).unwrap_err().kind(),
            io::ErrorKind::Interrupted
        );
        supervisor.build(3, &mut command("build-ok")).unwrap();
        let ticket = built(&mut supervisor).unwrap();
        supervisor
            .launch(
                ticket,
                &mut command("restore"),
                &state(),
                Duration::from_secs(5),
            )
            .unwrap();
        supervisor.supersede(4).unwrap();
        assert_eq!(settled(&mut supervisor), ReplacementStatus::Superseded);
        assert!(supervisor.current_is_running().unwrap());
    }
}
