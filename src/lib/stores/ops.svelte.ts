import { getApi, toApiError, type ClipboardFiles, type Drive, type OpResult, type TransferMode } from '../api';
import { errorText, itemsText, t } from '../i18n';
import { joinPath, parentPath, samePath } from '../paths';
import { dialogs } from './dialog.svelte';
import type { Pane } from './pane.svelte';
import type { Tabs } from './tabs.svelte';
import { toasts } from './toast.svelte';

const NO_RECYCLE_BIN = new Set(['remote', 'removable', 'cdrom']);

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
    return this.#tabs?.current ?? null;
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
    for (const pane of this.#tabs?.panes ?? []) {
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

  async copy(cut = false): Promise<void> {
    const pane = this.pane;
    if (!pane) return;
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
    if (!pane || pane.error || !pane.path) return;
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

  async newFolder(): Promise<void> {
    const pane = this.pane;
    if (!pane || pane.error || !pane.path) return;
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
