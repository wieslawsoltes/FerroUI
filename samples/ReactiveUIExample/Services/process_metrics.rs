//! The measurements of the current process: its processor time, its working set and its threads
//! (`Process.GetCurrentProcess()` with `TotalProcessorTime`, `WorkingSet64` and `Threads.Count`).
//!
//! Not an upstream file: the reference reads them through `System.Diagnostics.Process`. Here they
//! come from the operating system through the standard library and the bindings the workspace
//! already has:
//!
//! - Linux and Android: the resource usage of the process (`getrusage`) and `/proc/self/status`
//!   (`VmRSS`, `Threads`);
//! - macOS: the resource usage of the process and the task information of `proc_pidinfo`;
//! - Windows: `GetProcessTimes`, `K32GetProcessMemoryInfo` and a thread snapshot of the Tool Help
//!   library.
//!
//! Elsewhere `Process` is not supported, and reading fails as it does in the reference
//! (`PlatformNotSupportedException`); the browser has its own service
//! ([`BrowserMachineMetricsService`](super::BrowserMachineMetricsService)).

use ferroui_system::ObservableError;
use std::time::Duration;

/// One measurement of the current process.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ProcessSample {
    /// The processor time the process has used, in user and kernel mode (`TotalProcessorTime`).
    pub processor_time: Duration,
    /// The resident memory of the process, in bytes (`WorkingSet64`).
    pub working_set_bytes: u64,
    /// The number of threads of the process (`Threads.Count`).
    pub thread_count: i32,
}

/// Measures the current process; the error of the operating system, or a
/// [`PlatformNotSupportedError`](super::PlatformNotSupportedError) where the process cannot be
/// measured.
pub(crate) fn read_process() -> Result<ProcessSample, ObservableError> {
    platform::read()
}

/// The error of the operating system as the error of a measurement.
#[allow(dead_code)]
fn os_error(error: std::io::Error) -> ObservableError {
    std::rc::Rc::new(error)
}

#[cfg(any(target_os = "linux", target_os = "android", target_os = "macos"))]
mod unix {
    use super::os_error;
    use ferroui_system::ObservableError;
    use std::time::Duration;

    /// A time of the resource usage as a duration.
    fn duration_of(time: libc::timeval) -> Duration {
        Duration::from_secs(time.tv_sec.max(0) as u64) + Duration::from_micros(time.tv_usec.max(0) as u64)
    }

    /// The processor time of the process, in user and kernel mode (`getrusage(RUSAGE_SELF)`).
    pub(super) fn processor_time() -> Result<Duration, ObservableError> {
        // SAFETY: `rusage` is a plain C structure, for which all zeroes is a valid value.
        let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
        // SAFETY: the pointer is to a live `rusage` the call fills.
        if unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut usage) } != 0 {
            return Err(os_error(std::io::Error::last_os_error()));
        }
        Ok(duration_of(usage.ru_utime) + duration_of(usage.ru_stime))
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
mod platform {
    use super::{os_error, ProcessSample};
    use ferroui_system::ObservableError;

    /// The value of the field `name` of `/proc/self/status` (`VmRSS:   1234 kB`): its first number.
    fn status_field(status: &str, name: &str) -> Result<u64, ObservableError> {
        status
            .lines()
            .find_map(|line| line.strip_prefix(name))
            .and_then(|value| value.split_whitespace().next())
            .and_then(|value| value.parse::<u64>().ok())
            .ok_or_else(|| {
                os_error(std::io::Error::new(std::io::ErrorKind::InvalidData, format!("/proc/self/status has no field {name}")))
            })
    }

    pub(super) fn read() -> Result<ProcessSample, ObservableError> {
        let processor_time = super::unix::processor_time()?;
        let status = std::fs::read_to_string("/proc/self/status").map_err(os_error)?;
        let resident_kilobytes = status_field(&status, "VmRSS:")?;
        let threads = status_field(&status, "Threads:")?;
        Ok(ProcessSample { processor_time, working_set_bytes: resident_kilobytes * 1024, thread_count: threads as i32 })
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use super::{os_error, ProcessSample};
    use ferroui_system::ObservableError;

    pub(super) fn read() -> Result<ProcessSample, ObservableError> {
        let processor_time = super::unix::processor_time()?;
        // SAFETY: `proc_taskinfo` is a plain C structure, for which all zeroes is a valid value.
        let mut info: libc::proc_taskinfo = unsafe { std::mem::zeroed() };
        let size = std::mem::size_of::<libc::proc_taskinfo>() as libc::c_int;
        // SAFETY: the buffer is a live `proc_taskinfo` of `size` bytes, which the call fills.
        let written = unsafe {
            libc::proc_pidinfo(libc::getpid(), libc::PROC_PIDTASKINFO, 0, (&mut info as *mut libc::proc_taskinfo).cast::<libc::c_void>(), size)
        };
        if written != size {
            return Err(os_error(std::io::Error::last_os_error()));
        }
        Ok(ProcessSample { processor_time, working_set_bytes: info.pti_resident_size, thread_count: info.pti_threadnum })
    }
}

#[cfg(windows)]
mod platform {
    use super::{os_error, ProcessSample};
    use ferroui_system::ObservableError;
    use std::time::Duration;
    use windows_sys::Win32::Foundation::{CloseHandle, FILETIME, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, Thread32First, Thread32Next, TH32CS_SNAPTHREAD, THREADENTRY32};
    use windows_sys::Win32::System::ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetCurrentProcessId, GetProcessTimes};

    /// A time of `GetProcessTimes`, counted in units of 100 nanoseconds, as a duration.
    fn duration_of(time: FILETIME) -> Duration {
        let units = (u64::from(time.dwHighDateTime) << 32) | u64::from(time.dwLowDateTime);
        Duration::from_nanos(units.saturating_mul(100))
    }

    /// The processor time of the process, in user and kernel mode.
    fn processor_time() -> Result<Duration, ObservableError> {
        // SAFETY: `FILETIME` is a plain C structure, for which all zeroes is a valid value.
        let (mut creation, mut exit, mut kernel, mut user): (FILETIME, FILETIME, FILETIME, FILETIME) = unsafe { std::mem::zeroed() };
        // SAFETY: the pseudo handle of the current process needs no closing, and the pointers are
        // to live structures the call fills.
        if unsafe { GetProcessTimes(GetCurrentProcess(), &mut creation, &mut exit, &mut kernel, &mut user) } == 0 {
            return Err(os_error(std::io::Error::last_os_error()));
        }
        Ok(duration_of(kernel) + duration_of(user))
    }

    /// The working set of the process, in bytes.
    fn working_set_bytes() -> Result<u64, ObservableError> {
        // SAFETY: `PROCESS_MEMORY_COUNTERS` is a plain C structure, for which all zeroes is a valid
        // value.
        let mut counters: PROCESS_MEMORY_COUNTERS = unsafe { std::mem::zeroed() };
        let size = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
        counters.cb = size;
        // SAFETY: the pointer is to a live structure of `size` bytes the call fills.
        if unsafe { K32GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, size) } == 0 {
            return Err(os_error(std::io::Error::last_os_error()));
        }
        Ok(counters.WorkingSetSize as u64)
    }

    /// The number of threads the process owns, from a snapshot of the threads of the system.
    fn thread_count() -> Result<i32, ObservableError> {
        // SAFETY: the call takes no pointer; the handle it returns is closed below.
        let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) };
        if snapshot == INVALID_HANDLE_VALUE {
            return Err(os_error(std::io::Error::last_os_error()));
        }
        // SAFETY: the call takes no pointer.
        let process_id = unsafe { GetCurrentProcessId() };
        // SAFETY: `THREADENTRY32` is a plain C structure, for which all zeroes is a valid value.
        let mut entry: THREADENTRY32 = unsafe { std::mem::zeroed() };
        entry.dwSize = std::mem::size_of::<THREADENTRY32>() as u32;
        let mut count = 0;
        // SAFETY: the snapshot is open and the entry is a live structure whose size is set.
        let mut found = unsafe { Thread32First(snapshot, &mut entry) } != 0;
        while found {
            if entry.th32OwnerProcessID == process_id {
                count += 1;
            }
            // SAFETY: as above.
            found = unsafe { Thread32Next(snapshot, &mut entry) } != 0;
        }
        // SAFETY: the snapshot is open and closed once.
        unsafe { CloseHandle(snapshot) };
        Ok(count)
    }

    pub(super) fn read() -> Result<ProcessSample, ObservableError> {
        Ok(ProcessSample { processor_time: processor_time()?, working_set_bytes: working_set_bytes()?, thread_count: thread_count()? })
    }
}

#[cfg(not(any(target_os = "linux", target_os = "android", target_os = "macos", windows)))]
mod platform {
    use super::ProcessSample;
    use crate::services::PlatformNotSupportedError;
    use ferroui_system::ObservableError;

    pub(super) fn read() -> Result<ProcessSample, ObservableError> {
        Err(PlatformNotSupportedError::new("Process metrics are not supported on this platform.").into_error())
    }
}
