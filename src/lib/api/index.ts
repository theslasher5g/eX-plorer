import type { Api } from './types';

export * from './types';

const inTauri = '__TAURI_INTERNALS__' in window;

let instance: Promise<Api> | undefined;

/**
 * Liefert das Backend. Außerhalb von Tauri (reiner Browser, `npm run dev`)
 * gibt es nur im Entwicklungsmodus ein Demo-Backend mit erfundenen Daten.
 */
export function getApi(): Promise<Api> {
  instance ??= load();
  return instance;
}

async function load(): Promise<Api> {
  if (inTauri) return (await import('./tauri')).tauriApi;
  if (import.meta.env.DEV) return (await import('./mock')).mockApi;
  throw new Error('eX-plorer läuft nur innerhalb der Desktop-App.');
}
