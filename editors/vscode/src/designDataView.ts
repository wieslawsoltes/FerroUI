import type * as vscode from 'vscode';
import type { DesignerPanel } from './designerPanel';
import type { DesignDataState, DesignFixtureCatalog, FromDesignData, ToDesignData } from './designDataProtocol';
import { DockView } from './dockView';
import { errorMessage } from './protocol';
import { designDataHtml } from './view';

export const DATA_VIEW = 'ferroui.data';

export class DesignDataView implements vscode.Disposable {
    readonly view: DockView<DesignDataState, FromDesignData, ToDesignData>;
    private panel?: DesignerPanel;
    private key?: string;
    private file = '';

    constructor(root: vscode.Uri, changed: () => void) {
        this.view = new DockView(root, designDataHtml, changed, message => { void this.receive(message); });
    }

    update(panel: DesignerPanel | undefined, file: string): void {
        this.panel = panel;
        this.file = file;
        if (!this.view.visible) { this.key = undefined; return; }
        const token = panel?.currentToken;
        const version = panel?.document.version;
        const key = `${panel?.session}:${token}:${version}:${panel?.status ?? ''}`;
        if (this.key === key) return;
        this.key = key;
        const state = { session: panel?.session, version };
        if (panel?.language !== 'xaml' || token === undefined || panel.status) {
            const message = !panel ? 'Open a FerroUI XAML designer.' : panel.language !== 'xaml'
                ? 'Named design fixtures in this view are authored in XAML.' : panel.status ?? 'Rendering design data…';
            this.view.show({ ...state, message }, file);
            return;
        }
        const frame = { session: panel.session, token };
        void panel.studioQuery('designer_design_fixtures', {}, undefined, AbortSignal.timeout(15_000)).then(
            catalog => {
                if (this.key === key) this.view.show({ ...state, frame, catalog: catalog as DesignFixtureCatalog }, file);
            },
            error => { if (this.key === key) this.view.show({ ...state, message: errorMessage(error) }, file); },
        );
    }

    private async receive(message: FromDesignData): Promise<void> {
        const panel = this.panel;
        if (message.type === 'ready' || !panel || message.frame.session !== panel.session) return;
        try {
            if (panel.language !== 'xaml' || !panel.canUse(message.frame)) throw new Error('Wait for a current XAML preview.');
            const { token } = message.frame;
            const signal = AbortSignal.timeout(15_000);
            const tool = 'designer_design_data_edit';
            if (message.type === 'preview') {
                await panel.studioPresent(token, tool, message.choice === 'application' ? { action: 'enabled', enabled: false }
                    : { action: 'fixture', name: message.choice === 'named' ? message.name : null }, undefined, signal);
            } else {
                if (message.version !== panel.document.version) throw new Error('The source changed. Reload the draft and apply it again.');
                const catalog = await panel.studioQuery('designer_design_fixtures', {}, undefined, signal) as DesignFixtureCatalog;
                await panel.studioEdit(token, tool, {
                    action: 'set_value', node: catalog.node, property: 'Fixtures', value: message.source.trim() || null,
                }, undefined, signal);
                this.view.post({ type: 'applied', session: panel.session, version: message.version, resultVersion: panel.document.version });
            }
        } catch (error) {
            this.view.post({ type: 'error', message: errorMessage(error) });
            this.key = undefined;
            this.update(this.panel, this.file);
        }
    }

    dispose(): void { this.view.dispose(); }
}
