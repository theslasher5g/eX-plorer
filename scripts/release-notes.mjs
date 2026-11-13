// Gibt den Abschnitt einer Version aus CHANGELOG.md aus (für die Release-Notizen).
// Aufruf: node scripts/release-notes.mjs 0.1.0
import { readFileSync } from 'node:fs';

const version = process.argv[2];
if (!version) {
  console.error('Aufruf: node scripts/release-notes.mjs <version>');
  process.exit(2);
}
const lines = readFileSync(new URL('../CHANGELOG.md', import.meta.url), 'utf8').split('\n');
const start = lines.findIndex((l) => l.startsWith(`## [${version}]`));
if (start < 0) {
  console.error(`Keine Version ${version} im CHANGELOG.md gefunden.`);
  process.exit(1);
}
let end = lines.findIndex((l, i) => i > start && l.startsWith('## ['));
if (end < 0) end = lines.length;
// Verweis-Definitionen am Dateiende gehören nicht zu den Notizen.
const body = lines
  .slice(start + 1, end)
  .filter((l) => !/^\[[^\]]+\]: https?:\/\//.test(l))
  .join('\n')
  .trim();
console.log(body);
