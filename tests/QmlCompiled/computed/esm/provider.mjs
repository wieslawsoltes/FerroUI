export function namespace(locale) {
    return import(`../plugins/${locale}/index.js`)
}
export function load(locale) {
    return namespace(locale).then(m => m.language)
}
