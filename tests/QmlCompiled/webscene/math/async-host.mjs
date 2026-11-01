export async function* values(model) {
    await Promise.resolve()
    yield model.left
    await Promise.resolve()
    yield model.left + 1
}

export async function compute(model) {
    let total = 0
    for await (const value of values(model)) total += value
    return total
}
