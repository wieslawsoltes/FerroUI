//! Port of the `Vnc` directory of RemoteViewing: the types the server
//! session shares with the client of the library (pixel formats,
//! framebuffers, rectangles, key symbols, the event arguments of input) and
//! the stream the protocol is read from and written to.

mod authentication_method;
mod extended_desktop_size_reason;
mod extended_desktop_size_status;
mod i_vnc_framebuffer_source;
mod i_vnc_password_challenge;
mod key_changed_event_args;
mod key_sym;
mod pointer_changed_event_args;
mod remote_clipboard_changed_event_args;
pub mod server;
mod version;
mod vnc_encoding;
mod vnc_exception;
mod vnc_failure_reason;
mod vnc_framebuffer;
mod vnc_password_challenge;
mod vnc_pixel_format;
mod vnc_rectangle;
mod vnc_stream;
mod vnc_transport;
mod vnc_utility;

pub use authentication_method::AuthenticationMethod;
pub use extended_desktop_size_reason::ExtendedDesktopSizeReason;
pub use extended_desktop_size_status::ExtendedDesktopSizeStatus;
pub use i_vnc_framebuffer_source::IVncFramebufferSource;
pub use i_vnc_password_challenge::IVncPasswordChallenge;
pub use key_changed_event_args::KeyChangedEventArgs;
pub use key_sym::KeySym;
pub use pointer_changed_event_args::PointerChangedEventArgs;
pub use remote_clipboard_changed_event_args::RemoteClipboardChangedEventArgs;
pub use version::Version;
pub use vnc_encoding::VncEncoding;
pub use vnc_exception::VncError;
#[allow(deprecated)]
pub use vnc_exception::VncException;
pub use vnc_failure_reason::VncFailureReason;
pub use vnc_framebuffer::VncFramebuffer;
pub use vnc_password_challenge::VncPasswordChallenge;
pub use vnc_pixel_format::VncPixelFormat;
pub use vnc_rectangle::VncRectangle;
pub(crate) use vnc_stream::VncStream;
pub(crate) use vnc_transport::TransportWriter;
pub use vnc_transport::VncTransport;
pub(crate) use vnc_utility::VncUtility;
