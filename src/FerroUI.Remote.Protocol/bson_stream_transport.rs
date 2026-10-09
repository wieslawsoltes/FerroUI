//! A connection over a pair of byte streams.
//!
//! A message on the wire is a header of twenty bytes and a BSON document:
//! the length of the document (a little-endian `int32` that does not count
//! the header), the sixteen bytes of the identifier of the message class
//! (`Guid.ToByteArray`), then the document.

use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, Weak};

use crate::error::{catch_handler, Error};
use crate::guid::Guid;
use crate::i_message_type_resolver::IMessageTypeResolver;
use crate::i_transport::{
    Delegate, ExceptionHandler, HandlerToken, IFerroRemoteTransportConnection, Message, MessageHandler,
};
use crate::metsys_bson::{invalid_cast, BinaryReader, BsonObject, Deserializer, Serializer, Type, Value, ValueRef};
use crate::task::Task;

/// The fields the original guards with `_lock`.
#[derive(Default)]
struct Flags {
    write_operation_pending: bool,
    reading_already_started: bool,
    writer_is_broken: bool,
}

pub(crate) struct BsonStreamTransportConnection {
    this: Weak<BsonStreamTransportConnection>,
    resolver: Arc<dyn IMessageTypeResolver>,
    // The reader thread takes the input stream when reading starts.
    input_stream: Mutex<Option<Box<dyn Read + Send>>>,
    output_stream: Mutex<Box<dyn Write + Send>>,
    dispose_callback: Option<Box<dyn Fn() + Send + Sync>>,
    // Deviation (DEVIATIONS.md, Remote protocol): the original cancels its
    // pending reads and writes with a cancellation token and ignores the
    // `OperationCanceledException` of that token. A blocking read cannot be
    // cancelled: the dispose callback closes the stream, which ends the read,
    // and this flag says that what the read then reports is the cancellation.
    cancel: AtomicBool,
    output_block: Mutex<Vec<u8>>,
    lock: Mutex<Flags>,
    on_message: Mutex<Delegate<Message>>,
    on_exception: Mutex<Delegate<Error>>,
}

const ZERO_LENGTH: [u8; 4] = [0; 4];

impl BsonStreamTransportConnection {
    pub(crate) fn new(
        resolver: Arc<dyn IMessageTypeResolver>,
        input_stream: Box<dyn Read + Send>,
        output_stream: Box<dyn Write + Send>,
        dispose_callback: Option<Box<dyn Fn() + Send + Sync>>,
    ) -> Arc<BsonStreamTransportConnection> {
        Arc::new_cyclic(|this| BsonStreamTransportConnection {
            this: this.clone(),
            resolver,
            input_stream: Mutex::new(Some(input_stream)),
            output_stream: Mutex::new(output_stream),
            dispose_callback,
            cancel: AtomicBool::new(false),
            output_block: Mutex::new(Vec::new()),
            lock: Mutex::new(Flags::default()),
            on_message: Mutex::new(Delegate::new()),
            on_exception: Mutex::new(Delegate::new()),
        })
    }

    /// Starts the reader thread (`Task.Run(Reader, _cancel)` in the
    /// original). Panics when reading has already started, where the
    /// original throws an `InvalidOperationException`.
    pub(crate) fn start_reading(&self) {
        let already_started = std::mem::replace(&mut self.lock.lock().unwrap().reading_already_started, true);
        if already_started {
            panic!("Reading has already started");
        }
        let this = self.this.upgrade().expect("the connection is alive while a member of it is called");
        let input_stream = self.input_stream.lock().unwrap().take().expect("the input stream is taken once");
        std::thread::Builder::new()
            .name("remote-protocol-reader".to_string())
            .spawn(move || this.reader(input_stream))
            .expect("failed to start the reader thread");
    }

    fn read_exact(&self, input_stream: &mut dyn Read, buffer: &mut [u8]) -> Result<(), Error> {
        let mut read = 0;
        while read != buffer.len() {
            let read_now = match input_stream.read(&mut buffer[read..]) {
                Ok(read_now) => read_now,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e.into()),
            };
            if read_now == 0 {
                return Err(Error::EndOfStream);
            }
            read += read_now;
        }
        Ok(())
    }

    fn reader(&self, mut input_stream: Box<dyn Read + Send>) {
        let result = self.read_messages(&mut *input_stream);
        if let Err(e) = result {
            self.fire_exception(e);
        }
    }

    /// The loop of `Reader`: it ends with the first error, of the stream, of
    /// the deserializer, of the resolver or of a handler of the message.
    fn read_messages(&self, input_stream: &mut dyn Read) -> Result<(), Error> {
        loop {
            let mut info_block = [0u8; 20];
            self.read_exact(input_stream, &mut info_block)?;
            let length = i32::from_le_bytes([info_block[0], info_block[1], info_block[2], info_block[3]]);
            let guid = Guid::from_byte_array(&info_block[4..20])?;
            if length < 0 {
                // `new byte[length]`.
                return Err(Error::Overflow("Arithmetic operation resulted in an overflow.".to_string()));
            }
            let mut buffer = vec![0u8; length as usize];
            self.read_exact(input_stream, &mut buffer)?;
            let class = self.resolver.get_by_guid(guid)?;
            let message = Deserializer::deserialize_type(&mut BinaryReader::new(&buffer), &Type::Class(class), None)?;
            let message: Message = match message {
                Value::Object(object) => Arc::from(object),
                other => return Err(invalid_cast(&other, class.name)),
            };
            let handlers = self.on_message.lock().unwrap().snapshot();
            // An exception of a handler ends the loop in the original as any
            // other; a panic of a handler does here.
            catch_handler(|| {
                for handler in &handlers {
                    handler(self, &message);
                }
            })?;
        }
    }

    fn send_core(&self, data: &dyn BsonObject) -> Result<(), Error> {
        {
            let mut flags = self.lock.lock().unwrap();
            if flags.writer_is_broken {
                //Ignore further calls, since there is no point of writing to "broken" stream
                return Ok(());
            }
            if flags.write_operation_pending {
                return Err(Error::InvalidOperation("Previous send operation was not finished".to_string()));
            }
            flags.write_operation_pending = true;
        }
        let result = self.write(data);
        self.lock.lock().unwrap().write_operation_pending = false;
        result
    }

    fn write(&self, data: &dyn BsonObject) -> Result<(), Error> {
        let guid = self.resolver.get_guid(data.get_type())?.to_byte_array();
        let mut output_block = self.output_block.lock().unwrap();
        output_block.clear();
        output_block.extend_from_slice(&ZERO_LENGTH);
        output_block.extend_from_slice(&guid);
        let serialized = Serializer::serialize_object(ValueRef::Object(data))?;
        output_block.extend_from_slice(&serialized);
        let length = (output_block.len() as i32 - 20).to_le_bytes();
        output_block[0..4].copy_from_slice(&length);

        let written = if self.cancel.load(Ordering::SeqCst) {
            Err(std::io::Error::new(std::io::ErrorKind::Other, "The operation was canceled."))
        } else {
            let mut output_stream = self.output_stream.lock().unwrap();
            output_stream.write_all(&output_block).and_then(|_| output_stream.flush())
        };
        if let Err(e) = written {
            //We are only catching "network"-related exceptions here
            self.lock.lock().unwrap().writer_is_broken = true;
            self.fire_exception(e.into());
        }
        Ok(())
    }

    fn fire_exception(&self, e: Error) {
        // `if (cancel?.CancellationToken == _cancel) return;`
        if self.cancel.load(Ordering::SeqCst) {
            return;
        }
        let handlers = self.on_exception.lock().unwrap().snapshot();
        for handler in &handlers {
            handler(self, &e);
        }
    }
}

impl IFerroRemoteTransportConnection for BsonStreamTransportConnection {
    fn dispose(&self) {
        self.cancel.store(true, Ordering::SeqCst);
        if let Some(dispose_callback) = &self.dispose_callback {
            dispose_callback();
        }
    }

    /// Serializes and writes on the calling thread; the task is completed
    /// when it is returned. A second send while one is being written fails
    /// with "Previous send operation was not finished".
    fn send(&self, data: Message) -> Task {
        Task::from_result(self.send_core(&*data))
    }

    fn on_message(&self, handler: MessageHandler) -> HandlerToken {
        self.on_message.lock().unwrap().add(handler)
    }

    fn remove_on_message(&self, token: HandlerToken) {
        self.on_message.lock().unwrap().remove(token);
    }

    fn on_exception(&self, handler: ExceptionHandler) -> HandlerToken {
        self.on_exception.lock().unwrap().add(handler)
    }

    fn remove_on_exception(&self, token: HandlerToken) {
        self.on_exception.lock().unwrap().remove(token);
    }

    fn start(&self) {}
}

// Tests of the port over an in-memory duplex stream: the upstream project
// tests the transport over a socket only (`tests/remote_protocol_tests.rs`).
#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::io;
    use std::sync::mpsc::{channel, Receiver};
    use std::sync::Condvar;
    use std::time::Duration;

    use super::*;
    use crate::default_message_type_resolver::DefaultMessageTypeResolver;
    use crate::i_transport::{exception_handler, message_handler};
    use crate::metsys_bson::Object;
    use crate::viewport_messages::{FrameMessage, MeasureViewportMessage, PixelFormat};

    const TIMEOUT: Duration = Duration::from_secs(5);

    #[derive(Default)]
    struct PipeState {
        buffer: VecDeque<u8>,
        closed: bool,
    }

    /// One direction of an in-memory stream: what is written at one end is
    /// read at the other; a closed pipe reads as the end of the stream once
    /// it is empty and refuses writes.
    #[derive(Default)]
    struct Pipe {
        state: Mutex<PipeState>,
        changed: Condvar,
    }

    impl Pipe {
        fn close(&self) {
            self.state.lock().unwrap().closed = true;
            self.changed.notify_all();
        }
    }

    struct PipeReader(Arc<Pipe>);

    impl Read for PipeReader {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let mut state = self.0.state.lock().unwrap();
            while state.buffer.is_empty() && !state.closed {
                state = self.0.changed.wait(state).unwrap();
            }
            let count = buf.len().min(state.buffer.len());
            for slot in buf.iter_mut().take(count) {
                *slot = state.buffer.pop_front().unwrap();
            }
            Ok(count)
        }
    }

    struct PipeWriter(Arc<Pipe>);

    impl Write for PipeWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            let mut state = self.0.state.lock().unwrap();
            if state.closed {
                return Err(io::Error::new(io::ErrorKind::BrokenPipe, "the pipe is closed"));
            }
            state.buffer.extend(buf.iter().copied());
            self.0.changed.notify_all();
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn resolver() -> Arc<dyn IMessageTypeResolver> {
        Arc::new(DefaultMessageTypeResolver::new(&[]))
    }

    /// Two connections over an in-memory duplex stream. Disposing either
    /// closes both directions, as closing a socket does.
    fn pair() -> (Arc<BsonStreamTransportConnection>, Arc<BsonStreamTransportConnection>) {
        let a_to_b = Arc::new(Pipe::default());
        let b_to_a = Arc::new(Pipe::default());
        let close = |first: &Arc<Pipe>, second: &Arc<Pipe>| -> Option<Box<dyn Fn() + Send + Sync>> {
            let (first, second) = (first.clone(), second.clone());
            Some(Box::new(move || {
                first.close();
                second.close();
            }))
        };
        let a = BsonStreamTransportConnection::new(
            resolver(),
            Box::new(PipeReader(b_to_a.clone())),
            Box::new(PipeWriter(a_to_b.clone())),
            close(&a_to_b, &b_to_a),
        );
        let b = BsonStreamTransportConnection::new(
            resolver(),
            Box::new(PipeReader(a_to_b.clone())),
            Box::new(PipeWriter(b_to_a.clone())),
            close(&a_to_b, &b_to_a),
        );
        (a, b)
    }

    fn messages_of(connection: &BsonStreamTransportConnection) -> Receiver<Message> {
        let (sender, receiver) = channel();
        connection.on_message(message_handler(move |_, message| {
            let _ = sender.send(message.clone());
        }));
        receiver
    }

    fn exceptions_of(connection: &BsonStreamTransportConnection) -> Receiver<Error> {
        let (sender, receiver) = channel();
        connection.on_exception(exception_handler(move |_, e| {
            let _ = sender.send(e.clone());
        }));
        receiver
    }

    fn measure(width: f64, height: f64) -> Message {
        Arc::new(MeasureViewportMessage { width, height })
    }

    fn as_measure(message: &Message) -> MeasureViewportMessage {
        message.downcast_ref::<MeasureViewportMessage>().expect("a MeasureViewportMessage").clone()
    }

    #[test]
    fn a_message_is_a_header_of_length_and_identifier_and_a_document() {
        let wire = Arc::new(Pipe::default());
        let unused = Arc::new(Pipe::default());
        let connection = BsonStreamTransportConnection::new(
            resolver(),
            Box::new(PipeReader(unused)),
            Box::new(PipeWriter(wire.clone())),
            None,
        );
        connection.send(measure(1.0, 2.0)).wait().unwrap();
        let written: Vec<u8> = wire.state.lock().unwrap().buffer.iter().copied().collect();
        let expected: [u8; 56] = [
            0x24, 0, 0, 0, // the length of the document, without the header
            0x10, 0x53, 0x3C, 0x6E, 0xB1, 0xE2, 0x3D, 0x4C, 0x86, 0x88, 0x01, 0x18, 0x3A, 0xA4, 0x8C,
            0x5B, // the identifier
            0x24, 0, 0, 0, // the document
            0x01, 0x57, 0x69, 0x64, 0x74, 0x68, 0, 0, 0, 0, 0, 0, 0, 0xF0, 0x3F, // Width
            0x01, 0x48, 0x65, 0x69, 0x67, 0x68, 0x74, 0, 0, 0, 0, 0, 0, 0, 0, 0x40, // Height
            0,
        ];
        assert_eq!(written, expected);
    }

    #[test]
    fn messages_sent_both_ways_arrive_intact_and_in_order() {
        let (a, b) = pair();
        let (at_a, at_b) = (messages_of(&a), messages_of(&b));
        a.start_reading();
        b.start_reading();
        for i in 0..50 {
            a.send(measure(i as f64, 0.5)).wait().unwrap();
            b.send(measure(-(i as f64), 1.5)).wait().unwrap();
        }
        for i in 0..50 {
            let received = as_measure(&at_b.recv_timeout(TIMEOUT).unwrap());
            assert_eq!(received, MeasureViewportMessage { width: i as f64, height: 0.5 });
            let received = as_measure(&at_a.recv_timeout(TIMEOUT).unwrap());
            assert_eq!(received, MeasureViewportMessage { width: -(i as f64), height: 1.5 });
        }
        a.dispose();
    }

    #[test]
    fn many_small_messages_arrive_in_order() {
        let (a, b) = pair();
        let at_b = messages_of(&b);
        b.start_reading();
        for i in 0..2000 {
            a.send(measure(i as f64, 0.0)).wait().unwrap();
        }
        for i in 0..2000 {
            assert_eq!(as_measure(&at_b.recv_timeout(TIMEOUT).unwrap()).width, i as f64);
        }
        a.dispose();
    }

    #[test]
    fn a_large_message_arrives_intact() {
        let (a, b) = pair();
        let at_b = messages_of(&b);
        b.start_reading();
        let data: Vec<u8> = (0..4 * 1024 * 1024).map(|i| (i % 251) as u8).collect();
        let frame = FrameMessage {
            sequence_id: 7,
            format: PixelFormat::Bgra8888,
            data: Some(data),
            width: 1024,
            height: 1024,
            stride: 4096,
            dpi_x: 96.0,
            dpi_y: 192.0,
        };
        a.send(Arc::new(frame.clone())).wait().unwrap();
        let received = at_b.recv_timeout(TIMEOUT).unwrap();
        assert_eq!(received.downcast_ref::<FrameMessage>(), Some(&frame));
        a.dispose();
    }

    #[test]
    fn a_closed_connection_raises_the_exception_event_once() {
        let (a, b) = pair();
        let (errors_a, errors_b) = (exceptions_of(&a), exceptions_of(&b));
        a.start_reading();
        b.start_reading();
        a.dispose();
        assert!(matches!(errors_b.recv_timeout(TIMEOUT), Ok(Error::EndOfStream)));
        assert!(errors_b.recv_timeout(Duration::from_millis(200)).is_err());
        // The end that was disposed takes the end of its stream for the
        // cancellation it asked for.
        assert!(errors_a.recv_timeout(Duration::from_millis(200)).is_err());
    }

    #[test]
    fn a_send_after_dispose_is_dropped_without_an_exception() {
        let (a, _b) = pair();
        let errors = exceptions_of(&a);
        a.dispose();
        assert!(a.send(measure(1.0, 1.0)).wait().is_ok());
        assert!(a.send(measure(1.0, 1.0)).wait().is_ok());
        assert!(errors.recv_timeout(Duration::from_millis(100)).is_err());
    }

    #[test]
    fn a_failed_write_raises_the_exception_event_once_and_breaks_the_writer() {
        let (a, b) = pair();
        let errors = exceptions_of(&a);
        // The other end closes the stream; this end has not been disposed.
        b.dispose();
        assert!(a.send(measure(1.0, 1.0)).wait().is_ok());
        assert!(matches!(errors.recv_timeout(TIMEOUT), Ok(Error::Io(_))));
        assert!(a.send(measure(1.0, 1.0)).wait().is_ok());
        assert!(errors.recv_timeout(Duration::from_millis(100)).is_err());
    }

    #[test]
    fn a_message_of_an_unknown_class_fails_the_send_and_leaves_the_connection_usable() {
        let (a, b) = pair();
        let at_b = messages_of(&b);
        b.start_reading();
        assert!(matches!(a.send(Arc::new(Object)).wait(), Err(Error::KeyNotFound(_))));
        a.send(measure(3.0, 4.0)).wait().unwrap();
        assert_eq!(
            as_measure(&at_b.recv_timeout(TIMEOUT).unwrap()),
            MeasureViewportMessage { width: 3.0, height: 4.0 }
        );
        a.dispose();
    }

    #[test]
    fn an_unknown_identifier_ends_the_reading_with_an_exception() {
        let wire = Arc::new(Pipe::default());
        let unused = Arc::new(Pipe::default());
        let connection = BsonStreamTransportConnection::new(
            resolver(),
            Box::new(PipeReader(wire.clone())),
            Box::new(PipeWriter(unused)),
            None,
        );
        let errors = exceptions_of(&connection);
        let messages = messages_of(&connection);
        connection.start_reading();
        let mut frame = vec![5u8, 0, 0, 0];
        frame.extend_from_slice(&[0xAB; 16]);
        frame.extend_from_slice(&[5, 0, 0, 0, 0]);
        PipeWriter(wire.clone()).write_all(&frame).unwrap();
        assert!(matches!(errors.recv_timeout(TIMEOUT), Ok(Error::KeyNotFound(_))));
        // The reader has stopped: a valid message that follows is not read.
        let sender = BsonStreamTransportConnection::new(
            resolver(),
            Box::new(PipeReader(Arc::new(Pipe::default()))),
            Box::new(PipeWriter(wire.clone())),
            None,
        );
        sender.send(measure(1.0, 1.0)).wait().unwrap();
        assert!(messages.recv_timeout(Duration::from_millis(200)).is_err());
        wire.close();
    }

    #[test]
    fn a_negative_length_ends_the_reading_with_an_overflow() {
        let wire = Arc::new(Pipe::default());
        let connection = BsonStreamTransportConnection::new(
            resolver(),
            Box::new(PipeReader(wire.clone())),
            Box::new(PipeWriter(Arc::new(Pipe::default()))),
            None,
        );
        let errors = exceptions_of(&connection);
        connection.start_reading();
        let mut frame = vec![0xFFu8, 0xFF, 0xFF, 0xFF];
        frame.extend_from_slice(&MeasureViewportMessage::GUID.to_byte_array());
        PipeWriter(wire.clone()).write_all(&frame).unwrap();
        assert!(matches!(errors.recv_timeout(TIMEOUT), Ok(Error::Overflow(_))));
        wire.close();
    }

    #[test]
    fn a_stream_that_ends_inside_a_message_is_an_end_of_stream() {
        let wire = Arc::new(Pipe::default());
        let connection = BsonStreamTransportConnection::new(
            resolver(),
            Box::new(PipeReader(wire.clone())),
            Box::new(PipeWriter(Arc::new(Pipe::default()))),
            None,
        );
        let errors = exceptions_of(&connection);
        connection.start_reading();
        let mut frame = vec![0x24u8, 0, 0, 0];
        frame.extend_from_slice(&MeasureViewportMessage::GUID.to_byte_array());
        frame.extend_from_slice(&[0x24, 0, 0, 0, 0x01]);
        PipeWriter(wire.clone()).write_all(&frame).unwrap();
        wire.close();
        assert!(matches!(errors.recv_timeout(TIMEOUT), Ok(Error::EndOfStream)));
    }

    #[test]
    #[should_panic(expected = "Reading has already started")]
    fn reading_starts_once() {
        let (a, _b) = pair();
        a.start_reading();
        a.start_reading();
    }

    /// A stream whose write waits until it is released.
    struct BlockedWriter {
        entered: std::sync::mpsc::Sender<()>,
        release: Arc<(Mutex<bool>, Condvar)>,
    }

    impl Write for BlockedWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            let _ = self.entered.send(());
            let (released, changed) = &*self.release;
            let mut released = released.lock().unwrap();
            while !*released {
                released = changed.wait(released).unwrap();
            }
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn a_send_while_another_is_being_written_is_an_invalid_operation() {
        let (entered_sender, entered) = channel();
        let release = Arc::new((Mutex::new(false), Condvar::new()));
        let connection = BsonStreamTransportConnection::new(
            resolver(),
            Box::new(PipeReader(Arc::new(Pipe::default()))),
            Box::new(BlockedWriter { entered: entered_sender, release: release.clone() }),
            None,
        );
        let first = {
            let connection = connection.clone();
            std::thread::spawn(move || connection.send(measure(1.0, 1.0)).wait())
        };
        entered.recv_timeout(TIMEOUT).unwrap();
        match connection.send(measure(2.0, 2.0)).wait() {
            Err(Error::InvalidOperation(message)) => assert_eq!(message, "Previous send operation was not finished"),
            other => panic!("expected an invalid operation, got {:?}", other),
        }
        *release.0.lock().unwrap() = true;
        release.1.notify_all();
        assert!(first.join().unwrap().is_ok());
        // The pending flag is cleared when the write has finished.
        assert!(connection.send(measure(3.0, 3.0)).wait().is_ok());
    }
}
