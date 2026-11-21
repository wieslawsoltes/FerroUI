import * as path from 'node:path';
import { pathToFileURL } from 'node:url';
import * as vscode from 'vscode';
import type { StageManifest } from './stagesProtocol';

export interface StageFileMove { oldUri: vscode.Uri; newUri: vscode.Uri }

export function sameStageSource(first: vscode.Uri, second: vscode.Uri): boolean {
    return first.scheme === 'file' && second.scheme === 'file' && path.relative(first.fsPath, second.fsPath) === '';
}

export function stageUsesSource(manifest: StageManifest | undefined, uri: vscode.Uri, moves: readonly StageFileMove[]): boolean {
    return manifest?.stages.some(stage => {
        const source = vscode.Uri.parse(stage.source_uri, true);
        return sameStageSource(relocated(source, moves) ?? source, uri);
    }) ?? false;
}

export function retargetStageManifest(manifest: StageManifest | undefined, moves: readonly StageFileMove[]) {
    if (!manifest || !moves.length) return { manifest, bindings: undefined };
    const bindings: Record<string, string> = Object.create(null);
    const stages = manifest.stages.map(stage => {
        const destination = relocated(vscode.Uri.parse(stage.source_uri, true), moves);
        if (!destination) return stage;
        const source_uri = pathToFileURL(destination.fsPath).toString();
        bindings[stage.source_uri] = source_uri;
        return { ...stage, source_uri };
    });
    if (!Object.keys(bindings).length) return { manifest, bindings: undefined };
    return { manifest: { ...manifest, stages }, bindings };
}

function relocated(source: vscode.Uri, moves: readonly StageFileMove[]): vscode.Uri | undefined {
    let destination: vscode.Uri | undefined;
    for (const move of moves) {
        if (sameStageSource(source, move.oldUri)) source = destination = move.newUri;
    }
    return destination;
}
