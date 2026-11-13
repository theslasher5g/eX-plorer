// Kopiert die Laufzeitdateien von pdf.js (Standardschriften, WebAssembly) nach
// public/pdfjs, damit sie ohne Netzwerk aus der App geladen werden.
import { cpSync, existsSync, mkdirSync, rmSync } from 'node:fs';

const root = new URL('../', import.meta.url);
const source = new URL('node_modules/pdfjs-dist/', root);
const target = new URL('public/pdfjs/', root);

if (!existsSync(source)) {
  console.error('pdfjs-dist is missing. Run "npm install" first.');
  process.exit(1);
}
rmSync(target, { recursive: true, force: true });
mkdirSync(target, { recursive: true });
for (const dir of ['standard_fonts', 'wasm']) {
  cpSync(new URL(`${dir}/`, source), new URL(`${dir}/`, target), { recursive: true });
}
console.log('pdf.js-Dateien nach public/pdfjs kopiert.');
