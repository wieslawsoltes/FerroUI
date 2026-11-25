import { strict as assert } from 'node:assert';
import { deviceBuildProfile, simulatorBuildProfile } from '../src/iosBuildProfiles';
import { deviceAppProcesses, installedDeviceBundle } from '../src/iosDeviceProcesses';
import { signingEntitlements } from '../src/iosSigningEntitlements';
import { deviceBundleId } from './iosDeviceFixture';

export function verifyIosDeviceSigning(): void {
    const signing = { identity: 'f'.repeat(40), provisioningProfile: 'ContactReview.mobileprovision' };
    const build = deviceBuildProfile({ application: 'contact-review', signing, reloadOnSave: true }, 'contact')!;
    assert.equal(build.signing.identity, signing.identity.toUpperCase());
    assert.equal(build.executable, 'simulator_inspector');
    assert.ok(build.reloadOnSave && build.features.includes('inspector-example'));
    assert.throws(() => deviceBuildProfile({ application: 'contact-review', signing: { ...signing, identity: '-' } }, 'contact'), /certificate fingerprint/);
    assert.throws(() => deviceBuildProfile({ application: 'contact-review', signing, target: 'aarch64-apple-ios-sim' }, 'contact'), /aarch64-apple-ios for iOS device/);
    assert.throws(() => simulatorBuildProfile({ application: 'contact-review', target: 'aarch64-apple-ios' }, 'contact'), /simulator Rust target/);

    const device = 'ABCDEF1234567890';
    const expiration = '2030-11-02T12:00:00Z';
    const allowed = {
        'application-identifier': 'APPID12345.com.ferroui.*',
        'com.apple.developer.team-identifier': 'TEAM123456',
        'get-task-allow': true,
        'keychain-access-groups': ['APPID12345.*', 'com.apple.token'],
        'com.apple.security.application-groups': ['group.com.ferroui.ContactReview'],
    };
    const provision = { entitlements: allowed, devices: [device], expiration };
    const now = Date.parse('2026-11-23T12:00:00Z');
    const exact = 'APPID12345.' + deviceBundleId;
    const entitlements = signingEntitlements(provision, deviceBundleId, device.toLowerCase(), {}, now);
    assert.deepEqual(entitlements, {
        'application-identifier': exact,
        'com.apple.developer.team-identifier': 'TEAM123456',
        'get-task-allow': true,
        'keychain-access-groups': [exact],
    });
    const group = 'com.apple.security.application-groups';
    assert.deepEqual(signingEntitlements(provision, deviceBundleId, device, { [group]: allowed[group] }, now)[group], allowed[group]);
    assert.throws(() => signingEntitlements(provision, deviceBundleId, device, { [group]: ['group.com.other.app'] }, now), /authorize entitlement/);
    assert.throws(() => signingEntitlements(provision, deviceBundleId, device, { 'keychain-access-groups': ['APPID12345.*'] }, now), /authorize entitlement/);
    assert.throws(() => signingEntitlements(provision, deviceBundleId, device, { 'get-task-allow': false }, now), /authorize entitlement/);
    assert.throws(() => signingEntitlements(provision, 'unmatched.ContactReview', device, {}, now), /cover this iOS bundle/);
    assert.throws(() => signingEntitlements(provision, deviceBundleId, 'ffffffffffffffff', {}, now), /include this iOS device/);
    assert.throws(() => signingEntitlements(provision, deviceBundleId, device, {}, Date.parse(expiration)), /expired/);
    assert.throws(() => signingEntitlements({ ...provision, entitlements: { ...allowed, 'get-task-allow': false } },
        deviceBundleId, device, {}, now), /debugger access/);

    const url = 'file:////private/var/containers/Bundle/Application/a/Contact%20Review.app/';
    const bundle = installedDeviceBundle({ apps: [{ bundleIdentifier: deviceBundleId, url }] }, deviceBundleId);
    assert.equal(bundle, '/private/var/containers/Bundle/Application/a/Contact Review.app');
    assert.equal(installedDeviceBundle({ apps: [] }, deviceBundleId), undefined);
    assert.throws(() => installedDeviceBundle({ apps: [{ bundleIdentifier: deviceBundleId, url: 'https://device/app' }] }, deviceBundleId), /installed iOS app bundle/);
    assert.deepEqual(deviceAppProcesses({ runningProcesses: [
        { executable: url + 'ContactReview', processIdentifier: 42 },
        { executable: url.replace('private/', '') + 'ContactReview', processIdentifier: 43 },
        { executable: url + 'PlugIns/Extension.appex/Extension', processIdentifier: 51 },
        { executable: url.replace('/a/', '/b/') + 'ContactReview', processIdentifier: 52 },
    ] }, bundle!), [42, 43]);
}
