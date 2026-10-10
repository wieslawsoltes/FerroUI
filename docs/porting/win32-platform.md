# The Windows platform backend

The design of the port of upstream's `src/Windows/Avalonia.Win32` (the tracked commit, `17350180`), what its first stage built, and the order of what is left. Status of the subsystem is in `CRITICAL-PATH.md`, row 26; the hand-over for the next session is in `CONTINUATION.md`; the differences from upstream are in `DEVIATIONS.md`, "Windows platform backend".

Marks used below: **[V]** verified here (read in the sources named, or measured on this machine), **[CI]** established by the Windows jobs of the CI workflow, **[VM]** established in the owner's Windows 11 ARM64 virtual machine (section 10.4). The backend runs in those two places; it is written on macOS.

## 1. What is ported

| Upstream project | Files | Types | Members | State |
|---|---:|---:|---:|---|
| `Avalonia.Win32` | 95 | 276 | 2605 | in scope since 2026-10-10; stage 1 built and run, stage 2a in part, stage 2b for ANGLE (this document) |
| `Avalonia.Win32.Automation` | 40 | 64 | 598 | out of scope; the UI Automation providers, after stage 2 |
| `Avalonia.Win32.Interoperability` | 2 | - | - | out of scope; hosting in WPF and Windows Forms, which has no counterpart in Rust |
| `Avalonia.WinUI` | 15 | - | - | out of scope; a host inside a WinUI application |

`Avalonia.Win32` is one of the two windowing backends upstream has for the desktop on Windows and the only one an application uses: windows and popups over `HWND`, the message loop as the dispatcher, input from window messages, the clipboard and drag and drop over OLE, GPU rendering through ANGLE (OpenGL ES on Direct3D 11), WGL or Vulkan, and presentation through the redirection surface of the window, DirectComposition, Windows.UI.Composition or a DXGI swap chain.

## 2. The bindings of the Windows API

Upstream declares the API by hand in `Interop/UnmanagedMethods.cs` (2,800 lines of `DllImport`, structures and enumerations), adds generated bindings for a handful of calls (`NativeMethods.txt`, the list of the CsWin32 generator), and describes the COM interfaces it uses in four interface definition files that its MicroCom generator turns into proxies and vtables (`Win32Com/win32.idl`, `DComposition/dcomp.idl`, `DirectX/directx.idl`, `WinRT/winrt.idl`).

### 2.1 Functions and structures: `windows-sys`

| | `windows-sys` | `windows` |
|---|---|---|
| Version on crates.io **[V]** (`cargo info`, 2026-10-10) | 0.61.2, rust-version 1.71 | 0.62.2, rust-version 1.82 |
| What it is | declarations: `extern` functions, `#[repr(C)]` structures, constants, handle types as raw pointers | the same API behind wrapper types, plus a COM model of its own (`windows-core`: interface types, `implement` and `interface` macros, `HSTRING`, `Result`) |
| Features | 246, one per API namespace | 672 |
| Size of the sources in the registry **[V]** | 18 MB | 110 MB |
| In `Cargo.lock` before this work **[V]** | yes (0.61.2) | yes (0.61.3 and 0.62.2: `fontique` of the Vello backend enumerates fonts through DirectWrite with it) |
| Linking | `windows-link`: `raw-dylib` imports, no import libraries | the same |

The backend uses **`windows-sys`**, with one feature per namespace it calls (the manifest of the crate lists them: thirteen in stage 1).

- It is what upstream's file is: declarations. The port of `UnmanagedMethods.cs` is then a file of the same shape (section 3.2) and not a second layer of wrapper types over another crate's wrapper types.
- It adds no COM runtime. The port already has one, the MicroCom crate with its generator, which the macOS backend uses for its native interface; upstream's Windows backend is written against the same thing (section 2.2). Two COM models in one backend would mean two reference-counting disciplines and two ways to implement an interface.
- Build cost: the crate compiles declarations only. The backend with its thirteen features checks for the Windows target in seconds on top of the base and controls crates **[V]**.
- No import libraries are needed, so `cargo check --target x86_64-pc-windows-msvc` works on a Mac **[V]**: that is the first of the three ways this backend is verified (section 10).

A function that upstream probes for at run time, because older versions of Windows lack it, is not imported statically (a missing import stops the process at load): it is looked up with `GetProcAddress` as upstream does (`SetProcessDpiAwarenessContext`, `SetProcessDpiAwareness`, `GetDpiForMonitor`, `AdjustWindowRectExForDpi`). `RtlGetVersion` of ntdll is declared in the interop file itself, as a `raw-dylib` import.

### 2.2 COM: the MicroCom crate

| Use | Interfaces | Direction | File | Stage |
|---|---|---|---|---|
| Clipboard, drag and drop | `IDataObject`, `IEnumFORMATETC`, `IDropSource`, `IDropTarget`, `IStream`, `IDataObjectAsyncCapability` | implemented and consumed | `win32.idl` | 2d |
| File dialogs | `IFileDialog`, `IFileOpenDialog`, `IShellItem`, `IShellItemArray` | consumed | `win32.idl` | 2e |
| Taskbar | `ITaskbarList2/3` | consumed | hand-written vtable in `TaskBarList.cs` | 2e |
| Input pane | `IFrameworkInputPane`, `IFrameworkInputPaneHandler` | both | `win32.idl` | 2f |
| Direct3D 11, DXGI | devices, textures, swap chains, outputs | consumed | `directx.idl` | 2b |
| DirectComposition | device, target, visual, surface | consumed | `dcomp.idl` | 2c |
| Windows Runtime | activation factories, `ICompositor` and the composition tree, `UISettings`, `AccessibilitySettings`, `GlobalizationPreferences`, effects (`IGraphicsEffect`, implemented) | both | `winrt.idl` | 2a, 2c |
| UI Automation | providers | implemented | the automation project | later |

These are ported as upstream has them: the four `.idl` files are in the crate (`direct_x/directx.idl`, `d_composition/dcomp.idl`, `win32_com/win32.idl`, `win_rt/winrt.idl`), and the build script of the crate runs the port's MicroCom generator (`src/tools/MicroCom.CodeGenerator`) over them, as the build script of `ferroui-native` does for `frn.idl`. **Built in stage 2a:**

- **The generator takes the files of the reference.** Their interface declarations are unchanged. What changed is the head of each file: the lines that map a type name to a type of the C# runtime (`@clr-map`, with `@clr-namespace` and `@clr-access`) are lines for the Rust generator. `@rust-map NAME type` names the Rust type of a name the file does not declare (`@rust-map HWND isize`, `@rust-map REFIID *const Guid`); `@rust-types-from path` imports the module in which every other name is found, which is where the structures and enumerations the reference declares in C# beside an IDL file are ported (`@rust-types-from super`). Three more rules of the reference generator are ported with them: a trailing pointer parameter marked `[out]` or `[retval]` of a method that returns `HRESULT` is the result of the proxy (`get_desc1() -> Result<DXGI_ADAPTER_DESC1, HResult>`); the method that sets a property (`[propput]`) is `SetName`; attributes with arguments (`annotation("_In_")`, `contract(...)`) are read and ignored.
- **The calling convention is `extern "system"`** in the generated vtables and thunks and in the runtime crate (`ferroui-microcom`): `stdcall` on 32-bit x86 Windows, where COM is `stdcall`, and the C convention on every other target, so nothing changes for the macOS backend, whose native side is called with the C convention. One thing of 32-bit x86 is not modelled and is recorded in the runtime crate: there a COM method that returns a structure takes a hidden parameter; no interface the port binds returns one. No 32-bit target is built or tested.
- **What is compiled.** `directx.idl` (21 interfaces of DXGI and Direct3D 11) is compiled into the crate with the enumerations (`direct_x/direct_x_enums.rs`), the structures (`direct_x/direct_x_structs.rs`, with their sizes on a 64-bit target tested against the system headers) and the two functions (`direct_x/direct_x_unmanaged_methods.rs`: `CreateDXGIFactory1` and `D3D11CreateDevice`, imported from `dxgi.dll` and `d3d11.dll` as `raw-dylib`) the reference declares beside it. The bindings are values and vtable calls and compile on every host. The other three files are generated by a test of the generator (`tests/win32_idl.rs`), which holds them until the stage that ports the structures they name and the code that uses them compiles them: `dcomp.idl` and the composition part of `winrt.idl` with 2c, `win32.idl` with 2d and 2e. `HResult::check` treats every value but `S_OK` as a failure, as the proxies of the reference do.
- **Left of 2a**, with the reason: `OleContext`, the Windows Runtime helpers (`NativeWinRTMethods`, `WinRTInspectable`, `WinRTPropertyValue`, `WinRTApiInformation`, `WinRTColor`) and with them the theme, the accent colour, the contrast and the language of the platform settings. They do not fall out of the generator: they need `winrt.idl` compiled (its numerics and colour types ported), string handles (`HSTRING`), activation factories and the change notifications, and nothing of the GPU path waits for them. They are the next step of 2a.

## 3. The crate

`src/Windows/FerroUI.Win32`, package `ferroui-win32`, selected with `AppBuilder::use_win32()` (`Win32ApplicationExtensions`) and by `use_platform_detect` of `ferroui-desktop` on Windows, with Skia as the renderer. It depends on the base, controls and OpenGL crates, the MicroCom runtime, `bitflags` and, on Windows, `windows-sys`; with its default feature `angle` also on `mozangle`, which compiles ANGLE from C++ (section 6.2). Without the default features it depends on no renderer and on nothing with C or C++ sources, so the crate and its example are checked for a Windows target on any host (`--no-default-features`).

### 3.1 One source tree for every host

The crate compiles on every host, and what it compiles differs:

- **Everywhere**: the constants, flag sets and plain structures of the interop file; the key tables; the options; and the logic that reads the parameters of messages and decides (the styles of a set of window properties, the command a state is shown with, the placement a resize asks for, the state a `WM_SIZE` reports, the sizes of `WM_GETMINMAXINFO`, the frame `WM_NCCALCSIZE` takes off, the modifiers and the raw event of a mouse message, the bitmap header of a framebuffer). This is written as functions of values, in the file upstream has the logic in, and it is what the host tests run (section 10.2).
- **On Windows only** (`cfg(windows)`): the system calls and everything that makes one. In a file that has both parts, the second is a module `imp` at its end.

Where the logic of upstream sits inside a method that also calls the system, the method here calls the function. The window state machine is therefore tested as its decisions, with their inputs given as values, not against a mock of the system calls: a mock of sixty calls would be a second implementation of the system to keep right, and the decisions are where the logic is. What the calls do on a real system is the job of the CI run (section 10.3).

### 3.2 System calls are safe functions; handles are numbers

`interop/unmanaged_methods.rs` is the port of `UnmanagedMethods.cs`: the enumerations upstream declares (generated once from the upstream file by a script, values unchanged: `WindowsMessage`, `WindowStyles`, `VirtualKeyStates` and thirty-six more) and, in its `native` module, one function per system call the backend makes. Each of those functions is safe to call: it takes and returns values, owns the buffer the system writes to, and carries the `unsafe` block of its call with the reason the call is sound. A handle that has to be given back is owned by a value that gives it back when dropped (`PaintScope` for `BeginPaint`/`EndPaint`, the open clipboard). The rest of the backend is written without `unsafe`, with three exceptions that are marked and argued where they are: the three window procedures (`extern "system"` functions the system calls) and the readers of the message parameters that point at a structure (`WM_GETMINMAXINFO`, `WM_WINDOWPOSCHANGING`, `WM_NCCALCSIZE`, `WM_DPICHANGED`), which are `unsafe fn` whose contract is "the parameter of that message, while it is processed".

Handles (`HWND`, `HMONITOR`, `HCURSOR`) are `isize` throughout, as they are `IntPtr` upstream. A number can sit in a `Cell`, be a key of a map and be given to the render thread; a pointer type could not without a claim about what it points at, and a handle points at nothing this process may touch.

### 3.3 Classes that derive from the window

Upstream's `PopupImpl` and `EmbeddedWindowImpl` derive from `WindowImpl` and override five members between them (how the window is created, how it is shown, three messages, the focus on a click, the largest automatic size). Here a window has a kind (`WindowKind`: window, popup with its state, embedded), the members that differ ask it, and `PopupImpl::new` and `EmbeddedWindowImpl::new` create a window of their kind. The window implements the popup contract as well as the window contract, as the derived class does upstream.

### 3.4 Finding the window of a message

Upstream registers a window class per window with a delegate of the instance as its procedure. Here every class has the same procedure, which finds the window of a message in a map of the thread from handle to weak reference; a window that is being created (the system sends `WM_GETMINMAXINFO`, `WM_NCCREATE` and `WM_CREATE` before `CreateWindowEx` returns the handle) is found through a slot of the thread that names it. A window is kept alive by the list of the windows of the thread from its creation to `WM_DESTROY`, as upstream's `s_instances` does, and the procedure holds a reference of its own while it runs. No pointer to a window is stored in the window.

## 4. File table

<!-- FILE-TABLE-START -->
101 files of the upstream project directory (the 95 C# files the tracking counts, the four interface definition files, the project file and the binding list): 23 built, 11 built in part, 64 not built (three of them interface definition files that are in the crate and generated, not yet compiled), 3 not applicable.

| Upstream file | Lines | FerroUI file | Stage | State and notes |
|---|---:|---|---|---|
| `AngleOptions.cs` | 22 | `angle_options.rs` | 2b | built |
| `Avalonia.Win32.csproj` | 47 | - | - | not applicable: project file: `Cargo.toml` |
| `ClipboardFormatRegistry.cs` | 90 | `clipboard_format_registry.rs` | 2d | not built: clipboard format names and identifiers |
| `ClipboardImpl.cs` | 163 | `clipboard_impl.rs` | 1 | built in part: Unicode text through the clipboard functions; the OLE data object, ownership and flushing are 2d |
| `CursorFactory.cs` | 126 | `cursor_factory.rs` | 1 | built in part: the cursors of the system; a cursor from a bitmap needs `Win32Icon` (2e) |
| `DataTransferToOleDataObjectWrapper.cs` | 225 | `data_transfer_to_ole_data_object_wrapper.rs` | 2d | not built |
| `DragSource.cs` | 36 | `drag_source.rs` | 2d | not built |
| `EmbeddedWindowImpl.cs` | 40 | `embedded_window_impl.rs` | 1 | built: a window implementation of the embedded kind |
| `FramebufferManager.cs` | 192 | `framebuffer_manager.rs` | 1 | built |
| `IBlurHost.cs` | 15 | `i_blur_host.rs` | 2c | not built |
| `IconImpl.cs` | 88 | `icon_impl.rs` | 2e | not built |
| `IWindowsSurfaceFactory.cs` | 13 | `i_windows_surface_factory.rs` | 2c | not built: the composition modes register it |
| `NativeMethods.txt` | 9 | - | - | not applicable: the list for the C# binding generator: the bindings come from `windows-sys` |
| `NonPumpingWaitHelperImpl.cs` | 13 | `non_pumping_wait_helper_impl.rs` | - | not applicable: the base library has no `NonPumpingLockHelper`: Rust locks do not pump messages |
| `OffscreenParentWindow.cs` | 10 | `offscreen_parent_window.rs` | 1 | built |
| `OleContext.cs` | 63 | `ole_context.rs` | 2a | not built: OLE initialisation of the UI thread |
| `OleDataObjectHelper.cs` | 715 | `ole_data_object_helper.rs` | 2d | not built |
| `OleDataObjectToDataTransferItemWrapper.cs` | 22 | `ole_data_object_to_data_transfer_item_wrapper.rs` | 2d | not built |
| `OleDataObjectToDataTransferWrapper.cs` | 125 | `ole_data_object_to_data_transfer_wrapper.rs` | 2d | not built |
| `OleDragSource.cs` | 41 | `ole_drag_source.rs` | 2d | not built |
| `OleDropTarget.cs` | 232 | `ole_drop_target.rs` | 2d | not built |
| `OleVirtualFileData.cs` | 449 | `ole_virtual_file_data.rs` | 2d | not built: with its upstream tests (`OleVirtualFileDataTests`) |
| `PlatformConstants.cs` | 19 | `platform_constants.rs` | 1 | built |
| `PopupImpl.cs` | 187 | `popup_impl.rs` | 1 | built: a window implementation of the popup kind |
| `ScreenImpl.cs` | 104 | `screen_impl.rs` | 1 | built |
| `SimpleWindow.cs` | 92 | `simple_window.rs` | 1 | built |
| `SwapChainTopLevelImpl.cs` | 122 | `swap_chain_top_level_impl.rs` | 2c | not built |
| `TrayIconImpl.cs` | 401 | `tray_icon_impl.rs` | 2e | not built |
| `Win32DispatcherImpl.cs` | 121 | `win32_dispatcher_impl.rs` | 1 | built |
| `Win32GlManager.cs` | 116 | `win32_gl_manager.rs` | 2b | built in part: the order of the rendering modes with ANGLE and software; WGL and Vulkan are passed over (not built); the composition modes of 2c are passed over, so a list ends at the redirection surface |
| `Win32NativeControlHost.cs` | 218 | `win32_native_control_host.rs` | 2e | not built |
| `Win32NativeToManagedMenuExporter.cs` | 16 | `win32_native_to_managed_menu_exporter.rs` | 2e | not built: Windows has no native menu: the exporter of the managed menu |
| `Win32Platform.cs` | 378 | `win32_platform.rs` | 1 | built in part: left: the settings change notifications (2a), the tray icon messages and the icon loader (2e), the drag source (2d) |
| `Win32PlatformOptions.cs` | 179 | `win32_platform_options.rs` | 1 | built in part: left: `WglProfiles` (2b, needs the OpenGL crate) |
| `Win32PlatformSettings.cs` | 138 | `win32_platform_settings.rs` | 1 | built in part: the gesture metrics; the colour values and the language need the Windows Runtime (2a) |
| `Win32StorageProvider.cs` | 318 | `win32_storage_provider.rs` | 2e | not built |
| `Win32TopLevelSceneInfo.cs` | 10 | `win32_top_level_scene_info.rs` | 1 | built |
| `Win32TypeExtensions.cs` | 19 | `win32_type_extensions.rs` | 1 | built |
| `WindowImpl.AppWndProc.cs` | 1529 | `window_impl_app_wnd_proc.rs` | 1 | built in part: left: the pointer and touch messages and the mouse history (2f), the input method messages (2f), the automation provider (the automation project) |
| `WindowImpl.cs` | 1722 | `window_impl.rs` | 1 | built in part: left: icons and the taskbar (2e), the blur, acrylic and mica levels and the surface factory of the composition modes (2c), the WGL surface (2b), the drop target (2d), the features of later stages |
| `WindowImpl.CustomCaptionProc.cs` | 380 | `window_impl_custom_caption_proc.rs` | 2f | not built: the client area is not extended into the frame before it |
| `WindowImpl.WndProc.cs` | 34 | `window_impl.rs` | 1 | built: in `window_impl.rs` |
| `WindowsMountedVolumeInfoListener.cs` | 82 | `windows_mounted_volume_info_listener.rs` | 2e | not built |
| `WindowsMountedVolumeInfoProvider.cs` | 14 | `windows_mounted_volume_info_provider.rs` | 2e | not built |
| `WinScreen.cs` | 127 | `win_screen.rs` | 1 | built in part: left: the friendly name of a display (`QueryDisplayConfig`); the name is the device name, the fallback of the reference |
| `DComposition/dcomp.idl` | 164 | `d_composition/dcomp.idl` | 2c | in the crate, generated by a test of the generator; compiled with the code that uses it (2c) |
| `DComposition/DirectCompositedWindow.cs` | 53 | `d_composition/direct_composited_window.rs` | 2c | not built |
| `DComposition/DirectCompositedWindowSurface.cs` | 203 | `d_composition/direct_composited_window_surface.rs` | 2c | not built |
| `DComposition/DirectCompositionConnection.cs` | 155 | `d_composition/direct_composition_connection.rs` | 2c | not built |
| `DComposition/DirectCompositionShared.cs` | 24 | `d_composition/direct_composition_shared.rs` | 2c | not built |
| `DComposition/NativeMethods.cs` | 14 | `d_composition/native_methods.rs` | 2c | not built |
| `DComposition/NativeStructs.cs` | 20 | `d_composition/native_structs.rs` | 2c | not built |
| `DirectX/directx.idl` | 550 | `direct_x/directx.idl` | 2a | built: compiled by the build script of the crate |
| `DirectX/DirectXEnums.cs` | 234 | `direct_x/direct_x_enums.rs` | 2a | built |
| `DirectX/DirectXStructs.cs` | 247 | `direct_x/direct_x_structs.rs` | 2a | built: the layouts as the reference declares them (two of them are not the layouts of the system headers there and are not passed to the system) |
| `DirectX/DirectXUnmanagedMethods.cs` | 32 | `direct_x/direct_x_unmanaged_methods.rs` | 2a | built: `CreateDXGIFactory1` and `D3D11CreateDevice`; `CreateDXGIFactory`, which the reference declares and does not call, is not declared |
| `DirectX/DxgiConnection.cs` | 247 | `direct_x/dxgi_connection.rs` | 2c | not built: the low-latency DXGI swap chain mode |
| `DirectX/DxgiRenderTarget.cs` | 182 | `direct_x/dxgi_render_target.rs` | 2c | not built |
| `DirectX/DxgiSwapchainWindow.cs` | 32 | `direct_x/dxgi_swapchain_window.rs` | 2c | not built |
| `DirectX/IDirect3D11TexturePlatformSurface.cs` | 40 | `direct_x/i_direct3_d11_texture_platform_surface.rs` | 2c | not built |
| `Input/Imm32CaretManager.cs` | 36 | `input/imm32_caret_manager.rs` | 2f | not built |
| `Input/Imm32InputMethod.cs` | 455 | `input/imm32_input_method.rs` | 2f | not built |
| `Input/KeyInterop.cs` | 550 | `input/key_interop.rs` | 1 | built |
| `Input/WindowsInputPane.cs` | 111 | `input/windows_input_pane.rs` | 2f | not built: needs `IFrameworkInputPane` (win32.idl) |
| `Input/WindowsKeyboardDevice.cs` | 50 | `input/windows_keyboard_device.rs` | 1 | built: holds the keyboard device of the base library (section 8) |
| `Input/WindowsMouseDevice.cs` | 47 | `input/windows_mouse_device.rs` | 1 | built: holds a mouse device of the base library (section 8) |
| `Interop/TaskBarList.cs` | 76 | `interop/task_bar_list.rs` | 2e | not built: full-screen mark and overlay icon of the taskbar |
| `Interop/UnmanagedMethods.cs` | 2847 | `interop/unmanaged_methods.rs` | 1 | built in part: the constants and structures the built files use, and the system calls behind safe functions; grows with every stage |
| `Interop/Win32Icon.cs` | 364 | `interop/win32_icon.rs` | 2e | not built |
| `OpenGl/Angle/AngleD3DTextureFeature.cs` | 107 | `open_gl/angle/angle_d3d_texture_feature.rs` | 2c | not built: a render target over a Direct3D 11 texture, which the composition modes use |
| `OpenGl/Angle/AngleEglInterface.cs` | 48 | `open_gl/angle/angle_egl_interface.rs` | 2b | built: over the loader of the crate `mozangle` (feature `angle`) |
| `OpenGl/Angle/AngleExternalD3D11Texture2D.cs` | 107 | `open_gl/angle/angle_external_d3d11_texture2_d.rs` | 2c | not built |
| `OpenGl/Angle/AngleExternalObjectsFeature.cs` | 109 | `open_gl/angle/angle_external_objects_feature.rs` | 2c | not built |
| `OpenGl/Angle/AngleWin32EglDisplay.cs` | 232 | `open_gl/angle/angle_win32_egl_display.rs` | 2b | built |
| `OpenGl/Angle/AngleWin32PlatformGraphicsFactory.cs` | 48 | `open_gl/angle/angle_win32_platform_graphics_factory.rs` | 2b | built |
| `OpenGl/Angle/D3D11AngleWin32PlatformGraphics.cs` | 91 | `open_gl/angle/d3d11_angle_win32_platform_graphics.rs` | 2b | built in part: left: the two features of a context (`AngleD3DTextureFeature`, `AngleExternalObjectsFeature`: 2c) |
| `OpenGl/Angle/D3D9AngleWin32PlatformGraphics.cs` | 53 | `open_gl/angle/d3d9_angle_win32_platform_graphics.rs` | 2b | built |
| `OpenGl/Angle/SwapChainGlSurface.cs` | 252 | `open_gl/angle/swap_chain_gl_surface.rs` | 2c | not built |
| `OpenGl/WglConsts.cs` | 64 | `open_gl/wgl_consts.rs` | 2b | not built |
| `OpenGl/WglContext.cs` | 108 | `open_gl/wgl_context.rs` | 2b | not built |
| `OpenGl/WglDisplay.cs` | 170 | `open_gl/wgl_display.rs` | 2b | not built |
| `OpenGl/WglGdiResourceManager.cs` | 172 | `open_gl/wgl_gdi_resource_manager.rs` | 2b | not built |
| `OpenGl/WglGlPlatformSurface.cs` | 88 | `open_gl/wgl_gl_platform_surface.rs` | 2b | not built |
| `OpenGl/WglPlatformOpenGlInterface.cs` | 43 | `open_gl/wgl_platform_open_gl_interface.rs` | 2b | not built |
| `OpenGl/WglRestoreContext.cs` | 42 | `open_gl/wgl_restore_context.rs` | 2b | not built |
| `Vulkan/VulkanNativeInterop.cs` | 27 | `vulkan/vulkan_native_interop.rs` | 2b | not built: waits for the Vulkan project of the port (`Avalonia.Vulkan`, out of scope today) |
| `Vulkan/VulkanSupport.cs` | 72 | `vulkan/vulkan_support.rs` | 2b | not built: as above |
| `Win32Com/win32.idl` | 394 | `win32_com/win32.idl` | 2d | in the crate, generated by a test of the generator; compiled with the code that uses it (2d, 2e) |
| `WinRT/Composition/D2DEffects.cs` | 133 | `win_rt/composition/d2d_effects.rs` | 2c | not built |
| `WinRT/Composition/WinUiCompositedWindow.cs` | 130 | `win_rt/composition/win_ui_composited_window.rs` | 2c | not built |
| `WinRT/Composition/WinUiCompositedWindowSurface.cs` | 252 | `win_rt/composition/win_ui_composited_window_surface.rs` | 2c | not built |
| `WinRT/Composition/WinUiCompositionShared.cs` | 39 | `win_rt/composition/win_ui_composition_shared.rs` | 2c | not built |
| `WinRT/Composition/WinUiCompositionUtils.cs` | 177 | `win_rt/composition/win_ui_composition_utils.rs` | 2c | not built |
| `WinRT/Composition/WinUiCompositorConnection.cs` | 277 | `win_rt/composition/win_ui_compositor_connection.rs` | 2c | not built |
| `WinRT/Composition/WinUIEffectBase.cs` | 240 | `win_rt/composition/win_ui_effect_base.rs` | 2c | not built |
| `WinRT/NativeWinRTMethods.cs` | 158 | `win_rt/native_win_rt_methods.rs` | 2a | not built |
| `WinRT/winrt.idl` | 906 | `win_rt/winrt.idl` | 2a | in the crate, generated by a test of the generator; compiled with the Windows Runtime helpers (the next step of 2a) |
| `WinRT/WinRTApiInformation.cs` | 210 | `win_rt/win_rt_api_information.rs` | 2a | not built |
| `WinRT/WinRTColor.cs` | 21 | `win_rt/win_rt_color.rs` | 2a | not built |
| `WinRT/WinRTInspectable.cs` | 27 | `win_rt/win_rt_inspectable.rs` | 2a | not built |
| `WinRT/WinRTPropertyValue.cs` | 108 | `win_rt/win_rt_property_value.rs` | 2a | not built |
<!-- FILE-TABLE-END -->

The members of the built files that are left for a later stage are named in the notes above and at their place in the sources. A member that is not built is absent, or fails with a message that names its stage (`not_built` in `lib.rs`: the icon loader, a window icon, a cursor from a bitmap); nothing returns a made-up result. Three members answer what the contract defines for a platform without the feature, and say so: `create_tray_icon` returns `None` ("the platform has no tray icons"), the transparency levels that need a composition surface are not supported (the level stays the default), and the hint to extend the client area into the frame is not honoured (the window reports that its client area is not extended).

## 5. Threads

**The UI thread** is the thread that initialises the platform. It owns a message-only window (`Win32Platform`), every window, and the message loop.

- **The dispatcher** (`Win32DispatcherImpl`, a controlled dispatcher implementation with pending input): a signal is a message posted to the message window (`WM_DISPATCH_WORK_ITEM` with the two magic parameters of upstream), which is the one thing other threads do, through a handle that holds the number of the window; the timer is the timer of the message window; the loop is `GetMessage`, `TranslateMessage`, `DispatchMessage` until the token is cancelled; pending input is asked with `MsgWaitForMultipleObjectsEx` and `MWMO_INPUTAVAILABLE`, for the reason upstream's comment gives (`GetQueueStatus` only counts new input). There is no synchronization context to install: the dispatcher of the base library is the port's equivalent, and the non-pumping wait upstream installs around `WM_PAINT` exists so that a managed lock does not run the message loop, which a Rust lock never does.
- **Window procedures and panics.** The system calls a window procedure through its own frames, which a panic must not cross. A procedure of this backend runs its body under `wnd_proc_guard::guard`: a panic is caught and kept, the procedure answers with the default processing, and the message loop (or the member of the backend that made the call which sent the message: creating, showing, resizing) raises the panic again once the system has returned. Upstream has no counterpart because the managed runtime carries an exception through native frames.

**The render thread** is the thread of the render timer: `SleepLoopRenderTimer` at the highest refresh rate of the screens (`Win32Platform::update_timer_fps`, on start and on `WM_DISPLAYCHANGE`), or the UI thread itself with `should_render_on_ui_thread`. The compositor is created as upstream creates it on this platform (synchronous commits do not render on the UI thread). What the render thread touches of this backend is the surfaces of a window (`docs/porting/render-thread.md`: a render surface is `Send + Sync`):

| Surface | Holds | Calls it makes from the render thread |
|---|---|---|
| the window handle (`WindowImplPlatformHandle`, an `INativePlatformHandleSurface`) | the handle and the scaling, as atomics the UI thread writes | `GetClientRect` |
| the framebuffer (`FramebufferManager`) | the handle, the pixel memory under a lock | `GetClientRect`, `MonitorFromWindow`, `GetDpiForMonitor`, `GetDC`, `SetDIBitsToDevice`, `ReleaseDC` |

All of these may be called from any thread. The lock of the framebuffer is upstream's monitor (entered by `lock`, left when the locked framebuffer is disposed); a guard of a Rust mutex cannot live in an object behind a contract, so it is a flag with a condition variable, and a framebuffer that is dropped without being disposed releases it too.

## 6. Rendering

### 6.1 Rendering modes (`Win32PlatformOptions::rendering_mode`)

Upstream's default order is ANGLE, then software. The platform takes the first mode that initialises; a list in which nothing does is an error, as upstream.

| Mode | How it draws | What it needs here | Stage |
|---|---|---|---|
| `Software` | the renderer draws into memory of the window's size; `SetDIBitsToDevice` copies it to the window (`FramebufferManager`) | nothing | **1, built** |
| `AngleEgl` | OpenGL ES through ANGLE's EGL on a Direct3D 11 device; Skia's Ganesh on that context | the feature `angle` (default): ANGLE from the crate `mozangle` (section 6.2), the EGL of `ferroui-opengl`, `directx.idl`; Skia with the `gl` feature | **2b, built** |
| `Wgl` | the OpenGL of the system | `ferroui-opengl`, Skia with `gl` | 2b, not built: passed over like a mode that fails to initialise |
| `Vulkan` | Vulkan | the Vulkan project of the port (`Avalonia.Vulkan`, out of scope today) and the Vulkan GPU of the Skia backend, which waits for it | 2b, last |

`Win32GlManager` (`win32_gl_manager.rs`) is the loop: `Software` ends it without platform graphics; `AngleEgl` asks `AngleWin32PlatformGraphicsFactory` and takes the graphics only when they are the ones of Direct3D 11, then requires a composition mode (section 6.3); a mode that does not initialise is passed over and the next is tried, so the default options end at software rendering on a machine or in a build without ANGLE; a list in which nothing applies fails with upstream's message. `custom_platform_graphics` is honoured as upstream. The order with a mock of the factory is tested on every host.

**ANGLE, file by file** (`open_gl/angle/`):

| File | What it is here |
|---|---|
| `angle_egl_interface.rs` | `Win32AngleEglInterface`: the EGL interface of `ferroui-opengl` over the loader of the ANGLE that is linked in, and `eglCreateDeviceANGLE`/`eglReleaseDeviceANGLE`. Without the feature `angle` loading fails with a message that names the feature, which the factory logs as ANGLE not loadable. |
| `angle_win32_egl_display.rs` | `AngleWin32EglDisplay`: the display on Direct3D 9, on Direct3D 11 with the device of ANGLE, and on a Direct3D 11 device the backend creates (`D3D11CreateDevice` on the adapter chosen, feature levels 11.1 down to 9.1; `eglCreateDeviceANGLE`; `eglGetPlatformDisplayEXT` with `EGL_PLATFORM_DEVICE_EXT`); the device loss check (`GetDeviceRemovedReason`); the Direct3D device of a display; a Direct3D 11 texture as a surface, whole or a rectangle. The adapter choice (the callback of the options; away from an Adreno adapter by default on ARM64) is a function of values, tested on every host. |
| `d3d11_angle_win32_platform_graphics.rs` | The platform graphics: every context has a display and a device of its own; the OpenGL context factory (OpenGL ES 3 or an error). |
| `d3d9_angle_win32_platform_graphics.rs` | The graphics with one shared display and context. As upstream, the manager does not take them. |
| `angle_win32_platform_graphics_factory.rs` | The first allowed API that works (`AngleOptions::allowed_platform_apis`, Direct3D 11 alone by default). |

Not built of `OpenGl/Angle`: `AngleD3DTextureFeature`, `AngleExternalObjectsFeature` and `AngleExternalD3D11Texture2D` (rendering into and sharing Direct3D 11 textures: what the composition modes and the texture sharing of the compositor use, stage 2c) and `SwapChainGlSurface` (2c). A context answers that it does not have those features.

**The surface of a window.** With the platform graphics of ANGLE a window has three surfaces: its handle, an `EglGlPlatformSurface` of `ferroui-opengl` over what the window publishes of itself (handle, client size, scaling: `WindowImplPlatformHandle`, read by the thread that renders), and the framebuffer. The EGL window surface is ANGLE's swap chain on the window: the redirection-surface composition mode.

**Threads.** A display, a context and the EGL interface are objects of one thread. The platform graphics are shared with the render thread, which creates the contexts; so the graphics hold only the versions of OpenGL ES asked for and the identifier of the adapter that was chosen on the UI thread when the graphics were probed, and a context loads the interface and creates its display on the thread that asks for it. Upstream keeps the display of the probe for the first context; here the probe's display is disposed (one device more is created at start).

### 6.2 Skia on Windows

From the sources of `skia-safe` and `skia-bindings` 0.153.3 in the cargo registry **[V]**:

- Features that exist: `gl` (implies `ganesh`), `d3d` (implies `ganesh`: Ganesh on Direct3D 12, module `gpu::d3d`), `vulkan`, `graphite`, and the alias `all-windows` = `ganesh`, `gl`, `vulkan`, `d3d`, `textlayout`, `svg`, `skottie`, `webp`, `graphite`.
- **Graphite has no Direct3D backend in these bindings.** `src/gpu/graphite` has two backends, `mtl` and `vk`; Dawn is not bound. Graphite on Windows therefore means Graphite on Vulkan.
- On Windows Skia uses the system's font manager (DirectWrite) and no FreeType (`build_support/platform/windows.rs`: `uses_freetype` is false).
- A build takes a prebuilt binary when one was published for its key and otherwise builds Skia from source, which needs LLVM and an hour. The key is `<first 20 digits of the commit of the bindings>-<target>-<features, sorted, joined by '-'>` (`build_support/binary_cache/binaries.rs`), with `-static` appended for a static C runtime; for the raster build of the Skia backend on Windows that is `b7f043e0b1e2a850e702-x86_64-pc-windows-msvc-jpegd-jpege-pdf`, fetched from `https://github.com/rust-skia/skia-binaries/releases/download/0.153.3/skia-binaries-<key>.tar.gz`.
- Which keys were published for Windows cannot be read from the crate: it is decided by the release workflow of rust-skia. **[CI]** The Windows job asks the release for the raster key and for the GPU candidates (`ganesh-gl`, `d3d-ganesh`, `graphite-vulkan`, `ganesh-gl-graphite-vulkan`, and the four GPU features together) and prints which exist, before anything is built with Skia.

What follows for the port:

| Renderer on Windows | Skia features | Target of |
|---|---|---|
| raster | none (the default set) | stage 1: `use_platform_detect` selects Skia, which draws into the framebuffer surface |
| Ganesh on OpenGL ES (ANGLE) or OpenGL (WGL) | `gl` | stage 2b: the Ganesh GPU of the Skia backend (`gpu/open_gl`) exists and is built for the browser only today, because no published binary has Graphite and Ganesh together; on Windows it has no Graphite to conflict with |
| Graphite on Vulkan | `graphite`, `vulkan` | after the Vulkan project |
| Ganesh on Direct3D 12 | `d3d` | not planned: upstream has no such path, and its composition modes are built on Direct3D 11 textures shared with ANGLE |

The owner's rule "the Skia backend uses Graphite" cannot be met on Windows through Direct3D; it can through Vulkan, which upstream offers as its fourth mode and does not present through the composition modes. The port follows upstream's default (ANGLE, Ganesh) and leaves Graphite on Vulkan for the Vulkan stage.

**Which Skia binaries exist for Windows [CI]** (asked of the release `0.153.3` of `rust-skia/skia-binaries` by the Windows job, 2026-10-10), for `x86_64-pc-windows-msvc`:

| Feature set (key) | Published |
|---|---|
| `jpegd-jpege-pdf` (raster) | yes |
| `ganesh-gl-jpegd-jpege-pdf` (Ganesh on OpenGL) | yes |
| `d3d-ganesh-jpegd-jpege-pdf` (Ganesh on Direct3D 12) | yes |
| `graphite-jpegd-jpege-pdf-vulkan` (Graphite on Vulkan) | yes |
| `ganesh-gl-graphite-jpegd-jpege-pdf-vulkan` | no |
| `d3d-ganesh-gl-graphite-jpegd-jpege-pdf-vulkan` | no |

So the Skia backend asks for the feature `gl` of the bindings on Windows (`src/Skia/FerroUI.Skia/Cargo.toml`, a target table of its own beside the Apple and the browser tables, which are unchanged) and sets its `ferro_skia_ganesh_gl` configuration there (`build.rs`): the Ganesh GPU of the backend (`gpu/open_gl`), built for the browser alone until now, is built for Windows, and nothing is built from source on x86-64. Ganesh and Graphite together have no binary, as on the other platforms, so Graphite on Vulkan will need its own choice of feature set when the Vulkan stage comes. SKIA-ARM64-PLACEHOLDER

### 6.2.1 ANGLE: the crate `mozangle` (owner's decision, 2026-10-10)

Upstream ships its own build of ANGLE as a library beside the application (`av_libGLESv2.dll`, the package `Avalonia.Angle.Windows.Natives` 2.1.27548) and resolves its entry points through the `EGL_GetProcAddress` that library exports. The port may take crates from crates.io only, and **uses the crate `mozangle` 0.7.1** (pinned): Mozilla's fork of ANGLE as shipped in Firefox (`gfx/angle` of `FIREFOX_153_3_0esr_RELEASE`), packaged by the Servo project, BSD-3-Clause. Its feature `egl` compiles ANGLE's EGL and OpenGL ES implementation from C++ source with the `cc` crate and links it statically into the application: there is no library to ship, and `mozangle::egl::get_proc_address` is the loader function the EGL interface of the port is built from.

| | |
|---|---|
| What the build needs **[V]** (`build.rs` of the crate) | the MSVC C++ tools and a Windows SDK (already needed to link any Rust program for the MSVC targets), and **libclang**: the build script generates bindings with `bindgen` in its `runtime` mode, which loads `libclang.dll` when the application is built. An LLVM installation provides it (`LIBCLANG_PATH` names its `bin` directory when it is not found by itself). It also builds zlib from the source `libz-sys` carries. |
| Does a hosted runner build it without installs **[CI]** | yes, both: the images have LLVM (`windows-latest`: clang 20.1.8; `windows-11-arm`: clang 22.1.8). |
| Build time **[CI]** | 528 s on `windows-latest` (x86-64), 363 s on `windows-11-arm`, for `ferroui-win32` with the feature on top of a build without it: nearly all of it is the C++ of ANGLE. 429 s in the virtual machine (ARM64, four processors, three jobs) **[VM]**. The Rust cache of the job keeps it between runs. |
| ARM64 **[CI]** | builds and runs for `aarch64-pc-windows-msvc` (the build script has processor-specific flags for x86 only). |
| What it is **[CI]** | EGL 1.5, "ANGLE 2.1.19767 git hash: 7b0bc3d196d4"; OpenGL ES 3.0 on Direct3D 11 (`vs_5_0 ps_5_0`), OpenGL ES 2.0 on Direct3D 9. On the runners the device is the Microsoft Basic Render Driver (WARP). |
| Direct3D 9 **[CI]** | works on both runners (the crate's README says only Direct3D 11 is configured; its build data defines `ANGLE_ENABLE_D3D9` as well, and the display of `EGL_PLATFORM_ANGLE_TYPE_D3D9_ANGLE` initialises, creates a context and draws). So `AngleOptions::allowed_platform_apis` with Direct3D 9 creates its graphics; as upstream, the graphics manager takes only the graphics of Direct3D 11. |
| Extensions upstream's display uses **[CI]** | present: `EGL_ANGLE_device_creation`, `EGL_ANGLE_device_creation_d3d11`, `EGL_EXT_platform_device`, `EGL_EXT_device_query`, `EGL_ANGLE_platform_angle_d3d`, `EGL_ANGLE_d3d_texture_client_buffer`, `EGL_ANGLE_d3d_share_handle_client_buffer`, `EGL_ANGLE_surface_d3d_texture_2d_share_handle`, `EGL_ANGLE_keyed_mutex`, `EGL_ANGLE_direct_composition`, `EGL_ANGLE_windows_ui_composition`, `EGL_KHR_surfaceless_context`. Absent: `EGL_ANGLE_flexible_surface_compatibility`, which newer ANGLE no longer has and which upstream's display treats as optional (the port does the same). |
| Linker **[CI]** | a warning `LNK4217` for every EGL entry point (defined in the one static library and "imported" by the other): the two libraries are meant to be two DLLs; linked statically the calls are direct. Harmless. |

**The feature.** ANGLE is the cargo feature `angle` of `ferroui-win32`, and it is **on by default**: upstream's default rendering mode on Windows is ANGLE, and the hosted runners build it with what their images have. What a user needs installed to build an application for Windows with the default features: the Rust toolchain for an MSVC target, the Visual Studio Build Tools with the C++ workload and a Windows SDK, and LLVM (for `libclang.dll`). Without LLVM: `default-features = false` on `ferroui-win32` gives the backend with software rendering alone; the rendering mode `AngleEgl` then fails to initialise with a message that names the feature and the platform goes on to the next mode of the options, which by default is software. Software rendering is always available.

Everything that touches `mozangle` is three lines behind the feature (`open_gl/angle/angle_egl_interface.rs`, `load_angle`); the rest of the ANGLE code is written against a loader function and is compiled and tested on every host against an EGL made of functions of the test modules. The crate with the feature cannot be checked for a Windows target on another system (the build script compiles C++ for the target).

### 6.3 Composition modes (`Win32PlatformOptions::composition_mode`)

Composition is how a rendered frame reaches the screen. It only matters with ANGLE (upstream registers a composition mode only when the platform graphics are ANGLE on Direct3D 11); the order below is upstream's default order followed by the two modes that are not in it.

| Mode | Presents through | Needs | Gives | Stage |
|---|---|---|---|---|
| `WinUIComposition` | a surface of the Windows.UI.Composition tree of the window (Windows 10 build 17134 and later) | `winrt.idl`, a dispatcher queue, ANGLE on Direct3D 11 | true transparency, the acrylic and mica backdrops (the blur effects are graph effects the backend implements), frames paced by the compositor | 2c |
| `DirectComposition` | a DirectComposition surface (Windows 8 and later) | `dcomp.idl`, ANGLE | true transparency | 2c |
| `RedirectionSurface` | the redirection bitmap of the window: an EGL window surface, or GDI for the framebuffer | nothing more | what the software mode and, until 2c, ANGLE present through | **1** (software), **2b** (EGL), built |
| `LowLatencyDxgiSwapChain` | a DXGI swap chain with a waitable object, the render timer driven by it | `directx.idl`, ANGLE | the lowest input latency; not in the default order | 2c |

With ANGLE the manager requires a composition mode that applies: the three modes of 2c are passed over like modes the system does not support, so the default order (`WinUIComposition`, `DirectComposition`, `RedirectionSurface`) ends at the redirection surface, and a list without it fails with upstream's message.

A window is created with `WS_EX_NOREDIRECTIONBITMAP` when the composition mode draws to a surface of its own; until 2c a window always has the redirection bitmap, so its default transparency level is `None` and the one other level it supports is `Transparent` through `DwmEnableBlurBehindWindow`, upstream's fallback.

## 7. Windows

`WindowImpl` (`window_impl.rs`, with the procedure in `window_impl_app_wnd_proc.rs`) is the port of the three files of upstream's partial class that stage 1 covers.

- **Creation**: a class per window (`CS_OWNDC | CS_HREDRAW | CS_VREDRAW`, the arrow cursor, no background brush), an overlapped window, the window registered for touch, the DPI of its monitor read.
- **Properties to styles**: `update_window_properties` computes the styles from the window properties (decorations, resizable, minimizable, maximizable, taskbar button, state) with `window_styles_from_properties`, lets the styles callback of `Win32Properties` change them, and applies them; a window without a taskbar button and without an owner is owned by the hidden parent window (`OffscreenParentWindow`).
- **States**: shown with the command of its state (`show_window_command`); full screen is a restored window without a frame that covers its monitor, with the styles and the rectangle saved (`set_full_screen`, after Chromium's handler as upstream); the state the toolkit is told comes from `WM_SIZE`.
- **Size and position**: `resize` sets the restored rectangle through the window placement so that a maximized or minimized window keeps its restored size (`placement_for_resize`); the reason of a resize travels in a field around the call that causes it; minimum and maximum sizes are applied in `WM_GETMINMAXINFO`; a window without a caption maximizes to the working area of its screen (`WM_GETMINMAXINFO`, `WM_WINDOWPOSCHANGING`).
- **DPI**: `WM_DPICHANGED` sets the scaling, tells the toolkit, and moves the window to the rectangle the system suggests. The process is made per-monitor DPI aware (version 2 where the system has it) by the platform before any window exists.
- **Messages routed to the contract**: activation, closing (the callback may refuse) and destruction, paint (`BeginPaint`, the rectangle in device independent pixels, `EndPaint`), size, move, show and hide, display change, focus loss, and the input of section 8.

## 8. Input

| Input | Upstream | Here |
|---|---|---|
| Keys | `WM_KEYDOWN`/`UP`, `WM_SYSKEYDOWN`/`UP` to a raw key event: the key from the virtual key (`KeyInterop`, 169 keys; left and right modifiers told apart by the scan code and the extended bit), the physical key from the scan code (155 codes), the key symbol from `ToUnicodeEx` without changing the keyboard state, or from `MapVirtualKey` for system keys | **stage 1**, with the tables and their arithmetic tested on every host |
| Text | `WM_CHAR` to raw text input, unless the key down was handled or an input method is composing | **stage 1**, without the input method condition; the two messages of a character outside the basic plane are delivered as one text, where upstream delivers two strings of half a pair each, which a Rust string cannot hold |
| Dead keys | fall out of the two rows above: the key symbol of a dead key is its accent (`ToUnicodeEx` reports a negative length), and the composed character arrives as `WM_CHAR` | **stage 1** |
| Input methods | `Imm32InputMethod`: the composition window follows the caret, composition text goes to the text input client, `WM_IME_*` | 2f; until then the messages get the default processing, so the system shows its own composition window and the result still arrives as `WM_CHAR` |
| Mouse | `WM_MOUSEMOVE`, the button messages (client and non-client), `WM_MOUSEWHEEL`/`HWHEEL`, `WM_MOUSELEAVE`, `WM_CAPTURECHANGED`; modifiers from the keyboard state and the flags of the message; capture through the pointer of the mouse device | **stage 1**; the intermediate points of a move (`GetMouseMovePointsEx`) are 2f |
| Touch and pen | `WM_POINTER*` (Windows 8 and later) with pressure, tilt, contact rectangles and history; `WM_TOUCH` before | 2f; until then touch does nothing in a window (the mouse messages the system makes up for it are ignored as upstream ignores them) |
| Non-client hit testing | `WindowImpl.CustomCaptionProc.cs`, with an extended client area | 2f |

`WindowsKeyboardDevice` and `WindowsMouseDevice` derive from the devices of the base library upstream. Here the keyboard device stays the device of the base library, because the input manager and the focus code recover that type from the device contract, and `WindowsKeyboardDevice` holds the instance and reads the modifiers; `WindowsMouseDevice` holds a mouse device made with the pointer of this backend, whose capture is `SetCapture` on the window of the captured element.

## 9. Other services of stage 1

| Service | State |
|---|---|
| Screens (`ScreenImpl`, `WinScreen`) | the monitors of the desktop, their bounds, working areas, scaling (`GetDpiForMonitor`), primary flag, orientation and refresh rate; changes on `WM_DISPLAYCHANGE` and on a change of the work area. The display name is the device name, upstream's fallback; the friendly name is 2e |
| Cursors (`CursorFactory`) | the cursors of the system and the drag cursors of ole32; a cursor from a bitmap is 2e |
| Clipboard (`ClipboardImpl`) | Unicode text, with upstream's retries while another process holds the clipboard. Upstream goes through an OLE data object for every format; that is 2d |
| Platform settings (`Win32PlatformSettings`) | the tap and double-tap sizes and the double-click time of the system; theme, accent colour, contrast and language need the Windows Runtime (2a) and are the defaults of the base library until then, which is upstream's answer on a system without those types |
| Hotkeys | Control as the command modifier, Shift+F10 for the context menu, "Win" as the name of the meta key |
| Lifetime events | `WM_QUERYENDSESSION` to the shutdown-requested event, which may refuse |
| Z order | `get_windows_z_order` by enumerating the top-level windows |

## 10. Verification

Nothing of this backend can run on the machine it is written on. Four things stand in for that, and each proves a different part: the check for a Windows target on any host, the tests on the host, the Windows jobs of CI (x86-64 and ARM64), and the owner's Windows 11 ARM64 virtual machine. Section 10.5 says what was verified where.

### 10.1 Compiling for a Windows target on any host

`rustup target add x86_64-pc-windows-msvc`, then `cargo check -p ferroui-win32 --no-default-features --target x86_64-pc-windows-msvc --all-targets` **[V]** (green on macOS, the library, its tests and its example). It works because, without the default feature `angle`, the crate and the dev-dependency of its example have no C or C++ sources and `windows-sys` links without import libraries. The ANGLE code is part of what is checked, all but the three lines that call `mozangle`; the example's OpenGL painter too. It proves that every call matches the declaration of the function it calls. `ferroui-desktop` cannot be checked this way: the Skia bindings and the bundled HarfBuzz need a C++ toolchain of the target.

That is also why the example draws with **the Vello backend in its CPU mode** and not with Skia: `ferroui-vello` without its GPU features is Rust alone and checks for the Windows target **[V]**, so the example goes through the real render interface (a backend context, a render target over the surfaces of the window, a drawing context) and is still checked on a Mac.

### 10.2 Host tests

`cargo test -p ferroui-win32` on any host: 82 tests **[V]** of what is logic (section 3.1). Since stage 2: the ANGLE interface, display, platform graphics and factory against an EGL made of functions of the test modules (which display is asked for with which attributes, the Direct3D device of a display, a texture wrapped whole and as a rectangle, the flexible surface attribute, the probe that fails when no context can be created, a build without ANGLE); the adapter choice (the default, the Adreno rule of ARM64, the callback and its bounds); the order of the rendering modes and of the composition modes in the graphics manager; the values of the Direct3D enumerations and the sizes of the structures passed to the system; the options of ANGLE. Before stage 2: The key tables in both directions and the generic modifiers; the physical keys with the extended bit; key symbols with dead keys; the modifiers of a keyboard state; message parameters (signed coordinates, the low 32 bits, sizes, the wheel, the DPI, system commands); the raw events of the button messages and their modifiers; emulated mouse messages; the styles of every combination the decisions branch on; show commands; the state of a size message and of a placement; the placement of a resize, including the minimized cases; the track sizes; the captionless maximized rectangle and the frame of an extended client area against a model frame; the bitmap header and the minimum size of a framebuffer; the cursor table; the timer interval and the signal parameters of the dispatcher; the options and their defaults; version comparison.

Upstream's own tests of this backend are `tests/Avalonia.IntegrationTests.Win32`: they create `Window` controls on a live desktop and compare client sizes with the working area for every screen, state, decoration and resizability (`StandardWindowTests`, `ExtendClientAreaWindowTests`), drag a window (`BeginMoveDragTests`), and test the virtual files of a drag (`OleVirtualFileDataTests`) and the automation nodes. They need the `Window` class over this backend with a renderer on a Windows machine, so they are ported with the first step of stage 2 that has `ferroui-desktop` building in CI (section 11), not here. The assertion of the standard window tests that applies to a window with a caption is made by the smoke run of the example (the maximized client area against the working area).

### 10.3 The CI jobs on Windows runners

Two jobs in `.github/workflows/ci.yml`, beside the macOS job, with the same pinned toolchain: `windows` on `windows-latest` (x86-64; the image has Visual Studio and LLVM) and `windows-arm64` on `windows-11-arm` (the same steps for `aarch64-pc-windows-msvc`; informative as a whole, `continue-on-error`). The steps, in order, and what each proves:

| Step | Proves | Fails the job |
|---|---|---|
| Build the backend and its example without ANGLE (`--no-default-features`) | the crate links against the system's libraries (a wrong DLL or export name of a `raw-dylib` import is a link-time or load-time error, which a check on a Mac does not see), and builds where there is no libclang | yes |
| Tests (`cargo test -p ferroui-win32 --no-default-features`) | the host tests on the real target, with the widths of its types | yes |
| Smoke run, software rendering (`win32_window --smoke`) | the backend on a real system: see below | yes |
| Build with ANGLE (the default features) | `mozangle` builds on the runner with what the image has; the step prints the time | yes |
| Smoke run through ANGLE (`--smoke --rendering angle --angle-probe`) | the GPU path: see below | informative until its first green run |
| Skia binaries for Windows | section 6.2: which prebuilt keys exist; the key with Ganesh on OpenGL gates the next two steps | no: informative |
| Build and test `ferroui-desktop` | that the Skia backend (with Ganesh on OpenGL) and HarfBuzz build with MSVC, and `use_platform_detect` selects HarfBuzz, Win32 and Skia | no: informative until its first green run |
| `hello_window` for two seconds | the whole stack: `Application`, `Window`, the compositor and its render thread, Skia on the context of ANGLE | no: informative until its first green run |

**The smoke run** creates a real window on the desktop of the runner (GitHub's hosted Windows runners run their jobs in an interactive session with a desktop **[CI]**: 1024 by 768 at scaling 1), and prints every check with what it saw (`[ ok ]`, `[FAIL]`, `[info]`):

1. the version of Windows and the screens as the backend reads them;
2. the window: a handle, its surfaces (two; three with ANGLE, one of them an OpenGL surface), the client size asked for, the position, a frame around the client area;
3. twelve frames on a timer of the dispatcher, with a resize half way; every frame is read back and has to cover the window and differ from the others (the shapes move);
4. maximize and restore: the state callbacks, the client area against the working area of the screen, the restored size;
5. input: key, mouse and wheel messages posted to the window have to arrive as raw events with the right key, physical key, text and positions;
6. the clipboard: text set and read back (a clipboard held by another process is reported, not failed);
7. the dispatcher: work posted from another thread has to wake the message loop;
8. closing: the closed callback ends the main loop.

The exit code is 0 only if every frame was drawn and every check passed.

**With `--rendering software`** (the default) the frames are drawn through the render interface of the Vello backend in its CPU mode into the framebuffer surface, and read back from the framebuffer.

**With `--rendering angle`** the platform is initialised with the rendering mode `AngleEgl` alone (no fallback, so a mode that does not initialise fails the run) and the example draws with OpenGL ES itself through what the backend gives a renderer: the platform graphics registered with the services, a context of them (a display on a Direct3D 11 device the backend creates), the OpenGL surface among the surfaces of the window, its render target, and a session for each frame. A frame is rectangles drawn with scissored clears (bands of colour, a card, a rectangle that moves with every frame). **Before the session is disposed, which presents the frame, the whole default framebuffer is read back with `glReadPixels`** and compared with what was drawn at three places (the moving rectangle, the card, the first band); the checks `OpenGL context` (the strings of the context), `frames` and `frames of the GPU hold what was drawn` report it. So the path from the platform graphics to pixels of the window surface is verified, not assumed. What it does not verify is that the swap reached the screen (a capture of the window from the desktop would; the runners have no person to see it, and the composition of the desktop is not this backend's).

**`--angle-probe`** adds what ANGLE reports of itself: the client extensions, and for the display of Direct3D 11 and of Direct3D 9 the vendor, version and extensions of EGL, the Direct3D device, the strings of a context, and a frame cleared to a colour on a window surface and read back.

The Rust-only renderer of the example is the reason the GPU run does not draw with Skia: the example has to stay checkable on any host. Skia on the context of ANGLE is what `hello_window` of `ferroui-desktop` runs.

### 10.4 The virtual machine

The owner's test machine is a Windows 11 ARM64 virtual machine (Parallels, "Windows 11", build 26200, four processors, scaling 2 on a 3592 by 2016 display). `scripts/windows/vm-smoke.ps1` runs the same things there as the CI job, on any Windows machine without a person:

```
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\windows\vm-smoke.ps1 -TargetDir C:\ferroui-target -Jobs 3 -InteractiveUser <user> -SmokeArguments "--angle-probe"
```

- **It installs nothing.** It checks for the Rust toolchain (`cargo` on the path or in `-CargoHome`), the MSVC build tools (through `vswhere`, and two usual directories of the Build Tools) and LLVM (`libclang.dll` in `LIBCLANG_PATH`, `-LibclangPath` or `C:\Program Files\LLVM\bin`), prints what it found with versions, and stops with a list of what is missing.
- **It builds** `ferroui-win32` and its example for the architecture of the machine, first without the default features and then with ANGLE (unless `-NoAngle`), with the build output in `-TargetDir`; runs the tests of the crate; runs `win32_window --smoke` once for each rendering mode (`-Modes`; by default `software`, and `angle` when ANGLE was built); with `-Desktop` builds `ferroui-desktop` and runs `hello_window` for two seconds.
- **It writes a report** (`-Report`, plain text: the system, the tools, every step with its exit code and time, and the lines of the smoke runs) and the full output of every step beside it (`-LogDir`).
- **A window needs a desktop.** A process started by a service or by the tools of the virtual machine host runs in session 0, which has none: there the smoke run fails exactly the three checks of maximize and restore (no state change arrives; everything else, frames and input included, passes). With `-InteractiveUser <name>` the smoke runs are started in the session of that logged-on user, through a scheduled task that is created, run once and deleted.
- **The build output has to be on a disk of the machine.** On a folder shared from the host rustc fails at the first crate ("failed to build archive ... failed to remove temporary directory: The parameter is incorrect"). The sources may be on the share.

What the machine needs installed (done once, by the owner's authorisation, on 2026-10-10): Rust stable for `aarch64-pc-windows-msvc` from `https://win.rustup.rs/aarch64` (1.99.0, the version CI resolves; `RUSTUP_HOME=C:\Rust\rustup`, `CARGO_HOME=C:\Rust\cargo`, so that every account uses one installation), LLVM 23.1.3 for ARM64 from the LLVM project's releases (`C:\Program Files\LLVM`, for `libclang.dll`), and the Visual Studio Build Tools with the ARM64 C++ tools and the Windows SDK 10.0.26100, which the machine had.

### 10.5 What was verified where

| | CI x86-64 (`windows-latest`) | CI ARM64 (`windows-11-arm`) | Virtual machine (Windows 11 ARM64) |
|---|---|---|---|
| The crate builds and links, without ANGLE | yes | yes | yes |
| The tests of the crate on the target | yes | yes | VM-TESTS |
| Smoke run, software rendering: all 23 checks | yes (scaling 1) | yes (scaling 1) | yes, in the user's session (scaling 2) |
| `mozangle` builds | yes, 528 s | yes, 363 s | yes, 429 s |
| ANGLE probe: displays on Direct3D 11 and 9, a context, a cleared frame read back | yes (WARP) | yes (WARP) | VM-PROBE |
| Smoke run through ANGLE: frames through the platform graphics, read back with `glReadPixels` | CI-ANGLE-X64 | CI-ANGLE-ARM | VM-ANGLE |
| `ferroui-desktop` builds; `hello_window` | CI-DESKTOP | not run | not run |

Verified nowhere: a frame on the physical screen (nothing captures the desktop); the rendering modes that are not built; a GPU other than the software device of the runners and the device of the virtual machine; a 32-bit target; the adapter selection callback and the Adreno rule on a machine with more than one adapter (their logic is tested on the host); device loss.

## 11. Stages

Stage 1 is built and runs. Of stage 2, 2.0 is done but for the two items named in its row, 2a is built for what the GPU path needs, and 2b is built for ANGLE. The rest of the project follows in parts that each end with something the CI job proves; the order follows what unblocks the most.

| Stage | Content | The CI job proves |
|---|---|---|
| **1** (done 2026-10-10) | the crate and the bindings; platform initialisation and options; the dispatcher; windows, popups and embedded windows with their states and styles; screens; cursors; the framebuffer surface; clipboard text; mouse and keyboard input; `use_win32` and `use_platform_detect`; the example and the CI job | section 10.3 |
| **2.0** (2026-10-10, in part) | the first runs on Windows read and their one fault fixed (section 11.1); the Skia binary probe answered (section 6.2). Left: the informative steps of `ferroui-desktop` and `hello_window` made green and required; upstream's integration tests of standard windows and of the move drag ported as tests that run in the job (they need `ferroui-desktop` building there); the member extraction for the project (section 12) | the required steps of both Windows jobs |
| **2a** (2026-10-10, in part) | built: `extern "system"` vtables in MicroCom, the four interface definition files through the generator, `directx.idl` compiled with its enumerations, structures and functions (section 2.2). Left: `OleContext`, the Windows Runtime helpers, the rest of the platform settings (theme, accent, contrast, language and their change notifications) | the generator's tests; the device path of ANGLE uses the DXGI and Direct3D 11 bindings on the runner |
| **2b** (2026-10-10, ANGLE) | built: ANGLE from the crate `mozangle` (sections 6.1 and 6.2.1), `Win32GlManager`, `AngleOptions`, the EGL window surface of a window, the Ganesh GPU of the Skia backend enabled for Windows. Left: WGL (`OpenGl/Wgl*`, seven files) and its surface | frames drawn through ANGLE on the WARP device of the runner and read back (section 10.3) |
| 2c | composition: `dcomp.idl` and the composition part of `winrt.idl` compiled, the surface factory, the texture features of an ANGLE context (`AngleD3DTextureFeature`, `AngleExternalObjectsFeature`, `SwapChainGlSurface`), DirectComposition, Windows.UI.Composition with the blur effects, the DXGI swap chain, `SwapChainTopLevelImpl`; the transparency levels above `Transparent` | a window presented through each mode the runner supports, read back |
| 2d | data: the OLE data objects in both directions, the clipboard through them (every format, ownership, flushing), drag source and drop target, virtual files with upstream's tests | clipboard round trips of text, files and a custom format; `OleVirtualFileDataTests` |
| 2e | shell: icons (`IconImpl`, `Win32Icon`) and with them window icons, bitmap cursors and the icon loader; the taskbar list; tray icons; the storage provider (file dialogs) and the mounted volumes; the native control host; the menu exporter; display names | icons and tray icons created without error; dialogs are not testable without a person |
| 2f | input: the pointer and touch messages with history and the mouse history, the input method (`Imm32InputMethod`, the caret manager), the input pane, the custom caption procedure and with it the extended client area (`ExtendClientAreaWindowTests`) | synthetic pointer input (`InjectTouchInput`) if the runner allows it; upstream's extended client area tests |
| later | the automation project (`Avalonia.Win32.Automation`); Vulkan after the Vulkan project | |

### 11.1 What the first runs on Windows showed (2026-10-10)

Stage 1 was written on macOS without ever running. Its first run was the first run of the `windows` job; the second, with one fix, passed every check on three systems.

| Finding | Where | What was done |
|---|---|---|
| Showing a window panicked: "RefCell already borrowed" (`window_impl.rs`, `show_impl`). The parent of the window was read from its cell in the argument of the call that replaces what the cell holds; the borrow of an argument lasts until the call returns. | run 1, `windows-latest`: the first three checks passed (screens, the handle, two surfaces), then the panic | read in a statement of its own. The file was searched for the same shape; there is no other. |
| Everything else of stage 1 held as written: the imports link (`RtlGetVersion` included), platform initialisation, DPI awareness, the message window, the dispatcher, the window state machine, the input arms, the clipboard, the signal from another thread. | run 2: 23 of 23 checks on `windows-latest` (1024 by 768, scaling 1), on `windows-11-arm`, and in the virtual machine in the user's session (3592 by 2016, scaling 2: client 640 by 400 asked for and reported, maximized client 3592 by 1875 pixels against a working area of 3592 by 1920) | nothing |
| The hosted runners have an interactive desktop; session 0 of the virtual machine has none, and there the state changes of maximize and restore do not arrive (3 of 23 checks fail; frames, input, the clipboard and the dispatcher pass). | the virtual machine, run as the system account | the script starts the smoke runs in the user's session (section 10.4) |
| rustc cannot write its build output to a folder shared from the virtual machine host. | the virtual machine | the build output is on the disk of the machine (section 10.4) |

Windows has no native menu bar to export a menu to, so the native menu of a window on Windows is the managed menu, as upstream; `Win32NativeToManagedMenuExporter` (2e) is twelve lines that say so.

## 12. Tracking

`Avalonia.Win32` is in scope in `scripts/api-extract/projects.json` (the glob over `src/Windows` became four entries, the other three unchanged). The upstream extraction (`docs/porting/data/upstream-api.json`) was made while the project was out of scope and holds its files, its types and member counts, but no member names **[V]**; extracting them needs the .NET tool, which was not run for this work. Until it is (`scripts/port-status/run.sh --force`; the project list already asks for full detail), the tracking script matches the files and the types of the project and reports its members as a total (`port_status.py`: a project in scope with the detail `types`).

| | Before | After stage 1 | After stage 2b for ANGLE (2026-10-10) |
|---|---:|---:|---:|
| Files | 0/95 | 23/95 (24.2 %) | 33/95 (34.7 %) |
| Types | 0/276 | 80/276 (29.0 %) | 109/276 (39.5 %) |
| Members | 0/2605 | not measured: 2605 in total, names not extracted | not measured |

A file counts when it exists; a file that is built in part counts as a file and its missing members will count as missing once members are extracted. The type count is dominated by the interop file (119 types, most of them enumerations and structures of calls later stages make; 50 present).
