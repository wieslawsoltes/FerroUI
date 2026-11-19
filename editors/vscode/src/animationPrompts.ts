import { editText, editTime } from './studioContext';

export async function newClipFields(defaultSelector?: string): Promise<
    { name: string; selector: string; duration: number } | undefined
> {
    const name = (await editText('Animation name'))?.trim();
    if (!name) return;
    const selector = (await editText('FerroUI selector', defaultSelector))?.trim();
    if (!selector) return;
    const duration = await editTime('Duration in seconds', 600, 1);
    if (duration !== undefined && duration > 0) return { name, selector, duration };
}
