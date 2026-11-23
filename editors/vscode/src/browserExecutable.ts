import { constants } from 'node:fs';
import { access } from 'node:fs/promises';
import { homedir } from 'node:os';
import * as path from 'node:path';
import * as vscode from 'vscode';

export async function browserExecutable(): Promise<string> {
    const configured = vscode.workspace.getConfiguration('ferroui.browser').get<string>('executablePath', '').trim();
    if (configured && !path.isAbsolute(configured)) throw new Error('Set ferroui.browser.executablePath to an absolute Chromium executable path.');
    const candidates = configured ? [configured] : [process.env.CHROME, ...defaultBrowsers(), ...pathBrowsers()];
    for (const entry of candidates) {
        if (!entry || !path.isAbsolute(entry)) continue;
        try { await access(entry, constants.X_OK); return entry; }
        catch { /* Try the next local browser. */ }
    }
    throw new Error('Install Chrome or Chromium on the workspace host or set ferroui.browser.executablePath.');
}

function defaultBrowsers(): string[] {
    if (process.platform === 'darwin') {
        const chrome = 'Google Chrome.app/Contents/MacOS/Google Chrome';
        return [path.join('/Applications', chrome), path.join(homedir(), 'Applications', chrome), '/Applications/Chromium.app/Contents/MacOS/Chromium'];
    }
    if (process.platform === 'win32') {
        return [process.env.PROGRAMFILES, process.env['PROGRAMFILES(X86)'], process.env.LOCALAPPDATA]
            .filter((root): root is string => Boolean(root))
            .flatMap(root => ['Google/Chrome/Application/chrome.exe', 'Chromium/Application/chrome.exe'].map(browser => path.join(root, browser)));
    }
    return ['/usr/bin/google-chrome', '/usr/bin/google-chrome-stable', '/usr/bin/chromium', '/usr/bin/chromium-browser'];
}

function pathBrowsers(): string[] {
    const names = process.platform === 'win32' ? ['chrome.exe', 'chromium.exe'] : ['google-chrome', 'chromium', 'chrome'];
    return (process.env.PATH ?? '').split(path.delimiter).filter(root => path.isAbsolute(root))
        .flatMap(root => names.map(name => path.join(root, name)));
}
