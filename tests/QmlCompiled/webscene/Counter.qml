import QtQml
import "Labels.js" as Labels
import "Shared.js" as Shared

QtObject {
    function tick(): string { return Labels.nextLocal() + "/" + Shared.tick() }
}
