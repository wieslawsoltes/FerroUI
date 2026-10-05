//! The transport between the UI-thread compositor and the server
//! compositor: batches of serialized changes.

mod batch;
mod batch_stream;
mod batch_stream_array_pool;
mod server_list_proxy_helper;

pub use batch::{BatchCompletion, CommittedBatch, CompositionBatch};
pub use batch_stream::{
    BatchMarker, BatchObject, BatchResource, BatchStreamData, BatchStreamReader, BatchStreamWriter, BatchValue, BatchValueReader,
    ServerJob, ServerObjectJob, ServerObjectFactory,
};
pub use batch_stream_array_pool::{
    BatchStreamMemoryPool, BatchStreamMemoryPoolItems, BatchStreamObjectPool, BatchStreamObjectPoolItems,
    BatchStreamPoolBase, BatchStreamPoolStartTimer, IBatchStreamPoolItems,
};
pub use server_list_proxy_helper::{IRegisterForSerialization, ServerListProxyHelper};
