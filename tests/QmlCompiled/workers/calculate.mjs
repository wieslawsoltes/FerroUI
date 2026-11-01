import {format} from "./format.mjs"
let count = 0
WorkerScript.onMessage = message => {
    if (message.loop) {
        WorkerScript.sendMessage({text: "started"})
        for (;;) {}
    }
    if (message.groups) {
        const child = message.groups.get(0).entries
        message.groups.get(0).title = "updated"
        child.get(0).number = 17
        child.append({number: 23})
        try { message.groups.sync() } catch(e) {
            WorkerScript.sendMessage({text:"nested-conflict:" + e.message})
            return
        }
        WorkerScript.sendMessage({text: "nested-" + child.count})
        return
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
