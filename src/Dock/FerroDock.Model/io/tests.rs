// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::*;
use ferroui_system::IDisposable;
use std::{
    cell::{Cell, RefCell},
    io::{self, Read, Seek, SeekFrom, Write},
    rc::Rc,
};
fn failure<T>(result: io::Result<T>, expected: StreamError) -> bool {
    result.err().and_then(|error| {
        error
            .get_ref()
            .and_then(|error| error.downcast_ref::<StreamError>())
            .copied()
    }) == Some(expected)
}
#[test]
fn memory_stream_matches_original_source_reference_outcomes() {
    let corpus = include_str!("../../../../tests/Dock/data/serializer-stream-reference.tsv");
    let mut checked = Vec::new();
    let mut check = |name: &str, value: bool| {
        assert!(value, "{name}");
        assert!(
            corpus.lines().any(|row| row == format!("{name}\t1")),
            "source case missing: {name}"
        );
        checked.push(name.to_owned());
    };
    let stream = MemoryStream::new();
    let alias = stream.clone();
    check(
        "memory-new-capabilities",
        stream.can_read()
            && stream.can_write()
            && stream.can_seek()
            && stream.len().unwrap() == 0
            && stream.position().unwrap() == 0,
    );
    stream.write(&[1, 2, 3]).unwrap();
    check(
        "memory-write-position-length",
        stream.position().unwrap() == 3 && stream.len().unwrap() == 3,
    );
    stream.seek(SeekFrom::Start(0)).unwrap();
    let mut b = [0];
    stream.read(&mut b).unwrap();
    check(
        "memory-read-position",
        b[0] == 1 && stream.position().unwrap() == 1,
    );
    stream.seek(SeekFrom::Start(5)).unwrap();
    stream.write(&[9]).unwrap();
    check(
        "memory-expand-zero-gap",
        stream.to_array() == [1, 2, 3, 0, 0, 9] && stream.position().unwrap() == 6,
    );
    stream.set_len(2).unwrap();
    check(
        "memory-truncate-clamps-position",
        stream.position().unwrap() == 2 && stream.len().unwrap() == 2,
    );
    stream.set_len(5).unwrap();
    check("memory-regrow-zeroes", stream.to_array() == [1, 2, 0, 0, 0]);
    stream.set_position(4).unwrap();
    check(
        "memory-negative-seek-keeps-position",
        failure(stream.seek(SeekFrom::Current(-5)), StreamError::InvalidSeek)
            && stream.position().unwrap() == 4,
    );
    stream.close().unwrap();
    check(
        "memory-close-shared-capabilities",
        !alias.can_read() && !alias.can_write() && !alias.can_seek(),
    );
    check(
        "memory-close-read-rejected",
        failure(alias.read(&mut b), StreamError::Closed),
    );
    check(
        "memory-close-write-rejected",
        failure(alias.write(&[1]), StreamError::Closed),
    );
    check(
        "memory-close-seek-rejected",
        failure(alias.seek(SeekFrom::Start(0)), StreamError::Closed),
    );
    check(
        "memory-close-length-rejected",
        failure(alias.len(), StreamError::Closed),
    );
    check(
        "memory-close-position-rejected",
        failure(alias.position(), StreamError::Closed),
    );
    check(
        "memory-close-set-length-rejected",
        failure(alias.set_len(0), StreamError::NotSupported),
    );
    check(
        "memory-closed-negative-position-check-first",
        failure(alias.set_position(-1), StreamError::ArgumentOutOfRange),
    );
    check(
        "memory-closed-overflow-position-check-after-close",
        failure(alias.set_position(i32::MAX as i64 + 1), StreamError::Closed),
    );
    check(
        "memory-closed-invalid-length-check-first",
        failure(alias.set_len(-1), StreamError::ArgumentOutOfRange),
    );
    check(
        "memory-close-snapshot-retained",
        alias.to_array() == [1, 2, 0, 0, 0],
    );
    alias.close().unwrap();
    alias.dispose();
    alias.flush().unwrap();
    check(
        "memory-close-repeat-flush",
        !alias.can_read() && alias.to_array().len() == 5,
    );
    let bytes = Rc::new(RefCell::new(vec![3, 4, 5].into_boxed_slice()));
    let stream = MemoryStream::from_buffer(bytes.clone(), true);
    stream.write(&[8]).unwrap();
    check(
        "memory-fixed-buffer-original-identity",
        bytes.borrow()[0] == 8,
    );
    bytes.borrow_mut()[1] = 7;
    stream.read(&mut b).unwrap();
    check("memory-fixed-buffer-live-read", b[0] == 7);
    check(
        "memory-fixed-buffer-expand-rejected",
        failure(stream.set_len(4), StreamError::FixedCapacity),
    );
    stream.set_len(1).unwrap();
    stream.set_len(3).unwrap();
    check(
        "memory-fixed-regrow-zeroes-original-buffer",
        bytes.borrow().as_ref() == [8, 0, 0],
    );
    let readonly =
        MemoryStream::from_buffer(Rc::new(RefCell::new(vec![1, 2].into_boxed_slice())), false);
    check(
        "memory-readonly-capabilities",
        readonly.can_read() && !readonly.can_write() && readonly.can_seek(),
    );
    check(
        "memory-readonly-write-rejected",
        failure(readonly.write(&[9]), StreamError::NotSupported),
    );
    check(
        "memory-readonly-set-length-rejected",
        failure(readonly.set_len(1), StreamError::NotSupported),
    );
    readonly.flush().unwrap();
    check(
        "memory-readonly-flush-allowed",
        readonly.position().unwrap() == 0,
    );
    let stream = MemoryStream::new();
    stream.set_position(5).unwrap();
    stream.write(&[]).unwrap();
    check(
        "memory-empty-write-extends",
        stream.len().unwrap() == 5
            && stream.position().unwrap() == 5
            && stream.to_array().iter().all(|b| *b == 0),
    );
    let stream = MemoryStream::new();
    stream.set_position(5).unwrap();
    check(
        "memory-past-end-empty-read",
        stream.read(&mut []).unwrap() == 0
            && stream.position().unwrap() == 5
            && stream.len().unwrap() == 0,
    );
    check(
        "memory-max-position-without-allocation",
        stream.seek(SeekFrom::Start(i32::MAX as u64)).unwrap() == i32::MAX as u64
            && stream.len().unwrap() == 0,
    );
    check(
        "memory-position-overflow-rejected",
        failure(
            stream.set_position(i32::MAX as i64 + 1),
            StreamError::ArgumentOutOfRange,
        ) && stream.position().unwrap() == i32::MAX as u64,
    );
    check(
        "memory-seek-overflow-rejected",
        failure(
            stream.seek(SeekFrom::Start(i32::MAX as u64 + 1)),
            StreamError::ArgumentOutOfRange,
        ) && stream.position().unwrap() == i32::MAX as u64,
    );
    check(
        "memory-length-overflow-rejected",
        failure(
            stream.set_len(i32::MAX as i64 + 1),
            StreamError::ArgumentOutOfRange,
        ) && stream.len().unwrap() == 0,
    );
    assert_eq!(
        checked.len(),
        corpus
            .lines()
            .filter(|row| row.starts_with("memory-"))
            .count()
    );
}
#[test]
fn native_io_adapters_share_actual_identity_position_and_explicit_disposal() {
    let stream = MemoryStream::new();
    let source: Rc<dyn Stream> = stream.clone();
    let mut io = StreamIo::new(source.clone());
    assert!(Rc::ptr_eq(&io.stream(), &source));
    assert!(stream.is_empty().unwrap());
    let mut clone = io.clone();
    io.write_all(b"data").unwrap();
    assert_eq!(clone.stream().position().unwrap(), 4);
    clone.seek(SeekFrom::Start(0)).unwrap();
    let mut text = String::new();
    io.read_to_string(&mut text).unwrap();
    assert_eq!(text, "data");
    drop(io);
    assert!(source.can_read());
    clone.stream().close().unwrap();
    assert!(failure(clone.read(&mut [0]), StreamError::Closed));
    assert_eq!(stream.to_array(), b"data");
    assert!(clone.flush().is_ok());
}
struct CustomStream {
    inner: Rc<MemoryStream>,
    keep_open: bool,
    closes: Cell<usize>,
    before_read: RefCell<Option<Rc<dyn Fn()>>>,
}
impl CustomStream {
    fn new(keep_open: bool) -> Rc<Self> {
        Rc::new(Self {
            inner: MemoryStream::new(),
            keep_open,
            closes: Cell::new(0),
            before_read: RefCell::new(None),
        })
    }
}
impl Stream for CustomStream {
    fn can_read(&self) -> bool {
        self.inner.can_read()
    }
    fn can_write(&self) -> bool {
        self.inner.can_write()
    }
    fn can_seek(&self) -> bool {
        self.inner.can_seek()
    }
    fn len(&self) -> io::Result<u64> {
        self.inner.len()
    }
    fn position(&self) -> io::Result<u64> {
        self.inner.position()
    }
    fn set_position(&self, value: i64) -> io::Result<()> {
        self.inner.set_position(value)
    }
    fn set_len(&self, value: i64) -> io::Result<()> {
        self.inner.set_len(value)
    }
    fn read(&self, buffer: &mut [u8]) -> io::Result<usize> {
        let callback = self.before_read.borrow().clone();
        if let Some(callback) = callback {
            callback();
        }
        self.inner.read(buffer)
    }
    fn write(&self, buffer: &[u8]) -> io::Result<()> {
        self.inner.write(buffer)
    }
    fn seek(&self, from: SeekFrom) -> io::Result<u64> {
        self.inner.seek(from)
    }
    fn flush(&self) -> io::Result<()> {
        self.inner.flush()
    }
    fn close(&self) -> io::Result<()> {
        self.closes.set(self.closes.get() + 1);
        if self.keep_open {
            self.inner.flush()
        } else {
            self.inner.close()
        }
    }
}
impl IDisposable for CustomStream {
    fn dispose(&self) {
        self.close().unwrap();
    }
}
#[test]
fn custom_stream_retains_close_override_and_can_close_during_read_without_borrow_failure() {
    let custom = CustomStream::new(true);
    let stream: Rc<dyn Stream> = custom.clone();
    stream.write(b"original").unwrap();
    stream.close().unwrap();
    stream.dispose();
    assert_eq!(custom.closes.get(), 2);
    assert!(stream.can_read());
    let custom = CustomStream::new(false);
    let weak = Rc::downgrade(&custom);
    custom.before_read.replace(Some(Rc::new(move || {
        weak.upgrade().unwrap().close().unwrap()
    })));
    let source: Rc<dyn Stream> = custom.clone();
    let mut io = StreamIo::new(source);
    assert!(failure(io.read(&mut [0]), StreamError::Closed));
    assert_eq!(custom.closes.get(), 1);
}
#[test]
fn memory_default_and_native_failure_kinds_are_explicit() {
    let stream = MemoryStream::default();
    assert!(stream.is_empty().unwrap());
    for (error, kind) in [
        (StreamError::Closed, io::ErrorKind::BrokenPipe),
        (StreamError::NotSupported, io::ErrorKind::Unsupported),
        (StreamError::ArgumentOutOfRange, io::ErrorKind::InvalidInput),
        (StreamError::InvalidSeek, io::ErrorKind::Other),
        (StreamError::FixedCapacity, io::ErrorKind::Unsupported),
    ] {
        assert!(!error.to_string().is_empty());
        let native: io::Error = error.into();
        assert_eq!(native.kind(), kind);
        assert_eq!(
            native.get_ref().unwrap().downcast_ref::<StreamError>(),
            Some(&error)
        );
    }
}

#[test]
fn serializer_contract_preserves_null_streams_original_identity_and_generic_declared_types() {
    use crate::core::{DockStream, IDockSerializer, SerializationResult};
    use std::any::{Any, TypeId};
    #[derive(Default)]
    struct Probe {
        stream: RefCell<Option<Rc<dyn DockStream>>>,
        declared: Cell<Option<TypeId>>,
        value: RefCell<Option<Box<dyn Any>>>,
    }
    impl IDockSerializer for Probe {
        fn serialize<T: 'static>(&self, _: T) -> SerializationResult<String> {
            panic!("unused contract probe member")
        }
        fn deserialize<T: 'static>(&self, _: Option<&str>) -> SerializationResult<Option<T>> {
            panic!("unused contract probe member")
        }
        fn load<T: 'static>(
            &self,
            stream: Option<Rc<dyn DockStream>>,
        ) -> SerializationResult<Option<T>> {
            self.stream.replace(stream);
            self.declared.set(Some(TypeId::of::<T>()));
            Ok(None)
        }
        fn save<T: 'static>(
            &self,
            stream: Option<Rc<dyn DockStream>>,
            value: T,
        ) -> SerializationResult<()> {
            self.stream.replace(stream);
            self.declared.set(Some(TypeId::of::<T>()));
            self.value.replace(Some(Box::new(value)));
            Ok(())
        }
    }
    let corpus = include_str!("../../../../tests/Dock/data/serializer-stream-reference.tsv");
    let check = |name: &str, result: bool| {
        assert!(result, "{name}");
        assert!(corpus.lines().any(|row| row == format!("{name}\t1")));
    };
    let probe = Probe::default();
    let argument = Rc::new("original payload".to_owned());
    probe.load::<Rc<String>>(None).unwrap();
    check(
        "interface-load-null-dispatch",
        probe.stream.borrow().is_none() && probe.declared.get() == Some(TypeId::of::<Rc<String>>()),
    );
    probe.save(None, argument.clone()).unwrap();
    check(
        "interface-save-null-dispatch",
        probe.stream.borrow().is_none()
            && probe.declared.get() == Some(TypeId::of::<Rc<String>>())
            && Rc::ptr_eq(
                probe
                    .value
                    .borrow()
                    .as_ref()
                    .unwrap()
                    .downcast_ref::<Rc<String>>()
                    .unwrap(),
                &argument,
            ),
    );
    let original: Rc<dyn DockStream> = MemoryStream::new();
    probe.load::<String>(Some(original.clone())).unwrap();
    check(
        "interface-load-original-shared-stream",
        Rc::ptr_eq(probe.stream.borrow().as_ref().unwrap(), &original)
            && probe.declared.get() == Some(TypeId::of::<String>()),
    );
    probe.save(Some(original.clone()), 17_i32).unwrap();
    check(
        "interface-save-original-stream-and-generic-payload",
        Rc::ptr_eq(probe.stream.borrow().as_ref().unwrap(), &original)
            && probe.declared.get() == Some(TypeId::of::<i32>())
            && probe.value.borrow().as_ref().unwrap().downcast_ref::<i32>() == Some(&17),
    );
}
