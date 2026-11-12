// Demo-Backend für die Entwicklung im Browser. Wird nur im Dev-Modus geladen
// und greift auf kein echtes Dateisystem zu.
import type { Api, ClipboardFiles, DragDropEvent, Entry } from './types';

const DAY = 86_400_000;
const NOW = Date.UTC(2026, 10, 12, 9, 30);

function entry(name: string, isDir: boolean, size = 0, ageDays = 1, extra: Partial<Entry> = {}): Entry {
  return {
    name,
    isDir,
    size: isDir ? 0 : size,
    modifiedMs: NOW - ageDays * DAY,
    createdMs: NOW - (ageDays + 30) * DAY,
    attributes: 0,
    hidden: false,
    system: false,
    isLink: false,
    ...extra,
  };
}

const d = (name: string, age = 3, extra: Partial<Entry> = {}) => entry(name, true, 0, age, extra);
const f = (name: string, size: number, age = 3, extra: Partial<Entry> = {}) =>
  entry(name, false, size, age, extra);

const HOME = 'C:\\Users\\Demo';

const tree: Record<string, Entry[]> = {
  'C:\\': [
    d('Program Files', 40),
    d('Program Files (x86)', 40),
    d('Test', 2),
    d('Users', 90),
    d('Windows', 12),
    d('$Recycle.Bin', 200, { hidden: true, system: true }),
    f('pagefile.sys', 8_589_934_592, 0, { hidden: true, system: true }),
  ],
  'C:\\Users': [d('Demo', 1), d('Public', 300)],
  [HOME]: [
    d('Desktop', 0.2),
    d('Documents', 1),
    d('Downloads', 0.1),
    d('Pictures', 6),
    d('Music', 80),
    d('Videos', 45),
    d('OneDrive', 0.5),
    d('AppData', 3, { hidden: true }),
    f('.gitconfig', 214, 120),
    f('NTUSER.DAT', 12_582_912, 0, { hidden: true, system: true }),
  ],
  [`${HOME}\\Desktop`]: [f('Notizen.txt', 1_204, 0.3), f('Präsentation Q4.pptx', 4_812_330, 2)],
  [`${HOME}\\Documents`]: [
    d('Projekte', 1),
    d('Rechnungen', 14),
    d('Steuer 2025', 60),
    f('Bewerbung.docx', 48_211, 9),
    f('Budget 2026.xlsx', 91_830, 4),
    f('Lebenslauf.pdf', 233_914, 9),
    f('Datei 2.txt', 80, 5),
    f('Datei 10.txt', 81, 5),
    f('Datei 1.txt', 79, 5),
    f('README', 512, 30),
  ],
  [`${HOME}\\Documents\\Projekte`]: [d('eX-plorer', 0.1), d('Website', 21, { isLink: true })],
  [`${HOME}\\Downloads`]: [
    f('setup-7zip-x64.exe', 1_589_248, 0.1),
    f('Urlaub.zip', 734_003_200, 1),
    f('ubuntu-26.04-desktop-amd64.iso', 6_114_656_256, 12),
    f('Handbuch.pdf', 2_411_724, 3),
  ],
  [`${HOME}\\Pictures`]: [
    d('Screenshots', 2),
    f('IMG_0412.jpg', 3_812_044, 8),
    f('IMG_0413.jpg', 4_102_113, 8),
    f('Logo.svg', 2_048, 33),
    f('Panorama.png', 18_440_221, 70),
  ],
  [`${HOME}\\OneDrive`]: [d('Dokumente', 2), f('Geteilt.docx', 20_480, 1)],
  'D:\\': [d('Backups', 7), d('Medien', 30)],
  'E:\\': [f('Foto.jpg', 2_048_000, 4), d('USB-Ordner', 9)],
  'E:\\USB-Ordner': [],
  'C:\\Test': [d('100k', 1), d('Leer', 1)],
  'C:\\Test\\Leer': [],
};

function hugeFolder(): Entry[] {
  const exts = ['txt', 'jpg', 'pdf', 'log', 'json', 'dat'];
  const out: Entry[] = [];
  for (let i = 0; i < 100_000; i++) {
    out.push(f(`datei-${String(i).padStart(6, '0')}.${exts[i % exts.length]}`, (i * 7919) % 5_000_000, i % 400));
  }
  return out;
}

const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

// --- Veränderbares Dateisystem im Speicher -------------------------------

const lower = (s: string) => s.toLowerCase();
const norm = (path: string) => (path.length > 3 ? path.replace(/\\+$/, '') : path);
const parentOf = (path: string) => {
  const i = path.lastIndexOf('\\');
  return i <= 2 ? path.slice(0, 3) : path.slice(0, i);
};
const nameOf = (path: string) => path.slice(path.lastIndexOf('\\') + 1);
const join = (dir: string, name: string) => (dir.endsWith('\\') ? dir + name : `${dir}\\${name}`);
const driveOf = (path: string) => lower(path.slice(0, 2));
const isInside = (path: string, ancestor: string) =>
  lower(path) === lower(ancestor) || lower(path).startsWith(lower(ancestor.replace(/\\$/, '')) + '\\');

function key(path: string): string | undefined {
  return Object.keys(tree).find((k) => lower(k) === lower(path));
}

function find(dir: string, name: string): Entry | undefined {
  return tree[key(dir) ?? '']?.find((e) => lower(e.name) === lower(name));
}

function fail(code: string): never {
  throw { code, message: code };
}

function validName(name: string) {
  if (!name || /[\\/:*?"<>|]/.test(name) || /[. ]$/.test(name) || name === '.' || name === '..') fail('invalid_path');
  if (/^(con|prn|aux|nul|com\d|lpt\d)(\..*)?$/i.test(name)) fail('invalid_path');
}

/** Alle Schlüssel des Baums, die `path` selbst oder darunter liegen. */
function subtree(path: string): string[] {
  return Object.keys(tree).filter((k) => isInside(k, path));
}

function copyTree(from: string, to: string) {
  for (const k of subtree(from)) {
    const target = lower(k) === lower(from) ? to : to + k.slice(from.length);
    tree[target] = tree[k].map((e) => ({ ...e }));
  }
}

function removeTree(path: string) {
  for (const k of subtree(path)) delete tree[k];
}

const handlers = new Set<(path: string) => void>();
const dropHandlers = new Set<(event: DragDropEvent) => void>();

/** Simuliert den Watcher: meldet geänderte Ordner kurz verzögert. */
function changed(...dirs: string[]) {
  setTimeout(() => {
    for (const dir of new Set(dirs.map(norm))) handlers.forEach((h) => h(dir));
  }, 30);
}

let clipboard: ClipboardFiles | null = null;
let watched: string[] = [];

if (import.meta.env.DEV && typeof window !== 'undefined') {
  // Hilfen für automatisierte Tests im Browser.
  Object.assign(window, {
    __mockDrop: (type: DragDropEvent['type'], paths: string[], x: number, y: number) => {
      const dpr = window.devicePixelRatio;
      dropHandlers.forEach((h) => h({ type, paths, position: { x: x * dpr, y: y * dpr } }));
    },
    __mockState: () => ({ tree, clipboard, watched }),
  });
}

function uniqueName(dir: string, wanted: string, suffix: string): string {
  if (!find(dir, wanted)) return wanted;
  const dot = wanted.lastIndexOf('.');
  const [stem, ext] = dot > 0 ? [wanted.slice(0, dot), wanted.slice(dot)] : [wanted, ''];
  for (let n = 1; ; n++) {
    const candidate = `${stem}${suffix}${n > 1 ? ` (${n})` : ''}${ext}`;
    if (!find(dir, candidate)) return candidate;
  }
}

function doTransfer(sources: string[], dest: string, move: boolean) {
  const destKey = key(norm(dest));
  if (!destKey) fail('not_found');
  const names: string[] = [];
  for (const raw of sources) {
    const src = norm(raw);
    const entry = find(parentOf(src), nameOf(src));
    if (!entry) fail('not_found');
    if (src.length <= 3) fail('root_not_allowed');
    if (isInside(destKey, src)) fail('into_itself');
    const sameDir = lower(parentOf(src)) === lower(destKey);
    if (sameDir && move) continue;
    const name = uniqueName(destKey, entry.name, sameDir ? ' - Kopie' : ' (2)');
    tree[destKey].push({ ...entry, name, modifiedMs: NOW });
    if (entry.isDir) copyTree(src, join(destKey, name));
    if (move) {
      tree[key(parentOf(src))!] = tree[key(parentOf(src))!].filter((e) => e !== entry);
      if (entry.isDir) removeTree(src);
    }
    names.push(name);
  }
  return names;
}

export const mockApi: Api = {
  async appInfo() {
    return { material: 'solid', homeDir: HOME };
  },
  async sidebar() {
    return {
      favorites: [
        { kind: 'desktop', label: '', path: `${HOME}\\Desktop` },
        { kind: 'documents', label: '', path: `${HOME}\\Documents` },
        { kind: 'downloads', label: '', path: `${HOME}\\Downloads` },
        { kind: 'pictures', label: '', path: `${HOME}\\Pictures` },
      ],
      drives: [
        { path: 'C:\\', label: 'Windows', kind: 'fixed', totalBytes: 511e9, freeBytes: 187e9 },
        { path: 'D:\\', label: 'Daten', kind: 'fixed', totalBytes: 2e12, freeBytes: 1.2e12 },
        { path: 'E:\\', label: '', kind: 'removable', totalBytes: null, freeBytes: null },
        { path: 'Z:\\', label: '', kind: 'remote', totalBytes: null, freeBytes: null },
      ],
      onedrive: [{ kind: 'onedrive', label: 'OneDrive', path: `${HOME}\\OneDrive` }],
    };
  },
  async listDir(path, onBatch) {
    const normalized = norm(path);
    await sleep(40);
    if (lower(normalized) === 'c:\\windows') throw { code: 'access_denied', message: 'access denied' };
    const k = key(normalized);
    const entries = lower(normalized) === 'c:\\test\\100k' ? hugeFolder() : k ? tree[k] : undefined;
    if (!entries) throw { code: 'not_found', message: 'not found' };
    const copy = entries.map((e) => ({ ...e }));
    for (let i = 0; i < copy.length; i += 2000) onBatch(copy.slice(i, i + 2000));
    return { path: k ?? normalized, count: copy.length };
  },

  async newFolder(parent, base) {
    validName(base);
    const k = key(norm(parent));
    if (!k) fail('not_found');
    let final = base;
    for (let n = 2; find(k, final); n++) final = `${base} (${n})`;
    tree[k].push(d(final, 0));
    tree[join(k, final)] = [];
    changed(k);
    return final;
  },
  async renameItem(path, newName) {
    const src = norm(path);
    if (src.length <= 3) fail('root_not_allowed');
    validName(newName);
    const parent = parentOf(src);
    const entry = find(parent, nameOf(src));
    if (!entry) fail('not_found');
    if (entry.name === newName) return { name: newName, path: src, aborted: false };
    const clash = find(parent, newName);
    if (clash && clash !== entry) fail('already_exists');
    const target = join(parent, newName);
    entry.name = newName;
    if (entry.isDir) {
      for (const k of subtree(src)) {
        const moved = lower(k) === lower(src) ? target : target + k.slice(src.length);
        tree[moved] = tree[k];
        delete tree[k];
      }
    }
    changed(parent);
    return { name: newName, path: target, aborted: false };
  },
  async transfer(sources, dest, mode) {
    await sleep(30);
    const sameDrive = sources.every((s) => driveOf(s) === driveOf(dest));
    const move = mode === 'move' || (mode === 'auto' && sameDrive);
    const names = doTransfer(sources, dest, move);
    changed(dest, ...sources.map((s) => parentOf(norm(s))));
    return { aborted: false, count: names.length, moved: move, names };
  },
  async deleteItems(paths, permanent) {
    void permanent;
    await sleep(30);
    const names: string[] = [];
    for (const raw of paths) {
      const p = norm(raw);
      if (p.length <= 3) fail('root_not_allowed');
      const parent = key(parentOf(p));
      const entry = parent ? find(parent, nameOf(p)) : undefined;
      if (!parent || !entry) fail('not_found');
      tree[parent] = tree[parent].filter((e) => e !== entry);
      if (entry.isDir) removeTree(p);
      names.push(entry.name);
    }
    changed(...paths.map((p) => parentOf(norm(p))));
    return { aborted: false, count: names.length, moved: false, names };
  },
  async clipboardSet(paths, cut) {
    clipboard = { paths: paths.map(norm), cut };
  },
  async clipboardGet() {
    return clipboard ? { ...clipboard } : null;
  },
  async paste(dest) {
    if (!clipboard) fail('nothing_to_paste');
    const { paths, cut } = clipboard;
    const names = doTransfer(paths, dest, cut);
    if (cut) clipboard = null;
    changed(dest, ...paths.map((p) => parentOf(p)));
    return { aborted: false, count: names.length, moved: cut, names, cut };
  },
  async startDrag(paths) {
    void paths;
    return 1;
  },
  async watchDirs(paths) {
    watched = paths;
  },
  async onDirChanged(handler) {
    handlers.add(handler);
    return () => handlers.delete(handler);
  },
  async onDragDrop(handler) {
    dropHandlers.add(handler);
    return () => dropHandlers.delete(handler);
  },
  window: {
    async minimize() {},
    async toggleMaximize() {},
    async close() {},
    async isMaximized() {
      return false;
    },
    async onResized() {
      return () => {};
    },
  },
};
