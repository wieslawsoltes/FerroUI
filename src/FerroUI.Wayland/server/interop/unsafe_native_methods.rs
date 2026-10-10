//! The calls of the C library the backend makes (the port of
//! `UnsafeNativeMethods.cs`), each as a safe function over owned
//! descriptors and mappings. This module and the three classes over
//! `libxkbcommon` are where the `unsafe` of the backend outside EGL is.
//!
//! The declarations of `libwayland-cursor` of that file are not ported: the
//! cursor themes are read by the crate `wayland-cursor`.

use std::ffi::CStr;
use std::io;
use std::os::fd::{AsRawFd, BorrowedFd, FromRawFd, OwnedFd, RawFd};

/// The result of [`poll_two`]: which of the two descriptors can be read.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PollResult {
    pub first: bool,
    pub second: bool,
}

/// A pipe whose ends do not block and are closed in a child process (`pipe2` with
/// `O_NONBLOCK | O_CLOEXEC`): the reading end and the writing end.
pub fn pipe2_nonblocking() -> io::Result<(OwnedFd, OwnedFd)> {
    let mut fds = [0 as RawFd; 2];
    // SAFETY: `fds` is an array of two descriptors, which is what the call writes.
    if unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_NONBLOCK | libc::O_CLOEXEC) } != 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the call succeeded, so both are open descriptors nobody else owns.
    Ok(unsafe { (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) })
}

/// Reads at most one byte from a descriptor that does not block. The result is not looked at
/// by the caller, as in the reference.
pub fn read_one(fd: BorrowedFd<'_>) {
    let mut byte = 0u8;
    // SAFETY: the buffer is one byte, the length given is one.
    let _ = unsafe { libc::read(fd.as_raw_fd(), (&mut byte as *mut u8).cast(), 1) };
}

/// Writes one zero byte to a descriptor that does not block.
pub fn write_one(fd: BorrowedFd<'_>) {
    let byte = 0u8;
    // SAFETY: the buffer is one byte, the length given is one.
    let _ = unsafe { libc::write(fd.as_raw_fd(), (&byte as *const u8).cast(), 1) };
}

/// Waits without a limit until one of two descriptors can be read (or is in error, or was
/// hung up: the read that follows reports it). Interrupted waits are taken up again.
pub fn poll_two(first: BorrowedFd<'_>, second: BorrowedFd<'_>) -> io::Result<PollResult> {
    let mut fds = [
        libc::pollfd { fd: first.as_raw_fd(), events: libc::POLLIN, revents: 0 },
        libc::pollfd { fd: second.as_raw_fd(), events: libc::POLLIN, revents: 0 },
    ];
    loop {
        // SAFETY: `fds` is an array of two entries, the count given is two.
        let result = unsafe { libc::poll(fds.as_mut_ptr(), 2, -1) };
        if result <= 0 {
            let error = io::Error::last_os_error();
            // poll is not restarted after a signal
            if result < 0 && error.raw_os_error() == Some(libc::EINTR) {
                continue;
            }
            return Err(error);
        }
        return Ok(PollResult { first: fds[0].revents != 0, second: fds[1].revents != 0 });
    }
}

/// A file that lives in memory (`memfd_create` with `MFD_CLOEXEC`) of `length` bytes.
pub fn memfd_of_length(name: &CStr, length: usize) -> io::Result<OwnedFd> {
    // SAFETY: the name is a terminated string that lives for the call.
    let fd = unsafe { libc::memfd_create(name.as_ptr(), libc::MFD_CLOEXEC) };
    if fd == -1 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the call succeeded, so this is an open descriptor nobody else owns.
    let fd = unsafe { OwnedFd::from_raw_fd(fd) };
    let length = libc::off_t::try_from(length).map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?;
    // SAFETY: the descriptor is open.
    if unsafe { libc::ftruncate(fd.as_raw_fd(), length) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(fd)
}

/// A file mapped into memory, unmapped when the value is dropped.
pub struct MemoryMapping {
    address: *mut u8,
    length: usize,
    writable: bool,
}

impl MemoryMapping {
    /// Maps `length` bytes of a file for reading and writing, shared with whoever else maps
    /// the file (`PROT_READ | PROT_WRITE`, `MAP_SHARED`).
    pub fn shared_read_write(fd: BorrowedFd<'_>, length: usize) -> io::Result<Self> {
        Self::map(fd, length, libc::PROT_READ | libc::PROT_WRITE, libc::MAP_SHARED)
    }

    /// Maps `length` bytes of a file for reading, privately (`PROT_READ`, `MAP_PRIVATE`).
    pub fn private_read(fd: BorrowedFd<'_>, length: usize) -> io::Result<Self> {
        Self::map(fd, length, libc::PROT_READ, libc::MAP_PRIVATE)
    }

    fn map(fd: BorrowedFd<'_>, length: usize, protection: i32, flags: i32) -> io::Result<Self> {
        if length == 0 {
            return Err(io::Error::from(io::ErrorKind::InvalidInput));
        }
        // SAFETY: a new mapping at an address the system chooses; the descriptor is open.
        let address = unsafe { libc::mmap(std::ptr::null_mut(), length, protection, flags, fd.as_raw_fd(), 0) };
        if address == libc::MAP_FAILED || address.is_null() {
            return Err(io::Error::last_os_error());
        }
        Ok(Self { address: address.cast(), length, writable: protection & libc::PROT_WRITE != 0 })
    }

    pub fn address(&self) -> *mut u8 {
        self.address
    }

    pub fn len(&self) -> usize {
        self.length
    }

    pub fn is_empty(&self) -> bool {
        self.length == 0
    }

    /// The bytes of a mapping made for reading.
    pub fn as_slice(&self) -> &[u8] {
        // SAFETY: the mapping is `length` readable bytes until the value is dropped.
        unsafe { std::slice::from_raw_parts(self.address, self.length) }
    }
}

impl MemoryMapping {
    /// The bytes of a mapping made for writing; `None` for one made for reading only.
    pub fn as_mut_slice(&mut self) -> Option<&mut [u8]> {
        if !self.writable {
            return None;
        }
        // SAFETY: the mapping is `length` readable and writable bytes until the value is
        // dropped, and the borrow of the value is exclusive.
        Some(unsafe { std::slice::from_raw_parts_mut(self.address, self.length) })
    }
}

impl Drop for MemoryMapping {
    fn drop(&mut self) {
        // SAFETY: the address and the length are those of the mapping, unmapped once.
        unsafe {
            libc::munmap(self.address.cast(), self.length);
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;
    use std::os::fd::AsFd;

    #[test]
    fn a_pipe_wakes_a_poll_and_is_empty_after_the_read() {
        let (read, write) = pipe2_nonblocking().unwrap();
        let (other_read, _other_write) = pipe2_nonblocking().unwrap();
        write_one(write.as_fd());
        let result = poll_two(other_read.as_fd(), read.as_fd()).unwrap();
        assert_eq!(result, PollResult { first: false, second: true });
        read_one(read.as_fd());
        // Nothing left: reading again does not block.
        read_one(read.as_fd());
    }

    #[test]
    fn a_memory_file_is_shared_between_its_mappings() {
        let fd = memfd_of_length(c"ferroui-wayland-test", 4096).unwrap();
        let mut writable = MemoryMapping::shared_read_write(fd.as_fd(), 4096).unwrap();
        assert_eq!(writable.len(), 4096);
        writable.as_mut_slice().unwrap()[10] = 42;
        let mut readable = MemoryMapping::private_read(fd.as_fd(), 4096).unwrap();
        assert!(readable.as_mut_slice().is_none());
        assert_eq!(readable.as_slice()[10], 42);
        assert_eq!(readable.as_slice()[11], 0);
    }

    #[test]
    fn an_empty_mapping_is_refused() {
        let fd = memfd_of_length(c"ferroui-wayland-test", 0).unwrap();
        assert!(MemoryMapping::shared_read_write(fd.as_fd(), 0).is_err());
    }
}
