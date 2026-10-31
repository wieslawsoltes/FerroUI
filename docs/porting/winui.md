# The WinUI integration

The design of the port of upstream's `src/Windows/Avalonia.WinUI` (the tracked commit, `17350180`): the framework's content inside a WinUI 3 application of the Windows App SDK. The crate is `ferroui-winui` (`src/Windows/FerroUI.WinUI`), which held the reserved name until this port (`docs/release.md`, section 2.2). The differences from upstream are in `DEVIATIONS.md`, "WinUI integration"; the state is in `CRITICAL-PATH.md`, row 32.

Marks used below: **[V]** verified here (read in the sources named, or run on this machine), **[CI]** to be established by the step of the Windows job of CI (section 7.3), **[VM]** established in the owner's Windows 11 ARM64 virtual machine (section 7.4; the runs are section 7.5).

## 1. What is ported

| Upstream project | Files | Types | Members | State |
|---|---:|---:|---:|---|
| `Avalonia.WinUI` | 15 | 16 | 88 | ported file by file (this document); tracking 15/15, 16/16, 88/88 |
| `Avalonia.Win32.Interoperability` | 2 | - | - | not applicable: hosting in Windows Forms (section 8) |

Both samples of the reference that use the integration are ported as Rust programs: `samples/ControlGallery.WinUI` (the catalog in a WinUI 3 window, section 7.2) and `samples/WinUIEmbedSample` (controls of WinUI beside the content in one window, section 7.6).

## 2. What upstream does

`Avalonia.WinUI` is a library for a C# application of the Windows App SDK. Its one public class is `AvaloniaSwapChainPanel`, a `Microsoft.UI.Xaml.Controls.SwapChainPanel` that an application puts into the XAML of a window and gives a `Control` as `Content`. Everything else is internal:

| Upstream file | What it does |
|---|---|
| `AvaloniaSwapChainPanel.cs` | The panel. On `Loaded` it creates a `SwapChainGlSurface` and a `SwapChainTopLevelImpl` of the Windows backend and an `EmbeddableControlRoot` over them, starts rendering, and hands the swap chain the surface creates on the render thread to the panel (`ISwapChainPanelNative::SetSwapChain`, on the dispatcher queue of the panel). It forwards pointer, wheel, key and character events of WinUI as raw events of the framework, escapes the tab cycle of the content to the next WinUI element, maps the cursor the content asks for to an `InputCursor` of a transparent overlay grid (`ProtectedCursor` is protected), takes drags in (`DragEnter` builds a data transfer from the `DataPackageView`) and starts drags out (`StartDragAsync` of the overlay with the last pointer point), and creates its automation peer. A static constructor binds the drag source. |
| `WinUITextInputMethod.cs` | The text input method of the top-level over a `CoreTextEditContext` of the system: the text and the selection the input method sees, the composition (preedit, commit), the layout bounds in screen pixels. |
| `WinUICursorFactory.cs` | The standard cursors as `InputSystemCursor` shapes; a bitmap cursor falls back to the arrow (WinUI 3 has no cursor from an image). It is not registered: the cursors of the content are those of the Windows backend, which the panel maps back to their shapes. |
| `WinUIDragSource.cs` | The drag source of the platform: finds the panel of the top-level that started the drag and lets it start a drag of WinUI. |
| `WinUIKeyInterop.cs` | The key, physical key and symbol of a key event of WinUI through the key tables of the Windows backend (the scan code and the extended flag packed as the `lParam` of `WM_KEYDOWN`); the modifiers of a pointer event. |
| `Automation/*.cs` (10 files) | `AvaloniaSwapChainPanelAutomationPeer`, a `FrameworkElementAutomationPeer` whose children are the automation peers of the content, and `AvaloniaToXamlPeerProxy` (a partial class in eight files), an `AutomationPeer` of XAML for each peer of the framework, with nine patterns. `ControlTypeMap` maps the control types. |

What it uses of the Windows backend: `SwapChainGlSurface` (ANGLE, a composition swap chain), `SwapChainTopLevelImpl`, `KeyInterop`, `CursorImpl` and `CursorFactory`. It uses none of the composition modes of the backend (the panel presents through WinUI's own compositor), none of its input method (text input is `CoreTextEditContext`) and none of its UI Automation providers (accessibility goes through XAML's automation peers). A C# application of the sample sets the framework up with `UseWin32().UseSkia().UseHarfBuzz().SetupWithLifetime(...)` in its `OnLaunched` and runs XAML's message loop, which also pumps the message window of the backend on the same thread.

## 3. The bindings

### 3.1 Why the MicroCom generator

The reference calls WinUI through C#/WinRT, the projection of its runtime. The port has three choices for Rust:

| | MicroCom IDL (the port's generator) | the `windows` crate | a projection generated with `windows-bindgen` |
|---|---|---|---|
| WinUI 3 (`Microsoft.UI.*`) | declared in the crate's IDL | not covered (the crate projects the namespaces of the Windows SDK, not the Windows App SDK) | a build step over the metadata of the Windows App SDK, which is not in the repository |
| What the Windows backend uses | the same generator (`win_rt/winrt.idl` and three more files) | `windows-sys` only; `windows` is in the lock file for `fontique` of the Vello backend | - |
| Implementing interfaces (delegates, overrides) | `Impl` traits of the generator | the crate's `implement` macro | the same |

The integration uses the port's generator, as `win32-platform.md`, section 2.2, decided for the Windows backend: one COM model in the backends of Windows, and no second reference-counting discipline. The `windows` crate would not have removed the work anyway: it has none of the WinUI interfaces.

### 3.2 The interface definition file

`src/Windows/FerroUI.WinUI/interop/winui.idl` (2,470 lines) declares 132 interfaces and delegates, 22 enumerations and 15 structures; the build script of the crate runs the generator over it (`$OUT_DIR/winui.rs`, 40,000 lines of bindings, which compile on every host). The reference has no such file.

- **Where it comes from.** `scripts/winui/winui-idl.py` writes it from the metadata: `Microsoft.UI.Xaml.winmd` and `Microsoft.UI.winmd` of the package `Microsoft.WindowsAppSDK` 1.6.241114003, and `Windows.Foundation.UniversalApiContract.winmd` and `Windows.Foundation.FoundationContract.winmd` of `Microsoft.Windows.SDK.NET.Ref` 10.0.19041.57, both on nuget.org (both were in the package cache of this machine, restored by upstream's build). The script reads them through the IL disassembler of Mono (`ikdasm`) and emits every interface of its list with its identifier and its methods in the order of the vtable. Running it again reproduces the committed file byte for byte **[V]**.
- **Release 1.6.** Upstream builds against 1.8 (`Microsoft.WindowsAppSDK` 1.8.260209005). An interface of the Windows Runtime never changes once published, and every interface the reference calls exists in 1.6; the bindings work with any runtime from 1.6 on. The bootstrap asks for the release by its version (section 3.3), 1.6 by default.
- **Conventions.** A property is a `[propget]` and a `[propput]` method (`actual_width()`, `set_handled(..)`), an event the two methods `add_X` and `remove_X`, an overloaded method has the name of its overload attribute (`TryMoveFocusWithOptions`, `SetStorageItemsReadOnly`). A runtime class is passed as its default interface. A parameter of a type the file does not declare is `IInspectable*` or `void*`; an enumeration it does not declare is its underlying type. The ABI's `boolean` is one byte (`u8`): `win_rt.idl` of the backend maps it to four, which the backend never passes.
- **Parameterized types.** Each instance the integration uses (`IVector<AutomationPeer>`, `TypedEventHandler<CoreTextEditContext, CoreTextTextUpdatingEventArgs>`, `IAsyncOperation<StorageFile>`, 34 in all) is declared under a name of its own with the identifier the Windows Runtime derives from its signature (a version 5 UUID of SHA-1 over a namespace and the signature). The signature is in the comment above each; a test of the crate derives every identifier again with its own SHA-1 **[V]**, and checks the derivation against a value of the Windows SDK headers (`IVectorView<String>`).
- **Beside the IDL**, `ISwapChainPanelNative` (from `microsoft.ui.xaml.media.dxinterop.h`, the identifier the reference names) and `IAgileObject`.

### 3.3 The bootstrap of the Windows App SDK

A C# application of the SDK gets two initializers from the build tasks of the SDK's package: `MddBootstrapAutoInitializer` (calls `MddBootstrapInitialize2` of `Microsoft.WindowsAppRuntime.Bootstrap.dll`, which adds the framework package of the runtime to the package graph of an unpackaged process) and `UndockedRegFreeWinRT-AutoInitializer` (loads `Microsoft.WindowsAppRuntime.dll` and calls `WindowsAppRuntime_EnsureIsLoaded`, which makes the classes of the framework package activatable). A Rust program has no such build tasks. `windows_app_sdk.rs` does both: `WindowsAppSdk::initialize(&WindowsAppSdkOptions)` loads the bootstrap library at run time (`LoadLibraryW`, so nothing is linked against it and the crate builds without the SDK), calls `MddBootstrapInitialize2` with the release (`0x00010006`, no tag, any runtime version, no dialog by default where the SDK's initializer shows one), loads the runtime library and calls its export; the value calls `MddBootstrapShutdown` when dropped. A failure is a message that names what is missing.

What the machine needs: the Windows App Runtime of the release, installed (`windowsappruntimeinstall-<arch>.exe` of Microsoft), and `Microsoft.WindowsAppRuntime.Bootstrap.dll` of that release beside the executable (in the package `Microsoft.WindowsAppSDK`, `runtimes/win-<arch>/native`; the library is redistributable with an application). `scripts/winui/fetch-windows-app-sdk.ps1` puts the library beside an executable and, with `-InstallRuntime`, installs the runtime.

### 3.4 The application and its windows

The sample of the reference is C# and XAML that the XAML compiler turns into code: `App : Application` (`OnLaunched`), `MainWindow : Window` with a grid that holds the panel, a `Main` that calls `Application.Start`. `xaml_application.rs` is that code for Rust: `XamlApplication::start(on_launched)` initialises the thread as a single-threaded apartment (`RoInitialize`, the `[STAThread]` of the C# program), and calls `Application.Start` with a callback that creates the application object, composed (section 3.5) so that it overrides `OnLaunched`, which calls the closure. `XamlWindow` and `XamlGrid` are a window and a grid of WinUI.

The `App.xaml` of both samples of the reference has one thing in it: `XamlControlsResources` merged into `Application.Resources`, which gives the controls of WinUI their default styles and the theme resources (`SystemControlForegroundBaseMediumLowBrush`, ...). Its dictionaries are markup of the controls library, and XAML loads them through the type information of the application: the class the XAML compiler generates for `App` implements `IXamlMetadataProvider` and hands the types of the controls library to `XamlControlsXamlMetaDataProvider`. `XamlApplication::start` does both: the composed application answers `IXamlMetadataProvider` by forwarding to an instance of `XamlControlsXamlMetaDataProvider` (created when XAML first asks, as the generated provider is), and at the start of `OnLaunched`, before the closure of the application creates its windows, it initialises the controls library (`XamlControlsXamlMetaDataProvider.Initialize`, which the SDK has for applications without markup) and appends `XamlControlsResources` to the merged dictionaries of the resources of the application (a new dictionary is set first if it has none). The first version did this where the generated constructor of `App` calls `InitializeComponent`, inside the callback of `Application.Start` right after the composed application was created; on the CI runner that failed with `E_UNEXPECTED` (run 38787520732; the panic did not say which of the calls). Each call of the sequence now prints a `[step] application:` line when `FERROUI_WINUI_TRACE` is set (both smoke runs set it), and its error names the call. The markup of `WinUIEmbedSample` uses those theme resources; it is loaded only if they resolve (section 7.6). The interfaces are in the IDL: `IResourceDictionary` and its factory, `IXamlMetadataProvider` (with the structures `TypeName` and `XmlnsDefinition`), `IXamlReaderStatics`, and those the second sample uses (`IGridStatics`, `ITextBlock`, `ITextBox`, `IButtonBase`, `IRangeBase`, `ISelector`, their events, two more parameterized instances: 36 in all).

### 3.5 What the projection of the reference's runtime does, here

| Projection feature | Here |
|---|---|
| Deriving from a composable class (`class AvaloniaSwapChainPanel : SwapChainPanel`, a peer derived from `FrameworkElementAutomationPeer`, `App : Application`) | `interop/composable.rs`: the derived class is the outer object of a composition. It is a COM object of the MicroCom runtime made with the overridable interfaces it implements (`IUIElementOverrides`, `IAutomationPeerOverrides`, `IApplicationOverrides`); the composable factory of the base class creates the inner object for it; `ferroui_microcom::set_inner` (added to the runtime for this) makes the outer object ask the inner object for every other interface and keeps it alive. A method that is not overridden calls the inner object's implementation; a protected member (`ProtectedCursor`, `ProviderFromPeer`) is an interface of the inner object. |
| Delegates (`+=` of a lambda) | `interop/event_handlers.rs`: a COM object of the delegate's interface over a closure, run under the guard of the backend (section 6). A delegate passed as an `object` (`UIElement.AddHandler`) is boxed as an `IReference<T>` of it, as C#/WinRT and C++/WinRT box it. |
| `await` of an asynchronous operation | `interop/async_operation.rs`: a future whose completion handler (agile) wakes the task; the tasks run on the dispatcher of the UI thread (`start_local`), where the await of the reference resumes too. |
| `List<T>` passed as `IList<T>` or `IEnumerable<T>` | `interop/collections.rs`: `IVector<AutomationPeer>` (with its view and iterator) and `IIterable<IStorageItem>`. |
| `string` | string handles of the backend (`HStringInterop`), created and deleted around a call. |
| `ConditionalWeakTable` of the proxies | a table of the UI thread from peer to proxy that does not count the proxy (section 5). |

## 4. The panel

`FerroSwapChainPanel` (`ferro_swap_chain_panel.rs`) has the members of the reference. Ownership differs from the reference's, where the panel is the XAML element and lives as long as anything references it:

- The application owns the panel (`Rc<FerroSwapChainPanel>`, from `FerroSwapChainPanel::new()`) and adds its element (`as_ui_element()`) to the tree of a window. The panel holds its element; the element (the outer object) holds the panel weakly, and every handler it registers holds the panel weakly. Counted references would make a cycle (element, handler, panel, element) that only a garbage collector breaks.
- An element whose panel was dropped is a plain swap chain panel: its overrides call the base implementation and its handlers do nothing. Dropping the panel does what `Unloaded` does (stops rendering, disposes the root and the swap chain).

The life cycle is the reference's: `Loaded` creates the surface, the top-level and the root and starts rendering; `Unloaded` stops and disposes them; the size and the composition scale are read on the UI thread and handed to the render thread through atomics (the reference reads two fields from that thread).

### 4.1 File table

| Upstream file | Lines | FerroUI file | State and notes |
|---|---:|---|---|
| `AvaloniaSwapChainPanel.cs` | 915 | `ferro_swap_chain_panel.rs` | ported; the decisions of the pointer events and the text of a character are functions of values, tested on every host |
| `WinUICursorFactory.cs` | 91 | `win_ui_cursor_factory.rs` | ported; the shape of each standard cursor tested on every host |
| `WinUIDragSource.cs` | 42 | `win_ui_drag_source.rs` | ported; the drag runs as a future on the dispatcher |
| `WinUIKeyInterop.cs` | 62 | `win_ui_key_interop.rs` | ported; the key data and the modifiers tested on every host |
| `WinUITextInputMethod.cs` | 384 | `win_ui_text_input_method.rs` | ported; the text is kept as UTF-16, as the ranges of the edit context count; the operations on it tested on every host. Clients of the exact-edit connection (RichEditBox) take the additions of `win_ui_text_input_method_exact.rs`, `win_ui_text_input_method_layout.rs` and the `core_text_*.rs` files (`winui-rich-text.md`, R9c; `DEVIATIONS.md`, "WinUI integration"); every other client keeps the adapter of the reference, which `win_ui_text_input_method_core_text_tests.rs` checks with test doubles of the edit context |
| `Automation/AvaloniaSwapChainPanelAutomationPeer.cs` | 143 | `automation/ferro_swap_chain_panel_automation_peer.rs` | ported (composed `FrameworkElementAutomationPeer`) |
| `Automation/AvaloniaToXamlPeerProxy.cs` | 141 | `automation/ferro_to_xaml_peer_proxy.rs` | ported (composed `AutomationPeer`) |
| `Automation/AvaloniaToXamlPeerProxy.*.cs` (7) | 178 | `automation/ferro_to_xaml_peer_proxy_*.rs` | ported, one file per pattern as the partial class has; the state mappings tested on every host |
| `Automation/ControlTypeMap.cs` | 52 | `automation/control_type_map.rs` | ported, tested on every host |
| `Avalonia.WinUI.csproj` | 30 | `Cargo.toml` | not applicable: the project file |

### 4.2 Where the port differs, and why

- **A surrogate pair of `CharacterReceived`.** The reference sends each UTF-16 unit as a string, a lone surrogate among them; a Rust string holds no lone surrogate. The first half of a pair is kept and the text is sent with the second.
- **Intermediate points.** The reference fills one static list for every move event; the lazy value of the base library owns its points, so each event has a list of its own.
- **The swap chain on its way to the UI thread** is counted (the reference hands the pointer over uncounted and relies on the surface keeping it).
- **Errors.** A call of WinUI that fails is an `HResult`, not an exception: what the reference does with an exception (logs it, or lets it reach the system) is done with the result; where the reference has no handler, the result is ignored as the default value of the call would be, and a failure the reference lets escape (`Marshal.ThrowExceptionForHR` of `SetSwapChain`) is a panic, which the guard keeps (section 6).

## 5. Automation

The panel's element creates its peer through the override of `OnCreateAutomationPeer` on the outer object. The peer of the panel and the proxies are composed objects whose `IAutomationPeerOverrides` is the reference's: the class name, the control type, the children (the base's children and the proxy of the embedded root), the focused element and the element at a point, and for a proxy every member of its peer with the patterns `GetPatternCore` answers with the proxy itself. A pattern a provider of the framework refuses with `ElementNotEnabledException` answers `UIA_E_ELEMENTNOTENABLED`, as the exception of the reference reaches the client.

The reference keeps a proxy per peer in a `ConditionalWeakTable`, which lets the proxy live while its peer does. A counted proxy that holds its peer cannot be held by a table that lets the peer go; here the table does not count the proxy, a proxy leaves the table when it is destroyed, and while XAML holds a proxy, the same peer gets the same proxy. The proxies hold the peer of the panel weakly (it holds the panel weakly too).

## 6. Threads

Everything of the integration runs on the thread of the XAML application, which is the UI thread of the framework: `Application.Start` runs XAML's message loop, which dispatches the messages of the message window of the Windows backend too, so the dispatcher, its timers and its tasks work as they do in a window of the backend.

| Thread | What of the integration runs there |
|---|---|
| UI thread | every handler of WinUI and of the edit context, the overrides of the composed objects, the tasks of drag and drop |
| render thread (the render timer of the backend's composition mode) | the size and scaling functions of the surface, and the callback that receives the swap chain, which posts the work to the dispatcher queue of the panel (an agile `DispatcherQueueHandler`) |
| any thread | the completion handlers of asynchronous operations (agile; they only record the status and wake the task) |

**Panics.** The system calls a handler through its own frames, which a panic must not cross. Every handler runs under the guard of the Windows backend (`ferroui_win32::wnd_proc_guard`, public now): a panic is kept, the handler returns, and a hook the application sets ends XAML's message loop (`Application.Exit`); `XamlApplication::start` raises the panic again once `Application.Start` has returned. The reference's runtime carries an exception through the frames of the system instead.

## 7. How it is tested

### 7.1 On any host

- `cargo test -p ferroui-winui`: 23 tests **[V]** of what is logic (two of them, of the lines that name a thread and its start address at the end of the process, section 7.7, since 2026-10-16): the identifiers of the parameterized instances against their signatures, a few identifiers against the metadata and the headers, the layout of the structures of the ABI, the key data and the modifiers, the cursor shapes, the control types and the states of the patterns, the raw events of a press and a release, the text of a character and a surrogate pair, the operations on the UTF-16 text of the input method, the boolean of the ABI, the release of the bootstrap.
- `cargo test -p ferroui-microcom`: the composed object (a query answered by the inner object, the inner object released with the outer one) **[V]**.
- `cargo check -p ferroui-winui --target x86_64-pc-windows-msvc` and `--target aarch64-pc-windows-msvc` **[V]**, with `--tests` for the first: every call matches its binding. The workspace dependency on `ferroui-win32` is without ANGLE now, which makes this check possible (ANGLE compiles C++ for the target).
- The sample cannot be checked for a Windows target on a Mac (the catalog depends on Skia, and HarfBuzz and ANGLE compile C++); its own sources were checked against stand-ins of those three crates **[V]**.

### 7.2 The smoke run of the sample

`samples/ControlGallery.WinUI` (`control-gallery-winui`) is the sample of the reference: the bootstrap, `Application.Start`, `App::on_launched` (the framework set up with the catalog's application, the Windows backend, Skia and HarfBuzz, and a single view lifetime), `MainWindow` (a window of WinUI with a grid and the panel, whose content is the main view). With `--smoke` it prints each check and exits with 0 only if all passed:

1. the Windows App SDK is bootstrapped (the release, the runtime found);
2. the framework is set up; the catalog set the main view of the lifetime;
3. the main window with the panel is created and activated;
4. the panel loads (`Loaded` created the root), and the content has a client size;
5. the swap chain is set on the panel (`ISwapChainPanelNative::SetSwapChain` succeeded on the UI thread after the first render target created it);
6. 30 frames of the content are rendered (animation frames of its top-level);
7. the automation tree XAML builds of the panel (`FrameworkElementAutomationPeer.CreatePeerForElement` of its element): the class name `FerroSwapChainPanel` and the control type `Pane` (overrides of the composed peer), at least one child (the proxy of the embedded root), which has the content's peers as children;
8. the window closes, and the application ends.

A check that has not passed after 60 seconds fails the run. Each call of the system in step 7 and the close of step 8 is announced by a `[step]` line, and the automation objects of the integration print a `[trace]` line on the error stream for what XAML asks of them (`FERROUI_WINUI_TRACE`, which the smoke run sets): a run that ends in the frames of the system names the call it was in. When the window is closed, the application lets go of it (and of its panel) inside the handler of `Closed`, while XAML still runs, and then ends the loop. What it does not check: pixels of the swap chain (a capture of a window does not reliably hold a composition swap chain), input (it needs a person or injected input), the input method, drag and drop and the cursor.

### 7.3 CI

The `windows` job has a step after the pictures of the catalog: the tests of the crate on the target, the build of both samples, `fetch-windows-app-sdk.ps1 -InstallRuntime` (the runtime 1.6 installed on the runner, the bootstrap library beside the executables) and the smoke run of each sample through `scripts/winui/smoke-run.ps1`, which starts the executable with its output in files, waits at most 300 seconds, prints the output, and ends a process that has not ended with the exit code 124. Before it ends such a process it lists its threads (the state, the reason of a wait and the module and offset where each started) and prints the stacks of all of them through the console debugger of the image (`cdb -pv`, a non-invasive attach that also reads a process inside `ExitProcess`; the symbols of the executable beside it, those of the system from Microsoft's symbol server), when the image has the debugger.

**What the first runs showed** (the step was informative then, `continue-on-error`; runs 38741696373 of pull request 197 and 38769434562 of the third batch, `windows-latest`) **[CI]**: the image installs the Windows App Runtime 1.6 (the installer ends with 0, the x64 and x86 framework packages are installed) and runs the sample in its session; every check of section 7.2 passed (the client size 752 by 480, the swap chain set 1563 and 1687 ms after activation, 35 and 36 frames, the automation tree of the panel and of the content), and the window closed. Then the process did not end: nothing more was printed, and the step ran into its limit of 45 minutes. `timeout 300` of the shell of Git for Windows did not end the process either (it signals a program of the system instead of terminating it), which is why the step now runs the samples through `smoke-run.ps1`, which terminates.

**The likely cause** (not proven: the logs end with the close of the window). It is the hazard of the end of a process on Windows that `win32-platform.md`, section 11.3, recorded for the Windows backend: when `main` returns, the system ends every other thread wherever it is, and then the destructors of the thread-local values of the main thread run; a destructor that takes a lock an ended thread held never returns. The sample kept the main view of the catalog (the whole tree of controls) in the thread-local lifetime until then, while the thread of the render timer of the backend's composition mode, which nothing paces once the last surface is gone, is inside the locks of the render loop most of the time; the drop of the tree of controls reaches the compositor and the dispatcher, whose locks that thread shares (the lock of the pending batch of the compositor, the queue of the dispatcher it posts to). In the virtual machine the race went the other way twice (section 7.5), as it did for the backend's example before its fix. The fix: the application lets go of the main view, and of the window, in the handler of `Closed`, while the threads of the framework run (`App::shutdown` of both samples; `DEVIATIONS.md`); what is left in the thread-local values of the main thread at the end is what the desktop catalog leaves, whose runs end on the same runner. **To make the next run name the place if it is something else**, the smoke run of both samples prints a `[step]` line at each stage of the end (`Closed`, `Application.Exit`, `Application.Start returned`, `MddBootstrapShutdown`, `main returns`), and a watchdog thread started when the window is closed fails the run after 60 seconds with the last step and ends the process (code 97). A process that prints `main returns` and stays is the destructors of the main thread (the watchdog is ended by the system then; `smoke-run.ps1` ends the process). **The step is required** from this change on; its first run as a required step is the check of the fix.

**The first run on a runner [CI]** (run 38769434562, 2026-10-15, the integration branch of the third batch, `windows-latest`, x64): the image installs the runtime (`windowsappruntimeinstall-x64.exe --quiet`, exit code 0; the x64 and x86 packages of the runtime 1.6 installed), and the smoke run passed every check of section 7.2 up to and including the close of the window: the bootstrap, the framework, the window with the panel, `Loaded` (752 by 480), the swap chain set 1687 ms after activation, 35 frames, the peer of the panel with its class name, control type and one child, the peer of the content with one child, and "the window closes". The process then did not end, and the step was stopped by its timeout of 45 minutes. In the virtual machine the same program ends with 0 after the close (section 7.5). The release of the main view in `Closed` was not enough: the end of the process is section 7.7.

### 7.4 The virtual machine

`scripts/windows/vm-smoke.ps1 -WinUI` runs the tests of the crate, fetches the bootstrap library beside the samples, checks that the Windows App Runtime 1.6 is installed (it installs nothing; `-InstallWindowsAppRuntime` installs it, with the owner's authorisation), builds both samples and runs the smoke mode of each in the session of the interactive user (`winui-smoke-control-gallery-winui` and `winui-smoke-winui-embed-sample` in the logs). Steps, from the host:

```
scripts/windows/vm-sync.sh <commit>
# in the machine, from the drive the share is mapped to:
powershell -NoProfile -ExecutionPolicy Bypass -File <S>:\ferroui-vm-windows\src-<commit>\scripts\windows\vm-smoke.ps1 -TargetDir C:\ferroui-target -Jobs 3 -InteractiveUser <user> -WinUI -InstallWindowsAppRuntime -Report <S>:\ferroui-vm-windows\vm-smoke-report.txt
```

The runtime the machine needs: the Windows App Runtime 1.6 for ARM64, from Microsoft's installer `windowsappruntimeinstall-arm64.exe` of release 1.6.241114003 (`https://aka.ms/windowsappsdk/1.6/1.6.241114003/windowsappruntimeinstall-arm64.exe`), which `-InstallWindowsAppRuntime` downloads and runs with `--quiet`. Once installed, later runs omit the switch.

### 7.5 What the runs in the virtual machine showed (2026-10-15)

The first run in the virtual machine (commit `629520ed`; the Windows App Runtime 1.6 installed by `-InstallWindowsAppRuntime`, installer exit code 0) **[VM]**: the 21 tests passed on the target; the sample built (508 s); the smoke run passed steps 1 to 6 and the first two checks of step 7: the bootstrap, `Application.Start` with the composed application, the window with the composed panel, `Loaded` (client size 1334 by 666), the swap chain set on the panel 3.2 s after activation, 46 frames, and the peer of the panel created through the override of `OnCreateAutomationPeer` with the class name and the control type of the overrides of the composed peer. It then ended with an access violation (`0xC0000005`) in the next call, `GetChildren` of the peer of the panel.

The cause: the creation of a proxy (`FerroToXamlPeerProxy::new`) read the value of the proxy from the pointer of its `IAutomationPeer`, which is an interface of the inner object (an object of XAML), not of the outer object the runtime of the port made; it wrote the pointer of the proxy into the memory of XAML's object, and left the proxy without its own pointer (`SetParent` of each child then dereferenced null). The value is now read from the outer object's own first interface before the query. The other three places that read the value of a composed object read it from that interface already.

The second run (commit `c22d5163`, the same machine, the runtime installed by the first) **[VM]**: `RESULT: passed`. The 21 tests passed on the target (255 s), the bootstrap library was fetched, the sample built (578 s), and the smoke run exited with 0 and passed every check of section 7.2:

- the bootstrap of the Windows App SDK 1.6, the framework set up, the main view set, the window with the panel created and activated;
- `Loaded` with the client size 1334 by 666; the swap chain set on the panel 2044 ms after activation; 40 frames rendered;
- the peer of the panel: the class name `FerroSwapChainPanel` and the control type `Pane` of the overrides of the composed peer, one child (the proxy of the embedded root, created, given its parent with `SetParent` and appended to the list of the base, as the `[trace]` lines show); the peer of the content (`EmbeddableControlRoot`) with one child, whose class name is `Panel`; the peer of the content is not a control element, as the reference collapses it;
- the window closes, the application lets go of it inside `Closed` (the `[trace]` lines show the two proxies destroyed), and the process ends with 0.

The same run passed the steps of the Windows backend that `vm-smoke.ps1` runs before it (the builds with and without ANGLE, the tests, the smoke runs in software, through ANGLE, WGL and ANGLE with DirectComposition).

### 7.6 WinUIEmbedSample

`samples/WinUIEmbedSample` (`winui-embed-sample`) is the second sample of the reference: one window of WinUI with two columns, controls of WinUI on the left (a button that counts, a slider, a text box, a toggle switch, a drag source with a text, a combo box that sets the theme of both sides, and the diagnostics of the automation peer of the panel: the peer and its children probed in the constructor, on `Loading` and `Loaded` and by a button, a walk of the whole tree on the UI thread and one on a thread of its own, as a client of UI Automation calls), and the panel on the right, whose content is `EmbeddedView`: the same kinds of controls of the framework, with the keys pressed, the clipboard, a drag source and a drop target, the cursors, hover and an ink canvas that draws the intermediate points of the pointer.

- **The content is a library**, built and tested on every host: `FerroApp.xaml` (the application with the Fluent theme) and `EmbeddedView.xaml`, converted by `scripts/convert_catalog_xaml.py` (names only), compiled by the build of the crate, with their classes (`ferro_app.rs`, `embedded_view.rs`). 14 tests **[V]**: the generated tests of the two documents (loaded at run time, the classes constructed, the compiled markup against the loader), and the view in a window of the headless shell: it renders, the button counts clicks, the slider shows its value, hover changes the state, the ink canvas draws a dot for each point and intermediate point of the pointer and is cleared, and the texts of the key modifiers and of the rounding are those of the reference.
- **The application** (`main.rs`, `app.rs`, `main_window.rs`) is Windows only. The markup of the reference's `MainWindow.xaml` is loaded at run time (`XamlReader.Load`): the grid of the window, with the handlers of the markup added in code to the elements of their names and the panel added to the second column in code (`DEVIATIONS.md`). Checked for `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc` against stand-ins of Skia and HarfBuzz (and of the catalog for the other sample), the rest the real crates **[V]**.
- **`--smoke`** prints each check and exits with 0 only if all passed: the Windows App SDK bootstrapped; the framework set up; the main window created (the markup of the WinUI controls loaded, which needs the theme resources of `XamlControlsResources`, section 3.4) and activated; the embedded view is the main view; the constructor probed the peer; the panel loaded, the swap chain set, 30 frames; `Loading` and `Loaded` probed the peer (on `Loaded` the class `FerroSwapChainPanel` with one child, the proxy of the content); the slider of WinUI shows its value through `ValueChanged`; the combo box sets the dark theme on both sides (`RequestedTheme` of the content of the window and the theme variant of the application of the framework) and the system theme back; the probe of the peer; the deep walk reaches the peers of the content; the walk on a thread of its own comes back to the UI thread (its text is printed: XAML is expected to refuse the calls from that thread with `RPC_E_WRONG_THREAD`, as it throws in the reference); the slider of the embedded view shows its value; the window closes and the application ends, with the steps and the watchdog of section 7.3. The lines of the diagnostics are printed too (`[diag]`).

**The first run in the virtual machine** (commit `2e1b138f`, `vm-smoke.ps1 -WinUI -SkipTests`, Windows 11 ARM64, the Windows App Runtime 1.6 installed before) **[VM]**: `RESULT: passed`. The 21 tests of the crate passed on the target, both samples built (598 s), and both smoke runs exited with 0 in the session of the user:

- **The start of the application**, both samples, in the order of the `[step] application:` lines: `Application.Start`, the composed application created, then in `OnLaunched` `XamlControlsXamlMetaDataProvider.Initialize`, `Application.Resources` (the application has a dictionary already, so none is created), `new XamlControlsResources()`, during which XAML asks the application for the type information and the provider of the controls library is activated (the `IXamlMetadataProvider` of the composed application is used), `MergedDictionaries`, `Append`, merged. The markup of the WinUI controls of the second sample, with its `ThemeResource` references to resources of `XamlControlsResources`, loaded.
- **ControlGallery.WinUI**: every check of section 7.2 (the client size 1334 by 666, the swap chain set 2669 ms after activation, 41 frames, the automation tree of the panel and of the content).
- **WinUIEmbedSample**: every check of this section. The constructor and `Loading` probed the peer with no child (the root is created on `Loaded`), `Loaded` with one, `EmbeddableControlRoot` of type `Pane` with one child: the answer to the question the reference's comment asks (whether `GetChildren` works before the root is prepared: it does, with no child). The client size 667 by 666 (the right column), the swap chain 1790 ms after activation, 35 frames; the slider of WinUI showed "Slider: 60" through `ValueChanged`; the combo box set the dark theme on both sides (`RequestedTheme` 2 and the framework's dark variant) and the system theme back; the deep walk reached the peers of the content; the slider of the embedded view showed "Slider: 40". **The walk on a thread of its own** came back with `THREW: HRESULT 0x8001010E` (`RPC_E_WRONG_THREAD`): XAML refuses the call of a peer from another thread before any code of the integration runs, as it throws in the reference.
- **The end**, both samples: `Closed` (the window and the main view let go of), `Application.Exit`, `Application.Start returned`, `MddBootstrapShutdown`, `main returns`, and the process ended with 0.

A detail of the diagnostics: the text box of WinUI keeps a line break as a carriage return, so the lines the sample appends with a line feed are one line when the text is read back (the deep walk counted as one line, the line of `Loaded` printed with the lines before it); the checks look for their text anywhere in it and passed.

### 7.7 The end of the process (2026-10-16)

**What the run showed [CI]** (run 38832399344, job 116753884194, pull request 205, `windows-latest`, x64): ControlGallery.WinUI passed every check of section 7.2 and printed every step of the end, in order: `Window.Close`, `Closed` (the window and the main view let go of), `Application.Exit`, "the window closes", `Application.Start returned`, `MddBootstrapShutdown`, `main returns`. Then the process did not end. After 300 seconds `smoke-run.ps1` found one thread in it, in a wait with the reason `Unknown`, and ended it (124). In run 38787520732 (pull request 200) the same step passed, and in the virtual machine both samples end with 0 (section 7.6): a race, as in section 11.3 of `win32-platform.md`.

**What that tells.** One thread left after `main` has returned is the main thread inside `ExitProcess`: the system has ended every other thread (those of XAML, the thread of the compositor connection, the watchdog of the smoke run, which printed nothing) and runs the callbacks of the end of the process on the main thread: the thread-local destructors of that thread (the TLS callback of the standard library), the destructors of the per-thread state of the C++ libraries linked into the executable (ANGLE) and the `DLL_PROCESS_DETACH` of the libraries. `System.Diagnostics.ThreadWaitReason` names the reasons of the older kernels only; a newer one is `Unknown`, and the newer reason a waiting thread has without a handle is `WrAlertByThreadId`, the wait of a slim reader/writer lock and of `WaitOnAddress`, on which the locks of the Rust standard library and `std::mutex` of the C++ library are built (an inference from the reason; the log has no stack). A critical section is not it: the system ends a process whose critical section an ended thread held during the end (section 11.3 of `win32-platform.md`). So it is the hazard of that section: a callback on the main thread inside `ExitProcess` takes a lock an ended thread held, and nothing ever releases it. What was still alive on the main thread at that point, after the release in `Closed` (section 7.3): the framework of the UI thread (the dispatcher, the services, the platform, the handles of the compositor, the cursors of the cursor factory, the text input method, the drag source of the integration, the last of them objects of the Windows App SDK), and the state ANGLE keeps for the thread that created its display. Which lock it was in this run is not recorded.

**What the reference does.** Its generated `Main` returns from `Application.Start`, and the .NET runtime ends the process: it raises `ProcessExit`, finalizes nothing (a .NET process runs no finalizer at its end: statics and thread statics are never destroyed, and the wrappers of the objects of the Windows Runtime are never released), and the host returns to the C runtime, whose `exit` calls `ExitProcess` on the main thread. Inside `ExitProcess` only the detach callbacks of the native libraries run; nothing of the program's values is destroyed while the system ends the threads.

**The fix.** `ferroui_winui::end_process(code)` (`process_end.rs`) ends the process the same way: it flushes the standard output and error streams (and the streams of the C runtime), starts a thread of its own (`process end`) that holds none of the values of the UI thread and calls `exit(code)` of the C runtime (the handlers registered with `atexit`, the destructors of the static objects of the C++ libraries, then `ExitProcess`, whose detach callbacks run on that thread), and the UI thread waits in `park` holding nothing until the system ends it. The thread-local values of the UI thread are never destroyed, as the reference's are never finalized; the detach callbacks of the libraries still run, as they do for the reference's process. Both samples call it with their exit code after `MddBootstrapShutdown` (`DEVIATIONS.md`). Why not one of the other ways:

- releasing more before the end (the services, the dispatcher, the objects of the Windows App SDK) and stopping the thread of the compositor connection would have to empty every thread-local value of the UI thread, of every crate, and stop every thread that shares a lock with one of them, the threads of XAML among them, which the program cannot stop. Any value left over is the same race. The reference stops none of them either (the loop of its compositor connection ends with the process, `DEVIATIONS.md`);
- `TerminateProcess` after the flush ends the process with its code and runs nothing at all, the detach callbacks of the libraries included, which the reference's process does run: a further deviation that nothing here needs;
- the application on a thread of its own (the experiment of section 11.3 of `win32-platform.md`) destroys the values of the UI thread while the other threads run: destroying them is what the reference does not do, and the release of the objects of XAML after `Application.Start` has returned is outside what the integration is verified for.

The release of the main view and of the window in `Closed` (section 7.3) stays: it is the application letting go of its window while XAML runs. The examples of the Windows backend and the desktop entry point end through `main` as before (their runs end; section 11.3 of `win32-platform.md`).

**The diagnostics**, printed when `FERROUI_WINUI_TRACE` is set (the smoke runs set it), `[exit]` lines on the standard output after the step `main ends the process`:

1. `main hands the end of the process (code N) to a thread of its own`, with the threads alive then, each with its description and the module and offset where it started (Toolhelp32 and `ThreadQuerySetWin32StartAddress`);
2. `exit of the C runtime: the handlers registered with atexit run`, with the threads alive then (they all still run: `ExitProcess` comes after);
3. `ExitProcess: the other threads are ended, the detach callbacks run`, written without the lock of the standard library's `stdout` by the destructor of a value of the ending thread, which the standard library runs from the TLS callback of the executable inside `ExitProcess` (among the detach callbacks of the libraries);
4. from a watchdog thread, every 15 seconds while the process still runs before `ExitProcess`: `the process still runs N s after main handed its end over`, with the threads, each with where it started.

How a run that does not end reads: lines of the watchdog after line 1 or 2 name the threads of a stop before `ExitProcess` (in the handlers or the static destructors of the C runtime), and the watchdog of the smoke run ends the process after 60 seconds (code 97). Line 2 and then nothing (no line of the watchdog), with or without line 3, is a stop inside `ExitProcess` (the system has ended the watchdogs): in a detach callback of a library, since the values of the program are no longer destroyed there; `smoke-run.ps1` then prints the stacks of the remaining threads through `cdb` (section 7.3) before it ends the process, which names the library and the lock. The check of the fix is the required step of CI, run several times.

## 8. `Avalonia.Win32.Interoperability`: not applicable

The project hosts the content in Windows Forms: `WinFormsAvaloniaControlHost`, a `System.Windows.Forms.Control` that creates an `EmbeddableControlRoot` (whose top-level is the embedded window of the Windows backend) as its child window and forwards focus, size and dialog characters, and `WinFormsAvaloniaMessageFilter`, an `IMessageFilter` of the Windows Forms message loop that dispatches the key messages of the framework's own windows before Windows Forms takes them. Windows Forms is a toolkit of .NET; a Rust program has no Windows Forms to host in. What stands for it in Rust:

- a window of another toolkit hosts the content through the embedded window of the Windows backend (`EmbeddedWindowImpl`, the same class the reference's control host creates), as the native control host of the framework does the other way round;
- a WinUI 3 application hosts it through this crate.

The tracking records both files as not applicable with this reason (`docs/porting/data/path-overrides.toml`), the project is in scope with no crate, and the sample that uses it (`samples/interop/WindowsInteropTest`) is not applicable for the same reason (`docs/porting/data/samples.toml`).

## 9. What is not verified

The runs in the virtual machine (section 7.5) established the bootstrap, `Application.Start` with the composed application, the composed panel with its override of `OnCreateAutomationPeer`, `Loaded`, the swap chain set on the panel and frames rendered, the composed peer of the panel with its overrides, the proxies of the content's peers with their parents and children, and the close of the window with the release of the panel and of the proxies. Not verified yet, in the order a run would show them:

- that the protected interfaces are answered by the inner objects (the cursor of the overlay); the picture the swap chain presents, with the inverse scale transform;
- input: pointer, wheel, keys, characters, the tab boundary, the cursor of the overlay;
- the input method over `CoreTextEditContext` (`GetForCurrentView` in a desktop application of WinUI, as the reference calls it);
- drag and drop in both directions, and the agile completion handlers of the operations;
- the automation tree with a client of UI Automation, and a screen reader;
- the release of the composed objects (no leak, no use after release) when a window closes;
- that the process of a sample ends after its window has closed on the CI runner, run after run (section 7.7: the end through `end_process` and the diagnostics that name the place are in; the check is several green runs of the required step; in the virtual machine both end, section 7.6);
- `WinUIEmbedSample` by hand (its smoke run passed in the virtual machine, section 7.6): the controls of WinUI and of the framework side by side, focus moving between them, drag and drop between the two sides and with other applications, the clipboard, the cursors, hover and ink.
