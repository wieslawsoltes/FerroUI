export type Schema = {
    oneOf?: readonly Schema[];
    type?: string;
    enum?: readonly string[];
    minimum?: number;
    maximum?: number;
    exclusiveMaximum?: number;
    maxLength?: number;
    maxItems?: number;
    maxProperties?: number;
    items?: Schema;
    properties?: Record<string, Schema>;
    required?: readonly string[];
    additionalProperties?: boolean | Schema;
};

export const text = { type: 'string', maxLength: 1024 * 1024 };
export const object = { type: 'object' };
export const truth = { type: 'boolean' };
export const integer = (minimum: number, maximum = Number.MAX_SAFE_INTEGER) => ({ type: 'integer', minimum, maximum });
export const choice = (...values: string[]) => ({ type: 'string', enum: values });

export function tool(description: string, properties: Record<string, Schema> = {}, required: string[] = [], write = false, destructive = write) {
    return {
        description,
        inputSchema: { type: 'object', properties, required, additionalProperties: false },
        annotations: { readOnlyHint: !write, destructiveHint: destructive, openWorldHint: false },
    };
}

export function validate(schema: Schema, value: unknown): void {
    if (schema.oneOf) {
        const count = schema.oneOf.filter(alternative => {
            try { validate(alternative, value); return true; }
            catch { return false; }
        }).length;
        if (count !== 1) throw new Error('Expected one matching input form');
    }
    if (schema.enum && !schema.enum.includes(value as string)) throw new Error('Invalid choice');
    if (schema.type === 'number' || schema.type === 'integer') {
        const n = value as number;
        if (!Number.isFinite(value) || (schema.type === 'integer' && !Number.isSafeInteger(n))
            || n < (schema.minimum ?? -Infinity) || n > (schema.maximum ?? Infinity)
            || n >= (schema.exclusiveMaximum ?? Infinity)) throw new Error('Number is outside the allowed range');
    } else if (schema.type === 'string') {
        if (typeof value !== 'string' || value.length > (schema.maxLength ?? Infinity)) throw new Error('Expected a valid string');
    } else if (schema.type === 'boolean') {
        if (typeof value !== 'boolean') throw new Error('Expected a boolean');
    } else if (schema.type === 'array') {
        if (!Array.isArray(value) || value.length > (schema.maxItems ?? Infinity)) throw new Error('Expected a bounded array');
        if (schema.items) for (const item of value) validate(schema.items, item);
    } else if (schema.type === 'object') {
        if (!value || typeof value !== 'object' || Array.isArray(value)) throw new Error('Expected an arguments object');
        const arguments_ = value as Record<string, unknown>;
        if (Object.keys(arguments_).length > (schema.maxProperties ?? Infinity)) throw new Error('Too many object properties');
        for (const key of schema.required ?? []) if (!Object.hasOwn(arguments_, key)) throw new Error(`Missing ${key}`);
        for (const key of Object.keys(arguments_)) {
            const child = schema.properties && Object.hasOwn(schema.properties, key) ? schema.properties[key] : undefined;
            if (child) validate(child, arguments_[key]);
            else if (schema.additionalProperties === false) throw new Error(`Unknown argument ${key}`);
            else if (typeof schema.additionalProperties === 'object') validate(schema.additionalProperties, arguments_[key]);
        }
    }
}
