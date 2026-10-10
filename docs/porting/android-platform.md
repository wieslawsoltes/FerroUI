# The Android platform

The design of the Android backend of FerroUI, the decisions it rests on, the file table of the port and its stages. Upstream: `src/Android/Avalonia.Android` (58 files, 88 types, 447 members, 7068 lines) at the tracked commit (`TRACKING.md`), and the host of the sample, `samples/ControlCatalog.Android`.

Marks, as in the other platform documents: **[V]** verified from sources (the upstream files, the sources of a crate in the cargo registry, `cargo info`, the files of the SDK), **[M]** measured here (a build or a test run on the development machine, a Mac), **[E]** shown by a run on the emulator (the orchestrator runs the scripts of section 10; nothing of this kind was run when the document was first written), **[R]** recalled and not verified here.

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
| `FerroActivity` | `android.app.Activity` | `AvaloniaActivity` | `onCreate`, `onStart`, `onStop`, `onResume`, `onDestroy`; stage 2: `onNewIntent`, `onActivityResult`, `onRequestPermissionsResult`, the back button |
| `FerroMainActivity` | `FerroActivity` | `AvaloniaMainActivity` | states that it is the main activity |
| `FerroView` | `android.widget.FrameLayout` | `AvaloniaView` (both parts) | `onAttachedToWindow`, `onVisibilityAggregated`, `onVisibilityChanged`, `onConfigurationChanged`, the focus change of the view, the global layout, `dispatchTouchEvent`, `dispatchGenericPointerEvent`, `dispatchHoverEvent`; stage 2: `dispatchKeyEvent`, `onCreateInputConnection` |
| `FerroSurfaceView` | `android.view.SurfaceView`, `SurfaceHolder.Callback2` | `InvalidationAwareSurfaceView` and `TopLevelImpl.SurfaceViewImpl` | the five callbacks of the surface holder; `dispatchDraw` clears the canvas (upstream's workaround, which is drawing code of the view and stays in Java) |
| `MainLooperBridge` | `MessageQueue.IdleHandler` | the `Handler`, the three `Runnable`s and the `IdleHandler` of `AndroidDispatcherImpl` | the signal, the timer and the idle callback of the main looper |
| `PlatformHelper` | (static) | calls upstream makes inline on framework objects that return more than one value | the insets of a window, the displays, the system features, the insets listener of the decor view |

### 3.1 Objects on both sides

A Rust object that has a Java object owns a global reference to it. A Java object that calls back holds a `long`, the number under which its Rust object is registered on the UI thread (a map of the thread, to weak references for views and top-levels and to strong references for activities, which live from `onCreate` to `onDestroy`). A native method looks its object up by the number and does nothing when it is gone. No address of a Rust object is ever given to Java.

Construction runs in upstream's order, driven from Rust: `FerroActivity::on_create` creates the Rust `FerroView`, whose constructor creates the Java view, the top-level (which creates the Java surface view), adds the surface view to the view, creates and prepares the `EmbeddableControlRoot`, sets the transparent background and sends the configuration.

### 3.2 Events are copied in Java and translated in Rust

A `MotionEvent` is a Java object with some forty getters, valid only during the call. `FerroView` copies what upstream's helper reads into primitive arguments and two arrays (per pointer: id, tool type, x, y, pressure, orientation; then the history of each pointer) and passes them in one native call; `AndroidMotionEventsHelper` in Rust translates the copy, which makes the translation a function of values that the host tests run. Upstream reads the history lazily from the event; the copy reads it eagerly, because the event is recycled after the call.

### 3.3 Without AndroidX

| Upstream uses | The port uses | Difference |
|---|---|---|
| `AppCompatActivity` | `android.app.Activity` | no AppCompat theme and no local night mode: `SetFrameThemeVariant` sets the system bar theme only (the night mode of an activity through `UiModeManager`/configuration override is stage 2) |
| `OnBackPressedDispatcher`, `OnBackPressedCallback` | `OnBackInvokedDispatcher` (API 33) and `onBackPressed` below | stage 2 |
| `ViewCompat`, `WindowCompat`, `WindowInsetsCompat`, `WindowInsetsControllerCompat`, `WindowInsetsAnimationCompat` | `WindowInsets`, `WindowInsetsController`, `WindowInsetsAnimation` of the platform (API 30); below API 30 the deprecated calls the compatibility classes make themselves | below API 30 the visibility of the input method is not known (stage 2) |
| `WindowMetricsCalculator` (AndroidX Window) | `WindowManager.getMaximumWindowMetrics` (API 30), which is what the calculator calls there | none on API 30 and later |
| `ExploreByTouchHelper` | `AccessibilityNodeProvider` | stage 3 |
| `DocumentFile` | `DocumentsContract` | stage 2 |

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

### 5.1 Thread-local storage: the standard library is built with the application

Found by the first run on the emulator **[E]**: the application aborted while the application object registered its services, in `std::sys::thread_local::key::racy::LazyKey::lazy_init`, reached from the property cell of an input element.

- The Android targets of Rust have emulated TLS and are not marked as having thread-local statics (`rustc --print cfg --target aarch64-linux-android` has no `target_thread_local`; the target specification says `"tls-model": "emulated"` and nothing else about it) **[V]**. The standard library as it is distributed for them therefore keeps every `thread_local!` variable under a key of the C library of its own (`pthread_key_create`), created when the variable is first used.
- Bionic has 128 such keys for a process, and the virtual machine and the system libraries hold some. The class model of the port has a thread-local cell per registered property and per routed event (the definitions are per thread, `PORTING-GUIDE.md`, "Threading model"), and 230 more in ordinary code: an application passes 128 before its first control exists. The same build runs on every other platform because their targets have native thread-local storage.
- **What the scripts do**: the native library is built with a nightly toolchain and `-Zbuild-std=std,panic_unwind`, with `-Zhas-thread-local=yes` for the target, so that the standard library keeps its thread-local variables in thread-local statics, which the compiler lowers to the emulated TLS of the compiler runtime (`__emutls_get_address`: one key for all of them). The runtime library of the NDK that has that function (`libclang_rt.builtins`) is named first on the link line (`-Zpre-link-args`), because one of the dependencies names the shared C++ runtime, which exports the function too and would otherwise become a dependency of the library. Verified on the built library **[M]**: 787 emulated TLS variables, none of the key-based storage of the standard library, and the same seven system libraries as before.
- The toolchain is pinned in `scripts/android/env.sh` (`nightly-2026-07-01`, with the component `rust-src`). **This is a decision for the owner**: an Android build needs a nightly toolchain until the Android targets have thread-local statics in the distributed standard library, or until the class model stops taking a thread-local variable per property on this target (one table of the thread indexed by a number per property: a change of `ferroui-base` and of every crate that declares a `thread_local!`, which this work did not make). `cargo check` and `cargo build` with the stable toolchain still work and are what the CI job could do without nightly; the result does not run.

## 6. Rendering

`AndroidPlatformOptions::rendering_mode` is upstream's list with upstream's default: EGL, then software. The platform takes the first mode that initialises; a list in which nothing does is an error with upstream's message.

| Mode | How it draws | Stage |
|---|---|---|
| `Software` | `FramebufferManager`: the renderer draws into the buffer `ANativeWindow_lock` returns (RGBA 8888 premultiplied, or RGB 565 opaque, as the window says) and `ANativeWindow_unlockAndPost` shows it | 1 |
| `Egl` | OpenGL ES through the EGL of the system on the native window; Skia's Ganesh on that context | 1 |
| `Vulkan` | `VulkanSupport` | after the Vulkan project of the port (out of scope today); passed over like a mode that fails to initialise |

**EGL.** Upstream calls `EglPlatformGraphics.TryCreate()`, whose display loads `libEGL.so` by name. The EGL of the port (`ferroui-opengl`) takes a loader function instead of a library name, and its platform graphics are bound to the thread that created them, because a display and a context are objects of one thread there. On Android the compositor renders on the render thread, so the backend has platform graphics of its own (`android_egl.rs`), written like the ones of ANGLE on Windows (`win32-platform.md` 6.1, "Threads"): the graphics hold nothing of a thread; a context loads the interface (`dlopen("libEGL.so")`, entry points by `dlsym` and then `eglGetProcAddress`) and creates its display object on the thread that asks for it. The display of the system is one per process and counted (`eglInitialize`/`eglTerminate`), so a display object per context is the same display. The platform probes once on the UI thread (a display, a context, made current, disposed), as `TryCreate` does by creating the display.

**The surface of a top-level** is, as upstream, three surfaces: an `EglGlPlatformSurface` over what the surface view publishes (handle, size, scaling; waits skipped), the framebuffer manager, and the handle of the native window. The renderer takes the first it can use.

**Emulator.** The scripts start the emulator with `-gpu swiftshader_indirect` by default: a software implementation of OpenGL ES inside the emulator, which needs neither a window nor the GPU of the host, so a headless run draws the same on every machine **[R]**; `--gpu host` selects the host GPU. Which works headless on Apple Silicon is what the first run shows **[E]**.

## 7. The top-level, the view and input

- **`TopLevelImpl`** (`platform/skia_platform/top_level_impl.rs`): client size and scaling from the surface view (the frame of the surface holder and the density of the display metrics), the callbacks, the surfaces, the features (stage 1: the insets manager and the input pane, the screens; the others arrive with their stage), transparency levels (stage 2 beyond `None`), `SurfaceRedrawNeeded` as a synchronous paint and `SurfaceRedrawNeededAsync` through `Compositor::request_composition_update`.
- **`FerroView`** (`ferro_view.rs`): rendering starts when the surface exists and the view is visible and stops when either ends (`on_visibility_changed`), which is also how a destroyed surface stops the renderer.
- **Pointer input** (`AndroidMotionEventsHelper`): touch, pen and mouse devices by tool type; `Move` raises one event per pointer with the history as intermediate points; down and up by action and tool type (a finger: touch begin and end with the pointer id, so several fingers are several pointers); button press and release by the action button; scroll from the two scroll axes; cancel; modifiers from the meta state and the button state; positions divided by the scaling; pressure capped at one.
- **Keyboard and text** are stage 2: `AndroidKeyboardEventsHelper`, `AndroidKeyInterop`, `AndroidKeyboardDevice`, and the input method (`AndroidInputMethod`, `AvaloniaInputConnection`, `TextEditBuffer`, `EditCommand`). Until then the platform binds the keyboard device of the base library, the view returns no input connection and key events go to the base class.
- **Screens** (`AndroidScreens`): the displays of the display manager, keyed by display id; bounds from the maximum window metrics, the scaling from the density of the configuration, the orientation from the rotation and the natural orientation, as upstream; refreshed on the display listener and on a configuration change.
- **Insets** (`AndroidInsetsManager`, the insets manager and the input pane of a top-level): the safe area from the insets of the root window (status bars, navigation bars and the display cutout when the window is displayed edge to edge, which is forced when the application targets API 35 on a system of API 35), the preference, the system bar visibility, theme and colour, the occluded rectangle of the input method. Stage 2: the insets animation callback (the animated state change of the input pane), with the input method.

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
| `AvaloniaActivity.cs` | 259 | `ferro_activity.rs` | `FerroActivity` | 1 | stage 2: back button, activity results, permissions, intents |
| `AvaloniaAndroidApplication.cs` | 45 | `ferro_android_application.rs` | `FerroApplication` | 1 | with `android_application!` |
| `AvaloniaMainActivity.cs` | 47 | `ferro_main_activity.rs` | `FerroMainActivity` | 1 | the main activity is a kind of the activity |
| `AvaloniaView.cs` | 155 | `ferro_view.rs` | `FerroView` | 1 | |
| `AvaloniaView.Input.cs` | 72 | `ferro_view_input.rs` | `FerroView` | 1 | stage 2: key events, the input connection; stage 3: hover to the access helper |
| `BackPressedCallback.cs` | 23 | `back_pressed_callback.rs` | (in `FerroActivity`) | 2 | |
| `ChoreographerTimer.cs` | 126 | `choreographer_timer.rs` | | 1 | |
| `CursorFactory.cs` | 21 | `cursor_factory.rs` | | 1 | |
| `IActivityResultHandler.cs` | 13 | `i_activity_result_handler.rs` | | 2 | |
| `IAndroidNavigationService.cs` | 14 | `i_android_navigation_service.rs` | | 2 | |
| `IAvaloniaActivity.cs` | 11 | `i_ferro_activity.rs` | | 1 | the activation events; the two base contracts are stage 2 |
| `IInitEditorInfo.cs` | 11 | `i_init_editor_info.rs` | | 2 | |
| `PlatformIconLoader.cs` | 47 | `platform_icon_loader.rs` | | 1 | |
| `Stubs.cs` | 60 | `stubs.rs` | | 1 | |
| `Automation/*.cs` (9 files) | 359 | `automation/*.rs` | | 3 | the node info providers |
| `Platform/AndroidActivatableLifetime.cs` | 75 | `platform/android_activatable_lifetime.rs` | | 1 | |
| `Platform/AndroidDataFormatHelper.cs` | 43 | `platform/android_data_format_helper.rs` | | 2 | |
| `Platform/AndroidInsetsManager.cs` | 380 | `platform/android_insets_manager.rs` | `PlatformHelper` | 1 | stage 2: the insets animation |
| `Platform/AndroidLauncher.cs` | 62 | `platform/android_launcher.rs` | | 2 | |
| `Platform/AndroidNativeControlHostImpl.cs` | 137 | `platform/android_native_control_host_impl.rs` | | 2 | |
| `Platform/AndroidPlatformFeedback.cs` | 43 | `platform/android_platform_feedback.rs` | | 2 | |
| `Platform/AndroidPlatformSettings.cs` | 205 | `platform/android_platform_settings.rs` | | 2 | until then the default settings of the framework |
| `Platform/AndroidScreens.cs` | 154 | `platform/android_screens.rs` | `PlatformHelper` | 1 | |
| `Platform/AndroidSystemNavigationManager.cs` | 39 | `platform/android_system_navigation_manager.rs` | | 2 | |
| `Platform/ClipDataItemToDataTransferItemWrapper.cs` | 108 | `platform/clip_data_item_to_data_transfer_item_wrapper.rs` | | 2 | |
| `Platform/ClipDataToDataTransferWrapper.cs` | 58 | `platform/clip_data_to_data_transfer_wrapper.rs` | | 2 | |
| `Platform/ClipboardImpl.cs` | 149 | `platform/clipboard_impl.rs` | | 2 | |
| `Platform/PlatformSupport.cs` | 51 | `platform/platform_support.rs` | | 2 | |
| `Platform/Input/AndroidInputMethod.cs` | 235 | `platform/input/android_input_method.rs` | | 2 | |
| `Platform/Input/AndroidKeyboardDevice.cs` | 224 | `platform/input/android_keyboard_device.rs` | | 2 | |
| `Platform/Input/AvaloniaInputConnection.cs` | 341 | `platform/input/ferro_input_connection.rs` | (an input connection) | 2 | |
| `Platform/Input/EditCommand.cs` | 196 | `platform/input/edit_command.rs` | | 2 | |
| `Platform/Input/TextEditBuffer.cs` | 141 | `platform/input/text_edit_buffer.rs` | | 2 | |
| `Platform/SkiaPlatform/AndroidFramebuffer.cs` | 123 | `platform/skia_platform/android_framebuffer.rs` | | 1 | its declarations of the NDK are in `interop/ndk.rs` |
| `Platform/SkiaPlatform/FramebufferManager.cs` | 22 | `platform/skia_platform/framebuffer_manager.rs` | | 1 | |
| `Platform/SkiaPlatform/InvalidationAwareSurfaceView.cs` | 113 | `platform/skia_platform/invalidation_aware_surface_view.rs` | `FerroSurfaceView` | 1 | |
| `Platform/SkiaPlatform/TopLevelImpl.cs` | 415 | `platform/skia_platform/top_level_impl.rs` | `FerroSurfaceView` | 1 | stage 2: the features of stage 2, transparency beyond `None` |
| `Platform/Specific/Helpers/AndroidKeyInterop.cs` | 198 | `platform/specific/helpers/android_key_interop.rs` | | 2 | |
| `Platform/Specific/Helpers/AndroidKeyboardEventsHelper.cs` | 180 | `platform/specific/helpers/android_keyboard_events_helper.rs` | | 2 | |
| `Platform/Specific/Helpers/AndroidMotionEventsHelper.cs` | 252 | `platform/specific/helpers/android_motion_events_helper.rs` | `FerroView` | 1 | |
| `Platform/Storage/AndroidStorageItem.cs` | 671 | `platform/storage/android_storage_item.rs` | | 2 | |
| `Platform/Storage/AndroidStorageProvider.cs` | 344 | `platform/storage/android_storage_provider.rs` | | 2 | |
| `Platform/Vulkan/VulkanNativeInterop.cs` | 25 | `platform/vulkan/vulkan_native_interop.rs` | | later | with the Vulkan project |
| `Platform/Vulkan/VulkanSupport.cs` | 71 | `platform/vulkan/vulkan_support.rs` | | later | |

Files of the port without an upstream file: `interop/java.rs` (JNI, section 2.1), `interop/ndk.rs` (the NDK, section 2.2), `interop/natives.rs` (the table of native methods and the guard of section 2.3), `android_egl.rs` (the EGL library of the system and the platform graphics of section 6), `log.rs` (the log of the system, where the standard streams of a process go nowhere).

## 9. Packaging without Gradle

`scripts/android/apk.sh` builds an installable package from a cargo target with the tools of the SDK alone (`scripts/android/env.sh` names them and the pinned versions: NDK 28.2.13676358, build tools 36.0.0, platform `android-36`, Java 17):

1. `cargo build --target aarch64-linux-android` of the example or the library, with the clang of the NDK as the linker and as the C and C++ compiler through cargo's variables (`CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER`, `CC_aarch64_linux_android`, `CXX_...`, `AR_...`). The result is `lib<name>.so`; `llvm-strip --strip-unneeded` of the NDK removes what the loader does not need.
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
| the activity finishes itself and the top-level is disposed | the lifecycle of stage 1 |

The script runs the example twice: with the default options (EGL) and with `Software` (an intent extra).

**What an emulator run cannot show:** a real GPU driver (the emulator's OpenGL ES is SwiftShader or a translation to the host), more than one finger (the shell injects one pointer), a stylus or a mouse, a hardware keyboard, rotation and multi-window during a frame, other API levels than 36, other densities than the device's, 16 KB page devices, and performance.

## 11. Stages

| Stage | Content | What its emulator run proves |
|---|---|---|
| **1** | the crate, the Java layer, the bindings; platform initialisation and options; the application, the activity, the view; the top-level over the surface; the dispatcher; the choreographer timer; software and EGL rendering; pointer input; screens; insets; the smoke example; the packaging and emulator scripts | section 10, level 3 |
| **1b** | the host of the ControlCatalog (`samples/ControlCatalog.Android`) and its script | the catalog starts with the Fluent theme, shows its pages, and a picture of each of a handful is taken |
| 2a | keyboard: `AndroidKeyboardEventsHelper`, `AndroidKeyInterop`, `AndroidKeyboardDevice` | key events injected by the script (`input keyevent`, `input text`) arrive as key down, up and text |
| 2b | the input method: `AndroidInputMethod`, the input connection (`AvaloniaInputConnection`), `TextEditBuffer`, `EditCommand`, the insets animation of the input pane | text composed through the soft keyboard of the emulator; upstream's editing tests of the buffer on the host |
| 2c | lifecycle: pause and resume, surface loss and re-creation, `onNewIntent` and protocol and file activation, configuration changes (rotation, night mode), the back button (`BackPressedCallback`, `AndroidSystemNavigationManager`) | the script sends the application to the background and back, rotates, presses back |
| 2d | services: the clipboard, the platform settings, the launcher, the platform feedback | clipboard round trip through the shell; the theme follows `cmd uimode night` |
| 2e | the storage provider and items (the storage access framework), activity results, permissions | the pickers need a person; bookmarks and items against the media store of the emulator |
| 2f | the native control host | the embed page of the catalog shows a button and a web view |
| 3 | accessibility: the access helper and the node info providers | the node tree read with `uiautomator dump` |
| later | Vulkan, with the Vulkan project of the port | |

## 12. CI

A job on the Ubuntu runner builds the native library of the smoke example and its package. The runner image has an Android SDK (`ANDROID_HOME`) with build tools, platforms and several NDK versions; the job installs the pinned NDK version with `sdkmanager` when it is missing and names it to the script. No emulator runs in CI: a hosted Linux runner can run the x86-64 emulator with KVM, but the Skia binary for `x86_64-linux-android` is not verified (section 5) and a boot costs minutes of a job that would be flaky; the emulator run stays with the development machine until that is worth it.

## 13. Deviations

Recorded in `DEVIATIONS.md`, section "Android platform": the Java layer and what stays in Java; the platform classes in place of AndroidX; events copied in Java; the counted native window; the platform graphics per thread; the names (package, classes, the meta-data key, the log tag); the API level; what is not built yet and how each such place behaves.
