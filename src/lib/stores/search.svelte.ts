import { getApi, toApiError, type Entry, type SearchSummary } from '../api';

let nextId = 2_000_000_000;

/**
 * Suche in Unterordnern für eine Ordneransicht. Treffer kommen gestapelt aus
 * dem Backend; `name` enthält den Pfad relativ zum Ordner. Treffer direkt im
 * Ordner (ohne `\`) liefert die Ordneransicht selbst, sie werden hier ignoriert.
 */
export class SearchState {
  hits = $state.raw<Entry[]>([]);
  running = $state(false);
  summary = $state.raw<SearchSummary | null>(null);
  /** Auf Netzwerkpfaden gibt es keine Tiefensuche. */
  unsupported = $state(false);
  failed = $state(false);

  #id = 0;

  /** Läuft eine Suche oder liegen Ergebnisse vor? */
  get active(): boolean {
    return this.running || this.summary !== null || this.unsupported;
  }

  async run(path: string, query: string, showHidden: boolean): Promise<void> {
    this.#cancelBackend();
    const id = ++nextId;
    this.#id = id;
    this.hits = [];
    this.summary = null;
    this.unsupported = false;
    this.failed = false;
    this.running = true;
    try {
      const api = await getApi();
      const summary = await api.searchDir({ path, query, searchId: id, showHidden }, (batch) => {
        if (id !== this.#id) return;
        const below = batch.filter((entry) => entry.name.includes('\\'));
        if (below.length > 0) this.hits = this.hits.concat(below);
      });
      if (id === this.#id) this.summary = summary;
    } catch (err) {
      if (id !== this.#id) return;
      if (toApiError(err).code === 'unsupported') this.unsupported = true;
      else this.failed = true;
    } finally {
      if (id === this.#id) this.running = false;
    }
  }

  #cancelBackend(): void {
    if (this.running) void getApi().then((api) => api.searchCancel(this.#id)).catch(() => {});
  }

  /** Bricht eine laufende Suche ab und verwirft die Treffer. */
  clear(): void {
    if (!this.active && this.hits.length === 0) return;
    this.#cancelBackend();
    this.#id = 0;
    this.hits = [];
    this.summary = null;
    this.unsupported = false;
    this.failed = false;
    this.running = false;
  }
}
