//! The System V shared memory calls of the C library (the port of
//! `LibC.cs`), which the shared memory framebuffer uses.

use std::ffi::c_int;

/// create key if key does not exist
pub const IPC_CREAT: c_int = 0o1000;

/// private key
pub const IPC_PRIVATE: c_int = 0;

/// Remove the IPC object
pub const IPC_RMID: c_int = 0;

/// `shmget`: the identifier of a segment of `size` bytes, -1 on failure.
pub fn shmget(key: c_int, size: usize, shmflg: c_int) -> c_int {
    // SAFETY: plain call with values.
    unsafe { libc::shmget(key as libc::key_t, size, shmflg) }
}

/// `shmat(shmid, NULL, shmflg)`: the address the segment is attached at,
/// chosen by the system; `None` on failure (the reference compares the
/// result with -1).
pub fn shmat(shmid: c_int, shmflg: c_int) -> Option<*mut u8> {
    // SAFETY: with a null address the system chooses where to attach; the call fails for an
    // identifier that is not a segment.
    let address = unsafe { libc::shmat(shmid, std::ptr::null(), shmflg) };
    (address as isize != -1).then_some(address.cast())
}

/// `shmdt`.
///
/// # Safety
/// `shmaddr` must be an address [`shmat`] returned and that was not
/// detached yet, and nothing may use the memory afterwards.
pub unsafe fn shmdt(shmaddr: *mut u8) -> c_int {
    // SAFETY: the caller's contract.
    unsafe { libc::shmdt(shmaddr.cast_const().cast()) }
}

/// `shmctl(shmid, cmd, NULL)`: a command without a buffer (`IPC_RMID`).
pub fn shmctl(shmid: c_int, cmd: c_int) -> c_int {
    // SAFETY: the commands the backend sends take no buffer.
    unsafe { libc::shmctl(shmid, cmd, std::ptr::null_mut()) }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;

    #[test]
    fn the_constants_are_those_of_the_system() {
        assert_eq!(IPC_CREAT, libc::IPC_CREAT);
        assert_eq!(IPC_PRIVATE, libc::IPC_PRIVATE as c_int);
        assert_eq!(IPC_RMID, libc::IPC_RMID);
    }

    #[test]
    fn a_segment_is_created_attached_written_and_removed() {
        let shmid = shmget(IPC_PRIVATE, 4096, IPC_CREAT | 0o777);
        if shmid == -1 {
            // A system that refuses shared memory to the process (a sandbox): nothing to test.
            return;
        }
        let address = shmat(shmid, 0).expect("the segment that was just created attaches");
        // SAFETY: the segment has 4096 bytes at `address` until it is detached below.
        unsafe {
            address.write(0x5a);
            address.add(4095).write(0xa5);
            assert_eq!((address.read(), address.add(4095).read()), (0x5a, 0xa5));
            assert_eq!(shmdt(address), 0);
        }
        assert_eq!(shmctl(shmid, IPC_RMID), 0);
        // The identifier is gone.
        assert!(shmat(shmid, 0).is_none());
    }
}
