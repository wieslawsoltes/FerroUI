import { clamp } from './timelineViewport';

export type CubicEasing = [number, number, number, number];

const horizontalInset = 0.06;
const verticalInset = 0.12;
const decimal = /^[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:e[+-]?\d+)?$/i;

export function cubicEasing(source: string): CubicEasing | undefined {
    if (!source.trim() || source.trim() === 'LinearEasing') return [0, 0, 1, 1];
    const tokens = source.split(',').map(token => token.trim());
    if (tokens.length !== 4 || !tokens.every(token => decimal.test(token))) return;
    const [x1, y1, x2, y2] = tokens.map(Number);
    if (x1 >= 0 && x1 <= 1 && x2 >= 0 && x2 <= 1 && Number.isFinite(y1) && Number.isFinite(y2)) return [x1, y1, x2, y2];
}

export function easingPlot(points: CubicEasing | undefined, samples: readonly number[]) {
    const values = points ? [points[1], points[3]] : samples;
    const scale = Math.max(1, ...values.map(Math.abs));
    const low = Math.min(-0.25 / scale, ...values.map(value => value / scale));
    const high = Math.max(1.25 / scale, ...values.map(value => value / scale));
    const width = 1 - 2 * horizontalInset;
    const height = 1 - 2 * verticalInset;
    return {
        x: (value: number) => horizontalInset + value * width,
        y: (value: number) => verticalInset + height * (high - value / scale) / (high - low),
        progress: (x: number) => clamp((x - horizontalInset) / width, 0, 1),
        value: (y: number) => scale * (high - clamp((y - verticalInset) / height, 0, 1) * (high - low)),
    };
}

export function drawEasingCurve(canvas: HTMLCanvasElement, points: CubicEasing | undefined, samples: readonly number[],
    plot = easingPlot(points, samples)): void {
    const context = canvas.getContext('2d');
    if (!context) return;
    const { width, height } = canvas;
    context.clearRect(0, 0, width, height);
    if (!points && samples.length < 2) {
        canvas.setAttribute('aria-label', 'Incoming easing curve unavailable');
        return;
    }
    const x = (value: number) => plot.x(value) * width;
    const y = (value: number) => plot.y(value) * height;
    const style = getComputedStyle(canvas);
    context.strokeStyle = style.getPropertyValue('--vscode-editorWidget-border').trim()
        || style.getPropertyValue('--line').trim() || '#7f7f7f';
    context.lineWidth = 1;
    context.beginPath();
    context.moveTo(x(0), y(0));
    context.lineTo(x(1), y(1));
    if (points) {
        context.moveTo(x(0), y(0));
        context.lineTo(x(points[0]), y(points[1]));
        context.moveTo(x(1), y(1));
        context.lineTo(x(points[2]), y(points[3]));
    }
    context.stroke();
    context.strokeStyle = style.getPropertyValue('--vscode-charts-blue').trim()
        || style.getPropertyValue('--focus').trim() || '#6595dc';
    context.lineWidth = 2;
    context.beginPath();
    if (points) {
        const [x1, y1, x2, y2] = points;
        context.moveTo(x(0), y(0));
        context.bezierCurveTo(x(x1), y(y1), x(x2), y(y2), x(1), y(1));
    } else {
        context.moveTo(x(0), y(samples[0]));
        samples.slice(1).forEach((value, index) => context.lineTo(x((index + 1) / (samples.length - 1)), y(value)));
    }
    context.stroke();
    const overshoots = samples.some(value => value < 0 || value > 1);
    canvas.setAttribute('aria-label', overshoots ? 'Incoming easing curve with overshoot' : 'Incoming easing curve');
}
