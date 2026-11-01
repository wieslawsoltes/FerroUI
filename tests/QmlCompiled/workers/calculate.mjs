import {format} from "./format.mjs"
let count = 0
WorkerScript.onMessage = message => {
    if (message.loop) {
        WorkerScript.sendMessage({text: "started"})
        for (;;) {}
    }
    WorkerScript.sendMessage({text: format(++count, message.items)})
}
