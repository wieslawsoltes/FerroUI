use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::error::Error;
use crate::i_message_type_resolver::IMessageTypeResolver;
use crate::i_transport::IFerroRemoteTransportConnection;

/// What a connection calls when it is disposed.
pub type DisposeCallback = Box<dyn Fn() + Send + Sync>;

/// `DisposableServer`: what [`TcpTransportBase::listen`] returns. Disposing
/// it stops the listening; connections that were accepted stay open. As in
/// the original, a server that is never disposed keeps listening.
pub struct DisposableServer {
    address: SocketAddr,
    stopped: Arc<AtomicBool>,
}

impl DisposableServer {
    /// The address the server listens on, with the port the system assigned
    /// when the port asked for was zero. (An addition of the port: the
    /// original hands out only the `IDisposable`.)
    pub fn local_addr(&self) -> SocketAddr {
        self.address
    }

    pub fn dispose(&self) {
        if self.stopped.swap(true, Ordering::SeqCst) {
            return;
        }
        // `_l.Stop()`: a blocking accept of the standard library cannot be
        // stopped from another thread, so the listener thread is woken with a
        // connection and ends when it sees the flag.
        let mut address = self.address;
        if address.ip().is_unspecified() {
            let loopback = match address {
                SocketAddr::V4(_) => IpAddr::V4(Ipv4Addr::LOCALHOST),
                SocketAddr::V6(_) => IpAddr::V6(Ipv6Addr::LOCALHOST),
            };
            address.set_ip(loopback);
        }
        //Ignore
        let _ = TcpStream::connect_timeout(&address, Duration::from_secs(1));
    }
}

/// The two directions of a socket as the streams of a connection, and the
/// callback that closes the socket.
fn create_transport_of_stream<T: TcpTransportBase>(
    transport: &T,
    stream: TcpStream,
) -> Result<Arc<dyn IFerroRemoteTransportConnection>, Error> {
    let input = stream.try_clone()?;
    let output = stream.try_clone()?;
    let dispose_callback: DisposeCallback = Box::new(move || {
        let _ = stream.shutdown(Shutdown::Both);
    });
    Ok(transport.create_transport(transport.resolver(), Box::new(input), Box::new(output), dispose_callback))
}

/// A transport over TCP sockets. The abstract class of the original is a
/// trait: an implementor holds the resolver its constructor is given
/// ([`TcpTransportBase::resolver`]) and makes the connection of a socket
/// ([`TcpTransportBase::create_transport`]). It is `Clone` because the
/// listener thread keeps one.
pub trait TcpTransportBase: Clone + Send + 'static {
    /// `_resolver`: the resolver the transport was constructed with.
    fn resolver(&self) -> Arc<dyn IMessageTypeResolver>;

    /// Makes the connection over the stream of a socket. The original passes
    /// the one `Stream` of the socket, which is read and written; here the
    /// two directions are two handles of the socket.
    fn create_transport(
        &self,
        resolver: Arc<dyn IMessageTypeResolver>,
        input_stream: Box<dyn Read + Send>,
        output_stream: Box<dyn Write + Send>,
        dispose_callback: DisposeCallback,
    ) -> Arc<dyn IFerroRemoteTransportConnection>;

    /// Listens on the address and calls `cb` with the connection of every
    /// client. A port of zero lets the system choose
    /// ([`DisposableServer::local_addr`]). An address that cannot be bound is
    /// an error, as `TcpListener.Start` throws.
    ///
    /// Deviation (DEVIATIONS.md, Remote protocol): the original accepts with
    /// `AcceptTcpClientAsync` and calls back on the thread pool. Here a
    /// listener thread accepts, and each accepted connection gets a thread
    /// that makes its transport and calls `cb`, so that a callback that takes
    /// its time does not hold up the next client. The dispose callback of an
    /// accepted connection closes its socket (the original only completes a
    /// task), since that is what ends the blocking read of its reader thread.
    fn listen(
        &self,
        address: IpAddr,
        port: u16,
        cb: impl Fn(Arc<dyn IFerroRemoteTransportConnection>) + Send + Sync + 'static,
    ) -> Result<DisposableServer, Error> {
        let server = TcpListener::bind(SocketAddr::new(address, port))?;
        let local_address = server.local_addr()?;
        let stopped = Arc::new(AtomicBool::new(false));
        let cb = Arc::new(cb);
        let this = self.clone();
        let accept_stopped = stopped.clone();
        std::thread::Builder::new().name("remote-protocol-listener".to_string()).spawn(move || loop {
            let cl = match server.accept() {
                Ok((cl, _)) => cl,
                //Ignore and stop
                Err(_) => break,
            };
            if accept_stopped.load(Ordering::SeqCst) {
                break;
            }
            let this = this.clone();
            let cb = cb.clone();
            let _ = std::thread::Builder::new().name("remote-protocol-accept".to_string()).spawn(move || {
                //Ignore
                if let Ok(t) = create_transport_of_stream(&this, cl) {
                    cb(t);
                }
            });
        })?;
        Ok(DisposableServer { address: local_address, stopped })
    }

    /// Connects to a listening transport.
    ///
    /// Deviation (DEVIATIONS.md, Remote protocol): the original returns a
    /// task; this blocks until the connection is made or has failed.
    fn connect(&self, address: IpAddr, port: u16) -> Result<Arc<dyn IFerroRemoteTransportConnection>, Error> {
        let c = TcpStream::connect(SocketAddr::new(address, port))?;
        create_transport_of_stream(self, c)
    }
}
