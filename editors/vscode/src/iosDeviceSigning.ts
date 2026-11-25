import { createHash } from 'node:crypto';
import { copyFile, stat, writeFile } from 'node:fs/promises';
import * as path from 'node:path';
import type { IosSigningProfile } from './iosBuildProfiles';
import { appleTool, plistValue } from './iosPlist';
import { signingEntitlements } from './iosSigningEntitlements';

interface SigningInputs { identity: string; profile: string; entitlements: string }

export async function prepareDeviceSigning(signing: IosSigningProfile, folder: string, scratch: string, bundleId: string,
    udid: string | undefined, signal: AbortSignal): Promise<SigningInputs> {
    if (!udid) throw new Error('Xcode did not provide this device’s provisioning identifier.');
    const profile = path.join(scratch, 'embedded.mobileprovision');
    const payload = path.join(scratch, 'profile.plist');
    const entitlements = path.join(scratch, 'entitlements.plist');
    const input = path.resolve(folder, signing.provisioningProfile);
    const properties = await stat(input).catch(() => undefined);
    if (!properties?.isFile() || properties.size > 2 * 1024 * 1024) throw new Error('Choose a valid iOS development provisioning profile.');
    await copyFile(input, profile);
    try {
        await appleTool('/usr/bin/security', ['cms', '-D', '-i', profile, '-o', payload], signal);
    } catch {
        signal.throwIfAborted();
        throw new Error('Xcode could not read the selected iOS development provisioning profile.');
    }
    const [allowed, devices, expiration, requested] = await Promise.all([
        plistValue(payload, 'Entitlements', 'json', signal).then(JSON.parse),
        plistValue(payload, 'ProvisionedDevices', 'json', signal).then(JSON.parse, () => {
            signal.throwIfAborted();
            return undefined;
        }),
        plistValue(payload, 'ExpirationDate', 'raw', signal),
        signing.entitlements
            ? appleTool('/usr/bin/plutil', ['-convert', 'json', '-o', '-', path.resolve(folder, signing.entitlements)], signal).then(JSON.parse)
            : {},
    ]);
    const claims = signingEntitlements({ entitlements: allowed, devices, expiration }, bundleId, udid, requested);
    await verifySigningIdentity(signing.identity, payload, signal);
    const json = path.join(scratch, 'entitlements.json');
    await writeFile(json, JSON.stringify(claims), { mode: 0o600 });
    await appleTool('/usr/bin/plutil', ['-convert', 'xml1', '-o', entitlements, json], signal);
    return { identity: signing.identity, profile, entitlements };
}

export async function signDeviceBundle(bundle: string, signing: SigningInputs, signal: AbortSignal): Promise<void> {
    await copyFile(signing.profile, path.join(bundle, 'embedded.mobileprovision'));
    try {
        await appleTool('/usr/bin/codesign', ['--force', '--sign', signing.identity, '--entitlements', signing.entitlements,
            '--generate-entitlement-der', bundle], signal, 30_000);
        await appleTool('/usr/bin/codesign', ['--verify', '--strict', bundle], signal, 10_000);
    } catch {
        signal.throwIfAborted();
        throw new Error('Code signing failed. Check that macOS can use the selected Apple Development identity and private key.');
    }
}

async function verifySigningIdentity(identity: string, payload: string, signal: AbortSignal): Promise<void> {
    const count = Number(await plistValue(payload, 'DeveloperCertificates', 'raw', signal));
    if (!Number.isInteger(count) || count < 1 || count > 128) throw new Error('The iOS provisioning profile has no usable signing certificates.');
    let authorized = false;
    for (let index = 0; index < count && !authorized; index++) {
        const value = await plistValue(payload, `DeveloperCertificates.${index}`, 'raw', signal);
        authorized = createHash('sha1').update(Buffer.from(value, 'base64')).digest('hex').toUpperCase() === identity;
    }
    if (!authorized) throw new Error('The selected Apple Development certificate is outside this provisioning profile.');
    const identities = await appleTool('/usr/bin/security', ['find-identity', '-v', '-p', 'codesigning'], signal);
    if (!identities.split('\n').some(line => /^\s*\d+\) ([A-F0-9]{40}) /i.exec(line)?.[1]?.toUpperCase() === identity)) {
        throw new Error('Install the selected Apple Development identity and its private key on the workspace Mac.');
    }
}
