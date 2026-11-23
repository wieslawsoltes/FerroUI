import { isIP } from 'node:net';
import * as vscode from 'vscode';
import { workspaceFolders } from './workspacePaths';

interface BrowserProfile {
    id: string;
    folder: string;
    label: string;
    width: number;
    height: number;
}

export interface BrowserBuildProfile {
    application: string;
    toolchain: string;
    manifestPath: string;
    features: string[];
    threads: boolean;
}

export interface ServedBrowserApp extends BrowserProfile {
    url: string;
    debugPort?: number;
    build?: never;
}

export interface DevelopmentBrowserApp extends BrowserProfile {
    build: BrowserBuildProfile;
    url?: never;
    debugPort?: never;
}

export type BrowserApp = ServedBrowserApp | DevelopmentBrowserApp;

export interface BrowserRequest {
    profile: string;
    folder?: string;
}

export type BrowserMode = 'run' | 'debug' | 'attach';

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
    const { id, label, url, width = 960, height = 620, debugPort, build } = (value ?? {}) as Record<string, unknown>;
    const invalid = (message: string) => new Error(`Check the browser app profile in ${folder.name}. ${message}`);
    if (typeof id !== 'string' || !/^[A-Za-z0-9][A-Za-z0-9_-]{0,63}$/.test(id)
        || label !== undefined && (typeof label !== 'string' || !label.trim() || label.length > 96)
        || !within(width, 240, 2_560) || !within(height, 240, 1_600)) {
        throw invalid('Use an ID and a supported viewport size.');
    }
    const profile = { id, folder: folder.uri.toString(), width, height };
    if (build !== undefined) {
        if (url !== undefined || debugPort !== undefined) throw invalid('Choose a managed build or a served app URL.');
        const managed = buildProfile(build);
        if (!managed) throw invalid('Choose a browser application and valid build settings.');
        return { ...profile, label: typeof label === 'string' ? label.trim() : managed.application, build: managed };
    }
    const page = loopbackPage(url);
    if (!page || debugPort !== undefined && !within(debugPort, 1, 65_535)) {
        throw invalid('Use a loopback URL and an optional Chromium debug port.');
    }
    return { ...profile, url: page.href, label: typeof label === 'string' ? label.trim() : page.hostname, debugPort };
}

function buildProfile(value: unknown): BrowserBuildProfile | undefined {
    if (!value || typeof value !== 'object' || Array.isArray(value)) return;
    const { application, toolchain = '.', manifestPath = 'Cargo.toml', features = [], threads = false } = value as Record<string, unknown>;
    if (typeof application !== 'string' || !/^[A-Za-z0-9_][A-Za-z0-9_-]{0,95}$/.test(application)
        || !location(toolchain) || !location(manifestPath) || typeof threads !== 'boolean'
        || !Array.isArray(features) || features.length > 64
        || !features.every((feature): feature is string => typeof feature === 'string' && /^[A-Za-z0-9_-]{1,96}$/.test(feature))) return;
    return { application, toolchain, manifestPath, features: [...new Set(features)].sort(), threads };
}

function location(value: unknown): value is string {
    return typeof value === 'string' && value.trim() !== '' && value.length <= 4_096 && !/[\u0000\r\n]/.test(value);
}
