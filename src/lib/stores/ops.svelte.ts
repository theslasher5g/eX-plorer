import {
  getApi,
  toApiError,
  type ClipboardFiles,
  type Drive,
  type Entry,
  type OpResult,
  type TransferMode,
} from '../api';
import { errorText, itemsText, t } from '../i18n';
import { joinPath, parentPath, samePath, splitPath } from '../paths';
import { contextmenu } from './contextmenu.svelte';
import { dialogs } from './dialog.svelte';
import { favorites } from './favorites.svelte';
import { onedrive } from './onedrive.svelte';
import type { Pane } from './pane.svelte';
import type { Tabs } from './tabs.svelte';
import { toasts } from './toast.svelte';

const NO_RECYCLE_BIN = new Set(['remote', 'removable', 'cdrom']);

// Eigene Einträge im nativen Menü (Bereich 0x7001–0x70FF).
const MENU_KEEP = 0x7001;
const MENU_FREE = 0x7002;
const MENU_PIN = 0x7003;
const MENU_UNPIN = 0x7004;

/**
 * Dateioperationen der aktiven Ansicht: Kopieren, Ausschneiden, Einfügen,
 * Löschen, Umbenennen, Neuer Ordner und Verschieben per Drag & Drop. Die
 * eigentliche Arbeit macht das Backend; hier stehen Ablauf, Rückfragen und
 * Rückmeldungen.
 */
class Ops {
  /** Letzter bekannter Stand der Datei-Zwischenablage (für „ausgeschnitten“-Anzeige). */
  clipboard = $state.raw<ClipboardFiles | null>(null);

  #tabs: Tabs | null = null;
  #drives: () => Drive[] = () => [];

  bind(tabs: Tabs, drives: () => Drive[]): void {
    this.#tabs = tabs;
    this.#drives = drives;
  }

  get pane(): Pane | null {
    return this.#tabs?.target ?? null;
  }

  isCut(path: string): boolean {
    const clip = this.clipboard;
    return !!clip?.cut && clip.paths.some((p) => samePath(p, path));
  }

  selectedPaths(pane: Pane): string[] {
    return [...pane.selected].map((name) => joinPath(pane.path, name));
  }

  /** Gibt es dort keinen Papierkorb (Netzlaufwerk, USB, UNC)? */
  #noRecycleBin(path: string): boolean {
    if (path.startsWith('\\\\')) return true;
    const letter = path.slice(0, 2).toUpperCase();
    const drive = this.#drives().find((d) => d.path.slice(0, 2).toUpperCase() === letter);
    return !!drive && NO_RECYCLE_BIN.has(drive.kind);
  }

  #fail(err: unknown): void {
    toasts.show(errorText(toApiError(err).code), 'err');
  }

  /** Lädt die Ansichten neu, die einen der Ordner zeigen. */
  #refreshDirs(dirs: string[]): void {
    for (const pane of this.#tabs?.livePanes ?? []) {
      if (dirs.some((dir) => samePath(dir, pane.path))) void pane.refresh();
    }
  }

  async refreshClipboard(): Promise<void> {
    try {
      this.clipboard = await (await getApi()).clipboardGet();
    } catch {
      this.clipboard = null;
    }
  }

  /**
   * Verbindet mit einer Freigabe (`\\server\freigabe`) oder verbindet ein
   * zugeordnetes Laufwerk (`Z:\\`) neu. Den Dialog für Anmeldedaten zeigt
   * Windows; die App sieht nie ein Kennwort. `true`: verbunden.
   */
  async connect(target: string): Promise<boolean> {
    try {
      const result = await (await getApi()).networkConnect(target);
      if (result.connected) toasts.show(t('network.joined', { name: target.replace(/\\IPC\$$/, '').replace(/\\$/, '') }));
      return result.connected;
    } catch (err) {
      this.#fail(err);
      return false;
    }
  }

  /** Meldet sich an der Freigabe des aktuellen Ordners an und lädt neu. */
  async reconnect(pane: Pane): Promise<void> {
    const root = splitPath(pane.path)[0]?.path;
    if (root && (await this.connect(root))) void pane.refresh();
  }

  async copy(cut = false): Promise<void> {
    const pane = this.pane;
    if (!pane || pane.isNetwork) return;
    const paths = this.selectedPaths(pane);
    if (paths.length === 0) return;
    try {
      await (await getApi()).clipboardSet(paths, cut);
      this.clipboard = { paths, cut };
      toasts.show(t(cut ? 'ops.cutDone' : 'ops.copied'));
    } catch (err) {
      this.#fail(err);
    }
  }

  async paste(): Promise<void> {
    const pane = this.pane;
    if (!pane || pane.error || !pane.path || pane.isNetwork) return;
    try {
      const result = await (await getApi()).paste(pane.path);
      if (result.cut) this.clipboard = null;
      this.#afterTransfer(pane, result, result.moved);
    } catch (err) {
      this.#fail(err);
      void this.refreshClipboard();
    }
  }

  /** Verschiebt oder kopiert Dateien (z. B. nach einem Drop) in einen Ordner. */
  async transfer(paths: string[], dest: string, mode: TransferMode = 'auto'): Promise<void> {
    const pane = this.pane;
    if (!pane || paths.length === 0) return;
    try {
      const result = await (await getApi()).transfer(paths, dest, mode);
      this.#afterTransfer(pane, result, result.moved, [dest, ...paths.map((p) => parentPath(p) ?? '')]);
    } catch (err) {
      this.#fail(err);
    }
  }

  #afterTransfer(pane: Pane, result: OpResult, moved: boolean, dirs: string[] = []): void {
    if (result.aborted || result.count === 0) return;
    // Eingefügte Elemente markieren, wenn der Zielordner gerade offen ist.
    if (dirs.length === 0 || samePath(dirs[0], pane.path)) pane.selectAfterLoad(result.names);
    this.#refreshDirs(dirs.length ? dirs : [pane.path]);
    const key = moved ? 'ops.doneMoved' : 'ops.doneCopied';
    toasts.show(t(key, { items: itemsText(result.count) }));
  }

  async remove(permanent = false): Promise<void> {
    const pane = this.pane;
    if (!pane) return;
    const paths = this.selectedPaths(pane);
    if (paths.length === 0) return;

    const noBin = paths.some((p) => this.#noRecycleBin(p));
    const forever = permanent || noBin;
    const single = paths.length === 1;
    const name = [...pane.selected][0];
    const title = forever
      ? single
        ? t('delete.titleForeverOne', { name })
        : t('delete.titleForeverMany', { count: paths.length })
      : single
        ? t('delete.titleOne', { name })
        : t('delete.titleMany', { count: paths.length });
    const body = forever
      ? t(noBin ? 'delete.bodyNoBin' : 'delete.bodyForever')
      : t(single ? 'delete.bodyTrashOne' : 'delete.bodyTrashMany');

    const confirmed = await dialogs.confirm({
      title,
      body,
      confirm: t(forever ? 'delete.forever' : 'delete.toTrash'),
      cancel: t('delete.cancel'),
      danger: forever,
    });
    if (!confirmed) return;

    try {
      const result = await (await getApi()).deleteItems(paths, forever);
      this.#refreshDirs([pane.path]);
      if (!result.aborted && result.count > 0) {
        toasts.show(t(forever ? 'ops.doneDeleted' : 'ops.doneTrashed', { items: itemsText(result.count) }));
      }
    } catch (err) {
      this.#fail(err);
      this.#refreshDirs([pane.path]);
    }
  }

  /** Öffnet einen Eintrag: Ordner in der Ansicht, Dateien mit dem Standardprogramm. */
  async open(pane: Pane, entry: Entry): Promise<void> {
    const path = joinPath(pane.path, entry.name);
    if (entry.isDir) {
      void (pane.host ?? pane).navigate(path);
      return;
    }
    await this.openFile(path, entry.name);
  }

  /** Öffnet eine Datei mit dem Standardprogramm (ausführbare Typen nach Rückfrage). */
  async openFile(path: string, name: string): Promise<void> {
    try {
      const api = await getApi();
      const result = await api.openItem(path, false);
      if (!result.needsConfirm) return;
      // Ausführbare Dateitypen nur nach ausdrücklicher Bestätigung.
      const confirmed = await dialogs.confirm({
        title: t('open.confirmTitle', { name }),
        body: t('open.confirmBody'),
        confirm: t('open.confirm'),
        cancel: t('delete.cancel'),
        danger: true,
      });
      if (confirmed) await api.openItem(path, true);
    } catch (err) {
      this.#fail(err);
    }
  }

  /** Zeigt das native Windows-Kontextmenü für die Auswahl (oder die leere Fläche). */
  async contextMenu(pane: Pane): Promise<void> {
    if (pane.error || !pane.path || pane.isNetwork) return;
    const paths = this.selectedPaths(pane);
    const selected = pane.entries.filter((e) => pane.selected.has(e.name));
    const folder = selected.length === 1 && selected[0].isDir ? selected[0] : null;
    try {
      // Im OneDrive-Ordner die beiden Cloud-Aktionen anbieten, sofern OneDrive
      // sie nicht schon selbst im Menü hat.
      const extras =
        paths.length > 0 && onedrive.contains(pane.path)
          ? [
              { id: MENU_KEEP, label: t('onedrive.keep'), skipIf: ['Always keep on this device'] },
              { id: MENU_FREE, label: t('onedrive.free'), skipIf: ['Free up space'] },
            ]
          : [];
      // Ein einzelner Ordner lässt sich an die Favoriten heften (Windows' eigener
      // Eintrag „An Schnellzugriff anheften“ wirkt ebenso, siehe `#runVerb`).
      if (folder) {
        const pinned = favorites.isPinned(joinPath(pane.path, folder.name));
        extras.push(
          pinned
            ? { id: MENU_UNPIN, label: t('favorites.unpin'), skipIf: ['Unpin from Quick access', 'Von Schnellzugriff lösen'] }
            : { id: MENU_PIN, label: t('favorites.pin'), skipIf: ['Pin to Quick access', 'An Schnellzugriff anheften'] },
        );
      }
      const outcome = await contextmenu.show({
        paths,
        folder: paths.length === 0 ? pane.path : null,
        newTabLabel: folder ? t('menu.newTab') : null,
        extras,
      });
      switch (outcome.kind) {
        case 'newTab':
          if (folder) this.#tabs?.open(joinPath(pane.path, folder.name));
          break;
        case 'extra':
          if (outcome.id === MENU_PIN && folder) favorites.pin(joinPath(pane.path, folder.name));
          else if (outcome.id === MENU_UNPIN && folder) favorites.unpin(joinPath(pane.path, folder.name));
          else await this.setPinned(pane, paths, outcome.id === MENU_KEEP);
          break;
        case 'done':
          // Die Shell hat selbst gehandelt (z. B. „Neu“, Archiv, Eigenschaften).
          void pane.refresh();
          break;
        case 'verb':
          await this.#runVerb(pane, outcome.verb, selected);
          break;
        default:
      }
    } catch (err) {
      this.#fail(err);
    }
  }

  /** „Immer auf diesem Gerät behalten“ bzw. „Speicherplatz freigeben“. */
  async setPinned(pane: Pane, paths: string[], pinned: boolean): Promise<void> {
    try {
      const count = await (await getApi()).onedriveSetPinned(paths, pinned);
      toasts.show(t(pinned ? 'onedrive.doneKeep' : 'onedrive.doneFree', { items: itemsText(count) }));
      void pane.refresh();
    } catch (err) {
      this.#fail(err);
    }
  }

  /** Natives Kontextmenü für eine Zeile der Baumansicht. */
  async treeContextMenu(pane: Pane, path: string, name: string, isDir: boolean): Promise<void> {
    try {
      const outcome = await contextmenu.show({
        paths: [path],
        folder: null,
        newTabLabel: isDir ? t('menu.newTab') : null,
        extras: [],
      });
      if (outcome.kind === 'newTab') this.#tabs?.open(path);
      else if (outcome.kind === 'done') void pane.tree.start(pane.tree.root);
      else if (outcome.kind === 'verb' && outcome.verb === 'open') {
        if (isDir) void pane.navigate(path);
        else await this.openFile(path, name);
      }
    } catch (err) {
      this.#fail(err);
    }
  }

  /** Verben, die die App selbst ausführt, damit Rückfragen und Anzeige stimmen. */
  async #runVerb(pane: Pane, verb: string, selected: Entry[]): Promise<void> {
    switch (verb) {
      case 'cut':
        return this.copy(true);
      case 'copy':
        return this.copy(false);
      case 'paste':
        return this.paste();
      case 'delete':
        return this.remove();
      case 'rename':
        return this.startRename();
      case 'pintohome':
      case 'unpinfromhome': {
        // Gilt dem markierten Ordner, sonst dem Ordner, der gerade offen ist.
        const target = selected.length === 1 && selected[0].isDir ? joinPath(pane.path, selected[0].name) : pane.path;
        if (verb === 'pintohome') favorites.pin(target);
        else favorites.unpin(target);
        return;
      }
      case 'open':
        for (const entry of selected.slice(0, 10)) {
          if (entry.isDir && selected.length > 1) this.#tabs?.open(joinPath(pane.path, entry.name));
          else await this.open(pane, entry);
        }
        break;
      default:
    }
  }

  async newFolder(): Promise<void> {
    const pane = this.pane;
    if (!pane || pane.error || !pane.path || pane.isNetwork) return;
    try {
      const name = await (await getApi()).newFolder(pane.path, t('ops.newFolderName'));
      pane.selectAfterLoad([name]);
      await pane.refresh();
      pane.renaming = name;
    } catch (err) {
      this.#fail(err);
    }
  }

  /** Startet das Umbenennen des markierten Eintrags (nur bei genau einem). */
  startRename(): void {
    const pane = this.pane;
    if (!pane || pane.selected.size !== 1) return;
    pane.renaming = [...pane.selected][0];
    pane.scrollSignal++;
  }

  /** Gibt `true` zurück, wenn das Eingabefeld geschlossen werden darf. */
  async rename(pane: Pane, oldName: string, newName: string): Promise<boolean> {
    const name = newName.trim();
    if (!name || name === oldName) {
      pane.renaming = null;
      return true;
    }
    try {
      const result = await (await getApi()).renameItem(joinPath(pane.path, oldName), name);
      pane.renaming = null;
      if (!result.aborted) {
        pane.selectAfterLoad([result.name]);
        await pane.refresh();
      }
      return true;
    } catch (err) {
      this.#fail(err);
      return false;
    }
  }
}

export const ops = new Ops();
