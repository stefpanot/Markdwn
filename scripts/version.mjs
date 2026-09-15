import { readFileSync, writeFileSync, appendFileSync, realpathSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const root = new URL('../', import.meta.url);
const read = (path) => readFileSync(new URL(path, root), 'utf8');
const packageBlock = /(^\[package\]\r?\n)([\s\S]*?)(?=^\[|$(?![\s\S]))/m;
const lockBlock = /(^\[\[package\]\]\r?\nname = "markdwn"\r?\nversion = ")([^"]+)(")/m;

export function parseVersion(version) {
  const match = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-beta\.([1-9]\d*))?$/.exec(version);
  if (!match) throw new Error('Version attendue : X.Y.Z ou X.Y.Z-beta.N (N >= 1).');
  const [, major, minor, patch, beta] = match;
  if (+major > 255 || +minor > 255 || +patch > 65535 || (beta && +beta > 65534)) {
    throw new Error('Version hors des limites des installateurs Windows.');
  }
  return { version, prerelease: Boolean(beta) };
}

export function checkTag(version, tag) {
  if (tag !== `v${version}`) throw new Error(`Le tag ${tag} ne correspond pas à v${version}.`);
}

function main() {
  const [command = 'check', value, ...extra] = process.argv.slice(2);
  if (extra.length || !['check', 'set', 'tag'].includes(command)
      || (command === 'check' && value) || (command !== 'check' && !value)) {
    throw new Error('Usage : node scripts/version.mjs check | set VERSION | tag TAG');
  }
  for (const path of ['package.json', 'src-tauri/tauri.conf.json']) {
    if (Object.hasOwn(JSON.parse(read(path)), 'version')) {
      throw new Error(`${path} ne doit pas définir version : la source est Cargo.toml.`);
    }
  }
  const cargo = read('src-tauri/Cargo.toml');
  const block = cargo.match(packageBlock);
  const current = block?.[2].match(/^version = "([^"]+)"/m)?.[1];
  if (!current) throw new Error('Version du paquet introuvable dans Cargo.toml.');
  const lock = read('src-tauri/Cargo.lock');
  const locked = lock.match(lockBlock)?.[2];
  if (!locked) throw new Error('Paquet markdwn introuvable dans Cargo.lock.');
  const info = parseVersion(command === 'set' ? value : current);
  if (command === 'set') {
    const updatedBlock = block[0].replace(/^version = "[^"]+"/m, `version = "${value}"`);
    writeFileSync(new URL('src-tauri/Cargo.toml', root), cargo.replace(block[0], updatedBlock));
    writeFileSync(new URL('src-tauri/Cargo.lock', root), lock.replace(lockBlock, (_, before, previous, after) => `${before}${value}${after}`));
  } else if (locked !== current) {
    throw new Error('Cargo.lock désynchronisé : utiliser npm run version:set -- VERSION.');
  }
  if (command === 'tag') checkTag(current, value);
  console.log(`${info.version} (${info.prerelease ? 'beta' : 'stable'})`);
  if (process.env.GITHUB_OUTPUT) {
    appendFileSync(process.env.GITHUB_OUTPUT, `version=${info.version}\nprerelease=${info.prerelease}\n`);
  }
}

if (process.argv[1] && fileURLToPath(import.meta.url) === realpathSync(process.argv[1])) {
  try { main(); } catch (error) { console.error(error.message); process.exitCode = 1; }
}
