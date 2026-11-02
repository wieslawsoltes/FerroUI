var count = 0
WorkerScript.onMessage = function(n) {
    count += n
    WorkerScript.sendMessage(count)
}
