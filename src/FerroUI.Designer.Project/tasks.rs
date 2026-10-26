use crate::{require, ProjectDescriptor, ProjectError, Result};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskKind {
    Check,
    Build,
    Run { target: Option<String> },
}
#[derive(Clone, Debug, Serialize)]
pub struct BuildTask {
    program: String,
    args: Vec<String>,
    cwd: PathBuf,
    pub label: String,
}
impl BuildTask {
    pub fn cargo(project: &ProjectDescriptor, kind: TaskKind, release: bool) -> Result<Self> {
        require(
            project.manifest_path.is_file(),
            "Project has no Cargo manifest",
        )?;
        let mut args = vec![
            match &kind {
                TaskKind::Check => "check",
                TaskKind::Build => "build",
                TaskKind::Run { .. } => "run",
            }
            .to_owned(),
            "--manifest-path".into(),
            project
                .manifest_path
                .to_str()
                .ok_or_else(|| ProjectError("Manifest path must be UTF-8".into()))?
                .into(),
        ];
        if let TaskKind::Run { target } = kind {
            let binaries: Vec<_> = project
                .targets
                .iter()
                .filter(|t| t.kind.iter().any(|k| k == "bin"))
                .collect();
            let name = match target {
                Some(name) => {
                    require(
                        binaries.iter().any(|t| t.name == name),
                        "Unknown binary target",
                    )?;
                    name
                }
                None => {
                    require(binaries.len() == 1, "Choose a binary target")?;
                    binaries[0].name.clone()
                }
            };
            args.extend(["--bin".into(), name]);
        }
        if release {
            args.push("--release".into());
        }
        Ok(Self {
            program: "cargo".into(),
            label: format!("{} {}", project.name, args[0]),
            args,
            cwd: project.root.clone(),
        })
    }
    pub fn program(&self) -> &str {
        &self.program
    }
    pub fn arguments(&self) -> &[String] {
        &self.args
    }
    pub fn directory(&self) -> &Path {
        &self.cwd
    }
    /// Starts a shell-free child. Build scripts and the selected application execute with the
    /// host's privileges; the host decides when a user-authorized task may start.
    pub fn start(&self) -> Result<TaskHandle> {
        let mut command = Command::new(&self.program);
        command
            .args(&self.args)
            .current_dir(&self.cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        let mut child = command.spawn()?;
        let output = Arc::new(Mutex::new(OutputBuffer {
            readers: 2,
            ..Default::default()
        }));
        for (stream, reader) in [
            (
                OutputStream::Stdout,
                Box::new(child.stdout.take().unwrap()) as Box<dyn Read + Send>,
            ),
            (
                OutputStream::Stderr,
                Box::new(child.stderr.take().unwrap()) as Box<dyn Read + Send>,
            ),
        ] {
            let output = output.clone();
            std::thread::spawn(move || read_output(reader, stream, output));
        }
        Ok(TaskHandle {
            child,
            output,
            status: TaskStatus::Running,
            exit_code: None,
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputStream {
    Stdout,
    Stderr,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskOutput {
    pub stream: OutputStream,
    pub text: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Running,
    Exited { code: Option<i32> },
    Cancelled,
    Failed { message: String },
}
#[derive(Default)]
struct OutputBuffer {
    chunks: VecDeque<TaskOutput>,
    bytes: usize,
    dropped: usize,
    readers: usize,
}
pub struct TaskHandle {
    child: Child,
    output: Arc<Mutex<OutputBuffer>>,
    status: TaskStatus,
    exit_code: Option<Option<i32>>,
}
impl TaskHandle {
    pub fn poll(&mut self) -> TaskStatus {
        if self.status == TaskStatus::Running {
            match self.child.try_wait() {
                Ok(Some(status)) => {
                    self.exit_code = Some(status.code());
                    if self
                        .output
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .readers
                        == 0
                    {
                        self.status = TaskStatus::Exited {
                            code: status.code(),
                        };
                    }
                }
                Ok(None) => {}
                Err(error) => {
                    self.status = TaskStatus::Failed {
                        message: error.to_string(),
                    }
                }
            }
        }
        self.status.clone()
    }
    pub fn take_output(&mut self) -> Vec<TaskOutput> {
        let mut output = self.output.lock().unwrap_or_else(|e| e.into_inner());
        let mut chunks: Vec<_> = output.chunks.drain(..).collect();
        output.bytes = 0;
        if output.dropped > 0 {
            chunks.insert(
                0,
                TaskOutput {
                    stream: OutputStream::Stderr,
                    text: format!("[{} bytes of task output omitted]\n", output.dropped),
                },
            );
            output.dropped = 0;
        }
        chunks
    }
    pub fn cancel(&mut self) -> Result<()> {
        if self.poll() != TaskStatus::Running {
            return Ok(());
        }
        #[cfg(unix)]
        {
            // SAFETY: the child's process group was created by this handle. A negative process
            // ID targets that group, including descendants that keep output pipes open after
            // the leader exits. A live process group retains its group ID; no inherited group
            // or unrelated parent process group is targeted.
            let result = unsafe { libc::kill(-(self.child.id() as i32), libc::SIGKILL) };
            if result != 0 && self.exit_code.is_none() {
                self.child.kill()?;
            }
        }
        #[cfg(windows)]
        {
            let status = Command::new("taskkill")
                .args(["/PID", &self.child.id().to_string(), "/T", "/F"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            if !status.is_ok_and(|s| s.success()) && self.exit_code.is_none() {
                self.child.kill()?;
            }
        }
        #[cfg(not(any(unix, windows)))]
        self.child.kill()?;
        self.child.wait()?;
        self.status = TaskStatus::Cancelled;
        Ok(())
    }
}
impl Drop for TaskHandle {
    fn drop(&mut self) {
        let _ = self.cancel();
    }
}
fn read_output(
    mut reader: Box<dyn Read + Send>,
    stream: OutputStream,
    output: Arc<Mutex<OutputBuffer>>,
) {
    let mut bytes = [0u8; 4096];
    let mut pending = Vec::new();
    loop {
        let length = match reader.read(&mut bytes) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        };
        pending.extend_from_slice(&bytes[..length]);
        let keep = match std::str::from_utf8(&pending) {
            Ok(_) => 0,
            Err(e) if e.error_len().is_none() => pending.len() - e.valid_up_to(),
            Err(_) => 0,
        };
        let ready = pending.len() - keep;
        push_output(
            &output,
            TaskOutput {
                stream,
                text: String::from_utf8_lossy(&pending[..ready]).into_owned(),
            },
        );
        pending.drain(..ready);
    }
    if !pending.is_empty() {
        push_output(
            &output,
            TaskOutput {
                stream,
                text: String::from_utf8_lossy(&pending).into_owned(),
            },
        );
    }
    output.lock().unwrap_or_else(|e| e.into_inner()).readers -= 1;
}
fn push_output(buffer: &Mutex<OutputBuffer>, chunk: TaskOutput) {
    let mut output = buffer.lock().unwrap_or_else(|e| e.into_inner());
    output.bytes += chunk.text.len();
    output.chunks.push_back(chunk);
    while output.bytes > 1024 * 1024 {
        if let Some(old) = output.chunks.pop_front() {
            output.bytes -= old.text.len();
            output.dropped += old.text.len();
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    #[test]
    fn output_and_group_cancellation_are_observable_without_blocking_poll() {
        let task = BuildTask {
            program: "/bin/sh".into(),
            args: vec![
                "-c".into(),
                "printf 'hello'; printf 'error' >&2; sleep 30".into(),
            ],
            cwd: std::env::temp_dir(),
            label: "test".into(),
        };
        let mut handle = task.start().unwrap();
        let start = std::time::Instant::now();
        let mut out = Vec::new();
        while out.len() < 2 && start.elapsed() < std::time::Duration::from_secs(2) {
            out.extend(handle.take_output());
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        assert!(out
            .iter()
            .any(|s| s.stream == OutputStream::Stdout && s.text == "hello"));
        assert!(out
            .iter()
            .any(|s| s.stream == OutputStream::Stderr && s.text == "error"));
        assert_eq!(handle.poll(), TaskStatus::Running);
        handle.cancel().unwrap();
        assert_eq!(handle.poll(), TaskStatus::Cancelled);
    }
    #[test]
    fn exited_task_is_reported_only_after_both_streams_have_finished() {
        let task = BuildTask {
            program: "/bin/sh".into(),
            args: vec![
                "-c".into(),
                "printf '🦀 end'; printf 'final error' >&2".into(),
            ],
            cwd: std::env::temp_dir(),
            label: "output".into(),
        };
        let mut handle = task.start().unwrap();
        let start = std::time::Instant::now();
        while handle.poll() == TaskStatus::Running
            && start.elapsed() < std::time::Duration::from_secs(2)
        {
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        assert_eq!(handle.poll(), TaskStatus::Exited { code: Some(0) });
        let output = handle.take_output();
        assert_eq!(
            output
                .iter()
                .filter(|chunk| chunk.stream == OutputStream::Stdout)
                .map(|chunk| chunk.text.as_str())
                .collect::<String>(),
            "🦀 end"
        );
        assert_eq!(
            output
                .iter()
                .filter(|chunk| chunk.stream == OutputStream::Stderr)
                .map(|chunk| chunk.text.as_str())
                .collect::<String>(),
            "final error"
        );
    }
    #[test]
    fn cancel_terminates_descendants_after_process_group_leader_exits() {
        let task = BuildTask {
            program: "/bin/sh".into(),
            args: vec!["-c".into(), "sleep 30 & exit 0".into()],
            cwd: std::env::temp_dir(),
            label: "descendant".into(),
        };
        let mut handle = task.start().unwrap();
        let start = std::time::Instant::now();
        while handle.exit_code.is_none() && start.elapsed() < std::time::Duration::from_secs(2) {
            handle.poll();
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        assert!(handle.exit_code.is_some());
        handle.cancel().unwrap();
        while handle.output.lock().unwrap().readers > 0
            && start.elapsed() < std::time::Duration::from_secs(2)
        {
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        assert_eq!(handle.output.lock().unwrap().readers, 0);
        assert_eq!(handle.poll(), TaskStatus::Cancelled);
    }
}
