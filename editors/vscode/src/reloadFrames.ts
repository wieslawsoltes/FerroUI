export const reloadFramePrefix = 'FERROUI_RELOAD_FRAME_V1=';

export interface RuntimeFrameReceipt {
    frame: number;
    batch: number;
    status: 'rendered' | 'noSceneUpdate' | 'discarded';
    unixMs: number | null;
    afterAcknowledgementMs: number | null;
}

export interface ReloadFrameIdentity {
    generation: number;
    frame?: number;
    acknowledgedAt?: number;
}

export interface ReloadFrame {
    receipt: number;
    status: RuntimeFrameReceipt['status'] | 'pending' | 'processExited';
    batch?: number;
    elapsedMs?: number;
    afterAcknowledgementMs?: number;
}

export function positiveId(value: unknown): value is number {
    return typeof value === 'number' && Number.isSafeInteger(value) && value > 0;
}

export function parseFrameReceipt(text: string): RuntimeFrameReceipt | undefined {
    try {
        const receipt = JSON.parse(text) as RuntimeFrameReceipt;
        if (positiveId(receipt.frame) && positiveId(receipt.batch)
            && ['rendered', 'noSceneUpdate', 'discarded'].includes(receipt.status)
            && (receipt.unixMs === null || positiveId(receipt.unixMs))
            && (receipt.afterAcknowledgementMs === null
                || (Number.isFinite(receipt.afterAcknowledgementMs) && receipt.afterAcknowledgementMs >= 0))) return receipt;
    } catch { /* unrelated process output */ }
    return undefined;
}
