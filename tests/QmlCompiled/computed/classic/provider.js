.import "../dual.js" as LegacyAlias

function load(locale) {
    return import("../plugins/" + locale + "/index.js").then(m => m.language)
}

function rejectsClassicFactory() {
    const alias = "LegacyAlias"
    return import(alias).then(() => false, error => error instanceof TypeError)
}
