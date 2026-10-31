function load(locale) {
    return import("../plugins/" + locale + "/index.js").then(m => m.language)
}
