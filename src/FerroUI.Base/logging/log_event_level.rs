/// Specifies the meaning and relative importance of a log event.
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LogEventLevel {
    /// Anything and everything you might want to know about a running block
    /// of code.
    Verbose = 0,

    /// Internal system events that aren't necessarily observable from the
    /// outside.
    Debug = 1,

    /// The lifeblood of operational intelligence - things happen.
    Information = 2,

    /// Service is degraded or endangered.
    Warning = 3,

    /// Functionality is unavailable, invariants are broken or data is lost.
    Error = 4,

    /// If you have a pager, it goes off when one of these occurs.
    Fatal = 5,
}
