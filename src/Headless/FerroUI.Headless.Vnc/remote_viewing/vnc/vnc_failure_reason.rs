//! Port of `Vnc/VncFailureReason.cs` of RemoteViewing (BSD-2-Clause,
//! Copyright (c) 2013 James F. Bellinger; `NOTICE.md` of this crate).

/// Possible reasons a [`VncError`](super::VncError) is raised.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum VncFailureReason {
    /// Unknown reason.
    #[default]
    Unknown,

    /// The server isn't a VNC server.
    WrongKindOfServer,

    /// RemoteViewing can't speak the protocol versions this server offers.
    UnsupportedProtocolVersion,

    /// The server offered no authentication methods. This could mean that
    /// VNC is temporarily disabled.
    ServerOfferedNoAuthenticationMethods,

    /// The server offered no supported authentication methods.
    NoSupportedAuthenticationMethods,

    /// A password was required to authenticate but wasn't supplied.
    PasswordRequired,

    /// Authentication failed. This could mean you supplied an incorrect
    /// password.
    AuthenticationFailed,

    /// The server specified a pixel format RemoteViewing doesn't support.
    UnsupportedPixelFormat,

    /// A network error occured. The connection may have been lost.
    NetworkError,

    /// The server sent a value that seems unreasonable. This shouldn't
    /// happen in normal conditions.
    SanityCheckFailed,

    /// The server sent an unrecognized protocol element. This shouldn't
    /// happen in normal conditions.
    UnrecognizedProtocolElement,
}
