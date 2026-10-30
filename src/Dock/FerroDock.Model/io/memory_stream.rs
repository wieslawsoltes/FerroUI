// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::{Stream, StreamError};
use ferroui_system::IDisposable;
use std::{
    cell::{Cell, RefCell},
    io::{self, SeekFrom},
    rc::Rc,
};

enum Storage {
    Expandable(RefCell<Vec<u8>>),
    Fixed(Rc<RefCell<Box<[u8]>>>),
}
/// Shared in-memory source stream for serialization and native I/O consumers.
/// Closing keeps its bytes available through ToArray, as the source memory stream does.
/// Offset/capacity constructors and the complete framework stream surface are not exposed.
pub struct MemoryStream {
    storage: Storage,
    length: Cell<usize>,
    position: Cell<usize>,
    writable: bool,
    closed: Cell<bool>,
}
impl Default for MemoryStream {
    fn default() -> Self {
        Self {
            storage: Storage::Expandable(RefCell::new(Vec::new())),
            length: Cell::new(0),
            position: Cell::new(0),
            writable: true,
            closed: Cell::new(false),
        }
    }
}
impl MemoryStream {
    /// Creates an empty expandable stream.
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }
    /// Wraps the original fixed-size byte array, retaining its identity and live contents.
    pub fn from_buffer(buffer: Rc<RefCell<Box<[u8]>>>, writable: bool) -> Rc<Self> {
        let length = buffer.borrow().len();
        Rc::new(Self {
            storage: Storage::Fixed(buffer),
            length: Cell::new(length),
            position: Cell::new(0),
            writable,
            closed: Cell::new(false),
        })
    }
    /// Copies the logical contents into an independent array, including after Close.
    pub fn to_array(&self) -> Vec<u8> {
        match &self.storage {
            Storage::Expandable(bytes) => bytes.borrow()[..self.length.get()].to_vec(),
            Storage::Fixed(bytes) => bytes.borrow()[..self.length.get()].to_vec(),
        }
    }
    fn ensure_open(&self) -> io::Result<()> {
        if self.closed.get() {
            Err(StreamError::Closed.into())
        } else {
            Ok(())
        }
    }
    fn ensure_writable(&self) -> io::Result<()> {
        self.ensure_open()?;
        if self.writable {
            Ok(())
        } else {
            Err(StreamError::NotSupported.into())
        }
    }
    fn checked_argument(value: i64) -> io::Result<usize> {
        if !(0..=i32::MAX as i64).contains(&value) {
            Err(StreamError::ArgumentOutOfRange.into())
        } else {
            Ok(value as usize)
        }
    }
    fn resize(&self, length: usize) -> io::Result<()> {
        match &self.storage {
            Storage::Expandable(bytes) => {
                let mut bytes = bytes.borrow_mut();
                if length > bytes.len() {
                    let extra = length - bytes.len();
                    bytes.try_reserve(extra).map_err(io::Error::other)?;
                    bytes.resize(length, 0);
                }
                if length > self.length.get() {
                    bytes[self.length.get()..length].fill(0);
                }
            }
            Storage::Fixed(bytes) => {
                let mut bytes = bytes.borrow_mut();
                if length > bytes.len() {
                    return Err(StreamError::FixedCapacity.into());
                }
                if length > self.length.get() {
                    bytes[self.length.get()..length].fill(0);
                }
            }
        }
        self.length.set(length);
        Ok(())
    }
}
impl Stream for MemoryStream {
    fn can_read(&self) -> bool {
        !self.closed.get()
    }
    fn can_write(&self) -> bool {
        !self.closed.get() && self.writable
    }
    fn can_seek(&self) -> bool {
        !self.closed.get()
    }
    fn len(&self) -> io::Result<u64> {
        self.ensure_open()?;
        Ok(self.length.get() as u64)
    }
    fn position(&self) -> io::Result<u64> {
        self.ensure_open()?;
        Ok(self.position.get() as u64)
    }
    fn set_position(&self, value: i64) -> io::Result<()> {
        if value < 0 {
            return Err(StreamError::ArgumentOutOfRange.into());
        }
        self.ensure_open()?;
        let value = Self::checked_argument(value)?;
        self.position.set(value);
        Ok(())
    }
    fn set_len(&self, value: i64) -> io::Result<()> {
        let value = Self::checked_argument(value)?;
        if !self.can_write() {
            return Err(StreamError::NotSupported.into());
        }
        self.resize(value)?;
        if self.position.get() > value {
            self.position.set(value);
        }
        Ok(())
    }
    fn read(&self, buffer: &mut [u8]) -> io::Result<usize> {
        self.ensure_open()?;
        let count = buffer
            .len()
            .min(self.length.get().saturating_sub(self.position.get()));
        if count != 0 {
            let start = self.position.get();
            let end = start + count;
            match &self.storage {
                Storage::Expandable(bytes) => {
                    buffer[..count].copy_from_slice(&bytes.borrow()[start..end])
                }
                Storage::Fixed(bytes) => {
                    buffer[..count].copy_from_slice(&bytes.borrow()[start..end])
                }
            }
            self.position.set(end);
        }
        Ok(count)
    }
    fn write(&self, buffer: &[u8]) -> io::Result<()> {
        self.ensure_writable()?;
        let start = self.position.get();
        let end = start
            .checked_add(buffer.len())
            .filter(|n| *n <= i32::MAX as usize)
            .ok_or(StreamError::ArgumentOutOfRange)?;
        if end > self.length.get() {
            self.resize(end)?;
        }
        match &self.storage {
            Storage::Expandable(bytes) => bytes.borrow_mut()[start..end].copy_from_slice(buffer),
            Storage::Fixed(bytes) => bytes.borrow_mut()[start..end].copy_from_slice(buffer),
        }
        self.position.set(end);
        Ok(())
    }
    fn seek(&self, from: SeekFrom) -> io::Result<u64> {
        self.ensure_open()?;
        let value = match from {
            SeekFrom::Start(value) => {
                if value > i32::MAX as u64 {
                    return Err(StreamError::ArgumentOutOfRange.into());
                }
                value as i128
            }
            SeekFrom::Current(offset) => {
                if offset > i32::MAX as i64 {
                    return Err(StreamError::ArgumentOutOfRange.into());
                }
                self.position.get() as i128 + offset as i128
            }
            SeekFrom::End(offset) => {
                if offset > i32::MAX as i64 {
                    return Err(StreamError::ArgumentOutOfRange.into());
                }
                self.length.get() as i128 + offset as i128
            }
        };
        if !(0..=i32::MAX as i128).contains(&value) {
            return Err(StreamError::InvalidSeek.into());
        }
        self.position.set(value as usize);
        Ok(value as u64)
    }
    // Source MemoryStream.Flush remains a no-op even after disposal.
    fn flush(&self) -> io::Result<()> {
        Ok(())
    }
    fn close(&self) -> io::Result<()> {
        self.closed.set(true);
        Ok(())
    }
}
impl IDisposable for MemoryStream {
    fn dispose(&self) {
        self.closed.set(true);
    }
}
