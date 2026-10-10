# The iOS platform

The design of the iOS backend of FerroUI, the decisions it rests on, the file table of the port and its stages. Upstream: `src/iOS/Avalonia.iOS` (40 files, 57 types, 331 members, 5380 lines) at the tracked commit (`TRACKING.md`), with the sample host `samples/ControlCatalog.iOS`.

Marks, as in the other platform documents: **[V]** verified from sources (the upstream files, the sources of a crate in the cargo registry, `cargo info`, the system headers through the bindings), **[M]** measured here (a build, a test run or a tool run on the development machine: a Mac with Xcode 26.4.1, the iOS 26.4 SDK and simulator runtime, Rust 1.89 or later with the targets `aarch64-apple-ios` and `aarch64-apple-ios-sim`), **[S]** shown by a run in the simulator (section 12 says which runs happened), **[R]** recalled and not verified here.

## 1. The crate

| Crate | Directory | Upstream | Enabled by |
|---|---|---|---|
| `ferroui-ios` | `src/iOS/FerroUI.iOS` | `Avalonia.iOS` | `AppBuilder::use_ios()` / `use_ios_with_delegate(..)` (`IosApplicationExtensions`); an application starts with `ferroui_ios::run_application::<App>()` |

It depends on `ferroui-base`, `ferroui-controls`, `ferroui-metal` (the Metal contracts it implements), and on `ferroui-skia` and `ferroui-harfbuzz`, because the platform selects its renderer and its shaper as upstream's `UseiOS` does (`.UseHarfBuzz().UseSkia()`; the upstream project references both).

What is compiled where:

| Part | Compiled for | Why |
|---|---|---|
| Everything that calls UIKit, Core Animation or Metal | `target_os = "ios"` | The frameworks exist there only. |
| The dispatcher and its declarations (`dispatcher_impl.rs`, `interop.rs`) | `target_vendor = "apple"` | They need Core Foundation and libdispatch and nothing of UIKit, so the dispatcher runs, and is tested, on the run loop of a macOS process. |
| The logic of every file that is not a call into the system: the options and the choice of the rendering mode, the translation of a touch (device, event type, pressure, modifiers, timestamp), the safe area of a view, the view controller's state, the insets manager, the orientation and bounds of a screen, the stubs, the activation events | every target | It is what `cargo test -p ferroui-ios` runs on the development machine and in CI. |

A file with both kinds has its logic at the top and its UIKit part in a module `uikit` behind the configuration. The view controller is the example: the Objective-C class (`DefaultFerroViewController`) only forwards to `ViewControllerState`, which is what the view and the insets manager hold (as `Rc<dyn IFerroViewController>`) and what the tests drive.

The example (`examples/ios_view.rs`) needs a theme, an optional dependency behind the feature `example`, as in the X11 crate.

Minimum system: iOS 13.4. Upstream supports earlier systems with branches (`OperatingSystem.IsIOSVersionAtLeast(13)`, `(13, 4)`): a window created by the application delegate where there are no scenes, no modifier keys and button masks on touch events. The port takes the later branch everywhere and does not have the earlier ones (DEVIATIONS.md). The Rust targets set their own minimum (the simulator executable built here states 14.0 **[M]**, which is also the minimum of the published Skia binary for the simulator on Apple silicon **[V]**).

tvOS and Mac Catalyst, which the upstream project also targets, are not targets of the port: their branches (`#if TVOS`, `#if MACCATALYST`, the remote's swipe gestures) are left out and listed with the files.

## 2. Bindings: the `objc2` family, no native library

The macOS backend talks to a native Objective-C++ library (`native/FerroUI.Native`) through COM-style interfaces, because that is what upstream does there. Upstream's iOS backend has no native library: it calls UIKit from managed code through the bindings of the .NET iOS workload, and declares its own Objective-C classes with attributes (`[Export]`, `[Register]`). The port does the same in Rust with the **`objc2`** crates:

| Crate | Version | Licence | In the lock file before | Used for |
|---|---|---|---|---|
| `objc2` | 0.6.5 | MIT | yes (`wgpu`, the Vello backend) | messages, `define_class!` for the classes the platform registers |
| `block2` | 0.6.2 | MIT | yes | blocks of notification observers |
| `objc2-foundation` | 0.3.2 | MIT | yes | sets, strings, notifications, run loops |
| `objc2-core-foundation` | 0.3.2 | Zlib OR Apache-2.0 OR MIT | yes | `CGRect`, `CGSize`, `CGPoint` |
| `objc2-quartz-core` | 0.3.2 | Zlib OR Apache-2.0 OR MIT | yes | `CAMetalLayer`, `CADisplayLink` |
| `objc2-metal` | 0.3.2 | Zlib OR Apache-2.0 OR MIT | yes | device, command queue, drawable, textures |
| `objc2-ui-kit` | 0.3.2 | Zlib OR Apache-2.0 OR MIT | **no: the one new package** | `UIView`, `UIViewController`, `UIWindow`, scenes, touches, screens |

Versions and licences from `cargo info` **[V]**; all are pinned exactly in the manifest of the crate, each without its default features and with the list of framework headers the backend uses (the default is every header of a framework). The lock file gained one package **[M]**.

Why not a native shim (an Objective-C library compiled by `cc`, as on macOS):

| | `objc2` | Native shim |
|---|---|---|
| Follows upstream file by file | Yes: one Rust file per upstream file, the class in it declared where upstream declares it. | No: every class would be split into an Objective-C half and a Rust half with an interface between them that upstream does not have. |
| Build | Cargo only; cross-compiles from a Mac with the Rust target installed **[M]**. | Needs clang for the iOS SDK in the build script of the crate, per target, and an interface definition to keep in step. |
| Safety | The bindings carry what the headers say: main-thread-only classes need a `MainThreadMarker`, protocols state `Send + Sync` (`MTLDevice`, `MTLCommandQueue` **[V]**), nullability is `Option`. Most calls the backend makes are safe functions; `unsafe` remains where a header cannot state a contract (a selector, a block's thread, a class object). | Everything behind the interface is unchecked C. |
| Cost | A dependency with generated bindings for every header enabled. | None in dependencies. |

The `unsafe` of the crate, all of it around Objective-C, Core Foundation or Metal calls, each block with its argument: `define_class!` declarations (five classes) and the `msg_send![super(..)]` calls of their overrides; the notification observers; the display link; the scene configuration's delegate class; the `Send + Sync` of the layer handle and of the dispatcher's signal; the `extern` calls of `interop.rs`; the blit and the read-back of a frame capture.

Classes registered with the Objective-C runtime (names are process-wide, and none has the upstream name): `FerroAppDelegate`, `FerroSceneDelegate`, `FerroView`, `DefaultFerroViewController`, `FerroDisplayLinkTarget`.

## 3. Skia and HarfBuzz for iOS

- **Skia binaries are published for both targets with the feature set of macOS.** The Skia backend asks for `graphite` and `metal` on every Apple target (`cfg(target_vendor = "apple")` in its manifest), which with the default features of the bindings gives the key `graphite-jpegd-jpege-metal-pdf`. `skia-bindings` 0.153.3 builds the name of its binary from its commit, the target and the features (`build_support/binary_cache/binaries.rs`) **[V]**. Probed with a one-byte range request, as the Windows job of CI does: for `aarch64-apple-ios-sim`, `aarch64-apple-ios` and `x86_64-apple-ios` the release has `graphite-jpegd-jpege-metal-pdf`, `ganesh-jpegd-jpege-metal-pdf`, `ganesh-gl-jpegd-jpege-pdf` and `jpegd-jpege-pdf` **[M]**. The build scripts for both targets printed `DOWNLOAD AND INSTALL SUCCEEDED` for the Graphite key: no source build happened **[M]**.
- So the rendering path is the one of macOS: **Graphite on Metal**, with the Metal GPU of the Skia backend (`gpu/metal`, compiled for `target_vendor = "apple"` already). Ganesh on Metal is not needed, and Ganesh on OpenGL ES (for EAGL, stage 4) has a binary but cannot be in one build with Graphite (`browser-platform.md`, section 3).
- **One change in the Skia backend**, in its build script and for iOS only: Skia's Metal code uses `@available`, which needs the clang runtime. For the simulator that is `libclang_rt.iossim.a` (the library of the devices has no slice for it), and for both iOS libraries the directive is `static:-bundle`: the libraries are universal archives, which the compiler unpacks into a Rust library for macOS targets only (`Unsupported archive identifier` otherwise **[M]**); left to the final link, the system linker reads them. Nothing changes for macOS.
- **HarfBuzz** (`harfbuzz-sys` 0.8 with `bundled`) compiles for both targets with the `cc` crate and the Xcode toolchain, with nothing configured **[M]**.
- Fonts: Skia's font manager on Apple systems is Core Text, which iOS has; the text of the example is the first check of it **[S]**.

## 4. The application model

Upstream: the sample's `Main` calls `UIApplication.Main(args, null, typeof(AppDelegate))`; `AppDelegate` derives from the generic `AvaloniaAppDelegate<TApp>` and may override `CreateAppBuilder` and `CustomizeAppBuilder`; the delegate builds the application in `application:didFinishLaunchingWithOptions:` with a `SingleViewLifetime`, names `AvaloniaSceneDelegate` as the delegate of each scene, and the scene delegate creates the window, the view and the view controller when a scene connects.

The port:

- **The Rust executable owns `main` and calls `UIApplicationMain`.** `ferroui_ios::run_application::<App>()` (or `run_application_with(delegate)`) is the whole `main` of an application; it registers the delegate class and calls `UIApplication::main` with its name. An application is a Rust binary, not a static library linked into an Xcode project: nothing of the application is written in another language, and the bundle is made by a script (section 5). A static library for an existing Xcode application remains possible (`use_ios()` without a delegate and `FerroView::new` for embedding), as upstream's view "can be embedded into iOS visual tree".
- **The delegate class is one class, not a generic one.** A class of the Objective-C runtime can be neither generic nor derived from in Rust, and UIKit creates the delegate itself from the name of its class. `FerroAppDelegate` is that class; what an application would override is the trait `FerroApplicationDelegate` (`create_app_builder`, `customize_app_builder`), kept by `run_application_with` where the delegate object finds it. `run_application::<App>()` is the default: `AppBuilder::configure::<App>().use_ios_with_delegate(..)`.
- **Scenes.** The scene delegate (`FerroSceneDelegate`) follows upstream: only an application scene without a named configuration gets a window; the window gets a `FerroView` under a `DefaultFerroViewController`, and the lifetime gets the view. The property list of the bundle states `UIApplicationSceneManifest` without configurations, so that UIKit asks the application delegate for the configuration of a scene (section 5).
- **The lifetime** (`SingleViewLifetime`) is upstream's: the main view of the application is the content of the view; setting a new view disposes the old one. The controls crate has the contracts (`ISingleViewApplicationLifetime`, `ISingleTopLevelApplicationLifetime`), which the browser backend uses the same way.
- **Activations.** The delegate observes the background and foreground notifications of the application and raises them through `IFerroAppDelegate`, which the activatable lifetime of the platform turns into the events of the framework. URLs and user activities (the internal delegate interface) are stage 2, with the storage items they carry.

## 5. Packaging without an Xcode project

`scripts/ios/bundle.sh` **[M]**:

1. `cargo build --target aarch64-apple-ios-sim -p <package> --example <name>` (or `--bin`; the build directory is `CARGO_TARGET_DIR`).
2. A directory `<name>.app` with the executable, `Info.plist` and `PkgInfo` (`APPL????`). An iOS bundle is flat: no `Contents` directory.
3. `Info.plist`, written by the script and converted to the binary form with `plutil`: the bundle identifier (`org.ferroui.<name>`), `CFBundleExecutable`, `CFBundleSupportedPlatforms` (`iPhoneSimulator` or `iPhoneOS`), `MinimumOSVersion` read from the executable (`vtool -show-build`), the SDK and Xcode keys (`DTPlatformName`, `DTSDKName`, ...), `UIDeviceFamily` 1 and 2, the orientations, `UIRequiredDeviceCapabilities`, and two keys that matter to the platform: `UIApplicationSceneManifest` (scenes, above) and `UILaunchScreen` as an empty dictionary, which stands for a launch storyboard (without either the system runs an application letterboxed at the size of an old device **[R]**; the smoke run compares the size of the view with the screen, section 12).
4. `codesign --force --sign -` for the simulator: an ad hoc signature is all the simulator asks for. The result verifies (`valid on disk`, `satisfies its Designated Requirement`) **[M]**.

A bundle for a device is assembled the same way but left unsigned: installing on a device needs a development certificate, a provisioning profile and entitlements, which is the owner's decision and a later stage.

No asset catalog, no storyboard, no entitlements: the example needs none.

## 6. Threads

| Thread | What runs on it |
|---|---|
| Main | UIKit, the dispatcher, the property system, layout, input: everything of the UI thread of the framework. |
| `DisplayLinkTimer` | The display link's run loop. The render timer ticks here and "runs in the background", so the compositor of the platform is in the render-thread mode (`render-thread.md`): the server compositor draws on this thread. |

**The dispatcher** (`DispatcherImpl`, on `IDispatcherImplWithExplicitBackgroundProcessing`) is upstream's: a signal is a function queued on the main dispatch queue plus a wake-up of the main run loop, coalesced by an atomic flag; the timer is one run loop timer created in the distant future and moved (`CFRunLoopTimerSetNextFireDate`); an observer of the run loop (before sources, before waiting, after waiting) raises the signal and, before the loop waits, gives background processing its turn. The declarations (`interop.rs`) are upstream's platform invokes as `extern "C"`. The dispatcher is an object of the main thread kept in a thread-local, where the callbacks find it; what another thread holds is the signal handle (`IDispatcherSignal`: the flag and the two process-lifetime pointers). `tests/dispatcher_main_loop.rs` runs it on the main run loop of a macOS process: ten checks (signals from two threads and their coalescing, the timer at its due time, a cancelled timer, background processing once per request) **[M]**.

**The render timer** (`DisplayLinkTimer`): a `CADisplayLink` on a thread of its own, added to that thread's run loop in the common modes, ticking with the time since the timer was created. Two differences from upstream, both so that an object of UIKit is not shared between threads without a statement that it may be: the link is created on the timer thread (upstream creates it on the registering thread and adds it on the timer thread), and in the background its ticks are dropped by an atomic flag instead of the link being paused from the main thread. Each tick runs in an autorelease pool, which the run loop of a secondary thread does not provide.

## 7. Rendering

Upstream offers Metal and OpenGL ES (EAGL), default order Metal first. The view's layer class follows the graphics that were chosen.

**Metal (built).** Upstream has no software mode on iOS, and none is added: the contract would allow one (a framebuffer blitted to a `CGImage` or a Metal texture), but with Skia's Graphite binaries published for both targets there is nothing it would be needed for, and it would be code upstream does not have.

| Object | Upstream | Port |
|---|---|---|
| Graphics | `MetalPlatformGraphics`: the system default device | The same; `Send + Sync` because `MTLDevice` is. |
| Device | `MetalDevice`: device, command queue, a lock | The same, an object of the thread that creates it (the render thread). It announces itself as `IMetalDevice` through its optional features, which is how the render backends of the port find the Metal contract (as on macOS). |
| Surface | `MetalPlatformSurface`: the layer and the view | The layer (behind a handle that states which four calls the render thread makes on it: `setDevice:`, `setDrawableSize:`, `setFramebufferOnly:`, `nextDrawable`) and a state shared with the view. |
| Render target | `MetalRenderTarget`: `PendingLayout` set by the view; sets the drawable size when the layout changed; `nextDrawable` | The same; the pending layout is read from the shared state under a lock instead of being a property the UI thread sets on an object of the render thread. |
| Session | `MetalDrawingSession`: the drawable's texture; disposing presents (`presentDrawable`, `commit` on a new command buffer) | The same. |

Layer setup, as upstream: `contentsScale` from the main screen, the Metal layer not opaque. Size and scale: `layoutSubviews` reports the size in points to the top-level (`resized`), the scale when it changed (`scaling_changed`), and publishes the size in pixels (`bounds * contentScaleFactor`, truncated) with the scale; the next frame sets `drawableSize` from it.

**Additions for an application that tests itself** (DEVIATIONS.md): `FerroView::frames_presented()` and `FerroView::capture_next_frame(callback)`. A capture makes the layer not "framebuffer only" for the frame, copies the drawable's texture to a shared-storage texture with a blit on the queue the frame was drawn on, waits for it, and hands the bytes to the callback before the frame is presented.

**OpenGL ES over EAGL (stage 4, not built).** `Eagl/EaglDisplay.cs`, `EaglLayerSurface.cs`, `LayerFbo.cs` over the OpenGL contracts of `ferroui-opengl`. It needs Skia with Ganesh on GL (a binary exists, section 3) in place of Graphite, so it is a different build of the Skia backend, not a run-time choice next to Metal as upstream has it; and Apple deprecated OpenGL ES in iOS 12. Until built, the mode counts as one that could not be created: the default list (Metal, OpenGL) gives Metal, and a list with OpenGL alone fails with upstream's message.

## 8. The view and its top-level

`FerroView` (`UIView`, layer class `CAMetalLayer`) creates, as upstream's constructor: the top-level implementation, the input handler, an `EmbeddableControlRoot` over the implementation; prepares it, starts rendering, sets up the layer, enables multi-touch.

`TopLevelImpl` (`ITopLevelImpl`): client size = the bounds of the view; render and desktop scaling = `contentScaleFactor`; the compositor of the platform; no popups (in-window popups); no cursor, no transparency; points map to the screen one to one, as upstream; the frame theme variant sets the status bar style of the view controller. Features in stage 1: the insets manager and the screens. The platform handle of the view (`UIViewControlHandle`) is made when asked for, so that the view (which owns its top-level) is not retained by it.

Ownership: the view owns its state; the top-level implementation and the input handler refer to the view weakly. The Rust state lives in the instance variables of the Objective-C object and is released with it.

## 9. Input

Stage 1: **touches**. `touchesBegan/Moved/Ended/Cancelled:withEvent:` go to the input handler, which for each touch: ignores indirect touches (a remote's trackpad); gives the touch an identifier that lasts until it ends or is cancelled (a counter across views, keyed by the touch object); picks the device (touch, pen for a pencil, mouse for an indirect pointer); maps the phase to the raw event (touch begin, update, end, cancel for a finger; left or right button down and up, move, leave for the others, the right button from the event's button mask); position in the view's coordinates; pressure as half the force (0.5 without force sensing); the modifier keys of the event; the timestamp in milliseconds; and the coalesced touches of the event as lazy intermediate points.

The mapping is in functions over plain values, tested on the host (seven tests). What a test cannot do, on the host or in the simulator, is deliver a touch: an application cannot synthesize one for itself without private interfaces (section 12).

Stage 2: key presses (`pressesBegan:` and the three others, the key table, text from keys), the scroll wheel (a pan gesture recogniser for scroll events with inertia on a second display link), then text input (section 10).

## 10. Text input (stage 2)

Upstream: the view is the `ITextInputMethodImpl`; when a client is set it creates a `TextInputResponder` (a `UIResponder` implementing `UITextInput` and `UIKeyInput`, 683 lines in two files) and makes it first responder, which brings up the keyboard; the responder answers UIKit's questions (text in a range, selection, marked text, caret and selection rectangles, positions and ranges as its own `UITextPosition`/`UITextRange` subclasses) from the `TextInputMethodClient`, and applies insertions, deletions and marked text to it; the keyboard's traits (type, return key, autocorrection, secure entry) come from `TextInputOptions`; `UIKitInputPane` reports the keyboard's frame and animation from the keyboard notifications. In Rust that is three more classes declared with `define_class!` and the `UITextInput` protocol of the bindings. The contracts exist in the base crate and are used by the browser and macOS backends.

## 11. Insets and the safe area

`DefaultFerroViewController.viewDidLayoutSubviews` computes the safe area padding from the frame of the view and the frame of its safe area layout guide and raises a change. `InsetsManager` (on `InsetsManagerBase`): the padding is the controller's while the application draws edge to edge (the default) and zero otherwise; the system bar is the status bar of the controller. When edge to edge is turned off, the top-level gets the safe area as its padding at style priority, because iOS adds no margins by itself (upstream's comment and behaviour).

## 12. Verification

1. **Compilation** for `aarch64-apple-ios-sim` and `aarch64-apple-ios`: the crate and the example build and link **[M]**.
2. **Tests on the host** (`cargo test -p ferroui-ios`): 36 unit tests of the logic (section 1) and the dispatcher on a real run loop (ten checks) **[M]**. Upstream has no tests of this project to port.
3. **The smoke mode in the simulator**: `scripts/ios/sim-smoke.sh "<device>"` builds, bundles, boots the device if needed, installs, launches `ios_view --smoke` with the console attached, waits (180 seconds at most), takes a screenshot of the simulator, prints the checks and a result, uninstalls and shuts the device down (`--keep` leaves it). The application writes its lines to the standard output and to `Documents/ios_view_smoke.txt` in its data container, which the script copies out.

What the smoke mode proves, each a line `[ ok ]` or `[FAIL]`:

| Check | What it compares |
|---|---|
| platform | Metal graphics were created, the render timer exists. |
| view | The scene connected, the lifetime has a view, the view is in a window. Proves `UIApplicationMain`, the delegate class, the scene configuration, the scene delegate and the bundle's property list. |
| size | The client size of the top-level = the bounds of the view = the bounds of the window. |
| scale | The render scaling of the top-level = the view's content scale = the scale of the window's screen. |
| screen | The screens of the framework: a primary screen with the native scale and the native pixel size of the `UIScreen`. |
| safe area | The padding of the insets manager = the layout guide of the view = `safeAreaInsets` of the view. |
| frames | At least three frames were presented on the Metal layer (the display link thread ticks, the compositor renders on it, drawables are presented). |
| frame size | A frame read back from the Metal texture has the size of the view in pixels. |
| pixels | In that frame: the fill, the square in the middle and the circle above the bottom edge have their colours where layout puts them. Proves Skia's Graphite on Metal drew, at the right scale and the right way up. |
| text | The band of the text line has pixels that are not the fill: a system font was found, shaped by HarfBuzz and drawn. |

What it cannot prove: touch input (no synthetic touches from inside an application; `simctl` has no touch command); rotation and a change of the safe area; the background and foreground transitions; the launch screen. The screenshot the script takes shows what the simulator composited, for a person to look at; it is not compared.

Runs in the simulator (2026-10-10, "iPhone 17 Pro", iOS 26.4, a debug build; the orchestrator of the port runs the script, because the simulator may only run while no virtual machine does) **[S]**:

| Run | Result |
|---|---|
| 1 | The application started at the first attempt, without a crash: platform, view (402x874 points), size, scale 3, one screen of 1206x2622 pixels at 3, safe area (top 62, bottom 34, equal to the layout guide and to the insets of the view) passed. One frame was presented and no more, and no frame was read back: the example asked for a redraw by invalidating a panel that draws nothing, so nothing changed after the first frame. A fault of the example. |
| 2 | **SMOKE PASSED** at the fourth attempt (two seconds after the start of the checks): all ten checks. The example now changes the colour of a marker at every attempt. The frame read back is 1206x2622, `BGRA8Unorm`; the fill, the square and the circle have exactly the colours that were drawn ((51, 102, 153), (204, 51, 51), (51, 170, 85)); 11249 pixels of the text band are not the fill. |

Seen in the trace of run 2, and open: the first frame took about a second and a half (begun at the first attempt, presented by the fourth; the display link, whose handler was drawing, did not tick meanwhile: 19 ticks before, 38 after). A first frame compiles the pipelines of Graphite, in a debug build, in the simulator; it was not measured further, and a release build on a device is where it should be. After the frames the render loop detaches from the timer (`render loop attached: false`), as it does on every platform when nothing changes.

The screenshots of both runs failed: the simulator service may not write into the build directory on the external volume. The script now writes them to `~/Library/Caches/ferroui-ios-smoke/`; that has not been run yet.

4. **CI** (the job `ios` of `.github/workflows/ci.yml`, macOS runner): adds the two Rust targets, checks that the Skia features of the target are the Graphite and Metal set, builds the crate and the example for the simulator, checks the crate for the device target, runs the host tests, and bundles the example (`bundle.sh`, with the ad hoc signature verified). It does not run the simulator: a boot of a simulator on a hosted runner takes minutes and fails intermittently **[R]**, and the runner image decides which runtimes exist. The script is written so that a later step can call it (`sim-smoke.sh "<device>"`, exit code 0 on success) once a run has been shown to be reliable; expected cost two to five minutes on top of the build **[R]**.

## 13. File table

One row per upstream file. "built" files are on the branch; "part" means the file exists with the part named; the stage of an open file is the one that builds it.

| Upstream file (`src/iOS/Avalonia.iOS`) | Lines | Rust file (`src/iOS/FerroUI.iOS`) | Stage | State | Notes |
|---|---:|---|---|---|---|
| `ActivatableLifetime.cs` | 13 | `activatable_lifetime.rs` | 1 | built | |
| `AutomationPeerWrapper.cs` | 486 | `automation_peer_wrapper.rs` | 3 | open | accessibility elements over automation peers |
| `AvaloniaAppDelegate.cs` | 142 | `ferro_app_delegate.rs` | 1 | part | the delegate, the builder, scenes, background and foreground; URLs and user activities (`IAvaloniaAppInternalDelegate`) are stage 2 |
| `AvaloniaSceneDelegate.cs` | 88 | `ferro_scene_delegate.rs` | 1 | part | the window of a scene; the activations a scene carries are stage 2 |
| `AvaloniaView.cs` | 444 | `ferro_view.rs` | 1 | part | the view, layer, layout, touches, the top-level with the insets manager and the screens; presses, trait changes and the other features are stage 2; the tvOS gestures are not ported |
| `AvaloniaView.Text.cs` | 52 | `ferro_view.rs` | 2 | open | the text input method of the view |
| `AvaloniaView.Automation.cs` | 24 | `ferro_view.rs` | 3 | open | the accessibility container |
| `CombinedSpan3.cs` | 40 | `combined_span3.rs` | 2 | open | a helper of the text input responder |
| `DispatcherImpl.cs` | 133 | `dispatcher_impl.rs` | 1 | built | |
| `DisplayLinkTimer.cs` | 45 | `display_link_timer.rs` | 1 | built | |
| `Extensions.cs` | 23 | `extensions.rs` | 1 | part | sizes, points; the colour conversion is stage 2 |
| `IOSLauncher.cs` | 45 | `ios_launcher.rs` | 2 | open | |
| `IOSPlatformFeedback.cs` | 56 | `ios_platform_feedback.rs` | 2 | open | sound and haptics |
| `InputHandler.cs` | 570 | `input_handler.rs` | 1 | part | touches; keys and the scroll wheel are stage 2; the swipe gestures of a remote are not ported |
| `InsetsManager.cs` | 55 | `insets_manager.rs` | 1 | built | |
| `Interop.cs` | 58 | `interop.rs` | 1 | built | `extern` declarations |
| `NativeControlHostImpl.cs` | 154 | `native_control_host_impl.rs` | 1 | part | `UIViewControlHandle`; the host is stage 2 |
| `Platform.cs` | 140 | `platform.rs` | 1 | built | the default platform settings are bound until stage 2 |
| `PlatformSettings.cs` | 95 | `platform_settings.rs` | 2 | open | colour scheme, contrast, tint, language |
| `SingleViewLifetime.cs` | 40 | `single_view_lifetime.rs` | 1 | built | |
| `Stubs.cs` | 73 | `stubs.rs` | 1 | built | |
| `TextInputResponder.cs` | 587 | `text_input_responder.rs` | 2 | open | `UITextInput` |
| `TextInputResponder.Properties.cs` | 96 | `text_input_responder.rs` | 2 | open | the keyboard traits |
| `UIKitInputPane.cs` | 58 | `ui_kit_input_pane.rs` | 2 | open | |
| `ViewController.cs` | 82 | `view_controller.rs` | 1 | built | |
| `iOSScreens.cs` | 75 | `ios_screens.rs` | 1 | built | |
| `Clipboard/ClipboardDataFormatHelper.cs` | 61 | `clipboard/clipboard_data_format_helper.rs` | 2 | open | |
| `Clipboard/ClipboardImpl.cs` | 144 | `clipboard/clipboard_impl.rs` | 2 | open | `UIPasteboard` |
| `Clipboard/PasteboardItemToDataTransferItemWrapper.cs` | 101 | `clipboard/pasteboard_item_to_data_transfer_item_wrapper.rs` | 2 | open | |
| `Clipboard/PasteboardToDataTransferWrapper.cs` | 40 | `clipboard/pasteboard_to_data_transfer_wrapper.rs` | 2 | open | |
| `Eagl/EaglDisplay.cs` | 145 | `eagl/eagl_display.rs` | 4 | open | section 7 |
| `Eagl/EaglLayerSurface.cs` | 104 | `eagl/eagl_layer_surface.rs` | 4 | open | |
| `Eagl/LayerFbo.cs` | 156 | `eagl/layer_fbo.rs` | 4 | open | |
| `Metal/MetalDevice.cs` | 34 | `metal/metal_device.rs` | 1 | built | |
| `Metal/MetalDrawingSession.cs` | 34 | `metal/metal_drawing_session.rs` | 1 | built | with the frame capture |
| `Metal/MetalPlatformGraphics.cs` | 37 | `metal/metal_platform_graphics.rs` | 1 | built | |
| `Metal/MetalPlatformSurface.cs` | 25 | `metal/metal_platform_surface.rs` | 1 | built | with the state shared with the view |
| `Metal/MetalRenderTarget.cs` | 37 | `metal/metal_render_target.rs` | 1 | built | |
| `Properties/AssemblyInfo.cs` | 4 | - | - | n/a | assembly attributes |
| `Storage/IOSStorageItem.cs` | 348 | `storage/ios_storage_item.rs` | 2 | open | security-scoped URLs, bookmarks |
| `Storage/IOSStorageProvider.cs` | 436 | `storage/ios_storage_provider.rs` | 2 | open | the document pickers |

Stage 1 has 23 of the 40 files, 17 complete and 6 in part. `samples/ControlCatalog.iOS` (`Main.cs`, `AppDelegate.cs`, `EmbedSample.iOS.cs`, `Info.plist`, the launch screen) becomes the crate `control-catalog-ios` in stage 2: a `main` that calls `run_application_with`, the embed sample over the native control host, and a bundle made by `bundle.sh`.

## 14. Stages

| Stage | Content | What a simulator run proves |
|---|---|---|
| 1 (built) | The crate, the platform and its options, the application delegate, scenes, the single-view lifetime, the view and its top-level, the dispatcher, the display link timer, Metal with the Skia GPU, touches, the safe area and the insets manager, the screens, the example with its smoke mode, the bundle and simulator scripts, the CI job | Section 12 |
| 2a | Keys (`presses*`, the key table), the scroll wheel, platform settings (colour scheme, accent, language) with trait changes, the launcher, feedback | A hardware keyboard event through `simctl` is not available either; the settings can be checked against the simulator's appearance setting (`simctl ui appearance dark`), which the script can switch |
| 2b | Text input: `TextInputResponder`, the keyboard traits, the input pane | The keyboard appears for a focused text box (the input pane reports its frame); text entry itself needs a person or UI automation |
| 2c | Clipboard (`UIPasteboard`), the storage provider with the document pickers and storage items, activations by URL and user activity (`simctl openurl`) | The clipboard round trip (`simctl pbcopy`/`pbpaste` against the application); a URL activation; the pickers need a person |
| 2d | The native control host and the catalog's iOS host (`control-catalog-ios`) | The catalog starts and selects its pages in the simulator, as the desktop host does in its smoke mode |
| 3 | Accessibility (`AutomationPeerWrapper`, the container methods of the view) | The accessibility tree as the simulator's inspector shows it; by hand |
| 4 | OpenGL ES over EAGL, as a build of its own (section 7); a device build with signing | The smoke mode in the OpenGL mode |
