import * as vscode from 'vscode';
import { isViewport, type DesignerViewState } from './protocol';
import { asPosition } from './textDocument';

export const DESIGNER_VIEW = 'ferroui.designer';

export function restoredDesigner(value: unknown) {
    if (!value || typeof value !== 'object') return undefined;
    const { version, uri, caret, viewport, designData } = value as Partial<DesignerViewState>;
    if (version !== 1 || typeof uri !== 'string' || !caret || !isViewport(viewport)
        || ![caret.line, caret.character].every(number => Number.isSafeInteger(number) && number >= 0)) return undefined;
    const source = vscode.Uri.parse(uri);
    if (source.scheme !== 'file' || !source.path || source.query || source.fragment) return undefined;
    const data = designData && typeof designData.enabled === 'boolean'
        && (designData.fixture === undefined || (typeof designData.fixture === 'string' && designData.fixture.length > 0
            && Buffer.byteLength(designData.fixture) <= 128 && designData.fixture.trim() === designData.fixture
            && !/[\u0000-\u001f\u007f-\u009f]/u.test(designData.fixture))) ? designData : undefined;
    return { uri: source, caret: asPosition(caret), viewport, designData: data };
}
