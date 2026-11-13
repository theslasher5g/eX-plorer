import { getApi, type Settings, type Theme } from '../api';

export const DEFAULT_SETTINGS: Settings = {
  theme: 'auto',
  showExtensions: true,
  showSyncStatus: true,
  startView: 'list',
  networkTimeoutSecs: 10,
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
  networkTimeoutSecs: { min: 5, max: 60, step: 5 },
} as const;

export type SettingsSection = 'general' | 'tree' | 'onedrive' | 'network';

/** Welche Werte „Standardwerte“ in welchem Bereich zurücksetzt (Serverliste bleibt immer). */
const SECTION_KEYS: Record<SettingsSection, (keyof Settings)[]> = {
  general: ['theme', 'showExtensions', 'showHidden', 'tileSize', 'startView'],
  tree: ['treeDepth', 'treeTotal', 'treePerDir'],
  onedrive: ['showSyncStatus'],
  network: ['networkTimeoutSecs'],
};

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
    this.applyTheme();
  }

  /** Setzt das Erscheinungsbild: Seite (`data-theme`) und Fenster (Titelleiste, Mica). */
  applyTheme(): void {
    const theme: Theme = this.value.theme;
    const root = document.documentElement;
    if (theme === 'auto') delete root.dataset.theme;
    else root.dataset.theme = theme;
    void getApi()
      .then((api) => api.window.setTheme(theme === 'auto' ? null : theme))
      .catch(() => {});
  }

  update(patch: Partial<Settings>): void {
    this.value = { ...this.value, ...patch };
    if ('theme' in patch) this.applyTheme();
    // Regler feuern oft; gespeichert wird erst, wenn Ruhe einkehrt.
    clearTimeout(this.#timer);
    this.#timer = setTimeout(() => {
      void getApi()
        .then((api) => api.settingsSet(this.value))
        .catch(() => {});
    }, 300);
  }

  /** Stellt die Standardwerte des Bereichs wieder her, andere Bereiche und die Serverliste nicht. */
  reset(section: SettingsSection): void {
    const patch: Record<string, unknown> = {};
    for (const key of SECTION_KEYS[section]) patch[key] = DEFAULT_SETTINGS[key];
    this.update(patch as Partial<Settings>);
  }

  open(section: SettingsSection = 'general'): void {
    this.dialog = section;
  }

  close(): void {
    this.dialog = null;
  }
}

export const settings = new SettingsStore();
