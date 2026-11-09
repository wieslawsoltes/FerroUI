# Independent Android library consumer

This separate workspace depends on public FerroBoard crates and the independent
compiled-XAML consumer. It has no sample dependency. Its activity uses the SDK's
native Java bootstrap and loads the consumer's Hosted.xaml.

Source `scripts/env.sh`, set ANDROID_HOME to the installed SDK and source the
selected SDK's `scripts/android/env.sh`. Build from the repository root:

```
cargo build --locked --offline --manifest-path tests/FerroBoard.Native.Android/Cargo.toml --target aarch64-linux-android --lib
```

Package with the installed JDK on PATH. Choose a new SSD-local output directory:

```
python3 scripts/bundle_android.py --library /tmp/ferrokeyboard/target/aarch64-linux-android/debug/libferroboard_native_android.so --manifest tests/FerroBoard.Native.Android/AndroidManifest.xml --java-source-root tests/FerroBoard.Native.Android/java --out /tmp/ferrokeyboard/independent-android-package
```

The packager includes library-owned Java assets without sample Java. Mobile hosts
install `KeyboardHapticService::native()` at their composition root; VIBRATE is an
application manifest permission. No network or platform IME substitute is used.
The test field is private with suggestions disabled. Runtime evidence and limits:
`docs/NATIVE-ANDROID-SERVICES.md`. Development signing is not production signing.

Native contacts can be exercised through the packaged PointerChecks instrumentation.
Install this development APK only on an explicitly selected test device, then run:

```
python3 tests/FerroBoard.Native.Android/run_pointer_checks.py --adb <workspace>/sdks/Android/platform-tools/adb --serial emulator-5584 --log /tmp/ferrokeyboard/android-pointer-result.log
```

The runner fails unless instrumentation reports success; adb shell's exit status
alone does not establish test success. The earlier selected SDK failed the
multi-contact ACTION_CANCEL recovery case. The selected isolated dependency now
includes the focused cancel-all correction and its regression. Current results:
`docs/NATIVE-ANDROID-CANCELLATION.md`; historical failure evidence:
`docs/NATIVE-ANDROID-POINTERS.md`. The keyboard-free baseline/selected reproducer
is in `docs/reproducers/android-cancel-all`. These are synthetic native events,
not physical multitouch validation. The shared FerroUI checkout is untouched.

The suite now also verifies API33+ FLAG_CANCELED at both pointer indices and a
surviving Shift chord. These added checks require API33+. Startup waits up to30s
for the initial field; gesture assertions retain3s and source settings remain
unchanged. Injection is synchronous without added per-event idle delays. The
runner cancels its own outstanding contacts before finishing a failed assertion.
See docs/NATIVE-ANDROID-CANCELED-POINTER.md for current scope and limits.

Clipboard probe and Clipboard denial are native diagnostic quick actions. Run only
on a task-owned emulator/profile: they overwrite its clipboard with synthetic data.
The first action tests real OS I/O, Core memory-target privacy and native field
restoration. The denial action expects READ_CLIPBOARD set to ignore externally;
capture and restore the prior effective app-op mode. An explicit `default` enum
was not equivalent to this device's original allow policy. Current evidence/limits:
docs/NATIVE-ANDROID-CLIPBOARD.md. The production sample is unchanged.

Selection notifications have a separate opt-in fixture. Build with
`--features native-selection-checks`, package with `SelectionManifest.xml` and
`--java-source-root tests/FerroBoard.Native.Android/selection-java`, then use the
existing strict instrumentation runner with
`--package org.ferroboard.selectionconsumer --instrumentation org.ferroboard.consumer.SelectionChecks --expected-checks 4`.
The four native cases cover complete UTF16 ranges, reentrant replacement and its
caret, unchanged-caret collapse, and replacement while another field owns focus
and the keyboard target. The old field's IME client has no presenter in the last
case. Source privacy is kept through incognito mode and disabled learning/history.
No sample dependency or production engine/adapter workaround is used. These are
synthetic OS taps into real controls, not physical touch or complete OS IME proof.
See `docs/NATIVE-ANDROID-SELECTION.md` for exact SDK/package/run identities.
