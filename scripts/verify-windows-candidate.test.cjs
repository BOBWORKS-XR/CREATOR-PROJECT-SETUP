const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { validateInfo, validateGuard, probe, descriptor } = require('./verify-windows-candidate.cjs');
const version = '0.3.0-alpha.1';
const info = { schemaVersion: 1, appId: 'creator-project-setup', displayName: 'Creator Project Setup', version,
  platform: 'windows', architecture: 'x86_64', capabilities: ['launch.standalone'] };
const result = value => ({ status: 0, stdout: `${JSON.stringify(value)}\n`, stderr: '' });

test('exact standalone metadata is accepted without hosting claims', () => {
  assert.deepEqual(validateInfo(result(info), version), info);
});

test('wrong version, identity, platform, schema or advertised capabilities fail', () => {
  for (const patch of [{ version: '0.2.2' }, { appId: 'creator-works-mcp' }, { schemaVersion: 2 },
    { platform: 'linux' }, { architecture: 'aarch64' }, { capabilities: ['launch.standalone', 'launch.hosted'] }, { installerProtocol: 1 }]) {
    assert.throws(() => validateInfo(result({ ...info, ...patch }), version), /contract/);
  }
});

test('errors, timeout, oversized, multiline, noisy and invalid output fail', () => {
  for (const patch of [{ status: 2 }, { signal: 'SIGTERM' }, { error: Error('timeout') }, { stderr: 'warning' },
    { stdout: 'x'.repeat(4097) }, { stdout: `${JSON.stringify(info)}\nextra\n` }, { stdout: '{}' }, { stdout: '{broken}\n' }]) {
    assert.throws(() => validateInfo({ ...result(info), ...patch }, version));
  }
});

test('probe only executes bounded metadata and invalid-argument checks, never an install', () => {
  const calls = [];
  assert.deepEqual(probe('fixture.exe', version, (exe, args, options) => {
    calls.push({ exe, args, options });
    return calls.length === 1 ? result(info) : { status: 2, stdout: '', stderr: 'Unsupported arguments' };
  }), info);
  assert.deepEqual(calls.map(c => c.args), [['--creator-hub-info'], ['--creator-hub-info', '--unexpected'], ['--creator-hub-install']]);
  assert.ok(calls.every(c => c.options.timeout === 10000 && c.options.maxBuffer === 4096 && c.options.windowsHide && !c.options.shell));
});

test('accepting an unknown Hub command is a failed artifact check', () => {
  let count = 0;
  assert.throws(() => probe('fixture.exe', version, () => ++count === 1 ? result(info) : { status: 0, stdout: '' }), /not rejected/);
});

test('native probes use a separate state directory and detect unexpected writes', t => {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'setup-candidate-test-'));
  t.after(() => {
    assert.equal(path.dirname(path.resolve(directory)), path.resolve(os.tmpdir()));
    assert.ok(path.basename(directory).startsWith('setup-candidate-test-'));
    fs.rmSync(directory, { recursive: true });
  });
  let count = 0;
  const run = (exe, args, options) => {
    assert.equal(options.cwd, directory);
    for (const key of ['APPDATA', 'LOCALAPPDATA', 'USERPROFILE', 'TEMP', 'TMP']) assert.equal(options.env[key], directory);
    return ++count % 3 === 1 ? result(info) : { status: 2, stdout: '' };
  };
  assert.deepEqual(probe('fixture.exe', version, run, directory), info);
  fs.writeFileSync(path.join(directory, 'unexpected.json'), '{}');
  assert.throws(() => probe('fixture.exe', version, run, directory), /unexpected local state/);
});

test('guard evidence must match current source and prove refusal without killing', () => {
  const valid = { guardSha256: 'abc', passed: true, legacyExpected: false, legacyPageReached: false,
    exitCode: 10, fixtureSurvived: true, existingProcessesSurvived: true, dialog: 'Setup will not force-close it.' };
  validateGuard(valid, 'abc');
  for (const patch of [{ guardSha256: 'old' }, { passed: false }, { legacyExpected: true }, { legacyPageReached: true },
    { exitCode: 0 }, { fixtureSurvived: false }, { existingProcessesSurvived: false }, { dialog: '' }]) {
    assert.throws(() => validateGuard({ ...valid, ...patch }, 'abc'), /fixture/);
  }
});

test('descriptor uses extracted installed hash and never promotes lifecycle or installer', () => {
  const installer = { name: 'Creator-Project-Setup-0.3.0-alpha.1-Windows-setup.exe', byteLength: 123, sha256: 'installer' };
  const value = descriptor(version, installer, { sha256: 'installed-not-portable' });
  assert.equal(value.executableSha256, 'installed-not-portable');
  assert.equal(value.sha256, 'installer');
  assert.equal(value.identityProtocol, 1);
  assert.equal(value.lifecycleProtocol, 0);
  assert.equal(value.installerProtocol, 0);
  assert.equal(value.minHubVersion, '0.1.0-alpha.3');
  assert.equal(Object.keys(value).length, 14);
});
