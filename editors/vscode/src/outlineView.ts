import type { Uri } from 'vscode';
import type { DesignerPanel } from './designerPanel';
import { DockView } from './dockView';
import { errorMessage, type FrameTarget, type FromOutline, type OutlineMode, type OutlineRow, type OutlineState, type ToOutline } from './protocol';
import { outlineHtml } from './view';

interface RuntimeCache {
    key: string;
    rows?: OutlineRow[];
    error?: string;
}

export class OutlineView extends DockView<OutlineState, FromOutline, ToOutline> {
    private mode: OutlineMode = 'source';
    private cache?: RuntimeCache;
    private rows: OutlineRow[] = [];

    constructor(root: Uri, private readonly changed: () => void, receive: (message: FromOutline) => void) {
        super(root, outlineHtml, changed, receive);
    }

    switch(mode: OutlineMode, panel?: DesignerPanel): void {
        if (!['source', 'visual', 'logical', 'combined'].includes(mode) || this.mode === mode) return;
        this.mode = mode;
        if (mode === 'source') panel?.clearRuntimeSelection();
        this.changed();
    }

    async select(panel: DesignerPanel, frame: FrameTarget, id: string, openSource: boolean, extend = false): Promise<void> {
        const row = this.rows.find(row => row.id === id);
        if (row?.runtime) await panel.selectRuntime(frame, row.runtime, openSource);
        else if (row?.source !== undefined) await panel.select({ ...frame, node: row.source }, openSource, extend);
    }

    update(panel: DesignerPanel | undefined, frame: FrameTarget | undefined, file: string): void {
        const mode = this.mode;
        let message = panel?.status;
        if (mode === 'source') {
            this.rows = panel?.nodes.map(node => ({
                id: sourceKey(node.id), source: node.id, parent: node.parent === null ? null : sourceKey(node.parent),
                label: node.label, name: node.name, canInsert: node.canInsert, canMove: node.canMove,
            })) ?? [];
        } else {
            const key = frame ? `${frame.session}:${frame.token}:${mode}` : '';
            if (!panel || !frame || !this.visible) this.rows = [];
            else {
                if (this.cache?.key !== key) {
                    const cache: RuntimeCache = { key };
                    this.cache = cache;
                    void panel.runtimeTree(frame, mode).then(rows => {
                        cache.rows = rows.map(row => ({
                            id: runtimeKey(row.inspector_id), parent: row.parent === null ? null : runtimeKey(row.parent),
                            label: row.type, name: row.name, canInsert: false, canMove: false,
                            runtime: row, source: row.source_id ?? undefined,
                        }));
                    }, failure => { cache.error = errorMessage(failure); })
                        .finally(() => { if (this.cache === cache) this.changed(); });
                }
                this.rows = this.cache.rows ?? [];
                if (!this.cache.rows) message = this.cache.error ?? 'Reading the live preview tree…';
            }
        }
        const runtime = panel?.runtimeSelection;
        const selected = panel?.selectedNode?.id;
        const row = runtime
            ? this.rows.find(row => row.runtime?.inspector_id === runtime.inspector_id)
            : this.rows.find(row => row.source !== undefined && row.source === selected);
        const selectedRows = mode === 'source' ? panel?.selectedIds.map(sourceKey) : undefined;
        this.show({ frame, session: panel?.session, mode, nodes: this.rows, selected: row?.id, selectedRows, message }, file);
    }
}

function sourceKey(id: number): string { return `source:${id}`; }
function runtimeKey(id: number): string { return `runtime:${id}`; }
