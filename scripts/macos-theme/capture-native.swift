#!/usr/bin/env swift
// Copyright (c) FerroUI contributors. Licensed under the MIT License.
// Native macOS 26 evidence collection. Build with a macOS 26 SDK.
// Never substitute a requested activation state for an observed one.
import AppKit
import Foundation

struct Options {
    var output = URL(fileURLWithPath: "/tmp/ferroui-macos26", isDirectory: true)
    var screenCapture = false
    var requireStateCoverage = false
    init(arguments: [String]) throws {
        var index = 0
        while index < arguments.count {
            switch arguments[index] {
            case "--output":
                index += 1
                guard index < arguments.count else { throw ProbeError.message("--output requires a directory") }
                output = URL(fileURLWithPath: arguments[index], isDirectory: true)
            case "--screen-capture": screenCapture = true
            case "--require-state-coverage": requireStateCoverage = true
            case "--help":
                print("capture-native.swift [--output DIRECTORY] [--screen-capture] [--require-state-coverage]")
                exit(0)
            default: throw ProbeError.message("Unknown option: \(arguments[index])")
            }
            index += 1
        }
    }
}

enum ProbeError: Error, CustomStringConvertible {
    case message(String)
    var description: String { switch self { case let .message(value): return value } }
}

struct RGBA: Codable {
    let red: CGFloat, green: CGFloat, blue: CGFloat, alpha: CGFloat
    init(_ color: NSColor) throws {
        guard let rgb = color.usingColorSpace(.sRGB) else { throw ProbeError.message("Cannot resolve sRGB color") }
        red = rgb.redComponent; green = rgb.greenComponent
        blue = rgb.blueComponent; alpha = rgb.alphaComponent
    }
}

struct ControlMetric: Codable {
    let identifier: String, size: String, className: String
    let enabled: Bool
    let width: CGFloat, height: CGFloat, intrinsicWidth: CGFloat, intrinsicHeight: CGFloat
    let fittingWidth: CGFloat, fittingHeight: CGFloat
    let fontName: String?
    let fontSize: CGFloat?, ascender: CGFloat?, descender: CGFloat?, leading: CGFloat?
    let alignmentInsets: [CGFloat]
}

struct SurfaceMetric: Codable {
    let className: String
    let boundsInContent: [CGFloat]
    let fullyVisible: Bool
}

struct Activation: Equatable {
    let app: Bool, key: Bool, main: Bool
    @MainActor init(_ application: NSApplication, _ window: NSWindow) {
        app = application.isActive; key = window.isKeyWindow; main = window.isMainWindow
    }
    func matches(_ requested: Bool) -> Bool { requested ? app && key && main : !key && !main }
}

struct Preferences: Equatable {
    let motion: Bool, transparency: Bool, contrast: Bool
    @MainActor init() {
        let workspace = NSWorkspace.shared
        motion = workspace.accessibilityDisplayShouldReduceMotion
        transparency = workspace.accessibilityDisplayShouldReduceTransparency
        contrast = workspace.accessibilityDisplayShouldIncreaseContrast
    }
}

struct Capture: Codable {
    let appearance: String
    let requestedActiveWindow: Bool, activeWindow: Bool, applicationActive: Bool, mainWindow: Bool
    let scale: CGFloat
    let viewImage: String, windowServerImage: String?
    let colors: [String: RGBA]
    let controls: [ControlMetric]
    let glassSurface: SurfaceMetric
}

struct Manifest: Codable {
    let schemaVersion: Int
    let osVersion: String, systemVersion: String
    let reduceMotion: Bool, reduceTransparency: Bool, increaseContrast: Bool
    let controlFontPolicy: String
    let captureNotes: [String]
    let captures: [Capture]
}

@MainActor
@available(macOS 26.0, *)
final class ReferenceFixture {
    let window: NSWindow
    private var controls: [(String, String, NSControl)] = []
    private let glass = NSGlassEffectView()

    init(appearance: NSAppearance) {
        window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 1040, height: 760),
                          styleMask: [.titled, .closable, .miniaturizable, .resizable],
                          backing: .buffered, defer: false)
        window.title = "FerroUI — macOS 26 native reference"
        window.appearance = appearance
        window.isReleasedWhenClosed = false
        for (id, type) in [("window-close", NSWindow.ButtonType.closeButton),
                           ("window-minimize", .miniaturizeButton), ("window-zoom", .zoomButton)] {
            if let button = window.standardWindowButton(type) { controls.append((id, "window-chrome", button)) }
        }
        let content = NSStackView()
        content.orientation = .vertical; content.alignment = .leading; content.spacing = 8
        content.edgeInsets = NSEdgeInsets(top: 12, left: 16, bottom: 12, right: 16)
        content.translatesAutoresizingMaskIntoConstraints = false
        let host = NSView()
        window.contentView = host
        host.addSubview(content)
        NSLayoutConstraint.activate([
            content.leadingAnchor.constraint(equalTo: host.leadingAnchor),
            content.trailingAnchor.constraint(equalTo: host.trailingAnchor),
            content.topAnchor.constraint(equalTo: host.topAnchor),
            content.bottomAnchor.constraint(lessThanOrEqualTo: host.bottomAnchor)
        ])
        let heading = NSTextField(labelWithString: "Native AppKit control metrics")
        heading.font = .boldSystemFont(ofSize: 18)
        content.addArrangedSubview(heading)
        let description = NSTextField(labelWithString: "macOS 26 • actual preferences • measured points, not normative constants")
        description.textColor = .secondaryLabelColor
        content.addArrangedSubview(description)
        for (name, size) in [("mini", NSControl.ControlSize.mini), ("small", .small),
                             ("regular", .regular), ("large", .large), ("extra-large", .extraLarge)] {
            let title = NSTextField(labelWithString: name)
            title.font = .boldSystemFont(ofSize: 13)
            content.addArrangedSubview(title)
            let first = row(), second = row()
            let push = NSButton(title: "Button", target: nil, action: nil)
            push.bezelStyle = .rounded
            add(push, "push", name, size, first)
            let primary = NSButton(title: "Default", target: nil, action: nil)
            primary.bezelStyle = .rounded; primary.keyEquivalent = "\r"
            add(primary, "default-push", name, size, first)
            let disabled = NSButton(title: "Disabled", target: nil, action: nil)
            disabled.bezelStyle = .rounded; disabled.isEnabled = false
            add(disabled, "disabled-push", name, size, first)
            let check = NSButton(checkboxWithTitle: "Checkbox", target: nil, action: nil)
            check.state = .on
            add(check, "checkbox-on", name, size, first)
            let mixed = NSButton(checkboxWithTitle: "Mixed", target: nil, action: nil)
            mixed.allowsMixedState = true; mixed.state = .mixed
            add(mixed, "checkbox-mixed", name, size, first)
            let radio = NSButton(radioButtonWithTitle: "Radio", target: nil, action: nil)
            radio.state = .on
            add(radio, "radio-on", name, size, first)
            let toggle = NSSwitch(); toggle.state = .on
            add(toggle, "switch-on", name, size, first)
            add(NSTextField(string: "Text field"), "text-field", name, size, second, width: 140)
            let search = NSSearchField(string: ""); search.placeholderString = "Search"
            add(search, "search-field", name, size, second, width: 140)
            let popup = NSPopUpButton(frame: .zero, pullsDown: false)
            popup.addItems(withTitles: ["First option", "Second option", "Third option"])
            add(popup, "popup", name, size, second)
            let segments = NSSegmentedControl(labels: ["Day", "Week", "Month"], trackingMode: .selectOne, target: nil, action: nil)
            segments.selectedSegment = 1
            add(segments, "segmented", name, size, second)
            add(NSSlider(value: 65, minValue: 0, maxValue: 100, target: nil, action: nil), "slider", name, size, second, width: 140)
            let stepper = NSStepper(); stepper.minValue = 0; stepper.maxValue = 100; stepper.doubleValue = 25
            add(stepper, "stepper", name, size, second)
            content.addArrangedSubview(first); content.addArrangedSubview(second)
        }
        let materialTitle = NSTextField(labelWithString: "Native glass — compositor comparisons require a WindowServer image")
        materialTitle.font = .boldSystemFont(ofSize: 13)
        content.addArrangedSubview(materialTitle)
        glass.cornerRadius = 12
        let glassContent = NSStackView()
        glassContent.edgeInsets = NSEdgeInsets(top: 14, left: 16, bottom: 14, right: 16)
        glassContent.addArrangedSubview(NSTextField(labelWithString: "NSGlassEffectView"))
        glass.contentView = glassContent
        content.addArrangedSubview(glass)
        // NSGlassEffectView has no useful intrinsic height for this stack. Never
        // publish an absent/collapsed surface as a successful material reference.
        NSLayoutConstraint.activate([
            glass.widthAnchor.constraint(equalToConstant: 600),
            glass.heightAnchor.constraint(equalToConstant: 64)
        ])
        window.center()
    }

    private func row() -> NSStackView {
        let result = NSStackView()
        result.orientation = .horizontal; result.alignment = .centerY; result.spacing = 12
        return result
    }

    private func add(_ control: NSControl, _ id: String, _ size: String, _ controlSize: NSControl.ControlSize,
                     _ row: NSStackView, width: CGFloat? = nil) {
        control.controlSize = controlSize
        control.font = .systemFont(ofSize: NSFont.systemFontSize(for: controlSize))
        control.sizeToFit()
        row.addArrangedSubview(control)
        if let width { control.widthAnchor.constraint(equalToConstant: width).isActive = true }
        control.setContentCompressionResistancePriority(.required, for: .vertical)
        controls.append((id, size, control))
    }

    func metrics() throws -> [ControlMetric] {
        guard let host = window.contentView else { throw ProbeError.message("Missing content host") }
        return try controls.map { id, size, control in
            let rect = control.convert(control.bounds, to: host)
            guard control.frame.width > 0, control.frame.height > 0,
                  size == "window-chrome" || host.bounds.contains(rect) else {
                throw ProbeError.message("Clipped/empty control \(id)/\(size); enlarge the capture display")
            }
            let intrinsic = control.intrinsicContentSize, fitting = control.fittingSize
            let font = control.font, insets = control.alignmentRectInsets
            return ControlMetric(identifier: id, size: size, className: NSStringFromClass(type(of: control)),
                enabled: control.isEnabled, width: control.frame.width, height: control.frame.height,
                intrinsicWidth: intrinsic.width, intrinsicHeight: intrinsic.height,
                fittingWidth: fitting.width, fittingHeight: fitting.height,
                fontName: font?.fontName, fontSize: font?.pointSize, ascender: font?.ascender,
                descender: font?.descender, leading: font?.leading,
                alignmentInsets: [insets.top, insets.left, insets.bottom, insets.right])
        }
    }

    func surfaceMetric() throws -> SurfaceMetric {
        guard let host = window.contentView else { throw ProbeError.message("Missing content host") }
        let bounds = glass.convert(glass.bounds, to: host)
        let visible = !glass.isHiddenOrHasHiddenAncestor && bounds.width > 0 && bounds.height > 0 && host.bounds.contains(bounds)
        guard visible else { throw ProbeError.message("Glass reference is clipped or empty; enlarge the capture display") }
        return SurfaceMetric(className: NSStringFromClass(type(of: glass)),
                             boundsInContent: [bounds.minX, bounds.minY, bounds.width, bounds.height], fullyVisible: visible)
    }
}

@MainActor
func pumpEvents(_ application: NSApplication, for seconds: TimeInterval) {
    let deadline = Date(timeIntervalSinceNow: seconds)
    repeat {
        if let event = application.nextEvent(matching: .any, until: Date(timeIntervalSinceNow: 0.01), inMode: .default, dequeue: true) {
            application.sendEvent(event)
        }
        application.updateWindows()
        RunLoop.current.run(until: Date(timeIntervalSinceNow: 0.01))
    } while Date() < deadline
}

@MainActor
func requestActivation(_ application: NSApplication, window: NSWindow, focus: NSWindow, active: Bool) {
    application.activate(ignoringOtherApps: true)
    let target = active ? window : focus
    target.makeKeyAndOrderFront(nil)
    target.makeMain()
    let deadline = Date(timeIntervalSinceNow: 2.0)
    repeat {
        pumpEvents(application, for: 0.05)
        if Activation(application, window).matches(active) { break }
    } while Date() < deadline
    // Allow appearance invalidation to draw after activation. This is bounded;
    // a headless session that refuses activation must remain an observed failure.
    pumpEvents(application, for: 0.2)
}

@MainActor
func captureView(_ window: NSWindow, to url: URL) throws {
    guard let view = window.contentView?.superview else { throw ProbeError.message("Window has no frame view") }
    view.layoutSubtreeIfNeeded()
    guard let bitmap = view.bitmapImageRepForCachingDisplay(in: view.bounds) else { throw ProbeError.message("Cannot allocate bitmap") }
    view.cacheDisplay(in: view.bounds, to: bitmap)
    guard let png = bitmap.representation(using: .png, properties: [:]) else { throw ProbeError.message("Cannot encode PNG") }
    try png.write(to: url, options: .atomic)
}

func captureWindow(_ number: Int, to url: URL) throws {
    // Do not let a failed capture be mistaken for an older file in the directory.
    if FileManager.default.fileExists(atPath: url.path) { try FileManager.default.removeItem(at: url) }
    let task = Process()
    task.executableURL = URL(fileURLWithPath: "/usr/sbin/screencapture")
    task.arguments = ["-x", "-o", "-l", String(number), url.path]
    try task.run(); task.waitUntilExit()
    guard task.terminationStatus == 0, FileManager.default.fileExists(atPath: url.path) else {
        throw ProbeError.message("WindowServer capture failed; Screen Recording permission may be required")
    }
}

@MainActor
func colors(appearance: NSAppearance) throws -> [String: RGBA] {
    let semantic: [(String, NSColor)] = [
        ("controlAccent", .controlAccentColor), ("windowBackground", .windowBackgroundColor),
        ("controlBackground", .controlBackgroundColor), ("control", .controlColor),
        ("controlText", .controlTextColor), ("disabledControlText", .disabledControlTextColor),
        ("label", .labelColor), ("secondaryLabel", .secondaryLabelColor),
        ("tertiaryLabel", .tertiaryLabelColor), ("quaternaryLabel", .quaternaryLabelColor),
        ("keyboardFocusIndicator", .keyboardFocusIndicatorColor), ("separator", .separatorColor),
        ("selectedContentBackground", .selectedContentBackgroundColor),
        ("unemphasizedSelectedContentBackground", .unemphasizedSelectedContentBackgroundColor),
        ("selectedTextBackground", .selectedTextBackgroundColor), ("selectedText", .selectedTextColor),
        ("text", .textColor), ("textBackground", .textBackgroundColor),
        ("placeholderText", .placeholderTextColor), ("link", .linkColor)
    ]
    var result: [String: RGBA] = [:]
    var failure: Error?
    appearance.performAsCurrentDrawingAppearance {
        do { for (name, color) in semantic { result[name] = try RGBA(color) } }
        catch { failure = error }
    }
    if let failure { throw failure }
    return result
}

@MainActor
func run() throws {
    let options = try Options(arguments: Array(CommandLine.arguments.dropFirst()))
    let version = ProcessInfo.processInfo.operatingSystemVersion
    guard version.majorVersion == 26 else { throw ProbeError.message("Expected macOS 26, not \(version.majorVersion)") }
    guard #available(macOS 26.0, *) else { return }
    try FileManager.default.createDirectory(at: options.output, withIntermediateDirectories: true)
    let manifestURL = options.output.appendingPathComponent("native-reference.json")
    // An interrupted run must not leave a stale success manifest beside new images.
    if FileManager.default.fileExists(atPath: manifestURL.path) { try FileManager.default.removeItem(at: manifestURL) }
    let application = NSApplication.shared
    application.setActivationPolicy(.regular)
    application.finishLaunching()
    let preferences = Preferences()
    var captures: [Capture] = []
    var activationFailures = 0
    let appearances: [(String, NSAppearance.Name)] = [
        ("light", preferences.contrast ? .accessibilityHighContrastAqua : .aqua),
        ("dark", preferences.contrast ? .accessibilityHighContrastDarkAqua : .darkAqua)
    ]
    for (name, appearanceName) in appearances {
        guard let appearance = NSAppearance(named: appearanceName) else { throw ProbeError.message("Missing appearance \(name)") }
        let fixture = ReferenceFixture(appearance: appearance)
        let focus = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 160, height: 80),
                             styleMask: [.titled], backing: .buffered, defer: false)
        focus.isReleasedWhenClosed = false
        focus.title = "Inactive reference"
        // Keep the auxiliary focus window away from the reference content.
        focus.setFrameOrigin(NSPoint(x: 0, y: 0))
        defer { focus.close(); fixture.window.close() }
        fixture.window.orderFront(nil)
        for requested in [true, false] {
            requestActivation(application, window: fixture.window, focus: focus, active: requested)
            fixture.window.contentView?.layoutSubtreeIfNeeded()
            let before = Activation(application, fixture.window)
            let metrics = try fixture.metrics(), surface = try fixture.surfaceMetric()
            if !before.matches(requested) { activationFailures += 1 }
            let stem = name + "-requested-" + (requested ? "active" : "inactive")
                + "-observed-" + (before.key ? "active" : "inactive")
            let viewName = stem + "-view.png"
            try captureView(fixture.window, to: options.output.appendingPathComponent(viewName))
            var serverName: String?
            if options.screenCapture {
                let filename = stem + "-window.png"
                try captureWindow(fixture.window.windowNumber, to: options.output.appendingPathComponent(filename))
                serverName = filename
            }
            guard before == Activation(application, fixture.window), preferences == Preferences() else {
                throw ProbeError.message("Activation/accessibility changed during capture; rerun the fixture")
            }
            captures.append(Capture(appearance: name, requestedActiveWindow: requested,
                activeWindow: before.key, applicationActive: before.app, mainWindow: before.main,
                scale: fixture.window.backingScaleFactor, viewImage: viewName, windowServerImage: serverName,
                colors: try colors(appearance: appearance), controls: metrics, glassSurface: surface))
        }
    }
    let manifest = Manifest(schemaVersion: 2,
        osVersion: "\(version.majorVersion).\(version.minorVersion).\(version.patchVersion)",
        systemVersion: ProcessInfo.processInfo.operatingSystemVersionString,
        reduceMotion: preferences.motion, reduceTransparency: preferences.transparency, increaseContrast: preferences.contrast,
        controlFontPolicy: "Explicit NSFont.systemFont(ofSize: NSFont.systemFontSize(for: controlSize)); recorded per control",
        captureNotes: [
            "Measured AppKit values, not normative constants or evidence of FerroUI equivalence.",
            "Requested and observed activation are distinct. Failed activation is retained as diagnostic evidence.",
            "View cache captures may omit compositor effects; optics require separate WindowServer images.",
            "No hover/pressed sequences, open menus, sheets, or motion trajectories are measured.",
            "Accessibility preferences are observed and never changed by this fixture."
        ], captures: captures)
    let encoder = JSONEncoder(); encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
    try encoder.encode(manifest).write(to: manifestURL, options: .atomic)
    print(manifestURL.path)
    if activationFailures > 0 {
        let message = "\(activationFailures) activation requests failed; the manifest records actual state"
        if options.requireStateCoverage { throw ProbeError.message(message) }
        fputs("Warning: \(message)\n", stderr)
    }
}

do { try MainActor.assumeIsolated { try run() } }
catch { fputs("macOS 26 capture: \(error)\n", stderr); exit(1) }
