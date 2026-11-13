// Setzt die Version überall gleichzeitig: package.json, package-lock.json,
// src-tauri/tauri.conf.json und src-tauri/Cargo.toml (Workspace).
// Aufruf: npm run version:set -- 0.2.0
import { readFileSync, writeFileSync } from 'node:fs';

const version = process.argv[2];
if (!/^\d+\.\d+\.\d+(-[0-9A-Za-z.]+)?$/.test(version ?? '')) {
  console.error('Aufruf: npm run version:set -- <x.y.z>   (z. B. 0.2.0 oder 0.2.0-rc1)');
  process.exit(2);
}

const root = new URL('../', import.meta.url);
const json = (file, edit) => {
  const url = new URL(file, root);
  const data = JSON.parse(readFileSync(url, 'utf8'));
  edit(data);
  writeFileSync(url, `${JSON.stringify(data, null, 2)}\n`);
};

json('package.json', (d) => (d.version = version));
json('package-lock.json', (d) => {
  d.version = version;
  if (d.packages?.['']) d.packages[''].version = version;
});
json('src-tauri/tauri.conf.json', (d) => (d.version = version));

const cargo = new URL('src-tauri/Cargo.toml', root);
const text = readFileSync(cargo, 'utf8');
const next = text.replace(/(\[workspace\.package\][^[]*?\nversion\s*=\s*)"[^"]+"/, `$1"${version}"`);
if (next === text && !text.includes(`version = "${version}"`)) {
  console.error('Version in src-tauri/Cargo.toml nicht gefunden.');
  process.exit(1);
}
writeFileSync(cargo, next);
console.log(`Version ${version} gesetzt. Cargo.lock aktualisiert sich beim nächsten Build.`);
