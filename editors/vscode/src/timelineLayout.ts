export const timelineLayouts = ['tracks', 'clips'] as const;
export type TimelineLayout = typeof timelineLayouts[number];

export type TimelineLayoutMessage = { type: 'timelineLayout'; layout: TimelineLayout };

export function isTimelineLayout(layout: unknown): layout is TimelineLayout {
    return timelineLayouts.some(choice => layout === choice);
}
