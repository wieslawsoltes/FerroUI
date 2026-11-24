import { readFileSync, renameSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
const root = process.env.FERROUI_MANAGED_TEST_DIR;
const file = join(root, 'device.json');
const state = JSON.parse(readFileSync(file, 'utf8'));
const save = () => {
    const temporary = file + '.' + process.pid;
    writeFileSync(temporary, JSON.stringify(state));
    renameSync(temporary, file);
};
const device = 'emulator-5450';
const args = process.argv.slice(2).filter((arg, index) => index > 1 || (arg !== '-s' && arg !== device));
const [outer, operation, ...rest] = args;
const packageId = 'org.ferroui.managedfixture';
if (outer === 'devices') console.log(device + ' device product:fixture model:Managed_Test');
else if (outer === 'install' && args.includes(packageId)) {
    state.installed = readFileSync(args.at(-1), 'utf8');
    state.installations.push(state.installed);
    state.pid = null;
    save();
    console.log('Success');
} else if (outer === 'forward') {
    if (operation === '--no-rebind') {
        state.forwards[String(state.hostPort)] = rest.at(-1);
        save();
        console.log(state.hostPort);
    } else if (operation === '--list') {
        for (const [port, target] of Object.entries(state.forwards)) console.log(device + ' tcp:' + port + ' ' + target);
    } else if (operation === '--remove') {
        delete state.forwards[rest[0].slice(4)];
        save();
    }
} else if (outer === 'shell' && operation === 'getprop') console.log('arm64-v8a,armeabi-v7a');
else if (outer === 'shell' && operation === 'run-as' && rest[0] === packageId) console.log('10223');
else if (outer === 'shell' && operation === 'pidof') {
    if (state.pid) console.log(state.pid);
} else if (outer === 'shell' && operation === 'am' && args.includes(packageId + '/org.ferroui.android.FerroMainActivity')) {
    if (state.installed === 'bad') process.exitCode = 19;
    else {
        state.pid = ++state.next;
        state.receipt = { version: 1, port: 47000 + state.pid - 4000, pid: state.pid, token: state.pid.toString(16).padStart(64, '0') };
        save();
        console.log('Status: ok');
    }
} else if (outer === 'shell' && operation === 'am' && args.includes('force-stop')) {
    state.pid = null;
    save();
} else if (outer === 'exec-out' && operation === 'run-as' && rest[0] === packageId) console.log(JSON.stringify(state.receipt));
else {
    console.error('Unhandled ADB test arguments', JSON.stringify(args));
    process.exitCode = 20;
}
