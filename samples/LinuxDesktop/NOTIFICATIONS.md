# Desktop notifications

The Ferro Desktop login session serves `org.freedesktop.Notifications` on the session bus. Existing clients can use libnotify, `notify-send` or the D-Bus interface directly.

```sh
notify-send --app-name="Build service" "Build succeeded" "All targets passed."
```

Open **Notifications** on the shell or press **Ctrl+N** with shell focus. **Do Not Disturb** silences banners for this session and keeps incoming messages in the center. **Clear all** dismisses active notifications and removes expired history. Windowed previews leave notification delivery with the current desktop.

## Delivery

- The server advertises plain-text bodies and actions. Selecting a button emits `ActionInvoked`. A non-resident notification then emits `NotificationClosed` with reason 2.
- Replacing an active identifier updates its content and restarts its timeout. `CloseNotification` explicitly retires an active identifier with reason 3. An invalid identifier produces a D-Bus error.
- Expiration emits reason 1. Normal notifications use six seconds when the client requests the server default. Critical notifications with the default timeout and notifications with timeout zero stay active until recalled, dismissed or evicted. Banners yield the screen after six seconds.
- Expired messages remain as text in the history unless the sender set the transient hint. Retired actions cannot be invoked. The session keeps up to 64 entries, evicting expired history before active notifications. Evicting an active notification emits reason 4.
- One dispatcher timer tracks the next deadline. Native banner windows are created on demand and reused for replacements. The center mounts eight entries at a time. The service does not queue for or replace the name of another notification daemon.

## Native validation

With the native test dependencies plus GLib's `gdbus`, X11's `xprop`, and ImageMagick installed, run the protocol and visual smoke test from the repository root.

```sh
samples/LinuxDesktop/tests/notifications.sh target/release/ferro-desktop /tmp/ferro-notifications
```

It creates a private display and session bus, launches independent clients, and checks notification replacement, action invocation, expiration, recall, Do Not Disturb, eviction and name release. The output includes the D-Bus signal trace and screenshots of the banner and center.

[Freedesktop desktop notifications protocol](https://specifications.freedesktop.org/notification/latest/protocol.html)
