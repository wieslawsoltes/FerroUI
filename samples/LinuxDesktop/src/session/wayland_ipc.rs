//! Sway's native-endian i3 IPC framing. Only connect to the compositor that
//! also owns the active Wayland connection (nested sessions may inherit SWAYSOCK).
use serde_json::Value;
use std::io::{self, Read, Write};
use std::os::fd::{AsRawFd, RawFd};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const RUN: u32 = 0;
pub const WORKSPACES: u32 = 1;
pub const SUBSCRIBE: u32 = 2;
pub const TREE: u32 = 4;
pub const EVENT: u32 = 1 << 31;
const MAX_PAYLOAD: usize = 32 * 1024 * 1024;

pub fn invalid(text: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, text)
}

fn peer(fd: RawFd) -> io::Result<libc::pid_t> {
    let mut value = std::mem::MaybeUninit::<libc::ucred>::uninit();
    let mut size = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    // SAFETY: getsockopt only writes up to size bytes to the supplied ucred.
    let result = unsafe {
        libc::getsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            value.as_mut_ptr().cast(),
            &mut size,
        )
    };
    if result < 0 {
        return Err(io::Error::last_os_error());
    }
    if size as usize != std::mem::size_of::<libc::ucred>() {
        return Err(invalid("Invalid socket peer credentials"));
    }
    // SAFETY: the kernel initialized the complete ucred on success.
    Ok(unsafe { value.assume_init() }.pid)
}

pub fn compositor_socket() -> io::Result<(PathBuf, libc::pid_t)> {
    let path = std::env::var_os("SWAYSOCK")
        .filter(|p| !p.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::Unsupported,
                "This compositor does not expose a Sway IPC socket",
            )
        })?;
    if !path.is_absolute() {
        return Err(invalid("SWAYSOCK must be absolute"));
    }
    let pid = if let Ok(fd) = std::env::var("WAYLAND_SOCKET") {
        peer(fd.parse().map_err(|_| invalid("Invalid WAYLAND_SOCKET"))?)?
    } else {
        let display = PathBuf::from(
            std::env::var_os("WAYLAND_DISPLAY").unwrap_or_else(|| "wayland-0".into()),
        );
        let display = if display.is_absolute() {
            display
        } else {
            PathBuf::from(
                std::env::var_os("XDG_RUNTIME_DIR")
                    .ok_or_else(|| invalid("Missing Wayland runtime directory"))?,
            )
            .join(display)
        };
        peer(UnixStream::connect(display)?.as_raw_fd())?
    };
    Ok((path, pid))
}

pub fn connect(path: &Path, compositor_pid: libc::pid_t) -> io::Result<UnixStream> {
    let stream = UnixStream::connect(path)?;
    if peer(stream.as_raw_fd())? != compositor_pid {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "SWAYSOCK belongs to a different Wayland compositor",
        ));
    }
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    Ok(stream)
}

pub fn receive(mut stream: impl Read) -> io::Result<(u32, Value)> {
    let mut header = [0u8; 14];
    stream.read_exact(&mut header)?;
    if &header[..6] != b"i3-ipc" {
        return Err(invalid("Invalid Sway IPC preamble"));
    }
    let size = u32::from_ne_bytes(header[6..10].try_into().unwrap()) as usize;
    if size > MAX_PAYLOAD {
        return Err(invalid("Oversized Sway IPC message"));
    }
    let kind = u32::from_ne_bytes(header[10..].try_into().unwrap());
    let mut bytes = vec![0; size];
    stream.read_exact(&mut bytes)?;
    Ok((kind, serde_json::from_slice(&bytes)?))
}

pub fn request(stream: &mut UnixStream, kind: u32, payload: &str) -> io::Result<Value> {
    let mut frame = Vec::with_capacity(14 + payload.len());
    frame.extend_from_slice(b"i3-ipc");
    frame.extend_from_slice(&(payload.len() as u32).to_ne_bytes());
    frame.extend_from_slice(&kind.to_ne_bytes());
    frame.extend_from_slice(payload.as_bytes());
    stream.write_all(&frame)?;
    let (reply, value) = receive(stream)?;
    if reply != kind {
        return Err(invalid("Unexpected Sway IPC response"));
    }
    Ok(value)
}

pub fn ready(stream: &UnixStream) -> bool {
    let mut watch = libc::pollfd {
        fd: stream.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    // SAFETY: watch is one initialized pollfd, polled without waiting.
    unsafe { libc::poll(&mut watch, 1, 0) > 0 }
}
