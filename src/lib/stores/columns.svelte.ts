import { joinPath, samePath } from '../paths';
import { Pane } from './pane.svelte';

/**
 * Zustand der Spaltenansicht (wie im Finder). Die erste Spalte ist der Pane
 * des Tabs selbst; jede weitere zeigt den in der Spalte davor markierten
 * Ordner. Jede Spalte ist ein eigener `Pane`, damit Auswahl, Kopieren,
 * Löschen, Kontextmenü usw. für die aktive Spalte unverändert funktionieren.
 */
export class ColumnsState {
  /** Die Spalten rechts der ersten. */
  rest = $state.raw<Pane[]>([]);
  /** Index der Spalte, die die Tastatur bekommt. */
  activeIndex = $state(0);
  /** Nach dem Laden der aktiven Spalte deren ersten Eintrag markieren. */
  wantFirst = $state(false);

  readonly #root: Pane;

  constructor(root: Pane) {
    this.#root = root;
  }

  get all(): Pane[] {
    return [this.#root, ...this.rest];
  }

  get active(): Pane {
    const all = this.all;
    return all[Math.min(this.activeIndex, all.length - 1)];
  }

  /** Der markierte Ordner einer Spalte, sofern genau ein Ordner markiert ist. */
  #selectedFolder(column: Pane) {
    if (column.selected.size !== 1) return null;
    const [name] = column.selected;
    return column.entries.find((e) => e.name === name && e.isDir) ?? null;
  }

  /** Die einzelne markierte Datei der tiefsten Spalte (für die Vorschau). */
  readonly preview = $derived.by(() => {
    const all = this.all;
    const column = all[all.length - 1];
    if (column.selected.size !== 1) return null;
    const [name] = column.selected;
    const entry = column.entries.find((e) => e.name === name);
    return entry && !entry.isDir ? { column, entry } : null;
  });

  /** Anzahl markierter Elemente in der tiefsten Spalte, wenn es mehrere sind. */
  readonly multi = $derived.by(() => {
    const all = this.all;
    const column = all[all.length - 1];
    return column.selected.size > 1 ? column : null;
  });

  /** Liest alles, wovon die Spaltenfolge abhängt (für einen Effekt). */
  watchKey(): string {
    return this.all
      .map((c) => `${c.path}|${[...c.selected].join('\u0000')}|${c.entries.length}`)
      .join('\u0001');
  }

  /** Bringt die Spalten in Einklang mit den Auswahlen: Ordner öffnen, Rest abschneiden. */
  reconcile(): void {
    const all = this.all;
    const next: Pane[] = [];
    for (let i = 0; i < all.length; i++) {
      const folder = this.#selectedFolder(all[i]);
      if (!folder) break;
      const path = joinPath(all[i].path, folder.name);
      const existing = all[i + 1];
      if (existing && samePath(existing.path, path)) {
        next.push(existing);
      } else {
        const column = new Pane();
        column.host = this.#root;
        void column.navigate(path);
        next.push(column);
      }
    }
    const same = next.length === this.rest.length && next.every((c, i) => c === this.rest[i]);
    if (!same) this.rest = next;
    if (this.activeIndex > next.length) this.activeIndex = next.length;
  }

  activate(index: number): void {
    this.activeIndex = Math.max(0, Math.min(this.all.length - 1, index));
  }

  /** Rechts: in den markierten Ordner wechseln. */
  enter(): void {
    const active = this.active;
    if (!this.#selectedFolder(active) || this.activeIndex >= this.rest.length) return;
    this.activeIndex++;
    this.wantFirst = true;
  }

  /** Links: eine Spalte zurück; die Spalte, die man verlässt, verliert ihre Auswahl. */
  leave(): void {
    if (this.activeIndex === 0) return;
    this.active.clearSelection();
    this.activeIndex--;
    this.wantFirst = false;
  }
}
