globalThis.computedCount = (globalThis.computedCount || 0) + 1
export const language = "en"
export const child = key => import(`./${key}.mjs`).then(m => m.value)
