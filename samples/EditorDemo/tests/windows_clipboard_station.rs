//! Win32 process launcher for the clipboard acceptance test. Both peers use a
//! noninteractive window station, which owns a separate system clipboard.

use std::env;
use std::ffi::{OsStr, OsString};
use std::io;
use std::mem::size_of;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::{
    ERROR_ALREADY_EXISTS, GetLastError, WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
    SetInformationJobObject,
};
use windows_sys::Win32::System::StationsAndDesktops::{
    CloseDesktop, CloseWindowStation, CreateDesktopW, CreateWindowStationW,
    GetProcessWindowStation, GetUserObjectInformationW, HDESK, HWINSTA, SetProcessWindowStation,
    UOI_NAME,
};
use windows_sys::Win32::System::Threading::{
    CREATE_SUSPENDED, CreateMutexW, CreateProcessW, GetExitCodeProcess, PROCESS_INFORMATION,
    ResumeThread, STARTUPINFOW, TerminateProcess, WaitForSingleObject,
};

const MAXIMUM_ALLOWED: u32 = 0x0200_0000;
const DESKTOP_ALL_ACCESS: u32 = 0x000f_01ff;

struct PrivateStation {
    original: HWINSTA,
    station: HWINSTA,
    desktop: HDESK,
}

impl PrivateStation {
    fn enter(desktop_name: &OsStr) -> io::Result<(Self, OsString)> {
        // SAFETY: This borrows the process's current station handle.
        let original = unsafe { GetProcessWindowStation() };
        let original_name = station_name(original)?;
        // SAFETY: Null security uses the current token's default ACL. A null
        // name lets Windows choose the station for this logon session.
        let station = unsafe { CreateWindowStationW(null(), 0, MAXIMUM_ALLOWED, null()) };
        check(!station.is_null())?;
        let mut private = Self {
            original,
            station,
            desktop: null_mut(),
        };
        let name = station_name(station)?;
        if name == original_name || name.eq_ignore_ascii_case("WinSta0") {
            return Err(io::Error::other(
                "Could not select a separate clipboard station",
            ));
        }
        // SAFETY: The station handle stays open until Drop restores the original.
        check(unsafe { SetProcessWindowStation(station) } != 0)?;
        verify_peer(&name)?;
        let desktop_wide = wide(desktop_name);
        // SAFETY: The name is terminated. The returned desktop handle belongs to PrivateStation.
        private.desktop = unsafe {
            CreateDesktopW(
                desktop_wide.as_ptr(),
                null(),
                null(),
                0,
                DESKTOP_ALL_ACCESS,
                null(),
            )
        };
        check(!private.desktop.is_null())?;
        Ok((private, name))
    }
}

impl Drop for PrivateStation {
    fn drop(&mut self) {
        // SAFETY: The original station remains owned by the process. The
        // launcher owns the two handles created for the private station.
        unsafe {
            SetProcessWindowStation(self.original);
            if !self.desktop.is_null() {
                CloseDesktop(self.desktop);
            }
            CloseWindowStation(self.station);
        }
    }
}

pub(super) fn verify_peer(expected: &OsStr) -> io::Result<()> {
    // SAFETY: This borrows the process's current station handle.
    let actual = station_name(unsafe { GetProcessWindowStation() })?;
    if actual != expected || actual.eq_ignore_ascii_case("WinSta0") {
        return Err(io::Error::other("Process has no private clipboard station"));
    }
    Ok(())
}

fn station_name(station: HWINSTA) -> io::Result<OsString> {
    let mut name = [0_u16; 256];
    let mut used = 0;
    // SAFETY: The buffer and returned byte count are valid out-parameters.
    check(
        unsafe {
            GetUserObjectInformationW(
                station,
                UOI_NAME,
                name.as_mut_ptr().cast(),
                size_of::<[u16; 256]>() as u32,
                &mut used,
            )
        } != 0,
    )?;
    let end = name
        .iter()
        .position(|&unit| unit == 0)
        .unwrap_or(name.len());
    Ok(OsString::from_wide(&name[..end]))
}

pub(super) fn launch(args: &[OsString]) -> io::Result<u32> {
    let desktop = env::var_os("FERROUI_TEST_WINDOWS_DESKTOP")
        .ok_or_else(|| io::Error::other("Missing test desktop"))?;
    let is_peer = args.first().is_some_and(|arg| arg == "--fixture");
    // The unnamed station is shared by a logon session. Keep a second browser
    // test from writing to it while this browser and its editor peers run.
    let _exclusive = if is_peer { None } else { Some(browser_lock()?) };
    let (_station, station_name) = PrivateStation::enter(&desktop)?;
    let (program, child_args, timeout) = if is_peer {
        let mut child_args = vec![OsString::from("--peer"), station_name.clone()];
        child_args.extend_from_slice(&args[1..]);
        (env::current_exe()?.into_os_string(), child_args, 30_000)
    } else {
        let browser = env::var_os("FERROUI_TEST_EDGE_EXE")
            .ok_or_else(|| io::Error::other("Missing browser executable"))?;
        (browser, args.to_vec(), 240_000)
    };
    let mut qualified_desktop = station_name;
    qualified_desktop.push("\\");
    qualified_desktop.push(desktop);
    run_child(
        &program,
        &child_args,
        &mut wide(&qualified_desktop),
        timeout,
    )
}

fn browser_lock() -> io::Result<OwnedHandle> {
    let name = wide(OsStr::new("Local\\FerroUI.EditorClipboardTest"));
    // SAFETY: The name is terminated. CreateMutexW returns an owned handle.
    let handle = unsafe { CreateMutexW(null(), 0, name.as_ptr()) };
    // SAFETY: GetLastError reads this thread's status from CreateMutexW.
    let already_open = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
    check(!handle.is_null())?;
    // SAFETY: The handle is valid and is transferred to OwnedHandle.
    let handle = unsafe { OwnedHandle::from_raw_handle(handle) };
    if already_open {
        return Err(io::Error::other(
            "Another Windows clipboard acceptance test is running",
        ));
    }
    Ok(handle)
}

fn run_child(
    program: &OsStr,
    args: &[OsString],
    desktop: &mut [u16],
    timeout: u32,
) -> io::Result<u32> {
    let executable = wide(program);
    let mut command = Vec::new();
    quote_arg(program, &mut command);
    for arg in args {
        command.push(u16::from(b' '));
        quote_arg(arg, &mut command);
    }
    command.push(0);
    let startup = STARTUPINFOW {
        cb: size_of::<STARTUPINFOW>() as u32,
        lpDesktop: desktop.as_mut_ptr(),
        ..STARTUPINFOW::default()
    };
    let job = closing_job()?;
    let mut info = PROCESS_INFORMATION::default();
    // SAFETY: Both UTF-16 strings are terminated. The startup and process
    // structures, including the mutable command line, live through the call.
    check(
        unsafe {
            CreateProcessW(
                executable.as_ptr(),
                command.as_mut_ptr(),
                null(),
                null(),
                0,
                CREATE_SUSPENDED,
                null(),
                null(),
                &startup,
                &mut info,
            )
        } != 0,
    )?;
    // SAFETY: On success the launcher owns both handles in PROCESS_INFORMATION.
    let (process, thread) = unsafe {
        (
            OwnedHandle::from_raw_handle(info.hProcess),
            OwnedHandle::from_raw_handle(info.hThread),
        )
    };
    // SAFETY: The job and child process handles are valid. A failed assignment
    // leaves a suspended child, which this launcher terminates before returning.
    if unsafe { AssignProcessToJobObject(job.as_raw_handle(), process.as_raw_handle()) } == 0 {
        let error = io::Error::last_os_error();
        unsafe { TerminateProcess(process.as_raw_handle(), 1) };
        return Err(error);
    }
    // SAFETY: The suspended thread and its parent process are owned by the launcher.
    check(unsafe { ResumeThread(thread.as_raw_handle()) } != u32::MAX)?;
    // SAFETY: The wait uses the owned process handle and a bounded timeout.
    match unsafe { WaitForSingleObject(process.as_raw_handle(), timeout) } {
        WAIT_OBJECT_0 => {
            let mut code = 0;
            // SAFETY: The process handle and exit-code output are valid.
            check(unsafe { GetExitCodeProcess(process.as_raw_handle(), &mut code) } != 0)?;
            Ok(code)
        }
        WAIT_TIMEOUT => Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "Private clipboard process timed out",
        )),
        _ => Err(io::Error::last_os_error()),
    }
}

fn closing_job() -> io::Result<OwnedHandle> {
    // SAFETY: Null arguments select default security and an unnamed job.
    let job = unsafe { CreateJobObjectW(null(), null()) };
    check(!job.is_null())?;
    // SAFETY: CreateJobObjectW returned a valid owned handle.
    let job = unsafe { OwnedHandle::from_raw_handle(job) };
    let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
    limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    // SAFETY: The job handle is valid and the pointer and size describe its limit structure.
    check(
        unsafe {
            SetInformationJobObject(
                job.as_raw_handle(),
                JobObjectExtendedLimitInformation,
                (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        } != 0,
    )?;
    Ok(job)
}

fn wide(value: &OsStr) -> Vec<u16> {
    value.encode_wide().chain([0]).collect()
}

fn quote_arg(value: &OsStr, output: &mut Vec<u16>) {
    let slash = u16::from(b'\\');
    let quote = u16::from(b'"');
    output.push(quote);
    let mut slashes = 0;
    for unit in value.encode_wide() {
        if unit == slash {
            slashes += 1;
            continue;
        }
        let count = if unit == quote {
            slashes * 2 + 1
        } else {
            slashes
        };
        output.extend(std::iter::repeat_n(slash, count));
        output.push(unit);
        slashes = 0;
    }
    output.extend(std::iter::repeat_n(slash, slashes * 2));
    output.push(quote);
}

fn check(ok: bool) -> io::Result<()> {
    if ok {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}
