//! The server side of remote rendering.

mod remote_server_top_level_impl;
mod remote_server_top_level_impl_framebuffer;

pub use remote_server_top_level_impl::{
    OnMessageOverride, RemoteServerTopLevelImpl, RemoteServerTopLevelImplOverrides, RemoteServerUiThreadHandle,
};
