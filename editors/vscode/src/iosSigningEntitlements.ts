const appId = 'application-identifier';
const teamId = 'com.apple.developer.team-identifier';
const taskAccess = 'get-task-allow';
const keychain = 'keychain-access-groups';

interface DevelopmentProvision {
    entitlements: unknown;
    devices: unknown;
    expiration: string;
}

export function signingEntitlements(provision: DevelopmentProvision, bundleId: string, udid: string,
    requested: unknown, time = Date.now()): Record<string, unknown> {
    const allowed = dictionary(provision.entitlements);
    const expiration = Date.parse(provision.expiration);
    if (!allowed || allowed[taskAccess] !== true || typeof allowed[teamId] !== 'string' || !/^[A-Z0-9]{10}$/.test(allowed[teamId])) {
        throw new Error('Choose an iOS development provisioning profile with debugger access.');
    }
    if (!Number.isFinite(expiration) || expiration <= time) throw new Error('The iOS development provisioning profile has expired.');
    if (!Array.isArray(provision.devices) || !provision.devices.some(device => typeof device === 'string' && device.toUpperCase() === udid.toUpperCase())) {
        throw new Error('The provisioning profile does not include this iOS device.');
    }
    const pattern = allowed[appId];
    const prefix = typeof pattern === 'string' ? /^([A-Z0-9]{10})\./.exec(pattern)?.[1] : undefined;
    if (!prefix || !permitted(pattern, `${prefix}.${bundleId}`)) {
        throw new Error('The provisioning profile does not cover this iOS bundle identifier.');
    }
    const exactId = `${prefix}.${bundleId}`;
    const custom = dictionary(requested);
    if (!custom) throw new Error('Choose an iOS entitlements file containing a dictionary.');
    const base = { [appId]: exactId, [teamId]: allowed[teamId], [taskAccess]: true };
    for (const [key, value] of Object.entries(custom)) {
        const authorization = Object.hasOwn(base, key) ? base[key as keyof typeof base] : allowed[key];
        if (!permitted(authorization, value)) {
            throw new Error(`The iOS signing settings do not authorize entitlement ${key}.`);
        }
    }
    const keychainGroups = permitted(allowed[keychain], [exactId]) && custom[keychain] === undefined ? { [keychain]: [exactId] } : {};
    return { ...keychainGroups, ...custom, ...base };
}

function dictionary(value: unknown): Record<string, unknown> | undefined {
    return value && typeof value === 'object' && !Array.isArray(value) ? value as Record<string, unknown> : undefined;
}

function permitted(allowed: unknown, claimed: unknown): boolean {
    if (typeof claimed === 'string') {
        return typeof allowed === 'string' && !claimed.includes('*')
            && (allowed === claimed || allowed.endsWith('*') && claimed.startsWith(allowed.slice(0, -1)));
    }
    if (Array.isArray(claimed)) return Array.isArray(allowed) && claimed.every(item => allowed.some(match => permitted(match, item)));
    const fields = dictionary(claimed);
    if (fields) {
        const parent = dictionary(allowed);
        return !!parent && Object.entries(fields).every(([key, value]) => Object.hasOwn(parent, key) && permitted(parent[key], value));
    }
    return claimed !== undefined && allowed === claimed;
}
