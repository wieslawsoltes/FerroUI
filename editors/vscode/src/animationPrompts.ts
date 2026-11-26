import { editText, editTime } from './studioContext';

export async function newClipFields(defaultSelector?: string, title?: string): Promise<
    { name: string; selector: string; duration: number } | undefined
> {
    const name = (await editText('Animation name', undefined, undefined, title))?.trim();
    if (!name) return;
    const selector = (await editText('FerroUI selector', defaultSelector, undefined, title))?.trim();
    if (!selector) return;
    const duration = await editTime('Duration in seconds', 600, 1, title);
    if (duration !== undefined && duration > 0) return { name, selector, duration };
}
