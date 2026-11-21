export const templateLabels = {
    ItemTemplate: 'Item template',
    SelectionBoxItemTemplate: 'Selection box template',
    MenuItemTemplate: 'Menu item template',
    CellTemplate: 'Cell template',
    CellEditingTemplate: 'Cell editing template',
    Template: 'Control template',
} as const;

export type TemplateProperty = keyof typeof templateLabels;
