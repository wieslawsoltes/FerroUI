export function drawEasingCurve(canvas: HTMLCanvasElement, samples: readonly number[] = []): void {
    const context = canvas.getContext('2d');
    if (!context) return;
    const { width, height } = canvas;
    context.clearRect(0, 0, width, height);
    if (samples.length < 2 || samples.some(value => !Number.isFinite(value))) {
        canvas.setAttribute('aria-label', 'Incoming easing curve unavailable');
        return;
    }
    const extent = Math.max(1, ...samples.map(Math.abs));
    const scaled = samples.map(value => value / extent);
    const minimum = Math.min(0, ...scaled);
    const maximum = Math.max(1 / extent, ...scaled);
    const y = (value: number) => 5 + (height - 10) * (maximum - value) / (maximum - minimum);
    const style = getComputedStyle(canvas);
    context.strokeStyle = style.getPropertyValue('--vscode-editorWidget-border').trim()
        || style.getPropertyValue('--line').trim() || '#7f7f7f';
    context.lineWidth = 1;
    context.beginPath();
    context.moveTo(4, y(0));
    context.lineTo(width - 4, y(1 / extent));
    context.stroke();
    context.strokeStyle = style.getPropertyValue('--vscode-charts-blue').trim()
        || style.getPropertyValue('--focus').trim() || '#6595dc';
    context.lineWidth = 2;
    context.beginPath();
    scaled.forEach((value, index) => {
        const x = 4 + (width - 8) * index / (scaled.length - 1);
        if (index === 0) context.moveTo(x, y(value));
        else context.lineTo(x, y(value));
    });
    context.stroke();
    const overshoots = samples.some(value => value < 0 || value > 1);
    canvas.setAttribute('aria-label', overshoots ? 'Incoming easing curve with overshoot' : 'Incoming easing curve');
}
