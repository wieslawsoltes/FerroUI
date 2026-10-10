# NOTICE

This crate is ported from the `Avalonia.FreeDesktop` project of Avalonia (MIT;
see the `NOTICE.md` at the root of the repository).

The D-Bus interfaces it speaks are those of other projects, written here as
proxy traits from the interface descriptions the upstream project keeps in
`DBusXml/`: `org.freedesktop.IBus.Portal`, `.InputContext` and `.Service` of
IBus, `org.fcitx.Fcitx.InputMethod`, `.InputContext`, `.InputMethod1` and
`.InputContext1` of Fcitx. The key state and capability values of
`dbus_ime/ibus/ibus_enums.rs` and `dbus_ime/fcitx/fcitx_enums.rs` are the
values of those projects' protocols, as the upstream files have them.

## Dependencies

| Crate | Version | Licence | Used for |
|---|---|---|---|
| `zbus` | 5.19.0 (exact) | MIT | D-Bus: the connection to the session bus, proxies for the interfaces above, signals as streams. Pure Rust; nothing of `libdbus` is linked. Default features (`async-io`, `blocking-api`): the connection reads its socket on a thread of its own; no Tokio. In the tests also its feature `p2p` (a connection over a socket pair, without a bus). |
| `futures-util` | 0.3 | MIT OR Apache-2.0 | The streams of signals (`StreamExt`) and waiting on one of two futures. |
| `bitflags` | 2 | MIT OR Apache-2.0 | The flag sets of the input method protocols. |

`zbus` brings, among others: `zvariant`, `zbus_names`, `zbus_macros`,
`zvariant_derive`, `zvariant_utils` (MIT, the same project); `async-io`,
`async-executor`, `async-task`, `async-lock`, `async-broadcast`,
`async-channel`, `async-process`, `async-signal`, `async-recursion`,
`async-trait`, `blocking`, `event-listener`, `futures-lite`, `polling`,
`parking`, `piper`, `concurrent-queue`, `fastrand` (MIT OR Apache-2.0);
`serde`, `serde_repr`, `enumflags2`, `ordered-stream`, `hex`, `uuid`,
`tracing`, `endi`, `memoffset`, `zcheapstr` (MIT or MIT OR Apache-2.0).
`Cargo.lock` at the root of the repository has the exact versions.
