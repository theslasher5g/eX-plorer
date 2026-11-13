import { t } from '../i18n';
import { getApi, type MenuExtra, type MenuItem, type MenuOutcome } from '../api';

/** Kennung des Eintrags „Weitere Optionen“ (Windows vergibt nur positive). */
export const MORE_ID = -1;

/** Eine geöffnete Menüebene. */
export interface Level {
  items: MenuItem[];
  /** Positionen ab der obersten Ebene (leer für die oberste). */
  path: number[];
  /** Fensterposition: oberste Ebene = Mauszeiger, sonst der Eintrag, der sie öffnet. */
  anchor: { x: number; y: number; right?: number };
}

export interface MenuRequest {
  paths: string[];
  folder: string | null;
  newTabLabel: string | null;
  extras: MenuExtra[];
}

const BLANK: MenuItem = {
  position: -1,
  id: 0,
  separator: false,
  label: '',
  shortcut: null,
  disabled: false,
  checked: false,
  default: false,
  submenu: false,
  icon: null,
};

/** Zeichnet die Einträge des Windows-Kontextmenüs im Stil der App. */
class ContextMenu {
  levels = $state.raw<Level[]>([]);
  /** Letzte Position eines Rechtsklicks (Fensterkoordinaten). */
  point = { x: 100, y: 100 };

  #session = 0;
  #pending: { resolve: (outcome: MenuOutcome) => void; reject: (err: unknown) => void } | null = null;
  #urls: string[] = [];
  #opening = false;

  get isOpen(): boolean {
    return this.levels.length > 0;
  }

  /** Zeigt das Menü und liefert das Ergebnis der Wahl. */
  async show(request: MenuRequest): Promise<MenuOutcome> {
    if (this.#opening) return { kind: 'dismissed' };
    this.dismiss();
    this.#opening = true;
    try {
      const api = await getApi();
      const opened = await api.menuOpen(request.paths, request.folder, request.newTabLabel, request.extras);
      this.#session = opened.session;
      const items = this.#prepare(opened.items);
      // Rückfall für Erweiterungen, die sich nur im Windows-Menü richtig bedienen lassen.
      items.push(
        { ...BLANK, separator: true, disabled: true },
        { ...BLANK, id: MORE_ID, label: t('menu.more') },
      );
      this.levels = [{ items, path: [], anchor: { ...this.point } }];
      return await new Promise<MenuOutcome>((resolve, reject) => (this.#pending = { resolve, reject }));
    } finally {
      this.#opening = false;
    }
  }

  /** Symbole als Blob-URL (die CSP erlaubt keine data:-Bilder). */
  #prepare(items: MenuItem[]): MenuItem[] {
    const cleaned: MenuItem[] = [];
    for (const item of items) {
      const previous = cleaned.at(-1);
      // Keine Trennlinie am Anfang oder doppelt hintereinander.
      if (item.separator && (!previous || previous.separator)) continue;
      cleaned.push(
        item.icon ? { ...item, icon: this.#blobUrl(item.icon) } : item,
      );
    }
    if (cleaned.at(-1)?.separator) cleaned.pop();
    return cleaned;
  }

  #blobUrl(base64: string): string | null {
    try {
      const bytes = Uint8Array.from(atob(base64), (c) => c.charCodeAt(0));
      const url = URL.createObjectURL(new Blob([bytes], { type: 'image/png' }));
      this.#urls.push(url);
      return url;
    } catch {
      return null;
    }
  }

  /** Öffnet das Untermenü des Eintrags `item` in Ebene `depth`. */
  async openSub(depth: number, item: MenuItem, anchor: Level['anchor']): Promise<void> {
    const parent = this.levels[depth];
    if (!parent) return;
    const path = [...parent.path, item.position];
    this.levels = this.levels.slice(0, depth + 1);
    try {
      const items = await (await getApi()).menuSubmenu(this.#session, path);
      // Inzwischen anderswo hingefahren oder geschlossen?
      if (this.levels.length !== depth + 1 || !this.isOpen) return;
      this.levels = [...this.levels, { items: this.#prepare(items), path, anchor }];
    } catch {
      // Ein Untermenü, das sich nicht laden lässt, bleibt einfach zu.
    }
  }

  closeDeeperThan(depth: number): void {
    if (this.levels.length > depth + 1) this.levels = this.levels.slice(0, depth + 1);
  }

  async choose(item: MenuItem): Promise<void> {
    if (item.disabled || item.separator || item.submenu) return;
    if (item.id === MORE_ID) return this.native();
    const session = this.#session;
    this.#finish();
    await this.#settle(async (api) => api.menuInvoke(session, item.id));
  }

  /** „Weitere Optionen“: das Windows-Menü selbst. */
  async native(): Promise<void> {
    const session = this.#session;
    this.#finish();
    await this.#settle(async (api) => api.menuNative(session));
  }

  /** Schließt ohne Auswahl. */
  dismiss(): void {
    if (!this.isOpen) return;
    const session = this.#session;
    this.#finish();
    void getApi().then((api) => api.menuClose(session)).catch(() => {});
    this.#pending?.resolve({ kind: 'dismissed' });
    this.#pending = null;
  }

  async #settle(run: (api: Awaited<ReturnType<typeof getApi>>) => Promise<MenuOutcome>): Promise<void> {
    const pending = this.#pending;
    this.#pending = null;
    try {
      pending?.resolve(await run(await getApi()));
    } catch (err) {
      pending?.reject(err);
    }
  }

  #finish(): void {
    this.levels = [];
    for (const url of this.#urls) URL.revokeObjectURL(url);
    this.#urls = [];
  }
}

export const contextmenu = new ContextMenu();
