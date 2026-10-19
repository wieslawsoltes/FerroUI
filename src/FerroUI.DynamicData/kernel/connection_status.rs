//! `ConnectionStatus` (`Kernel/ConnectionStatus.cs`).

/// The state of the connection to a sequence (`ConnectionStatus`), as `MonitorStatus` reports it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ConnectionStatus {
    /// No value yet (`Pending`).
    #[default]
    Pending = 0,
    /// A value has arrived (`Loaded`).
    Loaded = 1,
    /// The sequence has failed (`Errored`).
    Errored = 2,
    /// The sequence has ended (`Completed`).
    Completed = 3,
}
