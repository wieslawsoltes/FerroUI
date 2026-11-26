//! Shell-free project processes share the same descendant cleanup on cancellation.
#[cfg(windows)]
use std::process::Stdio;
use std::{
    io,
    process::{Child, Command},
};

/// Spawn a child with its own process group on Unix. On Windows the process ID
/// anchors tree cleanup.
pub fn spawn_project_process(command: &mut Command) -> io::Result<Child> {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    command.spawn()
}

/// Stop a child started by [`spawn_project_process`], including descendants
/// that could retain its IO pipes after it exits.
pub fn stop_project_process(child: &mut Child, leader_exited: bool) -> io::Result<()> {
    #[cfg(unix)]
    {
        // SAFETY: spawn_project_process created this group with the child's PID.
        let stopped = unsafe { libc::kill(-(child.id() as i32), libc::SIGKILL) } == 0;
        if !stopped && !leader_exited {
            child.kill()?;
        }
    }
    #[cfg(windows)]
    {
        let stopped = Command::new("taskkill")
            .args(["/PID", &child.id().to_string(), "/T", "/F"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success());
        if !stopped && !leader_exited {
            child.kill()?;
        }
    }
    #[cfg(not(any(unix, windows)))]
    if !leader_exited {
        child.kill()?;
    }
    child.wait().map(|_| ())
}
