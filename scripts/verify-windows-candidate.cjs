const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const { spawnSync } = require('node:child_process');
const { parseArgs, isDeepStrictEqual } = require('node:util');

const repo = path.resolve(__dirname, '..');
const minHubVersion = '0.1.0-alpha.3';

function validateInfo(result, version) {
  if (result.error || result.signal || result.status !== 0 || result.stderr ||
      typeof result.stdout !== 'string' || Buffer.byteLength(result.stdout) > 4096 ||
      result.stdout.split('\n').length !== 2 || !result.stdout.endsWith('\n')) {
    throw Error('Metadata query did not return one bounded JSON line successfully.');
  }
  const info = JSON.parse(result.stdout);
  const expected = {
    schemaVersion: 1, appId: 'creator-project-setup', displayName: 'Creator Project Setup',
    version, platform: 'windows', architecture: 'x86_64', capabilities: ['launch.standalone'],
  };
  if (!isDeepStrictEqual(info, expected)) throw Error('Metadata differs from the standalone Windows candidate contract.');
  return info;
}

function probe(executable, version, run = spawnSync, directory) {
  const options = { encoding: 'utf8', timeout: 10000, maxBuffer: 4096, windowsHide: true, shell: false };
  if (directory) {
    options.cwd = directory;
    options.env = { ...process.env };
    for (const key of ['APPDATA', 'LOCALAPPDATA', 'HOME', 'USERPROFILE', 'XDG_CONFIG_HOME', 'XDG_DATA_HOME', 'XDG_CACHE_HOME', 'TEMP', 'TMP']) {
      options.env[key] = directory;
    }
  }
  const info = validateInfo(run(executable, ['--creator-hub-info'], options), version);
  for (const args of [['--creator-hub-info', '--unexpected'], ['--creator-hub-install']]) {
    const result = run(executable, args, options);
    if (result.error || result.signal || result.status !== 2 || result.stdout) {
      throw Error('Unsupported Hub arguments were not rejected before application startup.');
    }
  }
  if (directory && fs.readdirSync(directory).length !== 0) throw Error('Metadata probes wrote unexpected local state.');
  return info;
}

function validateGuard(guard, expectedHash) {
  if (guard.guardSha256 !== expectedHash || guard.passed !== true || guard.legacyExpected !== false ||
      guard.legacyPageReached !== false || guard.exitCode !== 10 || guard.fixtureSurvived !== true ||
      guard.existingProcessesSurvived !== true || !guard.dialog?.includes('Setup will not force-close it.')) {
    throw Error('A passing no-install guard fixture for the current hooks is required.');
  }
}

async function hash(file) {
  const digest = crypto.createHash('sha256');
  for await (const chunk of fs.createReadStream(file)) digest.update(chunk);
  return digest.digest('hex');
}

function descriptor(version, installer, executable) {
  return {
    schemaVersion: 1, appId: 'creator-project-setup', version, platform: 'windows', architecture: 'x86_64',
    packageType: 'nsis', assetName: installer.name, byteLength: installer.byteLength, sha256: installer.sha256,
    executableSha256: executable.sha256, identityProtocol: 1, lifecycleProtocol: 0,
    installerProtocol: 0, minHubVersion,
  };
}

function requireExe(file) {
  const stat = fs.lstatSync(file);
  if (!stat.isFile() || stat.isSymbolicLink() || stat.size < 64 || stat.size > 512 * 1024 * 1024) {
    throw Error('Expected a regular Windows executable within the candidate size limit.');
  }
  const fd = fs.openSync(file, 'r');
  try {
    const bytes = Buffer.alloc(2);
    fs.readSync(fd, bytes, 0, bytes.length, 0);
    if (bytes.toString('ascii') !== 'MZ') throw Error('Candidate is not a Windows executable.');
  } finally { fs.closeSync(fd); }
}

function git(args) {
  const result = spawnSync('git', args, { cwd: repo, encoding: 'utf8', timeout: 10000, maxBuffer: 1024 * 1024, windowsHide: true });
  if (result.error || result.status !== 0) throw Error('Cannot record candidate source revision.');
  return result.stdout.trim();
}

async function main() {
  const { values } = parseArgs({ options: Object.fromEntries(
    ['installer', 'portable', 'seven-zip', 'output', 'guard-report'].map(key => [key, { type: 'string' }]),
  ) });
  if (process.platform !== 'win32' || process.arch !== 'x64') throw Error('Native candidate verification requires Windows x64.');
  if (Object.values(values).length !== 5 || Object.values(values).some(value => !value)) {
    throw Error('Required: --installer FILE --portable FILE --seven-zip EXE --output NEW-DIRECTORY --guard-report FILE');
  }
  const version = JSON.parse(fs.readFileSync(path.join(repo, 'package.json'), 'utf8')).version;
  if (!/^\d+\.\d+\.\d+-[0-9A-Za-z.-]+$/.test(version)) throw Error('This script only stages prerelease candidates.');
  const output = path.resolve(values.output);
  if (fs.existsSync(output)) throw Error('Candidate output already exists; use a fresh directory.');
  for (const key of ['installer', 'portable', 'seven-zip']) requireExe(path.resolve(values[key]));
  const guardPath = path.resolve(values['guard-report']);
  const guard = JSON.parse(fs.readFileSync(guardPath, 'utf8').replace(/^\uFEFF/, ''));
  validateGuard(guard, await hash(path.join(repo, 'src-tauri/windows/installer-hooks.nsh')));
  const sourceRevision = git(['rev-parse', 'HEAD']);
  const sourceDirty = git(['status', '--porcelain', '--untracked-files=normal']) !== '';
  fs.mkdirSync(output, { recursive: true });
  const installerName = `Creator-Project-Setup-${version}-Windows-setup.exe`;
  const portableName = `Creator-Project-Setup-${version}-Windows.exe`;
  fs.copyFileSync(values.installer, path.join(output, installerName), fs.constants.COPYFILE_EXCL);
  fs.copyFileSync(values.portable, path.join(output, portableName), fs.constants.COPYFILE_EXCL);
  const extracted = path.join(output, 'installed-payload');
  fs.mkdirSync(extracted);
  // Extract only the installed app, never run the installer or its uninstaller.
  const extraction = spawnSync(path.resolve(values['seven-zip']),
    ['e', '-y', '-r', `-o${extracted}`, path.join(output, installerName), 'creator-project-setup.exe'],
    { encoding: 'utf8', timeout: 60000, maxBuffer: 1024 * 1024, windowsHide: true });
  if (extraction.error || extraction.status !== 0) throw Error('NSIS application extraction failed.');
  const extractedExe = path.join(extracted, 'creator-project-setup.exe');
  requireExe(extractedExe);
  const installedState = fs.mkdtempSync(path.join(output, 'probe-installed-'));
  const portableState = fs.mkdtempSync(path.join(output, 'probe-portable-'));
  const metadata = {
    installed: probe(extractedExe, version, spawnSync, installedState),
    portable: probe(path.join(output, portableName), version, spawnSync, portableState),
  };
  const filenames = [installerName, portableName, 'installed-payload/creator-project-setup.exe'];
  const files = [];
  for (const name of filenames) {
    const file = path.join(output, name);
    files.push({ name, byteLength: fs.statSync(file).size, sha256: await hash(file) });
  }
  const writeJson = (name, value) => fs.writeFileSync(path.join(output, name), `${JSON.stringify(value, null, 2)}\n`, { flag: 'wx' });
  writeJson('creator-hub-windows-x86_64.UNSIGNED.json', descriptor(version, files[0], files[2]));
  writeJson('guard-report.json', guard);
  writeJson('candidate-report.json', {
    schemaVersion: 1, version, sourceRevision, sourceDirty, recordedAt: new Date().toISOString(),
    artifactChecksPassed: true, signed: false, files, metadata,
    checks: { extractedInstalledExe: true, exactIdentity: true, unsupportedArgumentsRejected: true, noInstallGuardFixture: true },
    notTested: ['Actual installation', 'Installed 0.2.2 upgrade', 'Installed busy upgrade/uninstall',
      'Installed settings and project preservation', 'Persistent Hub adoption', 'Native macOS/Linux acceptance'],
    publicationReady: false,
  });
  filenames.push('creator-hub-windows-x86_64.UNSIGNED.json', 'guard-report.json', 'candidate-report.json');
  const checksums = [];
  for (const name of filenames) checksums.push(`${await hash(path.join(output, name))}  ${name}`);
  fs.writeFileSync(path.join(output, 'SHA256SUMS.txt'), `${checksums.join('\n')}\n`, { flag: 'wx' });
  process.stdout.write(`Candidate artifact checks passed: ${output}\nNot an installed-upgrade acceptance or publish approval.\n`);
}

module.exports = { validateInfo, validateGuard, probe, descriptor, hash, requireExe };
if (require.main === module) main().catch(error => { process.stderr.write(`${error.message}\n`); process.exitCode = 1; });
