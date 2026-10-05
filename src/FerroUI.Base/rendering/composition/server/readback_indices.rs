use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};

/// A helper class used to manage the current slots for writing data from the render thread
/// and reading it from the UI thread.
/// Used mostly by hit-testing which needs to know the last transform of the visual
///
/// The lock is held for the whole duration of a write: [`begin_write`]
/// returns a scope that owns the lock, and [`ReadbackWriteScope::end_write`]
/// (or dropping the scope) completes the write and releases it. The
/// revisions themselves are atomics and can be read from any thread without
/// taking the lock, as the properties of the C# class can.
///
/// Unlike the C# monitor the lock is not reentrant: calling
/// [`next_read`](Self::next_read), [`begin_write`](Self::begin_write) or
/// [`lock`](Self::lock) on the thread that currently holds a write scope or
/// a lock guard deadlocks.
///
/// [`begin_write`]: Self::begin_write
pub struct ReadbackIndices {
    lock: Mutex<()>,

    read_revision: AtomicU64,
    next_write_revision: AtomicU64,
    write_revision: AtomicU64,
    last_completed_write: AtomicU64,
}

impl ReadbackIndices {
    pub fn new() -> Self {
        Self {
            lock: Mutex::new(()),
            read_revision: AtomicU64::new(0),
            next_write_revision: AtomicU64::new(1),
            write_revision: AtomicU64::new(0),
            last_completed_write: AtomicU64::new(0),
        }
    }

    /// Takes the lock that guards the indices (the public `_lock` field of
    /// the C# class), blocking writes and [`next_read`](Self::next_read)
    /// while the guard is alive.
    pub fn lock(&self) -> MutexGuard<'_, ()> {
        self.lock.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn read_revision(&self) -> u64 {
        self.read_revision.load(Ordering::SeqCst)
    }

    pub fn write_revision(&self) -> u64 {
        self.write_revision.load(Ordering::SeqCst)
    }

    pub fn last_completed_write(&self) -> u64 {
        self.last_completed_write.load(Ordering::SeqCst)
    }

    pub fn next_read(&self) {
        let _guard = self.lock();
        self.read_revision.store(self.last_completed_write.load(Ordering::SeqCst), Ordering::SeqCst);
    }

    /// Takes the lock and starts the next write revision. The write lasts
    /// until the returned scope is ended.
    pub fn begin_write(&self) -> ReadbackWriteScope<'_> {
        let guard = self.lock();
        let revision = self.next_write_revision.fetch_add(1, Ordering::SeqCst);
        self.write_revision.store(revision, Ordering::SeqCst);
        ReadbackWriteScope { indices: self, _guard: guard }
    }
}

impl Default for ReadbackIndices {
    fn default() -> Self {
        Self::new()
    }
}

/// A write in progress, started by [`ReadbackIndices::begin_write`].
///
/// Ending the scope (explicitly with [`end_write`](Self::end_write) or by
/// dropping it) marks the write revision as completed and releases the lock.
#[must_use = "the write ends as soon as the scope is dropped"]
pub struct ReadbackWriteScope<'a> {
    indices: &'a ReadbackIndices,
    _guard: MutexGuard<'a, ()>,
}

impl ReadbackWriteScope<'_> {
    /// Completes the write and releases the lock (C# `EndWrite`).
    pub fn end_write(self) {}
}

impl Drop for ReadbackWriteScope<'_> {
    fn drop(&mut self) {
        // The guard field is released after this body runs.
        self.indices
            .last_completed_write
            .store(self.indices.write_revision.load(Ordering::SeqCst), Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn is_send_and_sync() {
        assert_send_sync::<ReadbackIndices>();
    }

    #[test]
    fn initial_state() {
        let indices = ReadbackIndices::new();
        assert_eq!(0, indices.read_revision());
        assert_eq!(0, indices.write_revision());
        assert_eq!(0, indices.last_completed_write());
        indices.next_read();
        assert_eq!(0, indices.read_revision());
    }

    #[test]
    fn write_revisions_advance_and_complete_on_end_write() {
        let indices = ReadbackIndices::new();

        let write = indices.begin_write();
        assert_eq!(1, indices.write_revision());
        assert_eq!(0, indices.last_completed_write());
        write.end_write();
        assert_eq!(1, indices.last_completed_write());
        // Readers only see a completed write after `next_read`.
        assert_eq!(0, indices.read_revision());
        indices.next_read();
        assert_eq!(1, indices.read_revision());

        indices.begin_write().end_write();
        indices.begin_write().end_write();
        assert_eq!(3, indices.write_revision());
        assert_eq!(3, indices.last_completed_write());
        assert_eq!(1, indices.read_revision());
        indices.next_read();
        assert_eq!(3, indices.read_revision());
    }

    #[test]
    fn dropping_the_scope_ends_the_write() {
        let indices = ReadbackIndices::new();
        {
            let _write = indices.begin_write();
            assert_eq!(0, indices.last_completed_write());
        }
        assert_eq!(1, indices.last_completed_write());
        // The lock is free again.
        drop(indices.lock());
    }

    #[test]
    fn the_lock_is_held_during_a_write() {
        let indices = ReadbackIndices::new();
        let write = indices.begin_write();
        assert!(indices.lock.try_lock().is_err());
        write.end_write();
        assert!(indices.lock.try_lock().is_ok());
    }

    #[test]
    fn next_read_waits_for_a_write_in_progress() {
        let indices = Arc::new(ReadbackIndices::new());
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let (finish_tx, finish_rx) = std::sync::mpsc::channel::<()>();

        let writer = {
            let indices = indices.clone();
            std::thread::spawn(move || {
                let write = indices.begin_write();
                started_tx.send(()).unwrap();
                finish_rx.recv().unwrap();
                write.end_write();
            })
        };

        started_rx.recv().unwrap();
        // The write is in progress: nothing is completed yet.
        assert_eq!(1, indices.write_revision());
        assert_eq!(0, indices.last_completed_write());

        let reader = {
            let indices = indices.clone();
            std::thread::spawn(move || {
                // Blocks until the writer releases the lock.
                indices.next_read();
                indices.read_revision()
            })
        };

        finish_tx.send(()).unwrap();
        writer.join().unwrap();
        assert_eq!(1, reader.join().unwrap());
    }
}
