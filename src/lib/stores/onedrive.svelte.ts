import { settings } from './settings.svelte';
import type { Entry, Place, SyncState } from '../api';

/** Die lokalen OneDrive-Ordner (ein Eintrag je Konto) und daraus abgeleitete Hilfen. */
class OneDrive {
  roots = $state.raw<string[]>([]);
  /** Erkannte Konten (für die Einstellungen). */
  accounts = $state.raw<Place[]>([]);

  setRoots(paths: string[]): void {
    this.roots = paths;
  }

  /** Liegt `path` in einem OneDrive-Ordner? */
  contains(path: string): boolean {
    const lower = path.toLowerCase();
    return this.roots.some((root) => {
      const base = root.toLowerCase().replace(/\\+$/, '');
      return lower === base || lower.startsWith(`${base}\\`);
    });
  }

  /** Lädt die Seitenleiste (und damit die Konten) neu; setzt `App.svelte`. */
  reloadHook: (() => Promise<void>) | null = null;

  async reload(): Promise<void> {
    await this.reloadHook?.();
  }

  /**
   * Sync-Zustand eines Eintrags. Das Backend meldet ihn für Cloud-Platzhalter.
   * Eine normale Datei im OneDrive-Ordner (kein Platzhalter, z. B. bei
   * ausgeschaltetem „Dateien bei Bedarf“) liegt vollständig auf dem Gerät und
   * gilt als „auf diesem Gerät“; „wird synchronisiert“ meldet nur Windows
   * selbst. Ordner sowie versteckte und Systemdateien (desktop.ini) bleiben
   * ohne Status.
   */
  stateOf(entry: Entry, dir: string): SyncState | null {
    if (!settings.value.showSyncStatus) return null;
    if (entry.sync) return entry.sync;
    if (entry.isDir || entry.hidden || entry.system) return null;
    return this.contains(dir) ? 'local' : null;
  }
}

export const onedrive = new OneDrive();
