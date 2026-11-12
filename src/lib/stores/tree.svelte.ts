import { getApi, toApiError, type ApiError, type TreeChunk, type TreeEntry } from '../api';
import { formatCount } from '../format';
import { t } from '../i18n';
import { joinPath, splitPath } from '../paths';
import { settings } from './settings.svelte';

interface TreeNode {
  entries: TreeEntry[];
  more: number;
  error: string | null;
}

export type TreeRow =
  | {
      kind: 'root';
      path: string;
      name: string;
    }
  | {
      kind: 'entry';
      path: string;
      dir: string;
      entry: TreeEntry;
      /** Je Vorfahren-Ebene: läuft dort eine senkrechte Linie weiter? */
      guides: boolean[];
      last: boolean;
      expanded: boolean;
      loading: boolean;
      error: string | null;
    }
  | {
      kind: 'more';
      path: string;
      dir: string;
      count: number;
      guides: boolean[];
      loading: boolean;
    };

let nextScanId = 1;

/** Zustand der Baumansicht eines Tabs. */
export class TreeState {
  root = $state('');
  scanning = $state(false);
  /** Bisher gelesene Einträge des laufenden bzw. letzten Scans. */
  scanned = $state(0);
  truncated = $state(false);
  cancelled = $state(false);
  remoteLimited = $state(false);
  error = $state<ApiError | null>(null);
  /** Pfad der markierten Zeile. */
  selected = $state<string | null>(null);

  // Der Baum selbst ist nicht reaktiv: Chunks kommen schnell hintereinander
  // und werden pro Frame gebündelt über `#version` sichtbar gemacht.
  #nodes = new Map<string, TreeNode>();
  #expanded = new Set<string>();
  #loading = new Set<string>();
  #version = $state(0);
  #frame = 0;
  #scanId = 0;

  #bump(): void {
    if (this.#frame) return;
    this.#frame = requestAnimationFrame(() => {
      this.#frame = 0;
      this.#version++;
    });
  }

  readonly rows = $derived.by((): TreeRow[] => {
    void this.#version;
    if (!this.root || !this.#nodes.has(this.root)) return [];
    const rows: TreeRow[] = [
      { kind: 'root', path: this.root, name: splitPath(this.root).at(-1)?.label ?? this.root },
    ];
    const walk = (dir: string, guides: boolean[]) => {
      const node = this.#nodes.get(dir);
      if (!node) return;
      const count = node.entries.length + (node.more > 0 ? 1 : 0);
      node.entries.forEach((entry, i) => {
        const path = joinPath(dir, entry.name);
        const last = i === count - 1;
        const child = this.#nodes.get(path);
        const expanded = entry.isDir && this.#expanded.has(path);
        rows.push({
          kind: 'entry',
          path,
          dir,
          entry,
          guides,
          last,
          expanded,
          loading: this.#loading.has(path),
          error: child?.error ?? null,
        });
        if (expanded && child) walk(path, [...guides, !last]);
      });
      if (node.more > 0) {
        rows.push({
          kind: 'more',
          path: `${dir}\\…`,
          dir,
          count: node.more,
          guides,
          loading: this.#loading.has(`${dir}\\…`),
        });
      }
    };
    walk(this.root, []);
    return rows;
  });

  #apply(base: string, chunk: TreeChunk, append: boolean): string {
    const dir = chunk.rel.reduce(joinPath, base);
    const previous = append ? this.#nodes.get(dir) : undefined;
    this.#nodes.set(dir, {
      entries: previous ? [...previous.entries, ...chunk.entries] : chunk.entries,
      more: chunk.more,
      error: chunk.error,
    });
    this.#expanded.add(dir);
    this.#bump();
    return dir;
  }

  /** Scannt `root` neu, mit den Limits aus den Einstellungen. */
  async start(root: string): Promise<void> {
    this.cancel();
    const scanId = (this.#scanId = nextScanId++);
    this.#nodes = new Map();
    this.#expanded = new Set();
    this.#loading = new Set();
    this.root = root;
    this.scanning = true;
    this.scanned = 0;
    this.truncated = false;
    this.cancelled = false;
    this.remoteLimited = false;
    this.error = null;
    this.selected = null;
    this.#bump();
    if (!root) {
      this.scanning = false;
      return;
    }

    const { treeDepth, treeTotal, treePerDir, showHidden } = settings.value;
    try {
      const api = await getApi();
      const summary = await api.treeScan(
        { path: root, scanId, depth: treeDepth, total: treeTotal, perDir: treePerDir, showHidden },
        (chunk) => {
          if (scanId !== this.#scanId) return;
          this.#apply(root, chunk, false);
          this.scanned += chunk.entries.length;
        },
      );
      if (scanId !== this.#scanId) return;
      this.truncated = summary.truncated;
      this.cancelled = summary.cancelled;
      this.remoteLimited = summary.remoteLimited;
    } catch (err) {
      if (scanId !== this.#scanId) return;
      this.error = toApiError(err);
    } finally {
      if (scanId === this.#scanId) this.scanning = false;
    }
  }

  /** Bricht den laufenden Scan ab; bereits Gelesenes bleibt sichtbar. */
  cancel(): void {
    if (!this.scanning) return;
    const scanId = this.#scanId;
    void getApi()
      .then((api) => api.treeCancel(scanId))
      .catch(() => {});
  }

  /** Liest genau einen Ordner (Aufklappen bzw. „weitere laden“). */
  async #loadDir(dir: string, offset: number, marker: string): Promise<void> {
    if (this.#loading.has(marker)) return;
    this.#loading.add(marker);
    this.#bump();
    const root = this.root;
    const { treePerDir, showHidden } = settings.value;
    try {
      const api = await getApi();
      await api.treeScan(
        { path: dir, scanId: nextScanId++, depth: 1, total: treePerDir, perDir: treePerDir, offset, showHidden },
        (chunk) => {
          if (root === this.root) this.#apply(dir, chunk, offset > 0);
        },
      );
    } catch (err) {
      if (root === this.root && offset === 0) {
        this.#nodes.set(dir, { entries: [], more: 0, error: toApiError(err).code });
      }
    } finally {
      this.#loading.delete(marker);
      this.#bump();
    }
  }

  toggle(path: string): void {
    if (this.#expanded.has(path)) {
      this.#expanded.delete(path);
      this.#bump();
      return;
    }
    this.#expanded.add(path);
    this.#bump();
    // Lazy Loading: gelesen wird erst jetzt.
    if (!this.#nodes.has(path)) void this.#loadDir(path, 0, path);
  }

  isExpanded(path: string): boolean {
    return this.#expanded.has(path);
  }

  loadMore(dir: string): void {
    const node = this.#nodes.get(dir);
    if (node) void this.#loadDir(dir, node.entries.length, `${dir}\\…`);
  }

  /** Der sichtbare Baum im Format des `tree`-Befehls. */
  toText(): string {
    const lines: string[] = [];
    for (const row of this.rows) {
      if (row.kind === 'root') {
        lines.push(row.path);
        continue;
      }
      const prefix = row.guides.map((open) => (open ? '│   ' : '    ')).join('');
      if (row.kind === 'more') {
        lines.push(`${prefix}└── ${t('tree.more', { count: formatCount(row.count) })}`);
        continue;
      }
      let label = row.entry.name;
      if (row.error) label += ` [${errorLabel(row.error)}]`;
      else if (row.entry.isLink) label += ` [${t('tree.link')}]`;
      lines.push(`${prefix}${row.last ? '└── ' : '├── '}${label}`);
    }
    return lines.join('\n');
  }
}

export function errorLabel(code: string): string {
  return t(code === 'access_denied' ? 'tree.denied' : 'tree.unreadable');
}
