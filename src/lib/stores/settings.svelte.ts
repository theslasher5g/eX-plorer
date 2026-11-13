import { getApi, type Settings } from '../api';

export const DEFAULT_SETTINGS: Settings = {
  showHidden: false,
  treeDepth: 4,
  treeTotal: 2000,
  treePerDir: 200,
  tileSize: 132,
  servers: [],
};

/** Grenzen der Regler; das Backend erzwingt dieselben Werte noch einmal. */
export const LIMITS = {
  treeDepth: { min: 1, max: 16, step: 1 },
  treeTotal: { min: 100, max: 50_000, step: 100 },
  treePerDir: { min: 10, max: 5_000, step: 10 },
  tileSize: { min: 100, max: 220, step: 4 },
} as const;

export type SettingsSection = 'general' | 'tree';

/** Benutzereinstellungen; Änderungen wirken sofort und werden gespeichert. */
class SettingsStore {
  value = $state.raw<Settings>({ ...DEFAULT_SETTINGS });
  /** Offener Bereich des Einstellungsdialogs, `null` = geschlossen. */
  dialog = $state<SettingsSection | null>(null);

  #timer: ReturnType<typeof setTimeout> | undefined;

  async load(): Promise<void> {
    try {
      this.value = await (await getApi()).settingsGet();
    } catch {
      // Ohne gespeicherte Einstellungen gelten die Standardwerte.
    }
  }

  update(patch: Partial<Settings>): void {
    this.value = { ...this.value, ...patch };
    // Regler feuern oft; gespeichert wird erst, wenn Ruhe einkehrt.
    clearTimeout(this.#timer);
    this.#timer = setTimeout(() => {
      void getApi()
        .then((api) => api.settingsSet(this.value))
        .catch(() => {});
    }, 300);
  }

  reset(): void {
    this.update({ ...DEFAULT_SETTINGS });
  }

  open(section: SettingsSection = 'general'): void {
    this.dialog = section;
  }

  close(): void {
    this.dialog = null;
  }
}

export const settings = new SettingsStore();
