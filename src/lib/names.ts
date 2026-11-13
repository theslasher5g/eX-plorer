import type { Entry } from './api';
import { settings } from './stores/settings.svelte';

/** Name, wie er in Listen steht: ohne Endung, wenn das so eingestellt ist (Ordner immer ganz). */
export function displayName(entry: Entry): string {
  if (entry.isDir || settings.value.showExtensions) return entry.name;
  const dot = entry.name.lastIndexOf('.');
  return dot > 0 && dot < entry.name.length - 1 ? entry.name.slice(0, dot) : entry.name;
}
