//! The descriptor that wakes the worker from its wait (the port of
//! `WakeupFd.cs`): a pipe with at most one byte in it.

use super::unsafe_native_methods::{pipe2_nonblocking, read_one, write_one};
use std::io;
use std::os::fd::{AsFd, BorrowedFd, OwnedFd};
use std::sync::{Mutex, PoisonError};

pub struct WakeupFd {
    read: OwnedFd,
    write: OwnedFd,
    signaled: Mutex<bool>,
}

impl WakeupFd {
    pub fn new() -> io::Result<Self> {
        let (read, write) = pipe2_nonblocking()?;
        Ok(Self { read, write, signaled: Mutex::new(false) })
    }

    /// The descriptor to wait on.
    pub fn poll_fd(&self) -> BorrowedFd<'_> {
        self.read.as_fd()
    }

    pub fn clear(&self) {
        let mut signaled = self.signaled.lock().unwrap_or_else(PoisonError::into_inner);
        if !*signaled {
            return;
        }
        read_one(self.read.as_fd());
        *signaled = false;
    }

    pub fn set(&self) {
        let mut signaled = self.signaled.lock().unwrap_or_else(PoisonError::into_inner);
        if *signaled {
            return;
        }
        write_one(self.write.as_fd());
        *signaled = true;
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;
    use crate::server::interop::unsafe_native_methods::poll_two;

    #[test]
    fn setting_twice_is_one_byte_and_clearing_empties_the_pipe() {
        let wakeup = WakeupFd::new().unwrap();
        let other = WakeupFd::new().unwrap();
        wakeup.set();
        wakeup.set();
        assert!(poll_two(other.poll_fd(), wakeup.poll_fd()).unwrap().second);
        wakeup.clear();
        // The pipe is empty again: only the other descriptor wakes a wait now.
        other.set();
        let result = poll_two(other.poll_fd(), wakeup.poll_fd()).unwrap();
        assert!(result.first);
        assert!(!result.second);
        // Clearing what is not set reads nothing (and so does not block).
        wakeup.clear();
    }
}
