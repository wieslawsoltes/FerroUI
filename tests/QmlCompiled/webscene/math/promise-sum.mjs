const base = await Promise.resolve(40);
export async function delayedSum(value) {
    return base + await Promise.resolve(value);
}
