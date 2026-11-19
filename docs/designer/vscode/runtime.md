---
title: "Run and test a FerroUI view in VS Code"
description: "Render with your application's registry, inspect live windows, hot reload or debug a Cargo target, and run native UI scenarios."
---

Use App Preview for a view mounted with your app’s registered controls and data. Use Live App for the actual application windows of a managed or attached session. Both are available beside the source editor.

## Render with your app in App Preview

1. Provide an opted-in Cargo binary that calls `RemoteDesignerEntryPoint::run_if_requested` from `ferroui-designer-support`. Register the app’s loader, types, assets, and theme in that entry point. The [Todo Workbench setup](../../../samples/TodoWorkbench/README.md) includes a working `app-preview` target.
2. Open the XAML or mapped RSX file and run **FerroUI: Open App-Hosted Design**. Select the binary with the preview feature enabled. If its registered markup assembly differs from its Cargo package name, set `ferroui.hostedPreview.assembly`.
3. Choose **Source preview** to mount the current buffer. For app-supplied state, use a registered data context fixture or Rust view factory. DataContext fixtures render the current source against an app model. RustView fixtures mount an actual compiled Rust view, including dynamic children and handlers.
4. Send native input to the frame. For supported animations, choose the clip and target in the toolbar, then play, pause, or enter a pose with the elapsed playhead. Use **Animation graph** to choose **Value curves** or **Velocity curves**. Both displays sample the app’s animator at authored times while the elapsed playhead stays in place. Drag a supported key or open **Edit application keyframe**. Apply a validated source-backed XAML or mapped Rust change in one Undo step. Agents can request `metric: "velocity"` inside the `designer_hosted_curve` arguments.

For a supported geometry track, use the component picker beside its scale. The app’s registered animator supplies the samples. Agents can read the track’s `curve_channels` from `designer_hosted_animations` and pass the selected component as `channel` to `designer_hosted_curve`.

App Preview holds its accepted image through compilation and invalid edits. For a compatible replacement, the first frame can restore the paused pose, selected animation, and targets. A playing preview resumes its elapsed position when the replacement supports it. Test the hosted examples with [HostedPreview.xaml](../../../samples/TodoWorkbench/HostedPreview.xaml) and its app fixtures. [App preview integration and registered animator examples](../../../samples/TodoWorkbench/README.md)

The App Preview transport also lists mapped Rust state routes. Choose one to scrub, play, and edit its timing with the app’s real control properties and animation registry. A compiled Rust fixture uses its ordinary application clock for input and state handlers. [App Preview state-route workflow](motion.md#scrub-a-rust-state-route-in-app-preview)

## Hot reload and debug the running application

1. From a trusted Cargo workspace, run **FerroUI: Run with Hot Reload**. Select a development binary. Try `hot-reload-demo` for registered XAML or `view-gallery` for reactive RSX. Use `ferroui.hotReload.targets` to name an exact Cargo manifest, package, binary, feature set, or argument list.
2. Save a change to a registered XAML view or resource, or change an eligible `rsx!` literal read by a mounted reactive view. Inspect the outcome in the status bar, **Hot Reload Output**, or **Diagnostics**. An accepted in-place update keeps the app’s PID. An unsupported structural edit or Rust logic change builds a candidate. The currently running app remains available until the new one acknowledges startup.
3. If the save is rejected or the build fails, inspect the logged message and correct the file. Save again or run **Rebuild Hot Reload Application**. A successful structural rebuild gets a new PID. It retains compatible application state when the app supplies the corresponding handoff or persistence.
4. To debug, configure a Rust attach request in `launch.json` using `${command:ferroui.hotReload.processId}` for the adapter’s process ID field. Run **Run and Debug with Hot Reload**, or choose **Attach Debugger to Managed App** for an existing session. The extension reuses the selected attach configuration after a replacement acknowledges startup.
5. Use **Stop Hot Reload Application** to end the managed session. For supported projects with explicit Rust patch slots, choose **Run with Registered Rust Patches** to start the owning supervisor. Configure its Python and pinned patch compiler when needed.

The XAML app must register a `ReloadManifest` and a `HotReloadSession` and acknowledge its startup. For RSX, use a debug binary with `ferroui-view`’s `hot-reload` feature and a compatible `launch` integration. [Framework reload prerequisites and registered patch workflow](../../development/hot-reload.md)

## Inspect the actual window in Live App

1. Start the managed development binary or run **FerroUI: Connect to Running MCP Server** to attach an authenticated loopback inspector. Use the `--mcp` flags of a participating app such as Todo Workbench to start its server. **Open Live App** opens the supported application windows in the editor. Pick a window from its toolbar.
2. In **Inspect**, select a control in the native frame or browse the visual, logical, and accessibility trees. Alt+click chooses among overlapping native hits. Read properties, their effective style values, resources, accessible names, roles, and native diagnostics. Switch to **Interact** for clicks, scroll, text, or keys when the app permits input.
3. Try a property override in the inspector. Just the running session receives the override. Use **Reveal Source** to locate the authored value. When the current revision and source mapping match, use **Apply to Source** to write the supported RSX literal or XAML value into the editor. Review it, then save if you want hot reload to apply the source change.

Managed debug builds record compiled RSX locations. Set `FERROUI_EDITOR_SOURCE=1` for an externally launched debug build that needs RSX source reveal. XAML source application requires compiled or runtime-loaded source information. If a change is stale or its target is untracked, reload the running app’s compatible revision or edit its authored source directly. [Runtime inspector and permissions](../../mcp.md)

## Read diagnostics and save history

Open **FerroUI → Diagnostics** to inspect the managed save outcome, acknowledgement time, binding errors, and native layout and scene counters. Open a binding result with a recorded source location to inspect its original declaration. **Show Hot Reload Output** includes build and process messages. Use **Preview Checks** in a design or stage for native accessible name, measured clipping, and supported contrast checks against the accepted preview.

## Run a native UI scenario

1. Open the [customer search scenario](../../../editors/vscode/test/fixtures/customer-search.ferroui-scenario.json) in the `editors/vscode/test/fixtures` folder as a workspace. The **FerroUI Scenarios** section in Testing lists saved `*.ferroui-scenario.json` files.
2. If the scenario needs its first baseline, choose **Accept FerroUI Screenshot Baselines** in the Testing run-profile picker. Then run the scenario. It mounts saved source in a fresh native preview, drives input, checks the rendered properties and accessibility tree, and compares images for screenshot assertions. A visual mismatch links to the expected image, actual image, and diff.
3. Run the same file in CI. Relative document and baseline paths resolve from the scenario file. Set `--workspace` for a scenario nested under its workspace root and `--artifacts` for output images. A failed assertion gives the command a failing status.

```sh
ferroui-designer-preview --scenario customer-search.ferroui-scenario.json
```

A Rust scenario can select an RSX invocation with a zero-based UTF-16 `caret`. Use `channelTolerance` or `maxChangedPixels` when a fixture allows image variation. To test a `.ferrostages` flow with the same engine, use `prototype`, a stage set and flow, then `activateLink`, `expectStage`, and `expectOverlay`. The [customer review scenario](../../../editors/vscode/test/fixtures/customer-review.ferroui-scenario.json) exercises two stages and an overlay. [Scenario format and CLI options](../../../editors/vscode/README.md#native-ui-scenarios)
