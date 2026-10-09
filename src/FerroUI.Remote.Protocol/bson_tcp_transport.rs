use std::io::{Read, Write};
use std::sync::Arc;

use crate::assembly::ASSEMBLY;
use crate::bson_stream_transport::BsonStreamTransportConnection;
use crate::default_message_type_resolver::DefaultMessageTypeResolver;
use crate::i_message_type_resolver::IMessageTypeResolver;
use crate::i_transport::IFerroRemoteTransportConnection;
use crate::tcp_transport_base::{DisposeCallback, TcpTransportBase};
use crate::transport_connection_wrapper::TransportConnectionWrapper;

/// The transport of the protocol over TCP: BSON messages over the stream of
/// a socket.
#[derive(Clone)]
pub struct BsonTcpTransport {
    resolver: Arc<dyn IMessageTypeResolver>,
}

impl BsonTcpTransport {
    /// `new BsonTcpTransport(resolver)`.
    pub fn new(resolver: Arc<dyn IMessageTypeResolver>) -> BsonTcpTransport {
        BsonTcpTransport { resolver }
    }

    /// `new BsonTcpTransport()`: with the resolver of the messages of this
    /// library.
    pub fn empty() -> BsonTcpTransport {
        BsonTcpTransport::new(Arc::new(DefaultMessageTypeResolver::new(&[&ASSEMBLY])))
    }
}

impl Default for BsonTcpTransport {
    fn default() -> Self {
        BsonTcpTransport::empty()
    }
}

impl TcpTransportBase for BsonTcpTransport {
    fn resolver(&self) -> Arc<dyn IMessageTypeResolver> {
        self.resolver.clone()
    }

    fn create_transport(
        &self,
        resolver: Arc<dyn IMessageTypeResolver>,
        input_stream: Box<dyn Read + Send>,
        output_stream: Box<dyn Write + Send>,
        dispose_callback: DisposeCallback,
    ) -> Arc<dyn IFerroRemoteTransportConnection> {
        let t = BsonStreamTransportConnection::new(resolver, input_stream, output_stream, Some(dispose_callback));
        let wrap = TransportConnectionWrapper::new(t.clone());
        t.start_reading();
        wrap
    }
}
