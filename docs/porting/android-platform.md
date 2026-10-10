# The Android platform

The design of the Android backend of FerroUI, the decisions it rests on, the file table of the port and its stages. Upstream: `src/Android/Avalonia.Android` (58 files, 88 types, 447 members, 7068 lines) at the tracked commit (`TRACKING.md`), and the host of the sample, `samples/ControlCatalog.Android`.

Marks, as in the other platform documents: **[V]** verified from sources (the upstream files, the sources of a crate in the cargo registry, `cargo info`, the files of the SDK), **[M]** measured here (a build or a test run on the development machine, a Mac), **[E]** shown by a run on the emulator (the scripts of section 10, run on 2026-10-10: section 10.1), **[R]** recalled and not verified here.

## 1. What is ported, and the crate

| Crate | Directory | Upstream | Enabled by |
|---|---|---|---|
| `ferroui-android` | `src/Android/FerroUI.Android` | `Avalonia.Android` | `AppBuilder::use_android()` (`AndroidApplicationExtensions`) |

The crate depends on `ferroui-base`, `ferroui-controls`, `ferroui-opengl` (the EGL of the framework), `ferroui-skia` and `ferroui-harfbuzz` (upstream's project references Skia and HarfBuzz and `UseAndroid` selects both), `bitflags`, and on Android on `jni-sys` and `libc` (section 2). It carries a Java source tree (`java/`, section 3) that the packaging script compiles (section 9).

Upstream builds on three AndroidX libraries (AppCompat, Window, DocumentFile; the sample also on Core SplashScreen). The port uses the classes of the platform only (section 3.3).

## 2. How Rust meets the Android framework

Upstream is managed code that derives from classes of the framework: an `Application`, an `Activity`, a `FrameLayout` (the view), a `SurfaceView`, a `BaseInputConnection`, listeners and callbacks. A Rust library cannot derive from a Java class. Two ways exist.

| | A native activity (`android-activity` 0.6.1, MIT or Apache-2.0 **[V]**) | A Java layer in the repository and JNI |
|---|---|---|
| Java in the application | none with `NativeActivity`; `GameActivity` is a Java class of an AndroidX library (Gradle and Maven to get it) | six classes of the port, compiled by `javac` and `d8` of the SDK |
| Structure | one activity that owns the window; the library gives an event loop of its own on a second thread, with the Java main thread elsewhere | upstream's: an application class, activities, a view in the view hierarchy, a surface view under it |
| A view | none: no `View` of the application exists, so the framework cannot be embedded in a layout, and a second activity or a native control host has nothing to attach to | `FerroView` is a `FrameLayout`: it embeds (`AvaloniaView` is public API upstream) and hosts native views |
| Input method | `NativeActivity` has no `InputConnection` (soft keyboard text arrives as key events, no composition, no surrounding text); `GameActivity` has `GameTextInput`, a fixed editor model of its own | `onCreateInputConnection` of the view returns the port of upstream's `AvaloniaInputConnection` (stage 2) |
| Threads | the application runs on a thread that is not the main thread: the dispatcher would not be on the main looper, and every call into the view system would have to be marshalled | the UI thread of the framework is the main thread, as upstream: the dispatcher is a `Handler` of the main looper |
| Insets, storage access framework, clipboard, back button | each needs Java calls anyway (through JNI by hand, without a Java class to receive a callback) | callbacks arrive in Java classes of the port and are forwarded |
| Dependencies | `android-activity`, `ndk`, `ndk-sys`, `ndk-context`, `jni`, `libc`, and more | `jni-sys` |
| Build without Gradle | yes | yes: `javac`, `d8`, `aapt2`, `zipalign`, `apksigner` (section 9) |

**Decision: the Java layer.** The port is exact, and upstream's structure is view-based: the input connection needs a `View`, the native control host needs a `ViewGroup`, the insets, the visibility and the configuration arrive as overrides of a view, and the dispatcher is the main looper. A native activity would give a backend with another shape and without an input method. The Java layer is small because it only receives and forwards: each class mirrors one upstream class, holds no state beyond the number of its Rust object, and calls a native method for every override upstream has; the logic stays in Rust, in the file of the upstream class.

### 2.1 JNI: `jni-sys`, and one module of safe functions

| Crate | Version **[V]** | Licence **[V]** | What it is |
|---|---|---|---|
| `jni` | 0.22.4 | MIT or Apache-2.0 | a safe wrapper: environments and references with lifetimes, exceptions as errors; since 0.22 with a procedural macro crate for names and signatures; depends on `jni-sys`, `jni-macros`, `combine`, `simd_cesu8`, `thiserror`, `log`, `cfg-if` and, to build, `walkdir` |
| `jni-sys` | 0.4.1 | MIT or Apache-2.0 | the declarations of `jni.h`; depends on `jni-sys-macros` |

The backend uses **`jni-sys` 0.4.1** (pinned), for the reasons the Windows backend uses `windows-sys` and the Linux backend `x11-dl` (`win32-platform.md` 2.1, `x11-platform.md` 2):

- What the backend needs of JNI is some thirty functions (classes, method lookups, calls with an argument array, global references, strings, arrays, exceptions, native method registration, thread attachment), all called from one module.
- `interop/java.rs` is that module, the counterpart of `xlib.rs` and `unmanaged_methods.rs`: every JNI call is a safe function there, with its safety argument. A Java object is a value that owns a global reference (`JavaObject`, released when dropped) or a local reference valid for the current native call (`JavaLocal`); a method is called by name and signature, and the arguments are checked against the signature before the call, because a call with arguments of other types than the signature states is undefined behaviour in JNI; an exception after a call is described to the log, cleared and returned as an error. Outside the module there is no `unsafe` for Java.
- One dependency instead of nine, and no second model of references beside the one the module needs.

### 2.2 The NDK: declarations in the file upstream has them in

Upstream declares the functions of `libandroid` it calls itself (`AndroidFramebuffer.cs`: `ANativeWindow_fromSurface`, `ANativeWindow_lock`, `ANativeWindow_unlockAndPost`, `AChoreographer_getInstance`, `AChoreographer_postFrameCallback64` and five more). The port does the same in `interop/ndk.rs` (twelve functions of `libandroid` and `liblog`, each a safe function with its argument), so the crates `ndk` 0.9.0 and `ndk-sys` 0.6.0 (both MIT or Apache-2.0 **[V]**) are not needed. `libc` (already in the workspace) gives `dlopen` and `dlsym` for the EGL library and for the one function of the NDK that is newer than the API level the library is linked against (section 5).

### 2.3 How the library is loaded and finds its application

- The native code of an application is one shared library (`crate-type = ["cdylib"]`). The manifest of the application names it in a meta-data element (`org.ferroui.android.library`), and `FerroApplication.onCreate` loads it with `System.loadLibrary`.
- The library exports one symbol, `JNI_OnLoad`, which the application writes with a macro: `ferroui_android::android_application!(build)`, where `build` is a function that returns the application builder (upstream's `CreateAppBuilder` and `CustomizeAppBuilder` of `AvaloniaAndroidApplication<TApp>`). `JNI_OnLoad` keeps the virtual machine, resolves the classes of the Java layer and registers their native methods with `RegisterNatives`. Nothing else has to be exported, so nothing depends on how the linker treats the symbols of a dependency.
- `FerroApplication.onCreate` then calls its native method, which is upstream's `InitializeAppLifetime`: the builder, an `ApplicationLifetime`, `setup_with_lifetime`.
- A panic must not cross into Java frames. Every native method runs under a guard that catches it, writes it to the log under the tag `ferroui` and throws a Java `RuntimeException` with the message, so the application stops as an application with an uncaught managed exception does upstream.

## 3. The Java layer

Package `org.ferroui.android`, sources in `src/Android/FerroUI.Android/java/org/ferroui/android/`.

| Java class | Derives from | Upstream class | What it forwards |
|---|---|---|---|
| `FerroApplication` | `android.app.Application` | `AvaloniaAndroidApplication<TApp>` | `onCreate` (after loading the library) |
| `FerroActivity` | `android.app.Activity` | `AvaloniaActivity`, and the `BackPressedCallback` it registers | `onCreate`, `onStart`, `onStop`, `onResume`, `onDestroy`, `onNewIntent` (and the intent the activity was created with), `onActivityResult`, `onRequestPermissionsResult`, `onBackPressed`, and the back callback of the window (API 33) |
| `FerroMainActivity` | `FerroActivity` | `AvaloniaMainActivity` | states that it is the main activity |
| `FerroView` | `android.widget.FrameLayout` | `AvaloniaView` (both parts) | `onAttachedToWindow`, `onVisibilityAggregated`, `onVisibilityChanged`, `onConfigurationChanged`, the focus change of the view, the global layout, `dispatchTouchEvent`, `dispatchGenericPointerEvent`, `dispatchHoverEvent`, `dispatchKeyEvent`, `onCreateInputConnection` |
| `FerroSurfaceView` | `android.view.SurfaceView`, `SurfaceHolder.Callback2` | `InvalidationAwareSurfaceView` and `TopLevelImpl.SurfaceViewImpl` | the five callbacks of the surface holder; `dispatchDraw` clears the canvas (upstream's workaround, which is drawing code of the view and stays in Java) |
| `MainLooperBridge` | `MessageQueue.IdleHandler` | the `Handler`, the three `Runnable`s and the `IdleHandler` of `AndroidDispatcherImpl` | the signal, the timer and the idle callback of the main looper |
| `FerroInputConnection` | `android.view.inputmethod.InputConnection` | `AvaloniaInputConnection` | every call of the input method of the system that upstream answers with more than a constant (section 7.1) |
| `ConfigurationChangedReceiver` | `android.content.BroadcastReceiver` | `AndroidPlatformSettings.ConfigurationChangedReceiver` | `onReceive` of `ACTION_CONFIGURATION_CHANGED` |
| `NativeClickListener` | `View.OnClickListener` | (the event binding of the managed runtime) | `onClick`, for a view an application written in Rust creates (`interop::listeners`) |
| `StorageHelper` | (static) | the calls of `AndroidStorageItem` and `AndroidStorageProvider` that upstream wraps in `try`/`catch`, and the queries `DocumentFile` makes | the columns of a document, the children of a tree, create, delete and move of a document, the persistable permissions, the descriptors of the streams; an exception is caught in Java and its text kept for the native side |
| `PlatformHelper` | (static) | calls upstream makes inline on framework objects that return more than one value, or that set fields | the insets of a window, the displays, the system features, the insets listener and the insets animation callback of the decor view, the colour and input values of the platform settings, the fields of an `EditorInfo`, an `ExtractedText`, the layout parameters of a native control |

### 3.1 Objects on both sides

A Rust object that has a Java object owns a global reference to it. A Java object that calls back holds a `long`, the number under which its Rust object is registered on the UI thread (a map of the thread, to weak references for views and top-levels and to strong references for activities, which live from `onCreate` to `onDestroy`). A native method looks its object up by the number and does nothing when it is gone. No address of a Rust object is ever given to Java.

Construction runs in upstream's order, driven from Rust: `FerroActivity::on_create` creates the Rust `FerroView`, whose constructor creates the Java view, the top-level (which creates the Java surface view), adds the surface view to the view, creates and prepares the `EmbeddableControlRoot`, sets the transparent background and sends the configuration.

### 3.2 Events are copied in Java and translated in Rust

A `MotionEvent` is a Java object with some forty getters, valid only during the call. `FerroView` copies what upstream's helper reads into primitive arguments and two arrays (per pointer: id, tool type, x, y, pressure, orientation; then the history of each pointer) and passes them in one native call; `AndroidMotionEventsHelper` in Rust translates the copy, which makes the translation a function of values that the host tests run. Upstream reads the history lazily from the event; the copy reads it eagerly, because the event is recycled after the call.

### 3.3 Without AndroidX

| Upstream uses | The port uses | Difference |
|---|---|---|
| `AppCompatActivity` | `android.app.Activity` | no AppCompat theme and no local night mode: `SetFrameThemeVariant` sets the system bar theme only. An activity of the platform has no night mode of its own (the platform has an application-wide one from API 31, `UiModeManager.setApplicationNightMode`, which upstream's comment rules out for the reason it gives against `DefaultNightMode`: the application would stop following the system). Open. |
| `OnBackPressedDispatcher`, `OnBackPressedCallback` | `OnBackInvokedDispatcher` with an `OnBackInvokedCallback` of default priority (API 33), `onBackPressed` below | upstream's callback disables itself and asks the dispatcher again to reach the default action; the port calls the default action of the activity (`super.onBackPressed()`), which on API 36 finishes a main activity **[E]** |
| `ViewCompat`, `WindowCompat`, `WindowInsetsCompat`, `WindowInsetsControllerCompat`, `WindowInsetsAnimationCompat` | `WindowInsets`, `WindowInsetsController`, `WindowInsetsAnimation` of the platform (API 30); below API 30 the deprecated calls the compatibility classes make themselves | below API 30 the input method counts as visible when the system window inset at the bottom exceeds the stable one, and its state follows the global layout of the decor view (written, never run: the emulator is API 36) |
| `WindowMetricsCalculator` (AndroidX Window) | `WindowManager.getMaximumWindowMetrics` (API 30), which is what the calculator calls there | none on API 30 and later |
| `ExploreByTouchHelper` | `AccessibilityNodeProvider` | stage 3 |
| `DocumentFile` | `DocumentsContract` | stage 2d |
| `ContextCompat.RegisterReceiver(..., ReceiverNotExported)` | `Context.registerReceiver` with `RECEIVER_NOT_EXPORTED` (API 33), without a flag below | none: the broadcast of a configuration change is one only the system sends |

The API level: the native code is built for API 26 (the level the published Skia binaries are built for, section 5), so `minSdkVersion` is 26; upstream's is 21. The target is API 36.

## 4. Threads

| Thread | What runs on it | Upstream |
|---|---|---|
| The main thread of the application (the UI thread) | everything of the framework that is not rendering; every call into the view system; every native method of the Java layer | the same |
| `Choreographer Thread` | a looper of its own with the `AChoreographer` of the NDK; the frame callback notes the frame time and wakes the render thread | the same (`ChoreographerTimer.Loop`) |
| `Render Thread` | waits for the frame event and calls the tick of the render loop: the compositor renders here (`render-thread.md`) | the same (`ChoreographerTimer.RenderLoop`) |

**The dispatcher** (`AndroidDispatcherImpl`: explicit background processing and pending input, as upstream) is a `Handler` of the main looper, held by `MainLooperBridge`. A signal posts the signal runnable unless one is already posted (an atomic flag); the timer posts the timer runnable with a delay; the idle handler of the message queue raises the background processing and, when more was requested, looks at the queue (`MessageQueue.isIdle`) as upstream does to decide whether to run again. The part other threads use is the signal: `IDispatcherSignal` holds the bridge object and posts through JNI, attaching its thread to the virtual machine the first time (a thread that was attached this way detaches when it ends). There is no loop to run: the looper of the system is the loop, and `setup_with_lifetime` returns to it.

**The render timer** (`ChoreographerTimer`) is upstream's: two threads, a lock around the pending flag and the last frame time, an auto-reset event between them, `runs_in_background` true. The frame callback is the 64-bit one where the system has it (API 29), looked up at run time because the library is linked against API 26.

**What crosses to the render thread** is `Send + Sync` by construction: the surfaces of a top-level hold what the surface view publishes of itself (`SurfaceShared`: the native window under a lock, the size and the scaling as atomics), never the view. The native window is counted: the software framebuffer and the EGL surface each hold a reference of their own while they use it, so the surface can be destroyed on the UI thread while a frame is in flight (upstream releases the window in `SurfaceDestroyed` without that; a deviation toward safety, recorded in DEVIATIONS.md).

## 5. Skia and HarfBuzz for Android

- **A Skia binary is published for the target with Ganesh on OpenGL [M].** `cargo build -p ferroui-skia --target aarch64-linux-android` with the feature `gl` of the bindings (a target table of the manifest of the Skia backend, beside the Apple, browser and Windows tables) fetched `skia-binaries-b7f043e0b1e2a850e702-aarch64-linux-android-ganesh-gl-jpegd-jpege-pdf.tar.gz` from the release `0.153.3` of `rust-skia/skia-binaries` and linked it; nothing was built from source. The key is the commit of the bindings, the target and the sorted features (`build_support/binary_cache/binaries.rs` **[V]**). For `x86_64-linux-android` the same key with that target is asked for; whether it is published is shown by the first build for that target (the CI job of section 12 is ARM64 only until then) **[R]**.
- What the binary is **[V]** (`build_support/platform/android.rs`): built for API 26 ("the first one with full Vulkan support"), with FreeType (embedded) and the font manager of Android (`skia_enable_fontmgr_android`), which reads the font configuration of the system (`/system/etc/fonts.xml`) and the fonts under `/system/fonts`; it links `log`, `android`, `c++_static`, `c++abi`, and with `gl` also `EGL` and `GLESv2`. The C++ runtime is static: no `libc++_shared.so` is packaged.
- **A build from source must never start.** The build script of the bindings falls back to it when the download fails, and for Android it then reads the variable `ANDROID_NDK`. The scripts of this backend do not set that variable (the linker and the compilers are named through cargo's own variables), so the fall-back stops at once with "ANDROID_NDK variable not set" instead of compiling for an hour.
- **The Skia backend** gains one line of configuration: `ferro_skia_ganesh_gl` is set for `target_os = "android"` (`build.rs`), so its Ganesh GPU (`gpu/open_gl`, built for the browser and for Windows until now) is built for Android. Nothing else of the backend changed.
- **HarfBuzz builds for the target [M]**: `harfbuzz-sys` 0.8.0 compiles its bundled sources with the `cc` crate and the clang of the NDK (`CC_aarch64_linux_android`, `CXX_...`, `AR_...`).
- The base, controls and OpenGL crates compile for the target unchanged **[M]**.

### 5.1 Thread-local storage: an Android application cannot be built with stable Rust as things are

Found by the first run on the emulator **[E]** (2026-10-10): the application aborted while the application object registered its services, in `std::sys::thread_local::key::racy::LazyKey::lazy_init`, reached from `std::sys::thread_local::os::Storage::get` for the cell of a routed event of the input element. No code of the Android backend had run yet beyond loading the library.

**The cause.**

- The Android targets of Rust are not marked as having thread-local statics. The target specification of `aarch64-linux-android` says `"tls-model": "emulated"` and has no `has-thread-local`, in the stable toolchain of the machine (1.90.0), in 1.99.0 and in the nightly of 2026-06-30 alike; the nightly prints no `target_thread_local` among the configuration of the target **[V]** (`rustc --print target-spec-json`, `rustc --print cfg`). The specification does not depend on the API level: nothing a build sets (the API level of the linker, `minSdkVersion`) changes it.
- The standard library as it is distributed for these targets therefore keeps every `thread_local!` variable under a key of the C library of its own (`pthread_key_create`), created when the variable is first used on any thread.
- Bionic has 128 such keys for a process (`PTHREAD_KEYS_MAX`), and the virtual machine and the system libraries hold some of them **[R]**.
- The port needs far more **[V]** (counted in the sources, tests left out): the class model keeps the definitions of a thread per thread (`PORTING-GUIDE.md`, "Threading model"), and four macros of `ferroui-base` expand to a thread-local cell per use: `ferro_property!` (871 uses), `ferro_routed_event!` (110), and the class registration of `ferro_class!` (460) and `ferro_static_type!` (26), in `ferro_property.rs`, `interactivity/routed_event.rs` and `type_system.rs`. 225 more `thread_local!` are written out: 93 in the base crate, 45 in the controls, 10 in this backend, 9 in the catalog, the rest in the other crates. An application passes 128 before its first control exists. Every other platform of the port has native thread-local storage, where a variable costs nothing of the kind.

**What is done now (alternative a).** The native library is built with a nightly toolchain and `-Zbuild-std=std,panic_unwind`, with `-Zhas-thread-local=yes` for the target (`CARGO_TARGET_<TARGET>_RUSTFLAGS`, so the build scripts of the host are not affected). The standard library then keeps its thread-local variables in thread-local statics, which the compiler lowers to the emulated TLS of the compiler runtime (`__emutls_get_address`: one key for all variables of a thread). The runtime library of the NDK that has that function (`libclang_rt.builtins`) is named first on the link line (`-Zpre-link-args`), because the build script of a dependency names the shared C++ runtime, which exports the function too and would otherwise become a library the package has to carry. Verified on the built library **[M]**: 787 emulated TLS variables, none of the key-based storage of the standard library, `__emutls_get_address` a local symbol, and the same seven system libraries as the stable build. This is confined to `scripts/android/apk.sh` (the toolchain is pinned in `scripts/android/env.sh`: `nightly-2026-07-01` with the component `rust-src`) and to the CI job.

**What it means for a user of the crate.** `cargo check` and `cargo build` of an application for an Android target succeed with stable Rust, and the result aborts at start. Until one of the alternatives below is in place, an application for Android is built with a nightly toolchain and the flags above (`scripts/android/apk.sh` is the reference).

**Alternatives, for the owner** (none but (a) was done):

| | What | Cost | Effect elsewhere |
|---|---|---|---|
| (a) | The nightly build with `-Zbuild-std` and `-Zhas-thread-local=yes`, as now. | Nothing in the sources. A nightly toolchain with `rust-src` for every Android build (the developer, CI, every user of the crate), pinned and moved by hand; the standard library is compiled with the application (about a minute); two unstable flags whose names may change. | None: other targets are built as before. |
| (b) | The class model takes no thread-local variable per item: one table of the thread (one `thread_local!`), indexed by a number each property, routed event and class takes once (a static atomic). The four macro sites of `ferroui-base` change (`ferro_property.rs` lines 755 and 784, `routed_event.rs` line 362, `type_system.rs` lines 1037 and 1437), and the 225 written-out `thread_local!` need the same treatment wherever an Android application reaches them (a `ferro_thread_local!` of the base crate that is `thread_local!` on targets with native storage and a slot of the table on Android): the base crate and the controls for certain (138 sites), the markup crates, Skia, HarfBuzz and the sample. The crates of the dependencies keep their own (the standard library itself, and whatever a dependency declares): they have to stay under the limit together, which has to be counted. | A change of the base crate that every crate follows: mechanical, wide (some twenty crates touched), and a rule for all later code ("no `thread_local!` outside the macro") that a check of the conventions job would have to enforce. A table lookup instead of a thread-local read on the hot path of a property accessor on Android. | With the macro defined as `thread_local!` on other targets, none in behaviour or cost; the diff touches the files other platform work edits. |
| (c) | Native ELF TLS: bionic has it since API 29, and the NDK links it when the minimum API level is 29. | Not available from stable Rust: the target specification is `emulated` without thread-local statics at every toolchain checked, whatever the API level, and the switches (`-Zhas-thread-local`, `-Ztls-model`, a custom target specification) are nightly and need the standard library rebuilt, so this is (a) with a higher minimum API (29 instead of 26) and no gain over emulated TLS that matters here. It becomes the answer by itself only if a future stable Rust marks the Android targets as having thread-local statics; then the stable build works with no change in this repository. | None. |

The recommendation of this document: (a) until the owner decides; (b) if Android is to be buildable with stable Rust, done as its own piece of work by whoever owns the base crate.

## 6. Rendering

`AndroidPlatformOptions::rendering_mode` is upstream's list with upstream's default: EGL, then software. The platform takes the first mode that initialises; a list in which nothing does is an error with upstream's message.

| Mode | How it draws | Stage |
|---|---|---|
| `Software` | `FramebufferManager`: the renderer draws into the buffer `ANativeWindow_lock` returns (RGBA 8888 premultiplied, or RGB 565 opaque, as the window says) and `ANativeWindow_unlockAndPost` shows it | 1 |
| `Egl` | OpenGL ES through the EGL of the system on the native window; Skia's Ganesh on that context | 1 |
| `Vulkan` | `VulkanSupport` | after the Vulkan project of the port (out of scope today); passed over like a mode that fails to initialise |

**EGL.** Upstream calls `EglPlatformGraphics.TryCreate()`, whose display loads `libEGL.so` by name. The EGL of the port (`ferroui-opengl`) takes a loader function instead of a library name, and its platform graphics are bound to the thread that created them, because a display and a context are objects of one thread there. On Android the compositor renders on the render thread, so the backend has platform graphics of its own (`android_egl.rs`), written like the ones of ANGLE on Windows (`win32-platform.md` 6.1, "Threads"): the graphics hold nothing of a thread; a context loads the interface (`dlopen("libEGL.so")`, entry points by `dlsym` and then `eglGetProcAddress`) and creates its display object on the thread that asks for it. The display of the system is one per process and counted (`eglInitialize`/`eglTerminate`), so a display object per context is the same display. The platform probes once on the UI thread (a display, a context, made current, disposed), as `TryCreate` does by creating the display.

**The surface of a top-level** is, as upstream, three surfaces: an `EglGlPlatformSurface` over what the surface view publishes (handle, size, scaling; waits skipped), the framebuffer manager, and the handle of the native window. The renderer takes the first it can use.

**Emulator.** The scripts start the emulator with `-gpu swiftshader_indirect` by default: a software implementation of OpenGL ES inside the emulator, which needs neither a window nor the GPU of the host, so a headless run draws the same on every machine; `--gpu host` selects the host GPU. On Apple Silicon the default works without a window **[E]**: the emulator reports `gles_mode_selected:swangle` (ANGLE on the Vulkan of SwiftShader), and the EGL mode of the platform draws through it. `--gpu host` was not tried.

## 7. The top-level, the view and input

- **`TopLevelImpl`** (`platform/skia_platform/top_level_impl.rs`): client size and scaling from the surface view (the frame of the surface holder and the density of the display metrics), the callbacks, the surfaces, the features (stage 1: the insets manager and the input pane, the screens; the others arrive with their stage), transparency levels (stage 2 beyond `None`), `SurfaceRedrawNeeded` as a synchronous paint and `SurfaceRedrawNeededAsync` through `Compositor::request_composition_update`.
- **`FerroView`** (`ferro_view.rs`): rendering starts when the surface exists and the view is visible and stops when either ends (`on_visibility_changed`), which is also how a destroyed surface stops the renderer.
- **Pointer input** (`AndroidMotionEventsHelper`): touch, pen and mouse devices by tool type; `Move` raises one event per pointer with the history as intermediate points; down and up by action and tool type (a finger: touch begin and end with the pointer id, so several fingers are several pointers); button press and release by the action button; scroll from the two scroll axes; cancel; modifiers from the meta state and the button state; positions divided by the scaling; pressure capped at one.
- **Keyboard** (`AndroidKeyboardEventsHelper`, stage 2a): `dispatchKeyEvent` of the Java view copies what upstream reads of a `KeyEvent` (time, action, key code, scan code, the character of the key with its meta state, the repeat count, control and shift, the characters of an event of several characters, the sources and the keyboard type of the device), and the helper translates the copy: a raw key event with the key of the key code (`AndroidKeyboardDevice.ConvertKey`, 112 key codes), the physical key of the scan code (`AndroidKeyInterop`, 162 scan codes), the key symbol and the device type, then a raw text event for a key down whose character is 32 or above. The key codes, actions, sources, input types and editor flags written in the sources were compared with `android.jar` of API 36 (`javap -constants`) **[M]**. Two things are upstream's and kept: the modifiers are control and shift only, and the device type test ("any bit in common" with the sources of a joystick or a gamepad) is true for every device with keys, so a key of an alphabetic keyboard has the device type of a gamepad. A key the shell injects has no scan code: no physical key, and for the enter key no key symbol **[E]**.
- **The view takes keys when it is focusable**, which upstream's input method makes it in its constructor; before stage 2b no key reached the view **[E]**.
- **Platform settings** (`AndroidPlatformSettings`, stage 2a): the theme (the night mode of the configuration), the contrast (the high contrast text of the secure settings), the accent colours (the three accent palettes of the system at tone 500 from API 31, the accent colour of the theme below), the hold duration and the double tap time of the view configuration, tap sizes from its slops and the density, the language of the first locale. The values are read from the application context through `PlatformHelper` in two calls and when `ConfigurationChangedReceiver` receives a configuration change, 100 ms later, as upstream; what changed is raised. The platform is initialised from `FerroApplication.onCreate`, so the application context exists.
- **The input method** is section 7.1.
- **Screens** (`AndroidScreens`): the displays of the display manager, keyed by display id; bounds from the maximum window metrics, the scaling from the density of the configuration, the orientation from the rotation and the natural orientation, as upstream; refreshed on the display listener and on a configuration change.
- **Insets** (`AndroidInsetsManager`, the insets manager and the input pane of a top-level): the safe area from the insets of the root window (status bars, navigation bars and the display cutout when the window is displayed edge to edge, which is forced when the application targets API 35 on a system of API 35), the preference, the system bar visibility, theme and colour, the occluded rectangle of the input method. Stage 2: the insets animation callback (the animated state change of the input pane), with the input method.

### 7.1 The input method (stage 2b)

| Upstream | Port | What it is |
|---|---|---|
| `AndroidInputMethod<TView>`, `IAndroidInputMethod` | `platform/input/android_input_method.rs` | `ITextInputMethodImpl` of the top-level. A client (a text box that has the focus) makes the view take the focus, restarts the input of the input method manager and shows the soft keyboard (`SHOW_IMPLICIT`); no client restarts and hides it (`HIDE_IMPLICIT_ONLY`). Changes of the text and of the selection of the client are reported to the system through the connection (`updateSelection`, `updateExtractedText` when the input method monitors). `SetOptions` gives the view what the next connection is made with: the input type, the action of the enter key and the flags of the editor from the options of the text input. |
| `IInitEditorInfo`, `AvaloniaView.OnCreateInputConnection` | `i_init_editor_info.rs`, `ferro_view_input.rs` | The view is asked by the system for a connection, calls what the input method gave it, sets the fields of the `EditorInfo` (through `PlatformHelper.setEditorInfo`: JNI by name and signature has no field access here) and returns a new `FerroInputConnection` of the Java layer that holds the number of the Rust object. |
| `AvaloniaInputConnection` | `platform/input/ferro_input_connection.rs`, `FerroInputConnection.java` | Batches (a level), a queue of commands applied when the outermost batch ends, the monitor mode and token of the extracted text, the editor action (done hides the keyboard, next moves the focus, then enter down and up), the context menu actions, the text around the cursor. The Java class forwards 18 methods; the seven upstream answers with `false` whatever the argument, and the handler (null), are answered in Java. |
| `TextEditBuffer` | `platform/input/text_edit_buffer.rs` | The text and the selection are the client's (`SurroundingText`, `Selection`); the buffer adds the composing region. A replacement selects the range, dispatches a forward delete key to the view and raises a text input through the top-level, as upstream does: the text box edits itself. |
| `EditCommand` and its eight classes | `platform/input/edit_command.rs` | An enum with a variant per class. |

Positions are UTF-16 code units on both sides (the text of the framework is a `String`; the buffer counts and cuts in code units). The input method, the view and the top-level are behind three traits (`IAndroidInputMethod`, `IInputMethodHost`, `IInputConnectionTopLevel`), so the whole of the logic runs in host tests against a client that edits a text as a text box does. The connection is called on the main thread (its handler is null), so what upstream guards with interlocked operations and a concurrent queue are plain cells here.

**The input pane.** From API 30 the decor view has a `WindowInsetsAnimation.Callback` (dispatch mode "stop"); `onStart` of an animation whose type mask has the input method reports the state with the rectangles of the lower and the upper bound, the duration and the interpolator as an easing (`AnimationEasing`, which calls `getInterpolation`). Below API 30 the global layout of the decor view sets the state. Measured **[E]**: the keyboard of the emulator opens to 312.4 logical pixels (820 px) over a client of 914.3, in 285 ms.

### 7.2 Services of a top-level (stage 2d)

- **Clipboard** (`ClipboardImpl`, the two clip data wrappers, `AndroidDataFormatHelper`): the primary clip of the clipboard manager as a data transfer. Formats are the MIME types of the clip description (text, a URI list as files, images as the bitmap format, `text/*` as formats of text, anything else as formats of bytes, with the prefix `application/frn-fmt.` for the formats of an application); an item answers text with `coerceToText`, a file with the storage item of its URI, a bitmap and bytes by reading that file. Setting data makes a `ClipData` from the first value each item has (text, the URI of a file, a string). A format does not carry the type of its values at run time in the port, so where upstream asks "is this a format of text" the port looks at the MIME type when reading and at the value when writing.
- **Storage** (`AndroidStorageProvider`, `AndroidStorageItem` with its file and folder, `PlatformSupport`): the pickers of the storage access framework (`ACTION_OPEN_DOCUMENT`, `ACTION_CREATE_DOCUMENT`, `ACTION_OPEN_DOCUMENT_TREE`, started for a result with a request code; the result arrives in `onActivityResult` of the activity and completes the future), bookmarks (persistable URI permissions), the well-known folders, items from paths, and the files and folders behind document URIs: properties from the columns of the provider, children of a tree, create, delete, move, streams over the file descriptor the content resolver opens (detached and owned by a `std::fs::File`). What `DocumentFile` of AndroidX does is done with `DocumentsContract` and queries of the content resolver in `StorageHelper`. A permission is asked for with `requestPermissions` and answered in `onRequestPermissionsResult`.
- **Launcher** (`AndroidLauncher`): a view intent for a URI or for the URI of a storage item; "no activity found" and "file URI exposed" are caught in Java (`PlatformHelper.tryStartActivity`) and answer false.
- **Platform feedback** (`AndroidPlatformFeedback`): the click sound of the audio manager, the haptic feedback of the view.
- **Native control host** (`AndroidNativeControlHostImpl`): a native control is a view of the system added to the Java view of the framework (a frame layout) over the surface, placed with frame layout parameters in pixels (the bounds times the scaling), hidden with visibility "gone". The default child is a frame layout. The catalog's embed page (`samples/ControlCatalog.Android/embed_sample_android.rs`) shows a button of the system that counts its clicks (its click listener is `interop::listeners::set_on_click_listener`, the stand-in for the event of the managed binding) and a web view.
- **File activation**: an intent whose data is a file or content URI raises the activation with the storage item of the URI.

## 8. File table

One row per upstream file. "Java" names the class of the Java layer that belongs to the file.

| Upstream file (`src/Android/Avalonia.Android`) | Lines | Rust file (`src/Android/FerroUI.Android`) | Java | Stage | Notes |
|---|---:|---|---|---|---|
| `AndroidDispatcherImpl.cs` | 163 | `android_dispatcher_impl.rs` | `MainLooperBridge` | 1 | |
| `AndroidPlatform.cs` | 141 | `android_platform.rs` | | 1 | options, `use_android`, the rendering mode loop |
| `AndroidRuntimePlatform.cs` | 53 | `android_runtime_platform.rs` | `PlatformHelper` | 1 | |
| `AndroidViewControlHandle.cs` | 26 | `android_view_control_handle.rs` | | 1 | |
| `ApplicationLifetime.cs` | 28 | `application_lifetime.rs` | | 1 | |
| `AvaloniaAccessHelper.cs` | 339 | `ferro_access_helper.rs` | (a node provider) | 3 | accessibility |
| `AvaloniaActivity.cs` | 259 | `ferro_activity.rs` | `FerroActivity` | 1, 2c, 2d | |
| `AvaloniaAndroidApplication.cs` | 45 | `ferro_android_application.rs` | `FerroApplication` | 1 | with `android_application!` |
| `AvaloniaMainActivity.cs` | 47 | `ferro_main_activity.rs` | `FerroMainActivity` | 1 | the main activity is a kind of the activity |
| `AvaloniaView.cs` | 155 | `ferro_view.rs` | `FerroView` | 1 | |
| `AvaloniaView.Input.cs` | 72 | `ferro_view_input.rs` | `FerroView` | 1, 2a, 2b | stage 3: hover, focus and key events to the access helper |
| `BackPressedCallback.cs` | 23 | `back_pressed_callback.rs` | (in `FerroActivity`) | 2c | |
| `ChoreographerTimer.cs` | 126 | `choreographer_timer.rs` | | 1 | |
| `CursorFactory.cs` | 21 | `cursor_factory.rs` | | 1 | |
| `IActivityResultHandler.cs` | 13 | `i_activity_result_handler.rs` | | 2c | |
| `IAndroidNavigationService.cs` | 14 | `i_android_navigation_service.rs` | | 2c | |
| `IAvaloniaActivity.cs` | 11 | `i_ferro_activity.rs` | | 1, 2c | |
| `IInitEditorInfo.cs` | 11 | `i_init_editor_info.rs` | | 2b | with the `EditorInfo` of values |
| `PlatformIconLoader.cs` | 47 | `platform_icon_loader.rs` | | 1 | |
| `Stubs.cs` | 60 | `stubs.rs` | | 1 | |
| `Automation/*.cs` (9 files) | 359 | `automation/*.rs` | | 3 | the node info providers |
| `Platform/AndroidActivatableLifetime.cs` | 75 | `platform/android_activatable_lifetime.rs` | | 1 | |
| `Platform/AndroidDataFormatHelper.cs` | 43 | `platform/android_data_format_helper.rs` | | 2d | |
| `Platform/AndroidInsetsManager.cs` | 380 | `platform/android_insets_manager.rs` | `PlatformHelper` | 1, 2b | |
| `Platform/AndroidLauncher.cs` | 62 | `platform/android_launcher.rs` | `PlatformHelper` | 2d | |
| `Platform/AndroidNativeControlHostImpl.cs` | 137 | `platform/android_native_control_host_impl.rs` | `PlatformHelper` | 2d | |
| `Platform/AndroidPlatformFeedback.cs` | 43 | `platform/android_platform_feedback.rs` | | 2d | |
| `Platform/AndroidPlatformSettings.cs` | 205 | `platform/android_platform_settings.rs` | `ConfigurationChangedReceiver`, `PlatformHelper` | 2a | |
| `Platform/AndroidScreens.cs` | 154 | `platform/android_screens.rs` | `PlatformHelper` | 1 | |
| `Platform/AndroidSystemNavigationManager.cs` | 39 | `platform/android_system_navigation_manager.rs` | | 2c | |
| `Platform/ClipDataItemToDataTransferItemWrapper.cs` | 108 | `platform/clip_data_item_to_data_transfer_item_wrapper.rs` | | 2d | |
| `Platform/ClipDataToDataTransferWrapper.cs` | 58 | `platform/clip_data_to_data_transfer_wrapper.rs` | | 2d | |
| `Platform/ClipboardImpl.cs` | 149 | `platform/clipboard_impl.rs` | | 2d | |
| `Platform/PlatformSupport.cs` | 51 | `platform/platform_support.rs` | | 2d | with the completion that stands in for a task completion source |
| `Platform/Input/AndroidInputMethod.cs` | 235 | `platform/input/android_input_method.rs` | | 2b | |
| `Platform/Input/AndroidKeyboardDevice.cs` | 224 | `platform/input/android_keyboard_device.rs` | | 2a | |
| `Platform/Input/AvaloniaInputConnection.cs` | 341 | `platform/input/ferro_input_connection.rs` | `FerroInputConnection` | 2b | |
| `Platform/Input/EditCommand.cs` | 196 | `platform/input/edit_command.rs` | | 2b | |
| `Platform/Input/TextEditBuffer.cs` | 141 | `platform/input/text_edit_buffer.rs` | | 2b | |
| `Platform/SkiaPlatform/AndroidFramebuffer.cs` | 123 | `platform/skia_platform/android_framebuffer.rs` | | 1 | its declarations of the NDK are in `interop/ndk.rs` |
| `Platform/SkiaPlatform/FramebufferManager.cs` | 22 | `platform/skia_platform/framebuffer_manager.rs` | | 1 | |
| `Platform/SkiaPlatform/InvalidationAwareSurfaceView.cs` | 113 | `platform/skia_platform/invalidation_aware_surface_view.rs` | `FerroSurfaceView` | 1 | |
| `Platform/SkiaPlatform/TopLevelImpl.cs` | 415 | `platform/skia_platform/top_level_impl.rs` | `FerroSurfaceView` | 1, 2a to 2d | the transparency levels beyond `None` are written and never ran |
| `Platform/Specific/Helpers/AndroidKeyInterop.cs` | 198 | `platform/specific/helpers/android_key_interop.rs` | | 2a | |
| `Platform/Specific/Helpers/AndroidKeyboardEventsHelper.cs` | 180 | `platform/specific/helpers/android_keyboard_events_helper.rs` | `FerroView` | 2a | |
| `Platform/Specific/Helpers/AndroidMotionEventsHelper.cs` | 252 | `platform/specific/helpers/android_motion_events_helper.rs` | `FerroView` | 1 | |
| `Platform/Storage/AndroidStorageItem.cs` | 671 | `platform/storage/android_storage_item.rs` | `StorageHelper` | 2d | |
| `Platform/Storage/AndroidStorageProvider.cs` | 344 | `platform/storage/android_storage_provider.rs` | | 2d | |
| `Platform/Vulkan/VulkanNativeInterop.cs` | 25 | `platform/vulkan/vulkan_native_interop.rs` | | later | with the Vulkan project |
| `Platform/Vulkan/VulkanSupport.cs` | 71 | `platform/vulkan/vulkan_support.rs` | | later | |

Files of the port without an upstream file: `interop/java.rs` (JNI, section 2.1), `interop/ndk.rs` (the NDK, section 2.2), `interop/natives.rs` (the table of native methods and the guard of section 2.3), `android_egl.rs` (the EGL library of the system and the platform graphics of section 6), `log.rs` (the log of the system, where the standard streams of a process go nowhere), `interop/listeners.rs` (a click listener whose handler is a closure, for the native controls of an application).

## 9. Packaging without Gradle

`scripts/android/apk.sh` builds an installable package from a cargo target with the tools of the SDK alone (`scripts/android/env.sh` names them and the pinned versions: NDK 28.2.13676358, build tools 36.0.0, platform `android-36`, Java 17):

1. `cargo build --target aarch64-linux-android` of the example or the library, with the pinned nightly toolchain and the standard library built along (section 5.1), with the clang of the NDK as the linker and as the C and C++ compiler through cargo's variables (`CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER`, `CC_aarch64_linux_android`, `CXX_...`, `AR_...`), and with the static C++ runtime named for the build scripts that compile C++ (`CXXSTDLIB_<target>=c++_static`: the `cc` crate names the shared runtime by default, and the package would have to carry `libc++_shared.so`; the Skia binary links the static one, so the library has one C++ runtime). The result is `lib<name>.so`; `llvm-strip --strip-unneeded` of the NDK removes what the loader does not need. The script refuses a library that exports no `JNI_OnLoad` or that needs anything but libraries of the system.
2. `javac` compiles the Java layer (and the Java sources of the application, if it has any) against `android.jar` of the platform; `d8` turns the classes into `classes.dex`.
3. `aapt2 compile` and `aapt2 link` make the package from the manifest and the resources (the manifest is a template of the script: the application id, the label, the library name, the activity class; an application may bring its own).
4. `zip` adds `classes.dex` and `lib/arm64-v8a/lib<name>.so`; `zipalign` aligns; `apksigner` signs with a debug key that `keytool` generates once under the build directory.

Nothing of this writes outside the build directory. A release configuration (`--release`) uses the release profile of cargo; signing for a store is the application's business.

## 10. Verification

Three levels, because nothing of Android runs on the development machine without the emulator, and the emulator may only run while no virtual machine does (the orchestrator serialises it).

1. **Compilation for the target** **[M]**: `cargo build -p ferroui-android --target aarch64-linux-android` with the example, and the package.
2. **Host tests** (`cargo test -p ferroui-android`): what is logic and not a call into the system compiles on every host and is tested there: the options and the loop over the rendering modes; the translation of motion events (as copies, the way the Java layer delivers them); the orientation and the bounds of a screen; the safe area and the occluded rectangle from insets; the format of a locked buffer; the check of JNI arguments against a signature; the stubs.
3. **The emulator** **[E]**: `scripts/android/emu-smoke.sh` starts the device `ferroui_api36` headless, installs the package of the smoke example, starts its activity, reads the log for the tag until the result line, pulls the report file, takes pictures with `screencap`, uninstalls and shuts the emulator down. `scripts/android/emu-catalog.sh` does the same with the ControlCatalog host and takes a picture per page.

The smoke example (`examples/android_smoke.rs`, an APK) asks the system and not only the framework, in the order of the table below; each check is a line `[ ok ]` or `[FAIL]` under the tag `ferroui-smoke` and in `files/smoke-report.txt`, and the last line is `RESULT: PASS` or `RESULT: FAIL`.

| Check | What it proves |
|---|---|
| the library loaded, the natives registered, the application set up | section 2.3; the dispatcher on the main looper |
| the activity created its view, the surface was created, with a size | sections 3.1 and 7 |
| the client size times the scaling is the surface size; the scaling is the density of the display | the top-level |
| the screen: one primary screen, bounds at least the surface, scaling equal | `AndroidScreens` |
| the insets: the window is displayed edge to edge on API 36, the safe area has a top inset (the status bar) | `AndroidInsetsManager` |
| a dispatcher timer fires and a job posted from another thread runs on the main thread | the dispatcher, the signal through JNI from a thread the virtual machine did not start |
| frames are drawn: the render timer ticks on the render thread, and pixels read back at several points have the colours drawn (from the surface with `glReadPixels` in the EGL mode, from the locked buffer in the software mode) | the choreographer, rendering, Skia and text |
| a touch sequence injected by the script (`input tap`, `input swipe`) arrives as pointer pressed, moved and released at the right logical position | pointer input |
| keys injected by the script (`input keyevent`, `input text`): the key A as key down with its symbol and key up, the text of six keys, the enter key without text (2a) | `dispatchKeyEvent`, the keyboard helper, the key table |
| the settings of the platform have accent colours, tap sizes and a language; the night mode the script switches (`cmd uimode night`) arrives as a change of the colour values, both ways (2a) | `AndroidPlatformSettings`, the receiver |
| an editor of the application takes the focus: the input pane opens with a rectangle and an animated change; two taps of the script on keys of the soft keyboard arrive as text through the input connection; without a client the pane closes (2b) | the input method, the connection, the insets animation |
| the home button deactivates the application; started again it is activated and frames are drawn to the new surface with the colours drawn (2c) | the surface lost and created again, EGL and software |
| the display rotated by a quarter turn: the client size, the orientation of the screen, frames at the new size; and back (2c) | configuration changes |
| an intent with a URI for the running activity is a protocol activation with that URI (2c) | `onNewIntent`, `HandleIntent` |
| the back button raises the back request of the top-level; handled, the activity stays; not handled, the default action of the system follows (2c) | the back callback, the navigation manager |
| the activity is started again in the process: surface, screen, insets, dispatcher, frames and pixels again (2c) | a second activity object, a second view and top-level |
| text set on the clipboard reads back, with characters outside the basic plane; cleared, the clipboard has no text (2d) | `ClipboardImpl`, the clip data wrappers |
| the feedback performs a haptic hold and a click sound, and no hold as a sound; a URI nobody handles is not launched; the storage provider can open, save and pick folders and has a documents folder (2d) | the features of the top-level, `tryStartActivity`, the well-known folders |
| a default child of the native control host, attached and shown in a rectangle, is a visible child view at those pixels; hidden it is gone; disposed it has no parent (2d) | `AndroidNativeControlHostImpl` |
| the activity finishes itself and the top-level is disposed | the lifecycle of stage 1 |

The script runs the example twice: with the default options (EGL) and with `Software` (a properties file in the files directory of the application, because the platform is initialised before an activity and its intent exist). The application asks the script for what only the outside can do with lines `SCRIPT <command> <arguments>` in its log (a picture, a tap, a swipe, key codes, text, the night mode, home, start, rotate, an intent with a URI, back); the script does each once, in order (`emu-smoke.sh` lists them). The touch position is a pixel of the view, which is a pixel of the screen while the window is displayed edge to edge (always on API 35 and later).

**What an emulator run cannot show:** a real GPU driver (the emulator's OpenGL ES is SwiftShader or a translation to the host), more than one finger (the shell injects one pointer), a stylus or a mouse, a hardware keyboard (the keys the shell injects have no scan code, so the scan code table and the key symbols of control keys are host tests only), other input methods than the one of the image (composition as an Asian input method does it: the editing commands are host tests), a remote control or a gamepad, multi-window, other API levels than 36 (everything below API 33 of the back button and below API 30 of the insets is written and never ran), other densities than the device's, 16 KB page devices, and performance.

### 10.1 Measured on the emulator (2026-10-10) **[E]**

The device `ferroui_api36` (Pixel 7, Android 16, API 36, `arm64-v8a`, 1080 by 2400 at 420 dpi), headless on an Apple Silicon Mac, `-gpu swiftshader_indirect`. Three runs:

| Run | Result |
|---|---|
| 1 | The application aborted at start, in both modes, before any code of the backend ran: the keys of thread-local storage (section 5.1). |
| 2, the library built with the standard library | **Software: every check passes.** EGL: every check but the last; the activity was destroyed nine seconds after `finish` (the close transition of the system), one second after the application stopped waiting. The catalog did not load: its library needed `libc++_shared.so` (section 9, step 1). |
| 3, the wait raised to 40 s, the C++ runtime static | **Both modes pass every check** (`SMOKE PASSED`), and **the ControlCatalog starts and shows its pages** (`CATALOG PASSED`: Home, Buttons, TextBlock, ListBox, Image, Calendar, a picture of each). |

What the checks of run 3 say, the same in both modes unless noted: the client size is 411.43 by 914.29 at scaling 2.625, a surface of 1080 by 2400 pixels; one screen, primary, "Built-in Screen", bounds 1080 by 2400, scaling 2.625, portrait; the window is displayed edge to edge (forced on API 36) with a safe area of 51.8 at the top and 24 at the bottom; the dispatcher timer fires and a job posted from a thread the virtual machine did not start runs on the main thread; three frames are drawn on the thread `Render Thread` and read back from a Skia surface of 1080 by 2400 pixels (EGL: a GPU surface; software: raster, the buffer of the native window); the three pixels read back are exactly the colours drawn; the line of text has 8495 nearly white pixels (8493 in software): the font manager of Skia on Android finds the default font of the system; the tap arrives as a pointer pressed within a fifth of a logical pixel of where it was injected, and the tap and the swipe as 2 pressed, 8 to 14 moved and 2 released events; the activity finishes, the view is released and the top-level disposed.

The catalog (a debug build, 230 MB of library in a package of 62 MB, EGL): five seconds from loading the library to the main view, with the Fluent theme from compiled markup, the embedded assets and the fonts of the system; no warning or error of the framework in the log during the run. Pictures: `images/control_catalog_android.png`, `images/control_catalog_android_buttons.png`, `images/android_smoke.png`.

Nothing of the platform code had to be changed for these runs: the two changes were to how the library is built, and one to how long the smoke application waits. What the runs did not exercise: more than one activity, a second start of the activity in the same process, rotation and other configuration changes, the surface lost and created again (pause and resume), the transparency levels, system bar colours and visibility set by an application, more than one pointer, a mouse or a pen, `--gpu host`.

### 10.2 Stages 2a to 2c on the emulator (2026-10-10) **[E]**

Three runs of `emu-smoke.sh` on the device of 10.1, both rendering modes each time.

| Run | Result |
|---|---|
| 1, stage 2a alone | No key reached the view: a frame layout is not focusable, and upstream makes its view focusable in the constructor of its input method, which is stage 2b. The settings check passed. The night mode check passed in the software run and timed out in the EGL run, the first after the boot: the broadcast of the configuration change was late on a device that had just booted (the wait is 60 s since). |
| 2, with 2b | Every check but one of the application's own: it expected the enter key to have the key symbol of the physical enter key, which a key without a scan code does not have (the expectation was wrong, not the port). |
| 3, with 2c | **Both modes pass every check** (`SMOKE PASSED`), and the catalog passes with the TextBox page. |

What run 3 says, the same in both modes: the key A arrives as key down with the symbol "a" and key up, seven keys go down and up, the text of the keys is "aferro", the enter key raises no text. The settings: light, no contrast preference, three accent colours of the system (96, 118, 172; 112, 119, 139; 140, 109, 140), a tap size of 16 and a double tap size of 200.4 logical pixels (slops of 21 and 263 px at 2.625). The night mode arrives as one change to dark and one back. With the editor focused the input pane opens to a rectangle of 411.4 by 312.4 at y 577.9, reported once with a duration of 285 ms and an easing; two taps on the keyboard make the text "gn" through two text inputs; without a client the pane closes, reported the same way. Home deactivates, the start activates, and frames are drawn to the new surface of 1080 by 2400 with the colours drawn (EGL: the render target made a new EGL surface for the new native window; nothing of the platform code had to change for it). Rotated, the client is 914.3 by 411.4, the screen landscape, frames of 2400 by 1080; and back. The URI `ferroui-smoke://hello/world?answer=42` arrives as a protocol activation. The back button raises one back request each time; handled, the activity stays; not handled, the system finishes the activity (API 36), and started again in the same process the new activity passes the checks of the surface, the dispatcher, the frames and the pixels again.

The catalog: the TextBox page is in the list. The first run of the script tapped where no editor is (the page is a list of samples); the script now opens the first sample, taps into its text box and takes the picture when `dumpsys input_method` says the keyboard is shown.

Not exercised by these runs: the transparency levels, system bar colours and visibility set by an application, more than one pointer, a mouse or a pen, `--gpu host`, the paths of other API levels.

### 10.3 Stage 2d on the emulator (2026-10-10) **[E]**

One run of `emu-smoke.sh` and two of `emu-catalog.sh` on the device of 10.1.

**The smoke application: both modes pass every check** (`SMOKE PASSED`). What the checks of stage 2d say, the same in both modes: the text "FerroUI żółw" with a character outside the basic plane, set on the clipboard of the system, reads back the same, and after a clear the clipboard has no text; the feedback performs a haptic hold and a click sound and refuses a hold as a sound; a URI of a scheme nobody handles is answered with false by the launcher (the exception of `startActivity` caught in Java); the storage provider can open, save and pick folders, and its documents folder is `file:///storage/emulated/0/Android/data/org.ferroui.smoke/files/Documents/`; a default child of the native control host shown in the rectangle (20, 30, 100, 50) is a visible child of the Java view at (52, 78) with 262 by 131 px, which is the rectangle times 2.625 truncated, hidden it has the visibility "gone", and disposed it has no parent.

**The catalog passes with eight pages.** The TextBox page: the script opens the first sample, taps into its text box, and `dumpsys input_method` reports the keyboard shown; the picture has the box focused with the word the script typed, the text bound to it updated beside it, and the keyboard of the system over the page (`catalog-03-TextBox-keyboard.png`).

**The Native Embed page, and what the first picture of it showed.** In the first run the page had its own controls and two empty areas where the button and the web view belong, although both views existed (the log has the web view loading its page). The second run printed the children of the Java view half way through the page, laid the main view out once more, and printed them again:

| | Button | WebView |
|---|---|---|
| before the layout | at (1132, 566), 976 by 809 px, visible | at (1132, 1473), 976 by 809 px, visible |
| after the layout | at (52, 566), 976 by 811 px, visible | at (52, 1475), 976 by 810 px, visible |

The view is 1080 px wide: before the layout both native views were exactly one view width to the right of where the page shows them. This is the case the iOS backend met (`ios-platform.md`, section 12): the page slides in with a render transform, and `NativeControlHost` places its control when its bounds or the bounds or visibility of an ancestor change, not when a render transform does (the port matches upstream there), so the controls stay where the page was when it was laid out until the next change of layout. It is not the z-order: the native views are the second and third child of the frame layout, after the surface view, whose surface is behind the window of the activity, and after the layout they are drawn over the page (`catalog-07-Native_Embed.png`: the button of the system with "Hello world", the web view with the page of android.com). The catalog host did in the smoke run of that day what the iOS host did (one more layout of the main view per page, with the two lines in the log).

**Fixed in the controls crate since (branch `native-control-host-transforms`, not yet run on the emulator).** `NativeControlHost` places its control also when a render transform of itself or of an ancestor changes (`DEVIATIONS.md`, "Native control host": an upstream defect, fixed; reproduced and tested without a device in `native_control_host_tests.rs` of the controls crate). The smoke run of the catalog host no longer lays the main view out again: it writes one line, `Native views of <header> after the transition: ..`, half way through the time of a page, followed by `PAGE-SHOWN`. The run that proves it is `scripts/android/emu-catalog.sh` with the Native Embed page: that line must have the button and the web view at x = 52 (they were at 1132), and the picture must show them in the page.

**Not proven by a run:** the pickers (they need a person, or automation of the picker of the system), and with them bookmarks, document trees, the streams of a picked file and the permission request; the file activation; the launcher with a URI that is handled (it leaves the application); a click on the native button; clipboard formats other than text.

## 11. Stages

| Stage | Content | What its emulator run proves |
|---|---|---|
| **1** (built and run, 2026-10-10) | the crate, the Java layer, the bindings; platform initialisation and options; the application, the activity, the view; the top-level over the surface; the dispatcher; the choreographer timer; software and EGL rendering; pointer input; screens; insets; the smoke example; the packaging and emulator scripts | section 10, level 3 |
| **1b** (built and run, 2026-10-10) | the host of the ControlCatalog (`samples/ControlCatalog.Android`) and its script | the catalog starts with the Fluent theme, shows its pages, and a picture of each of a handful is taken |
| **2a** (built and run, 2026-10-10) | keyboard: `AndroidKeyboardEventsHelper`, `AndroidKeyInterop`, `AndroidKeyboardDevice`; the platform settings (`AndroidPlatformSettings`) | key events injected by the script arrive as key down, up and text; the theme follows `cmd uimode night` |
| **2b** (built and run, 2026-10-10) | the input method: `AndroidInputMethod`, the input connection (`AvaloniaInputConnection`), `TextEditBuffer`, `EditCommand`, `IInitEditorInfo`, the insets animation of the input pane | the soft keyboard of the emulator opens for an editor, its keys arrive as text, it closes; the editing commands and the connection in host tests |
| **2c** (built and run, 2026-10-10) | lifecycle: the surface lost and created again, `onNewIntent` and protocol activation, activity and permission results, rotation, the back button (`BackPressedCallback`, `IActivityNavigationService`, `AndroidSystemNavigationManagerImpl`) | the script sends the application to the background and back, rotates, sends an intent, presses back, starts the activity a second time |
| **2d** (built and run, 2026-10-10; with what this table had as 2e and 2f) | services: the clipboard, the launcher, the platform feedback; the storage provider and items; the native control host and the embed sample of the catalog | the checks of 10.3; the pickers themselves need a person |
| 3 | accessibility: the access helper and the node info providers | the node tree read with `uiautomator dump` |
| later | Vulkan, with the Vulkan project of the port | |

## 12. CI

The job `android` of `.github/workflows/ci.yml`, on the Ubuntu runner: the host tests of the crate; the package of the smoke application built by `scripts/android/apk.sh` (with the nightly toolchain of section 5.1, installed with `rustup` and its component `rust-src`); the Skia feature set of the target checked; the ControlCatalog host checked for the target with the stable toolchain. The runner image has an Android SDK (`ANDROID_HOME`) with build tools, platforms and several NDK versions **[R]**; the job prints what it has and installs the pinned NDK, build tools and platform with `sdkmanager` when they are missing. The job had not run when this was written.

No emulator runs in CI: a hosted Linux runner can run the x86-64 emulator with KVM, but the Skia binary for `x86_64-linux-android` is not verified (section 5) and a boot costs minutes of a job that would be flaky; the emulator runs stay with the development machine (section 10.1) until that is worth it.

## 13. Deviations

Recorded in `DEVIATIONS.md`, section "Android platform": the Java layer and what stays in Java; the platform classes in place of AndroidX; events copied in Java; the counted native window; the platform graphics per thread; the names (package, classes, the meta-data key, the log tag); the API level; what is not built yet and how each such place behaves.
