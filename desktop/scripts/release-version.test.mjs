// @ts-check
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { prepare, setCargoVersion, verifyRelease } from './release-version.mjs';

const script = fileURLToPath(new URL('./release-version.mjs', import.meta.url));
const planner = fileURLToPath(new URL('./plan-release.mjs', import.meta.url));

/** @param {import('node:test').TestContext} t */
function fixture(t) {
  const cwd = mkdtempSync(join(tmpdir(), 'gamepad-release-'));
  t.after(() => rmSync(cwd, { recursive: true, force: true }));
  mkdirSync(join(cwd, 'desktop'));
  writeFileSync(
    join(cwd, 'desktop/Cargo.toml'),
    '[package]\nname = "gamepad-desktop"\nversion = "0.1.0"\n\n[dependencies]\nexample = { version = "0.1.0" }\n',
  );
  writeFileSync(
    join(cwd, 'desktop/Cargo.lock'),
    'version = 4\n\n[[package]]\nname = "example"\nversion = "0.1.0"\n\n[[package]]\nname = "gamepad-desktop"\nversion = "0.1.0"\ndependencies = ["example"]\n',
  );
  return cwd;
}

/** @param {string} cwd @param {string} path */
const read = (cwd, path) => readFileSync(join(cwd, path), 'utf8');

test('updates the application in both Cargo files without changing dependencies', (t) => {
  const cwd = fixture(t);
  setCargoVersion('1.2.3', cwd);
  assert.match(read(cwd, 'desktop/Cargo.toml'), /name = "gamepad-desktop"\nversion = "1.2.3"/);
  assert.match(read(cwd, 'desktop/Cargo.toml'), /example = \{ version = "0.1.0" \}/);
  assert.match(read(cwd, 'desktop/Cargo.lock'), /name = "gamepad-desktop"\nversion = "1.2.3"/);
  assert.match(read(cwd, 'desktop/Cargo.lock'), /name = "example"\nversion = "0.1.0"/);
  const previous = read(cwd, 'desktop/Cargo.lock');
  setCargoVersion('1.2.3', cwd);
  assert.equal(read(cwd, 'desktop/Cargo.lock'), previous);
});

test('preserves Windows line endings', (t) => {
  const cwd = fixture(t);
  for (const path of ['desktop/Cargo.toml', 'desktop/Cargo.lock']) {
    writeFileSync(join(cwd, path), read(cwd, path).replaceAll('\n', '\r\n'));
  }
  setCargoVersion('2.0.0', cwd);
  assert.match(read(cwd, 'desktop/Cargo.toml'), /version = "2.0.0"\r\n/);
  assert.match(read(cwd, 'desktop/Cargo.lock'), /version = "2.0.0"\r\n/);
});

test('does not change the extension or root npm package version', (t) => {
  const cwd = fixture(t);
  mkdirSync(join(cwd, 'extension'));
  for (const path of ['package.json', 'extension/package.json']) {
    writeFileSync(join(cwd, path), '{"version":"0.1.0"}\n');
  }
  setCargoVersion('1.2.3', cwd);
  assert.equal(read(cwd, 'package.json'), '{"version":"0.1.0"}\n');
  assert.equal(read(cwd, 'extension/package.json'), '{"version":"0.1.0"}\n');
});

test('rejects invalid versions and malformed lockfiles before writing either file', (t) => {
  const cwd = fixture(t);
  const original = read(cwd, 'desktop/Cargo.toml');
  assert.throws(() => setCargoVersion('1.2.3\nother=value', cwd), /version/i);
  writeFileSync(join(cwd, 'desktop/Cargo.lock'), 'version = 4\n');
  assert.throws(() => setCargoVersion('1.2.3', cwd), /Cargo.lock/);
  assert.equal(read(cwd, 'desktop/Cargo.toml'), original);
});

test('publication rejects an unplanned or different version before changing Cargo', (t) => {
  const cwd = fixture(t);
  const context = { cwd, nextRelease: { version: '1.2.3' }, options: { dryRun: false } };
  for (const version of [undefined, '1.2.2']) {
    assert.throws(
      () => prepare({}, { ...context, env: { GAMEPAD_DESKTOP_RELEASE_VERSION: version } }),
      /planned/i,
    );
  }
  assert.match(read(cwd, 'desktop/Cargo.toml'), /version = "0.1.0"/);
  verifyRelease({}, { ...context, env: {}, options: { dryRun: true } });
  prepare({}, { ...context, env: { GAMEPAD_DESKTOP_RELEASE_VERSION: '1.2.3' } });
  assert.match(read(cwd, 'desktop/Cargo.toml'), /version = "1.2.3"/);
});

for (const [message, expected] of [
  ['fix: repair keyboard dismissal', '0.5.1'],
  ['feat: add guided onboarding', '0.6.0'],
  ['feat: change the controller contract\n\nBREAKING CHANGE: controller bindings changed', '1.0.0'],
  ['chore: refresh documentation', ''],
]) {
  test(`plans ${expected || 'no release'} for ${message} without publishing or editing Cargo`, (t) => {
    const cwd = fixture(t);
    const remote = join(cwd, 'remote.git');
    /** @param {string[]} args */
    const git = (args) =>
      execFileSync('git', args, { cwd, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });
    git(['init', '--initial-branch=main']);
    git(['config', 'user.email', 'release-test@example.invalid']);
    git(['config', 'user.name', 'Release test']);
    git(['init', '--bare', '--initial-branch=main', remote]);
    git(['remote', 'add', 'origin', remote]);
    writeFileSync(
      join(cwd, '.releaserc.json'),
      JSON.stringify({
        branches: ['main'],
        tagFormat: 'desktop-v${version}',
        repositoryUrl: pathToFileURL(remote).href,
        plugins: ['@semantic-release/commit-analyzer', script],
      }),
    );
    git(['add', 'desktop', '.releaserc.json']);
    git(['commit', '-m', 'chore: initial']);
    git(['tag', 'desktop-v0.5.0']);
    git(['commit', '--allow-empty', '-m', message]);
    git(['push', '--tags', '-u', 'origin', 'main']);
    const original = read(cwd, 'desktop/Cargo.toml');
    const tags = git(['tag', '--list']);
    const output = join(cwd, 'github-output');
    execFileSync(process.execPath, [planner], {
      cwd,
      env: { ...process.env, CI: 'false', GITHUB_ACTIONS: '', GITHUB_OUTPUT: output },
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    assert.equal(readFileSync(output, 'utf8'), `version=${expected}\n`);
    assert.equal(read(cwd, 'desktop/Cargo.toml'), original);
    assert.equal(git(['tag', '--list']), tags);
    if (expected) {
      execFileSync(process.execPath, [script], {
        cwd,
        env: { ...process.env, GAMEPAD_DESKTOP_RELEASE_VERSION: expected },
        stdio: ['ignore', 'pipe', 'pipe'],
      });
      assert.ok(read(cwd, 'desktop/Cargo.toml').includes(`version = "${expected}"`));
      assert.ok(read(cwd, 'desktop/Cargo.lock').includes(`version = "${expected}"`));
    }
  });
}

test('publishes matching Cargo metadata and tag only after verifying the planned version', (t) => {
  const cwd = fixture(t);
  const remote = join(cwd, 'remote.git');
  /** @param {string[]} args */
  const git = (args) =>
    execFileSync('git', args, { cwd, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });
  git(['init', '--initial-branch=main']);
  git(['config', 'user.email', 'release-test@example.invalid']);
  git(['config', 'user.name', 'Release test']);
  git(['init', '--bare', '--initial-branch=main', remote]);
  git(['remote', 'add', 'origin', remote]);
  writeFileSync(
    join(cwd, '.releaserc.json'),
    JSON.stringify({
      branches: ['main'],
      tagFormat: 'desktop-v${version}',
      repositoryUrl: pathToFileURL(remote).href,
      plugins: [
        '@semantic-release/commit-analyzer',
        script,
        ['@semantic-release/git', { assets: ['desktop/Cargo.toml', 'desktop/Cargo.lock'] }],
      ],
    }),
  );
  git(['add', 'desktop', '.releaserc.json']);
  git(['commit', '-m', 'chore: initial']);
  git(['tag', 'desktop-v0.5.0']);
  git(['commit', '--allow-empty', '-m', 'feat: improve onboarding']);
  git(['push', '--tags', '-u', 'origin', 'main']);
  const command = `import semanticRelease from ${JSON.stringify(import.meta.resolve('semantic-release'))}; await semanticRelease({ ci: false });`;
  /** @param {string} version */
  const publish = (version) =>
    execFileSync(process.execPath, ['--input-type=module', '--eval', command], {
      cwd,
      env: {
        ...process.env,
        CI: 'false',
        GITHUB_ACTIONS: '',
        GAMEPAD_DESKTOP_RELEASE_VERSION: version,
      },
      stdio: ['ignore', 'pipe', 'pipe'],
    });
  assert.throws(() => publish('0.5.1'), /planned/);
  assert.equal(git(['tag', '--list']), 'desktop-v0.5.0\n');
  assert.match(read(cwd, 'desktop/Cargo.toml'), /version = "0.1.0"/);
  publish('0.6.0');
  for (const path of ['desktop/Cargo.toml', 'desktop/Cargo.lock']) {
    assert.ok(git(['show', `desktop-v0.6.0:${path}`]).includes('version = "0.6.0"'));
  }
  assert.ok(git(['--git-dir', remote, 'tag', '--list']).includes('desktop-v0.6.0'));
});
