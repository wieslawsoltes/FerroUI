import {format} from "./format.mjs"
let count = 0
WorkerScript.onMessage = message => {
    if (message.loop) {
        WorkerScript.sendMessage({text: "started"})
        for (;;) {}
    }
    if (message.model) {
        const model = message.model
        model.append({number: 8})
        model.get(0).number = 9
        try { model.sync() } catch (e) {
            WorkerScript.sendMessage({text: "conflict:" + e.message})
            return
        }
        WorkerScript.sendMessage({text: "synced-" + model.count})
        return
    }
    WorkerScript.sendMessage({text: format(++count, message.items)})
}
