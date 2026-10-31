import QtQml
import "nested" as Nested
import "dual.js" as Dual
import "classic/provider.js" as Classic
import "esm/provider.mjs" as Esm

QtObject {
    property string selected: "en"
    property Nested.Pane pane: Nested.Pane {}

    function evaluations(): int { return globalThis.computedCount || 0 }
    function fromQml(locale: string) {
        return import(`./plugins/${locale}/index.js`).then(m => m.language)
    }
    function fromChild(locale: string) { return pane.choose(locale) }
    function fromClassic(locale: string) { return Classic.load(locale) }
    function fromEsm(locale: string) { return Esm.load(locale) }
    function fromNested(locale: string) {
        return import(`./plugins/${locale}/index.js`).then(m => m.child("leaf"))
    }
    function fromCoercion() {
        return import({toString() { return "./plugins/" + selected + "/index.js" }})
                .then(m => m.language)
    }
    function sameNamespace() {
        return Promise.all([import("./plugins/en/index.js"), Esm.namespace("en")])
                .then(([left,right]) => left === right && Object.isFrozen(right))
    }
    function dualRole(name: string) {
        const classicCount = Dual.marker()
        return import(name).then(module => classicCount + "/" + typeof module.marker + "/" + globalThis.dualExecutions)
    }
    function rejectUnknown(path: string) {
        return import(path).then(() => false, error => error instanceof TypeError)
    }
}
