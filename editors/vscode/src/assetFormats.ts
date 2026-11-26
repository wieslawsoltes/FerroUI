import * as path from 'node:path';

const rasterMime = new Map<string, string>([
    ['.png', 'image/png'], ['.jpeg', 'image/jpeg'], ['.jpg', 'image/jpeg'],
    ['.webp', 'image/webp'], ['.gif', 'image/gif'], ['.bmp', 'image/bmp'], ['.ico', 'image/x-icon'],
]);
const formats = {
    Image: [...rasterMime.keys(), '.svg'],
    Font: ['.ttf', '.otf', '.woff', '.woff2'],
    Media: ['.wav', '.mp3', '.ogg', '.mp4', '.webm'],
    XAML: ['.xaml', '.axaml', '.fxaml'],
};
const excluded = ['.git', 'target', 'node_modules', '.vs', 'bin', 'obj', 'dist', '.ferroui'];

const suffixes = Object.values(formats).flat().map(ext => Array.from(ext.slice(1), char => {
    const upper = char.toUpperCase();
    return upper === char ? char : `[${char}${upper}]`;
}).join(''));
export const assetFiles = `**/*.{${suffixes.join(',')}}`;
export const ignoredAssets = `**/{${excluded.join(',')}}/**`;
export const assetFilters = Object.fromEntries(Object.entries(formats).map(([kind, exts]) => [kind, exts.map(ext => ext.slice(1))]));

export function assetFormat(file: string) {
    const extension = path.extname(file).toLowerCase();
    const kind = Object.entries(formats).find(([, exts]) => exts.includes(extension))?.[0];
    if (kind) return { kind, extension, mime: rasterMime.get(extension) };
}

export function isAssetLocation(relative: string): boolean {
    return !relative.split(path.sep).slice(0, -1).some(part => excluded.includes(part));
}
