export function scale(value) {
    const weights = [1, 2, 3];
    return weights.map(weight => weight * value).reduce((sum, n) => sum + n, 0);
}
