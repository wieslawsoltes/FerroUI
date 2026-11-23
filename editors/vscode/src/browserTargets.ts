import { isIP } from 'node:net';
import * as vscode from 'vscode';
import { workspaceFolders } from './workspacePaths';

export interface BrowserApp {
    id: string;
    folder: string;
    label: string;
    url: string;
    width: number;
    height: number;
    debugPort?: number;
}

export interface BrowserRequest {
    profile: string;
    folder?: string;
}

export type BrowserMode = 'run' | 'attach';

export function hasBrowserProfiles(): boolean {
    return workspaceFolders().some(folder => settings(folder).length > 0);
}

export function browserProfiles(folder?: string): BrowserApp[] {
    return workspaceFolders(folder).flatMap(workspace => {
        const profiles = settings(workspace).map(value => appProfile(value, workspace));
        if (new Set(profiles.map(profile => profile.id)).size !== profiles.length) {
            throw new Error(`Give the browser app profiles in ${workspace.name} unique IDs.`);
        }
        return profiles;
    });
}

function settings(folder: vscode.WorkspaceFolder): unknown[] {
    const raw = vscode.workspace.getConfiguration('ferroui.browser', folder.uri).get('apps');
    return Array.isArray(raw) ? raw : [];
}

export function loopbackPage(value: unknown): URL | undefined {
    if (typeof value !== 'string' || value.length > 2_048) return;
    try {
        const url = new URL(value);
        const host = url.hostname;
        if ((url.protocol === 'http:' || url.protocol === 'https:') && !url.username && !url.password
            && (host === 'localhost' || host === '[::1]' || isIP(host) === 4 && host.startsWith('127.'))) return url;
    } catch { /* Invalid app address. */ }
}

function within(value: unknown, minimum: number, maximum: number): value is number {
    return typeof value === 'number' && Number.isInteger(value) && value >= minimum && value <= maximum;
}

function appProfile(value: unknown, folder: vscode.WorkspaceFolder): BrowserApp {
    const { id, label, url, width = 960, height = 620, debugPort } = (value ?? {}) as Record<string, unknown>;
    const page = loopbackPage(url);
    if (!page || typeof id !== 'string' || !/^[A-Za-z0-9][A-Za-z0-9_-]{0,63}$/.test(id)
        || label !== undefined && (typeof label !== 'string' || !label.trim() || label.length > 96)
        || !within(width, 240, 2_560) || !within(height, 240, 1_600)
        || debugPort !== undefined && !within(debugPort, 1, 65_535)) {
        throw new Error(`Check the browser app profile in ${folder.name}. Use an ID, a loopback URL, and optional viewport or debug port.`);
    }
    return { id, folder: folder.uri.toString(), url: page.href, label: typeof label === 'string' ? label.trim() : page.hostname,
        width, height, debugPort };
}
