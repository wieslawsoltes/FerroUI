import { execFileSync } from 'node:child_process';
import { writeNativeMobileApp } from './mobileNativeWorkspace.mjs';

export async function iosNativeWorkspace(workspace, toolkit) {
    const name = 'contact-review';
    const target = process.arch === 'arm64' ? 'aarch64-apple-ios-sim' : 'x86_64-apple-ios';
    await writeNativeMobileApp(workspace, toolkit, {
        name, kind: 'bin',
        platform: { 'ferroui-ios': ['src/iOS/FerroUI.iOS', ['quartz-metal'], false] },
        example: 'src/iOS/FerroUI.iOS/examples/simulator_inspector.rs',
        subtitle: 'TextBlock { text: "Renewal queue", foreground: "#214B40", font_size: 19.0 }',
        profile: ['[profile.dev]', 'debug = 1'],
    });
    // Resolve the fixture package into the copied lock before the production packager uses --locked.
    execFileSync('cargo', ['metadata', '--format-version', '1', '--filter-platform', target], {
        cwd: workspace, stdio: ['ignore', 'ignore', 'inherit'], timeout: 120_000,
    });
    return {
        'ferroui.iosSimulator.apps': [{ id: name, label: 'Managed Contact Review', bundleId: 'org.ferroui.managed-contact-review',
            args: ['--mcp', '--mcp-input'], build: { package: name, binary: name, toolchain: toolkit, target, reloadOnSave: true } }],
    };
}
