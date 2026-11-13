import { getApi, toApiError, type ApiError, type Entry } from '../api';
import { dateBoundaries, groupOf, type Group, type GroupInfo, type GroupKey } from '../grouping';
import { locale } from '../i18n';
import { extension, joinPath, NETWORK_PATH, parentPath, samePath, splitPath } from '../paths';
import { ColumnsState } from './columns.svelte';
import { settings } from './settings.svelte';
import { TreeState } from './tree.svelte';

export type SortKey = 'name' | 'modified' | 'type' | 'size';
export type ViewMode = 'list' | 'grid' | 'columns' | 'tree';

const collator = new Intl.Collator(locale, { numeric: true, sensitivity: 'base' });

let nextId = 1;

/** Zustand einer Ordneransicht: Pfad, Verlauf, Einträge, Sortierung, Auswahl. */
export class Pane {
  /** Stabile Kennung, z. B. als Schlüssel für Tabs. */
  readonly id = nextId++;
  path = $state('');
  entries = $state.raw<Entry[]>([]);
  loading = $state(false);
  error = $state<ApiError | null>(null);
  /** Zählt erfolgreiche Ladevorgänge; Ansichten setzen daran ihre Scrollposition zurück. */
  revision = $state(0);

  sortKey = $state<SortKey>('name');
  sortAsc = $state(true);
  filter = $state('');
  view = $state<ViewMode>('list');
  /** Gruppierung in Liste und Kacheln. */
  groupKey = $state<GroupKey>('none');
  groupAsc = $state(true);
  /** Zustand der Baumansicht dieses Tabs. */
  readonly tree = new TreeState();
  /** Zustand der Spaltenansicht dieses Tabs. */
  readonly columns = new ColumnsState(this);
  /** Gesetzt bei Spalten der Spaltenansicht: der Pane des Tabs, zu dem sie gehören. */
  host: Pane | null = null;

  /** Alle Ordner, die dieser Tab gerade anzeigt (bei Spalten: jede Spalte). */
  get livePanes(): Pane[] {
    return this.view === 'columns' ? this.columns.all : [this];
  }

  /** Versteckte Elemente anzeigen: eine globale Einstellung. */
  get showHidden(): boolean {
    return settings.value.showHidden;
  }

  set showHidden(value: boolean) {
    settings.update({ showHidden: value });
  }

  /** Name des Eintrags, der gerade umbenannt wird. */
  renaming = $state<string | null>(null);
  /** Wird erhöht, wenn die Ansicht den Fokus-Eintrag ins Bild holen soll. */
  scrollSignal = $state(0);

  selected = $state.raw<ReadonlySet<string>>(new Set());
  /** Index des Fokus-Eintrags in `visible`, -1 wenn keiner. */
  cursor = $state(-1);

  #history = $state.raw<string[]>([]);
  #index = $state(-1);
  #anchor = -1;
  #generation = 0;
  #pendingSelect: string[] | null = null;

  /** Zeigt die Netzwerk-Seite statt eines Ordners. */
  readonly isNetwork = $derived(this.path === NETWORK_PATH);

  readonly canBack = $derived(this.#index > 0);
  readonly canForward = $derived(this.#index < this.#history.length - 1);
  readonly canUp = $derived(parentPath(this.path) !== null);

  /** Name des aktuellen Ordners, z. B. als Tab-Titel. */
  readonly title = $derived(splitPath(this.path).at(-1)?.label ?? '');

  /** Gesamtgröße der ausgewählten Dateien in Bytes. */
  readonly selectedBytes = $derived.by(() => {
    const selected = this.selected;
    if (selected.size === 0) return 0;
    let sum = 0;
    for (const entry of this.entries) if (!entry.isDir && selected.has(entry.name)) sum += entry.size;
    return sum;
  });

  readonly hiddenCount = $derived(
    this.showHidden ? 0 : this.entries.reduce((n, e) => n + (e.hidden ? 1 : 0), 0),
  );

  /** Sortierte, gefilterte und ggf. gruppierte Einträge samt Gruppen. */
  readonly #sorted: { list: Entry[]; groups: Group[] } = $derived.by(() => {
    const needle = this.filter.trim().toLowerCase();
    const showHidden = this.showHidden;
    const list = this.entries.filter(
      (e) => (showHidden || !e.hidden) && (!needle || e.name.toLowerCase().includes(needle)),
    );
    // Spalten sortieren wie der Tab, zu dem sie gehören.
    const sorter: Pane = this.host ?? this;
    const direction = sorter.sortAsc ? 1 : -1;
    const key = sorter.sortKey;

    // Gruppiert wird nur in Liste und Kacheln.
    const groupKey = this.groupKey;
    const grouped = groupKey !== 'none' && !this.host && (this.view === 'list' || this.view === 'grid');
    const bounds = dateBoundaries(Date.now());
    const infos = grouped ? new Map<Entry, GroupInfo>(list.map((e) => [e, groupOf(e, groupKey, bounds)])) : null;
    const groupDirection = this.groupAsc ? 1 : -1;

    list.sort((a, b) => {
      if (infos) {
        const ga = infos.get(a);
        const gb = infos.get(b);
        if (ga && gb && ga.id !== gb.id) {
          const byGroup = ga.order - gb.order || collator.compare(ga.label, gb.label);
          if (byGroup !== 0) return byGroup * groupDirection;
        }
      }
      // Ordner stehen unabhängig von der Sortierrichtung oben.
      if (a.isDir !== b.isDir) return a.isDir ? -1 : 1;
      let result = 0;
      if (key === 'size') result = a.size - b.size;
      else if (key === 'modified') result = (a.modifiedMs ?? 0) - (b.modifiedMs ?? 0);
      else if (key === 'type') result = collator.compare(extension(a.name), extension(b.name));
      if (result !== 0) return result * direction;
      // Bei Gleichstand nach Name; die Richtung gilt dafür nur, wenn nach Name sortiert wird.
      return collator.compare(a.name, b.name) * (key === 'name' ? direction : 1);
    });

    const groups: Group[] = [];
    if (infos) {
      list.forEach((entry, index) => {
        const info = infos.get(entry);
        if (!info) return;
        const last = groups.at(-1);
        if (last && last.id === info.id) last.count++;
        else groups.push({ id: info.id, label: info.label, start: index, count: 1 });
      });
    }
    return { list, groups };
  });

  readonly visible: Entry[] = $derived(this.#sorted.list);
  /** Gruppen von `visible` in Reihenfolge; leer ohne Gruppierung. */
  readonly groups: Group[] = $derived(this.#sorted.groups);

  /** Öffnet einen Ordner und legt einen Verlaufseintrag an. */
  navigate(path: string): Promise<void> {
    const target = path.trim();
    if (!target) return Promise.resolve();
    if (this.path && samePath(target, this.path) && !this.error) return this.refresh();
    if (target === NETWORK_PATH && this.path === NETWORK_PATH) return this.refresh();
    const history = this.#history.slice(0, this.#index + 1);
    history.push(target);
    this.#history = history;
    this.#index = history.length - 1;
    return this.#load(target);
  }

  back(): Promise<void> {
    if (!this.canBack) return Promise.resolve();
    this.#index--;
    return this.#load(this.#history[this.#index]);
  }

  forward(): Promise<void> {
    if (!this.canForward) return Promise.resolve();
    this.#index++;
    return this.#load(this.#history[this.#index]);
  }

  up(): Promise<void> {
    const parent = parentPath(this.path);
    return parent ? this.navigate(parent) : Promise.resolve();
  }

  refresh(): Promise<void> {
    if (this.path === NETWORK_PATH) {
      // Die Netzwerk-Seite prüft bei jedem Wechsel dieses Zählers neu.
      this.revision++;
      return Promise.resolve();
    }
    return this.path ? this.#load(this.path, true) : Promise.resolve();
  }

  /** Öffnet einen Ordner (in Spalten: im Tab, zu dem die Spalte gehört). */
  open(entry: Entry): void {
    if (entry.isDir) void (this.host ?? this).navigate(joinPath(this.path, entry.name));
  }

  async #load(target: string, keepSelection = false): Promise<void> {
    const generation = ++this.#generation;
    this.error = null;
    this.path = target;
    if (target === NETWORK_PATH) {
      this.entries = [];
      this.loading = false;
      this.renaming = null;
      this.#resetSelection();
      this.revision++;
      return;
    }
    this.loading = true;
    if (!keepSelection) this.renaming = null;

    const collected: Entry[] = [];
    let shown = false;
    try {
      const api = await getApi();
      const summary = await api.listDir(target, (batch) => {
        if (generation !== this.#generation) return;
        for (const entry of batch) collected.push(entry);
        // Beim Navigieren den ersten Stapel sofort zeigen; der Rest folgt
        // gesammelt am Ende, damit große Ordner nicht dutzendfach neu
        // sortiert werden. Beim Aktualisieren flackert nichts: erst am Ende.
        if (!keepSelection && !shown) {
          shown = true;
          this.#show(collected.slice(), false, false);
        }
      });
      if (generation !== this.#generation) return;
      // Der Verlauf merkt sich die vom Backend normalisierte Schreibweise.
      this.path = summary.path;
      const history = this.#history.slice();
      history[this.#index] = summary.path;
      this.#history = history;
      this.#show(collected, keepSelection || shown, true);
    } catch (err) {
      if (generation !== this.#generation) return;
      this.entries = [];
      this.#pendingSelect = null;
      this.renaming = null;
      this.#resetSelection();
      this.error = toApiError(err);
    } finally {
      if (generation === this.#generation) this.loading = false;
    }
  }

  #show(entries: Entry[], keepSelection: boolean, final: boolean): void {
    this.entries = entries;
    if (keepSelection) {
      const names = new Set(entries.map((e) => e.name));
      this.selected = new Set([...this.selected].filter((name) => names.has(name)));
    } else {
      this.filter = '';
      this.#resetSelection();
      this.revision++;
    }
    if (!final) return;

    if (this.renaming && !entries.some((e) => e.name === this.renaming)) this.renaming = null;
    const wanted = this.#pendingSelect;
    this.#pendingSelect = null;
    if (wanted) {
      const names = new Set(wanted);
      const items = this.visible;
      const picked = items.filter((e) => names.has(e.name));
      if (picked.length > 0) {
        this.selected = new Set(picked.map((e) => e.name));
        this.cursor = items.indexOf(picked[0]);
        this.#anchor = this.cursor;
        this.scrollSignal++;
      }
    }
  }

  /** Markiert nach dem nächsten vollständigen Laden diese Namen. */
  selectAfterLoad(names: string[]): void {
    this.#pendingSelect = names;
  }

  #resetSelection(): void {
    this.selected = new Set();
    this.cursor = -1;
    this.#anchor = -1;
  }

  setSortDirection(ascending: boolean): void {
    if (this.sortAsc === ascending) return;
    this.sortAsc = ascending;
    this.#resetSelection();
  }

  setSort(key: SortKey): void {
    if (this.sortKey === key) {
      this.sortAsc = !this.sortAsc;
    } else {
      this.sortKey = key;
      // Größe und Datum beginnen absteigend, wie im Explorer üblich.
      this.sortAsc = key === 'name' || key === 'type';
    }
    this.#resetSelection();
  }

  setGroup(key: GroupKey): void {
    if (this.groupKey === key) return;
    this.groupKey = key;
    this.#resetSelection();
    this.revision++;
  }

  setGroupDirection(ascending: boolean): void {
    if (this.groupAsc === ascending) return;
    this.groupAsc = ascending;
    this.#resetSelection();
  }

  /** Markiert alle Einträge einer Gruppe (mit `add` zusätzlich zur bisherigen Auswahl). */
  selectGroup(group: Group, add = false): void {
    const items = this.visible;
    const next = new Set(add ? this.selected : []);
    for (let i = group.start; i < group.start + group.count; i++) next.add(items[i].name);
    this.selected = next;
    this.cursor = group.start;
    this.#anchor = group.start;
  }

  /** Setzt die Auswahl auf die Einträge mit diesen Indizes (Gummiband); `base` bleibt zusätzlich markiert. */
  setSelection(indices: number[], base: ReadonlySet<string> | null = null): void {
    const items = this.visible;
    const next = new Set(base ?? []);
    for (const index of indices) if (items[index]) next.add(items[index].name);
    this.selected = next;
    if (indices.length > 0) {
      this.cursor = indices[indices.length - 1];
      this.#anchor = indices[0];
    }
  }

  /** Auswahl per Maus: einfach, Strg (umschalten), Umschalt (Bereich). */
  select(index: number, mode: { toggle?: boolean; range?: boolean } = {}): void {
    const items = this.visible;
    if (index < 0 || index >= items.length) return;
    if (mode.range && this.#anchor >= 0) {
      const from = Math.min(this.#anchor, index);
      const to = Math.max(this.#anchor, index);
      const next = new Set(mode.toggle ? this.selected : []);
      for (let i = from; i <= to; i++) next.add(items[i].name);
      this.selected = next;
    } else if (mode.toggle) {
      const next = new Set(this.selected);
      const name = items[index].name;
      if (!next.delete(name)) next.add(name);
      this.selected = next;
      this.#anchor = index;
    } else {
      this.selected = new Set([items[index].name]);
      this.#anchor = index;
    }
    this.cursor = index;
  }

  /** Bewegt den Fokus per Tastatur; mit `extend` wird die Auswahl erweitert. */
  moveCursor(to: number, extend = false): void {
    const count = this.visible.length;
    if (count === 0) return;
    const index = Math.max(0, Math.min(count - 1, to));
    this.select(index, { range: extend });
  }

  selectAll(): void {
    this.selected = new Set(this.visible.map((e) => e.name));
  }

  clearSelection(): void {
    this.#resetSelection();
  }
}
