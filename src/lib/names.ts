import type { Entry } from './api';
import { settings } from './stores/settings.svelte';

/** Dateiname ohne den Unterordner-Anteil eines Suchtreffers (`a\\b\\x.txt` → `x.txt`). */
export function leafName(entry: Entry): string {
  return entry.name.slice(entry.name.lastIndexOf('\\') + 1);
}

/** Unterordner eines Suchtreffers relativ zum durchsuchten Ordner, sonst leer. */
export function locationOf(entry: Entry): string {
  const cut = entry.name.lastIndexOf('\\');
  return cut < 0 ? '' : entry.name.slice(0, cut);
}

/** Name, wie er in Listen steht: ohne Endung, wenn das so eingestellt ist (Ordner immer ganz). */
export function displayName(entry: Entry): string {
  const name = leafName(entry);
  if (entry.isDir || settings.value.showExtensions) return name;
  const dot = name.lastIndexOf('.');
  return dot > 0 && dot < name.length - 1 ? name.slice(0, dot) : name;
}
