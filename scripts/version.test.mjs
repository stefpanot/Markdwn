import { test } from 'node:test';
import assert from 'node:assert/strict';
import { parseVersion, checkTag } from './version.mjs';
import { mkdtempSync, mkdirSync, copyFileSync, readFileSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';

test('stable et beta sont distinguées', () => {
  assert.equal(parseVersion('0.2.0').prerelease, false);
  assert.equal(parseVersion('0.2.0-beta.12').prerelease, true);
});
test('versions ambiguës ou incompatibles rejetées', () => {
  for (const version of ['01.2.3', '1.2', '1.2.3-beta.0', '1.2.3-beta.01', '1.2.3-rc.1', '1.2.3+build', '256.0.0', '1.256.0', '1.2.65536', '1.2.3-beta.65535']) {
    assert.throws(() => parseVersion(version), undefined, version);
  }
});
test('le tag doit correspondre exactement à la version', () => {
  checkTag('0.2.0-beta.1', 'v0.2.0-beta.1');
  assert.throws(() => checkTag('0.2.0-beta.1', 'v0.2.0'));
  assert.throws(() => checkTag('0.2.0', '0.2.0'));
});

test('la commande synchronise uniquement le projet et détecte les dérives', () => {
  const dir = mkdtempSync(join(tmpdir(), 'markdwn-version-'));
  try {
    mkdirSync(join(dir, 'scripts'));
    mkdirSync(join(dir, 'src-tauri'));
    for (const path of ['scripts/version.mjs', 'package.json', 'src-tauri/Cargo.toml', 'src-tauri/Cargo.lock', 'src-tauri/tauri.conf.json']) {
      copyFileSync(new URL(`../${path}`, import.meta.url), join(dir, path));
    }
    const run = (...args) => spawnSync(process.execPath, [join(dir, 'scripts/version.mjs'), ...args], { encoding: 'utf8', env: { ...process.env, GITHUB_OUTPUT: '' } });
    const lockPath = join(dir, 'src-tauri/Cargo.lock');
    const before = readFileSync(lockPath, 'utf8');
    const changed = run('set', '0.2.0-beta.1');
    assert.equal(changed.status, 0, changed.stderr);
    assert.equal(run('tag', 'v0.2.0-beta.1').status, 0);
    const normalize = (text) => text.replace(/(name = "markdwn"\r?\nversion = ")[^"]+/, '$1VERSION');
    assert.equal(normalize(readFileSync(lockPath, 'utf8')), normalize(before));
    assert.equal(run('set', '0.2.0').status, 0);
    assert.equal(run('check').status, 0);
    const stable = readFileSync(lockPath, 'utf8');
    assert.notEqual(run('set', 'garbage').status, 0);
    assert.equal(readFileSync(lockPath, 'utf8'), stable);
    writeFileSync(lockPath, before);
    assert.notEqual(run('check').status, 0);
    assert.equal(run('set', '0.2.0').status, 0);
    for (const path of ['package.json', 'src-tauri/tauri.conf.json']) {
      const fullPath = join(dir, path);
      const original = readFileSync(fullPath, 'utf8');
      writeFileSync(fullPath, JSON.stringify({ ...JSON.parse(original), version: '0.2.0' }));
      assert.notEqual(run('check').status, 0, path);
      writeFileSync(fullPath, original);
    }
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
