# Recorded anomalies

## A-001: Caret/popup-related contrast changes

**Disposition: deferred at the user's direction; continue the keyboard port.**
Do not resume this rendering investigation unless the user reopens it.

The user supplied `Screen Recording 2026-10-28 at 18.23.24.mov` and observed that
caret blinking appeared to change the window's contrast, possibly while a menu or
popup was open. Decoded frames show about 80% of the window changing, with a
representative key grey changing from approximately 65 to 58. These are recorded
video RGB values, not calibrated display measurements.

A separate plain Skia/Headless check with explicit caret states changes only 14
caret pixels. That does not diagnose the native-window/popup path. The running
validation app was the earlier c9bf285f826541626c0c35485e118df9706efa77d6f0b8a189db4cfe00f68fb2
build. No root cause or renderer correction is verified. No caret suppression or
rendering fallback was applied.

Evidence, extracted frames, the temporary probe source/binaries and anomaly.json
are retained at `/tmp/ferrokeyboard/caret-contrast-stage`. The diagnostic
window/process is closed and its example entry/source were removed from the
reusable library/sample project. Existing app data and neighboring ports are
unchanged. The original recording remains on the user's Desktop.
