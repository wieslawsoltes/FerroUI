globalThis.dualExecutions = (globalThis.dualExecutions || 0) + 1
function marker() { return globalThis.dualExecutions }
