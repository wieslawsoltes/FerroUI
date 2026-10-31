import QtQml

QtObject {
    function choose(locale: string) {
        return import("../plugins/" + locale + "/index.js").then(m => m.language)
    }
}
