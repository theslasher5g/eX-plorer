// Demo-Backend für die Entwicklung im Browser. Wird nur im Dev-Modus geladen
// und greift auf kein echtes Dateisystem zu.
import type { Api, Entry } from './types';

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
    const normalized = path.length > 3 ? path.replace(/\\+$/, '') : path;
    await sleep(40);
    if (normalized === 'C:\\Windows') throw { code: 'access_denied', message: 'access denied' };
    const entries = normalized === 'C:\\Test\\100k' ? hugeFolder() : tree[normalized];
    if (!entries) throw { code: 'not_found', message: 'not found' };
    for (let i = 0; i < entries.length; i += 2000) onBatch(entries.slice(i, i + 2000));
    return { path: normalized, count: entries.length };
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
