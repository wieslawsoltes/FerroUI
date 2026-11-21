import { action, choose, editText, type StudioContext, type StudioItem } from './studioContext';

type Axis = 'rows' | 'columns';
interface Track { length: string; editable: boolean }
interface AxisTracks { axis: Axis; tracks?: Track[]; error?: string }
interface GridTracks { panel: number | null; writable: boolean; axes: AxisTracks[] }
const axisLabels: Record<Axis, string> = { rows: 'Rows', columns: 'Columns' };
const trackLabels: Record<Axis, string> = { rows: 'Row', columns: 'Column' };

async function readGrid(context: StudioContext): Promise<GridTracks | undefined> {
    if (context.node === undefined) return;
    const model = await context.read<GridTracks>('designer_grid_tracks', { node: context.node });
    if (context.panel.language === 'rust' && !context.panel.snapshot?.counterpart.editable) model.writable = false;
    return model;
}

export async function gridItems(context: StudioContext): Promise<StudioItem[]> {
    const grid = await readGrid(context);
    if (!grid?.panel) return [{ id: 'grid:empty', label: 'Select a Grid or one of its children' }];
    const panel = grid.panel;
    return [
        action('grid:source', 'Reveal Grid in Source', () => context.reveal(panel), 'go-to-file'),
        ...grid.axes.map((axis): StudioItem => ({
            id: `grid:${axis.axis}`,
            label: axisLabels[axis.axis],
            description: axis.tracks ? String(axis.tracks.length) : undefined,
            children: async () => axis.tracks
                ? axis.tracks.map((track, index) => ({
                    id: `grid:${axis.axis}:${index}`,
                    label: `${trackLabels[axis.axis]} ${index}`, description: track.length,
                    run: grid.writable ? () => editTrack(context, panel, axis.axis, axis.tracks!, index) : undefined,
                }))
                : [{ id: `grid:${axis.axis}:error`, label: axis.error ?? 'Grid definitions unavailable', icon: 'info' }],
        })),
    ];
}

export async function chooseGridTrack(context: StudioContext): Promise<void> {
    const grid = await readGrid(context);
    if (!grid?.panel) throw new Error('Select a Grid or one of its children.');
    if (!grid.writable) throw new Error('Track editing requires a current Grid with an editable source mapping.');
    const choices = grid.axes.flatMap(axis => (axis.tracks ?? []).map((track, index, tracks) => ({
        label: `${trackLabels[axis.axis]} ${index}`, description: track.length, axis: axis.axis, tracks, index,
    })));
    if (!choices.length) throw new Error(grid.axes.find(axis => axis.error)?.error ?? 'Grid definitions unavailable.');
    const choice = await choose('Edit Grid tracks', choices);
    if (choice) await editTrack(context, grid.panel, choice.axis, choice.tracks, choice.index);
}

async function editTrack(context: StudioContext, panel: number, axis: Axis, tracks: Track[], index: number): Promise<void> {
    const kinds: { label: string; action: 'set' | 'insert' | 'remove'; index: number }[] = [];
    if (tracks[index].editable) kinds.push({ label: 'Set track length…', action: 'set', index });
    kinds.push({ label: 'Insert track before…', action: 'insert', index },
        { label: 'Insert track after…', action: 'insert', index: index + 1 });
    if (tracks.length > 1) kinds.push({ label: 'Remove track', action: 'remove', index });
    const selected = await choose(`Grid · ${trackLabels[axis]} ${index}`, kinds);
    if (!selected) return;
    const args = { panel, axis, action: selected.action, index: selected.index };
    if (selected.action === 'remove') {
        await context.edit('designer_grid_track_edit', args);
        return;
    }
    const length = await editText('Track length in logical pixels, star units, or Auto', selected.action === 'set' ? tracks[index].length : '*');
    if (length?.trim()) await context.edit('designer_grid_track_edit', { ...args, length });
}
