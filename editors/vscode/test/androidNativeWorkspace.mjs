import { writeNativeMobileApp } from './mobileNativeWorkspace.mjs';

export async function androidNativeWorkspace(workspace, toolkit, adbPath) {
    await writeNativeMobileApp(workspace, toolkit, {
        name: 'android-inspector', kind: 'cdylib',
        platform: { 'ferroui-android': ['src/Android/FerroUI.Android', ['live-inspector']] },
        example: 'src/Android/FerroUI.Android/examples/android_inspector.rs',
        subtitle: 'TextBlock { text: "Review open cases for your team", foreground: "#405679" }',
    });
    return {
        'ferroui.android.adbPath': adbPath,
        'ferroui.android.apps': [{ id: 'service-review', label: 'Managed Service Review', packageId: 'org.ferroui.androidmanagedreview',
            build: { package: 'android-inspector', toolchain: toolkit, debugger: true, reloadOnSave: true } }],
    };
}
