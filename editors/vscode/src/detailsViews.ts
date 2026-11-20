import type { Uri } from 'vscode';
import type { DesignerPanel } from './designerPanel';
import { DockView } from './dockView';
import { errorMessage, type DetailsState, type DetailsView, type FrameTarget, type FromDetails, type Inspection, type ResourceRow, type ToDetails } from './protocol';
import { detailsHtml } from './view';

export const EVENTS_VIEW = 'ferroui.events';
export const RESOURCES_VIEW = 'ferroui.resources';

interface ResourceCache {
    key: string;
    data?: ResourceRow[];
    error?: string;
}

export class DetailsViews {
    readonly events;
    readonly resources;
    private cache?: ResourceCache;

    constructor(root: Uri, private readonly changed: () => void,
        receive: (view: DetailsView, message: FromDetails) => void) {
        const create = (view: DetailsView) => new DockView<DetailsState, FromDetails, ToDetails>(
            root, (webview, root) => detailsHtml(webview, root, view), changed, message => receive(view, message));
        this.events = create('events');
        this.resources = create('resources');
    }

    update(panel: DesignerPanel | undefined, frame: FrameTarget | undefined, file: string,
        inspection: { data?: Inspection; message?: string }, codeBehind: boolean): void {
        const node = panel?.selectedNode;
        const state: DetailsState = {
            frame, target: frame && node ? { ...frame, node: node.id } : undefined,
            language: panel?.language, label: node?.name || node?.label, message: panel?.status,
        };
        this.events.show({ ...state, events: inspection.data?.events, codeBehind,
            message: state.message ?? inspection.message }, file);
        const resources = this.resources.visible ? this.readResources(panel, frame) : {};
        this.resources.show({ ...state, resources: resources.data, message: state.message ?? resources.message }, file);
    }

    private readResources(panel?: DesignerPanel, target?: FrameTarget): { data?: ResourceRow[]; message?: string } {
        if (!panel || !target) return {};
        if (panel.language === 'rust') return { data: [], message: 'Open a XAML design to browse its resource dictionaries.' };
        const selected = panel.selectedNode?.id;
        const key = `${target.session}:${target.token}:${selected ?? ''}`;
        if (this.cache?.key !== key) {
            const cache: ResourceCache = { key };
            this.cache = cache;
            void panel.resources(target, selected).then(data => { cache.data = data; },
                error => { cache.error = errorMessage(error); }).finally(() => {
                if (this.cache === cache) this.changed();
            });
        }
        return { data: this.cache.data, message: this.cache.error ?? (this.cache.data ? undefined : 'Reading resource scopes…') };
    }

    dispose(): void {
        this.events.dispose();
        this.resources.dispose();
    }
}
