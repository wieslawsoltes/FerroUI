import { createHash } from 'node:crypto';
import type * as vscode from 'vscode';
import type { CanvasGuidesState, GuideCatalog, GuidePreferences } from './canvasGuidesProtocol';
import type { FrameTarget, Position } from './protocol';
import { asPosition, offsetAfterChanges } from './textDocument';

export type GuideAnchor = { kind: 'document' } | { kind: 'rsx' | 'definition'; start: Position };
interface GuideViewKey { kind: GuideAnchor['kind']; offset: number }
interface GuideView extends GuideViewKey { preferences: GuidePreferences }
export interface GuideViewsState { stamp?: string; views: GuideView[] }
const maxViews = 24;

/** Keeps view preferences in the editor tab, scoped by authored source rather than render IDs. */
export class GuideViews {
    private views: GuideView[] = [];
    private restored?: GuideViewsState;
    private stamp?: { version: number; hash: string };
    private epoch = 0;

    constructor(private readonly document: vscode.TextDocument, restored: unknown) {
        if (restored && typeof restored === 'object') {
            const state = restored as GuideViewsState;
            if (Array.isArray(state.views) && state.views.length <= maxViews) this.restored = state;
        }
    }

    sourceChanged(changes: readonly vscode.TextDocumentContentChangeEvent[]): void {
        // If the first preview has yet to open, offsets restored for the old source are stale.
        if (this.restored) this.restored = { views: this.restored.views.filter(view => view?.kind === 'document') };
        this.views = this.views.flatMap(view => {
            if (view.kind === 'document') return [view];
            if (changes.some(change => change.rangeOffset <= view.offset && view.offset < change.rangeOffset + change.rangeLength)) return [];
            return [{ ...view, offset: offsetAfterChanges(view.offset, changes) }];
        });
    }

    state(frame: FrameTarget, anchor: GuideAnchor, catalog: GuideCatalog): CanvasGuidesState {
        this.restore(catalog);
        const key = this.key(anchor);
        const view = this.views.find(candidate => sameView(candidate, key));
        const { defaults, ...limits } = catalog;
        return { frame, scope: anchor.kind, epoch: this.epoch, settings: { ...defaults, ...view?.preferences }, ...limits };
    }

    remember(anchor: GuideAnchor, preferences: GuidePreferences): void {
        const key = this.key(anchor);
        const { rulers_visible, guides_visible, guides } = preferences;
        this.views = [{ ...key, preferences: { rulers_visible, guides_visible, guides } },
            ...this.views.filter(view => !sameView(view, key))].slice(0, maxViews);
        this.epoch++;
    }

    get saved(): GuideViewsState | undefined {
        if (this.restored) return this.restored;
        if (this.views.length) return { views: this.views,
            stamp: this.views.some(view => view.kind !== 'document') ? this.sourceStamp() : undefined };
    }

    private key(anchor: GuideAnchor): GuideViewKey {
        return { kind: anchor.kind, offset: anchor.kind === 'document' ? -1 : this.document.offsetAt(asPosition(anchor.start)) };
    }

    private restore(catalog: GuideCatalog): void {
        const saved = this.restored;
        if (!saved) return;
        this.restored = undefined;
        const sameSource = typeof saved.stamp === 'string' && saved.stamp === this.sourceStamp();
        const length = sameSource ? this.document.getText().length : 0;
        this.views = saved.views.filter(view => view && validPreferences(view.preferences, catalog)
            && (view.kind === 'document' && view.offset === -1 || sameSource
                && (view.kind === 'rsx' || view.kind === 'definition')
                && Number.isSafeInteger(view.offset) && view.offset >= 0 && view.offset < length));
    }

    private sourceStamp(): string {
        if (this.stamp?.version !== this.document.version) {
            this.stamp = { version: this.document.version, hash: createHash('sha256').update(this.document.getText()).digest('hex') };
        }
        return this.stamp.hash;
    }
}

function sameView(a: GuideViewKey, b: GuideViewKey): boolean { return a.kind === b.kind && a.offset === b.offset; }

function validPreferences(value: unknown, catalog: GuideCatalog): value is GuidePreferences {
    if (!value || typeof value !== 'object') return false;
    const { rulers_visible, guides_visible, guides } = value as GuidePreferences;
    return typeof rulers_visible === 'boolean' && typeof guides_visible === 'boolean'
        && Array.isArray(guides) && guides.length <= catalog.maxGuides
        && guides.every(guide => guide && Number.isSafeInteger(guide.id) && guide.id > 0
            && (guide.axis === 'Horizontal' || guide.axis === 'Vertical')
            && Number.isFinite(guide.position) && Math.abs(guide.position) <= catalog.maxPosition)
        && new Set(guides.map(guide => guide.id)).size === guides.length
        && Object.keys(value).every(key => ['rulers_visible', 'guides_visible', 'guides'].includes(key));
}
