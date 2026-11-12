import { convertFileSrc } from '@tauri-apps/api/core';
import { inTauri } from './api';
import { fileCategory } from './filetypes';
import { extension } from './paths';

const FILE_ATTRIBUTE_OFFLINE = 0x1000;
const FILE_ATTRIBUTE_RECALL_ON_OPEN = 0x40000;
const FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS = 0x400000;

/** Nur in der Cloud (OneDrive): dafür werden keine Icons oder Thumbnails angefragt. */
export function isCloudPlaceholder(attributes: number): boolean {
  return (
    (attributes &
      (FILE_ATTRIBUTE_OFFLINE | FILE_ATTRIBUTE_RECALL_ON_OPEN | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS)) !==
    0
  );
}

/**
 * URL des Windows-Icons bzw. Thumbnails einer Datei (vom Backend-Schema
 * `shellimg`). Die Änderungszeit in der URL sorgt dafür, dass sich geänderte
 * Dateien neu laden, alles andere aber aus dem Browser-Cache kommt.
 */
export function shellImageUrl(path: string, size: number, modifiedMs: number | null): string {
  if (inTauri) {
    const base = convertFileSrc('', 'shellimg');
    return `${base}?p=${encodeURIComponent(path)}&s=${size}&m=${modifiedMs ?? 0}`;
  }
  return demoImage(path, size);
}

// Platzhalterbilder für die Entwicklung im Browser (ohne Backend).
const DEMO_COLORS: Record<string, string> = {
  pdf: '#B3261E',
  doc: '#1F5FBF',
  xls: '#17784A',
  ppt: '#B4540A',
  img: '#7A3FB8',
  vid: '#B0245F',
  zip: '#8A6A00',
  txt: '#5B6372',
};

function demoImage(path: string, size: number): string {
  const name = path.slice(path.lastIndexOf('\\') + 1);
  const color = DEMO_COLORS[fileCategory(name)] ?? '#5B6372';
  const label = extension(name).slice(0, 4).toUpperCase();
  const svg =
    `<svg xmlns="http://www.w3.org/2000/svg" width="${size}" height="${size}" viewBox="0 0 32 32">` +
    `<path d="M7 2h13l6 6v20a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2z" fill="#fff" stroke="${color}" stroke-width="1.6"/>` +
    `<text x="16" y="23" font-family="sans-serif" font-size="7" font-weight="700" text-anchor="middle" fill="${color}">${label}</text></svg>`;
  return `data:image/svg+xml,${encodeURIComponent(svg)}`;
}
