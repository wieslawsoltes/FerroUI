import type { RuntimeClient } from './runtimeClient';
import type { RuntimeSource } from './runtimeProtocol';

export interface RuntimeLog {
    sequence: number;
    level: string;
    area: string;
    message: string;
    values: string[];
    timestamp: number;
    source?: RuntimeSource;
}

export interface RuntimeFrameStats {
    node_id: number;
    layout_ms: number;
    layout_timing_available?: boolean;
    layout_pass_count: number;
    rendered_scene_updates: number;
    last_interval_ms?: number;
}

export async function readRuntimeDiagnostics(client: RuntimeClient) {
    const [frames, logs] = await Promise.all([
        client.call<RuntimeFrameStats[]>('get_frame_stats'),
        client.call<{ cursor: number; entries: RuntimeLog[] }>('get_logs'),
    ]);
    return { frames, logs };
}

export function runtimeLogText(log: RuntimeLog): string {
    let index = 0;
    return log.message.replace(/\{[^}]+\}/g, token => log.values[index++] ?? token);
}
