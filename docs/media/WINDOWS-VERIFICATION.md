# Windows native verification — 2026-11-01

Starting revision: `6aac17e56e81e3ecd2b14113fc02ea0979be0f59` from
`origin/codex/portable-media-framework`. Platform work is isolated on
`codex/portable-media-windows`. All 19 existing media-branch commits already use
`Wiesław Šoltés <wieslawsoltes@users.noreply.github.com>` as committer; no history
rewrite was needed. Authors remain unchanged.

## Environment and fixes

Windows 11 Pro build 26200, NVIDIA GeForce GTX 1660 Ti, driver 32.0.16.1088,
Rust/Cargo 1.93.0, MSVC 14.50.35717 and Windows SDK 10.0.26100.0. ANGLE is the
repository's statically linked mozangle 0.7.1. Native compilation is enabled;
`FERROUI_NATIVE_MEDIA_CHECK_ONLY` is unset.

The first full MSVC build linked successfully. Native fixture playback then
exposed an actual format mismatch: MF RGB32 produced XRGB storage that the
bundled ANGLE cannot import. Request ARGB32/BGRA instead and reject XRGB at
admission. No CPU conversion or preview fallback was added.

Controlled context recovery exposed a paused seek/Stop race: up to three
read-ahead frames could become GPU-ready before the first frame was published.
The worker subsequently replaced the paused frame with later queued frames.
Publish the first sought frame once and retain future frames for resume. An
independent native test exercises the queued GPU-ready frames and resume.

Keyboard verification exposed Play at Ended hanging against MF's EOF state.
Restart seekable playback in a new publication epoch, wait for that epoch's
first native frame, and prevent an old EOS iteration from overwriting a newer
seek/play command. The `--ended-restart-smoke` harness requires a newer source
sequence, a near-start timestamp and an advancing Playing clock before continuing
the normal pause/seek/Stop checks; an accepted Play command alone is insufficient.

Count the requested MF RGB-processing boundary once per admitted sample, rather
than again on a failed admission/retry. This remains an accounting boundary,
not an audit of native MFT/codec/driver internals or proof of hardware decoding.

## Evidence

- Two opt-in native tests pass. Independent D3D11 GPU clears produce four
  quadrants in texture-array slice 1. Actual D3D readiness and ANGLE image/FBO
  diagnostic reads preserve the pixels. Native sample/array retention,
  incompatible device/size/bind flags, unsupported XRGB, WARP rejection and
  paused read-ahead/resume pass. These diagnostic/photo reads are test-only;
  production preview remains unmapped.
- The shared media, camera, GL ownership and video suites pass **40 tests**
  (5 + 7 + 3 + 25), including queued cleanup and unknown-completion accounting.
- Native AVC/AAC resource playback, pause, achieved 500 ms seek, resume,
  Stop/frame zero and final allocation retirement pass. The same source feeds
  the normal view and the client-owned GLES projected renderer. Controlled
  context replacement preserves paused intent/position, creates a new source
  generation, resumes rendering, and retires both source and renderer storage.
- Ended/restart plus renderer reuse passes the strict near-start/new-frame gate
  and then pause/seek/Stop/retirement. Final resource, combined renderer/recovery
  and repeated recovery runs pass. Admitted production preview counters show
  **zero CPU attempts/bytes and zero GPU copies**. Required native RGB processing
  is counted separately.
- Camera discovery returns zero devices with NotDetermined permission before
  and after and `acquired=false`. There is no physical camera on this profile.
- Actual Windows UI Automation exposes Play/Pause, Stop, Playback position,
  Volume, Mute and the playback-status text. Tab and Shift+Tab move focus between
  transport buttons; Space activates Stop (Paused at 0.0 s) and Play (Playing
  with an advancing position). Visual inspection shows the four source
  quadrants and projected renderer. Full screen-reader acceptance is separate.
- A copy of the executable runs the resource smoke from a separate directory.
  PE dependency inspection shows system MF/D3D/UI DLLs and the MSVC/UCRT runtime;
  ANGLE does not require companion EGL/GLES DLLs. This host already has the
  Visual C++ runtime installed; this is not a clean-machine installer test.

## Reproduction

From the worktree root in PowerShell:

```powershell
$env:PATH = "C:\Users\Dan\.cargo\bin;" + $env:PATH
$env:CARGO_TARGET_DIR = 'F:\build-cache\ferroui-portable-media-windows'
cargo build --locked -p portable-media --target x86_64-pc-windows-msvc
cargo test --locked -p ferroui-media-windows --features native-frame-tests --target x86_64-pc-windows-msvc --lib -- --nocapture
cargo test --locked -p ferroui-media -p ferroui-media-video -p ferroui-media-camera -p ferroui-media-opengl --features ferroui-media-opengl/controls --target x86_64-pc-windows-msvc --lib
$app = Join-Path $env:CARGO_TARGET_DIR 'x86_64-pc-windows-msvc\debug\portable-media.exe'
& $app tests/media/fixtures/avc-aac.mp4 --resource --smoke
& $app tests/media/fixtures/avc-aac.mp4 --resource --smoke --renderer --device-recovery-smoke
& $app tests/media/fixtures/avc-aac.mp4 --resource --smoke --renderer --ended-restart-smoke
& $app --camera-discovery
```

Raw logs are outside Git beside the Windows worktree in
`C:\Users\Dan\.codex\worktrees\native-desktop-platforms\`:
`windows-build-final.log`, `windows-native-frames-final.log`,
`windows-media-contracts.log`, `windows-final-resource.log`,
`windows-final-renderer-recovery.log`, `windows-final-ended-restart.log`,
`windows-final-recovery-repeat.log`, `windows-camera-discovery.log`,
`windows-packaged-playback.log`, `windows-pe-dependencies.log`,
`windows-environment.json` and `windows-accessibility.json`.
The original format and recovery failures remain in the earlier logs.

## Remaining acceptance

Physical camera preview, privacy denial/cancellation, hotplug and independently
decoded JPEG capture require a device. Wider codecs, geometry/color profiles,
abrupt driver loss, independently measured native audio/AV sync, full Narrator
coverage, older Windows/clean-machine installation and frozen sustained
performance workloads remain open. WGL/Vulkan direct media routes and desktop
native QR are not established by the exercised ANGLE/GLES profile. Functional
passes here do not declare complete portable-media release acceptance.
