var local = 0
function caption(value) {
    return `qty-${String(value).padStart(3, "0")}`
}
function nextLocal() { return ++local }

function scopeLabel() { return label }
