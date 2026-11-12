import type { Entry } from './api';
import { t } from './i18n';
import { extension } from './paths';

/** Farbfamilie des Dateityp-Kürzels (CSS-Variable `--t-<kategorie>`). */
export type FileCategory = 'pdf' | 'doc' | 'xls' | 'ppt' | 'img' | 'vid' | 'zip' | 'txt';

const KNOWN: Record<string, [FileCategory, string]> = {
  pdf: ['pdf', 'PDF-Dokument'],
  doc: ['doc', 'Word-Dokument'],
  docx: ['doc', 'Word-Dokument'],
  odt: ['doc', 'OpenDocument-Text'],
  rtf: ['doc', 'RTF-Dokument'],
  xls: ['xls', 'Excel-Tabelle'],
  xlsx: ['xls', 'Excel-Tabelle'],
  ods: ['xls', 'OpenDocument-Tabelle'],
  csv: ['xls', 'CSV-Datei'],
  ppt: ['ppt', 'PowerPoint-Präsentation'],
  pptx: ['ppt', 'PowerPoint-Präsentation'],
  jpg: ['img', 'JPEG-Bild'],
  jpeg: ['img', 'JPEG-Bild'],
  png: ['img', 'PNG-Bild'],
  gif: ['img', 'GIF-Bild'],
  webp: ['img', 'WebP-Bild'],
  bmp: ['img', 'Bitmap-Bild'],
  svg: ['img', 'SVG-Grafik'],
  heic: ['img', 'HEIC-Bild'],
  ico: ['img', 'Symbol'],
  mp4: ['vid', 'MP4-Video'],
  mov: ['vid', 'QuickTime-Video'],
  mkv: ['vid', 'Matroska-Video'],
  avi: ['vid', 'AVI-Video'],
  webm: ['vid', 'WebM-Video'],
  mp3: ['vid', 'MP3-Audio'],
  wav: ['vid', 'WAV-Audio'],
  flac: ['vid', 'FLAC-Audio'],
  zip: ['zip', 'ZIP-Archiv'],
  '7z': ['zip', '7-Zip-Archiv'],
  rar: ['zip', 'RAR-Archiv'],
  tar: ['zip', 'TAR-Archiv'],
  gz: ['zip', 'GZip-Archiv'],
  iso: ['zip', 'Datenträgerabbild'],
  exe: ['zip', 'Anwendung'],
  msi: ['zip', 'Windows-Installer-Paket'],
  txt: ['txt', 'Textdokument'],
  md: ['txt', 'Markdown-Datei'],
  log: ['txt', 'Protokolldatei'],
  json: ['txt', 'JSON-Datei'],
};

export function fileCategory(name: string): FileCategory {
  return KNOWN[extension(name)]?.[0] ?? 'txt';
}

/** Kürzel für das Typ-Symbol, höchstens vier Zeichen; leer ohne Endung. */
export function extensionBadge(name: string): string {
  return extension(name).slice(0, 4);
}

export function typeLabel(entry: Entry): string {
  if (entry.isDir) return entry.isLink ? t('type.link') : t('type.folder');
  const ext = extension(entry.name);
  if (!ext) return t('type.file');
  return KNOWN[ext]?.[1] ?? t('type.fileExt', { ext: ext.toUpperCase() });
}
