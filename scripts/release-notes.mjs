// Prints the section of one version from CHANGELOG.md (used for the release notes).
// Usage: node scripts/release-notes.mjs 0.1.0
import { readFileSync } from 'node:fs';

const version = process.argv[2];
if (!version) {
  console.error('Usage: node scripts/release-notes.mjs <version>');
  process.exit(2);
}
const lines = readFileSync(new URL('../CHANGELOG.md', import.meta.url), 'utf8').split('\n');
const start = lines.findIndex((l) => l.startsWith(`## [${version}]`));
if (start < 0) {
  console.error(`Version ${version} not found in CHANGELOG.md.`);
  process.exit(1);
}
let end = lines.findIndex((l, i) => i > start && l.startsWith('## ['));
if (end < 0) end = lines.length;
// Link reference definitions at the end of the file are not part of the notes.
const body = lines
  .slice(start + 1, end)
  .filter((l) => !/^\[[^\]]+\]: https?:\/\//.test(l))
  .join('\n')
  .trim();
console.log(body);
