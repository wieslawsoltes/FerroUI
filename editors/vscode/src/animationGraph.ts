export const animationGraphModes = ['keys', 'value', 'velocity'] as const;
export type AnimationGraphMode = typeof animationGraphModes[number];

export type AnimationGraphMessage = { type: 'animationGraphMode'; mode: AnimationGraphMode };

export function isAnimationGraphMode(mode: unknown): mode is AnimationGraphMode {
    return animationGraphModes.some(choice => mode === choice);
}

export function availableGraphModes(curves: boolean, velocity: boolean): readonly AnimationGraphMode[] {
    if (!curves) return ['keys'];
    return velocity ? animationGraphModes : ['keys', 'value'];
}
