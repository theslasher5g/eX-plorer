import type { Pane } from './stores/pane.svelte';

/**
 * Tippsuche für Tastaturereignisse einer Ansicht. Liefert `null`, wenn die
 * Taste nichts damit zu tun hat (Kürzel, Pfeile, Eingabefeld …), sonst den
 * Index des markierten Eintrags (-1: nichts gefunden). Ein Leerzeichen zählt
 * nur mitten in einer Eingabe; sonst bleibt es die Schnellvorschau.
 */
export function handleTypeAhead(event: KeyboardEvent, pane: Pane): number | null {
  if (event.ctrlKey || event.altKey || event.metaKey || event.key.length !== 1) return null;
  if (event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement) return null;
  if (event.key === ' ' && !pane.typing) return null;
  event.preventDefault();
  event.stopPropagation();
  return pane.typeAhead(event.key);
}
