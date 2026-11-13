import type { Entry } from './api';
import { dnd } from './stores/dnd.svelte';
import { ops } from './stores/ops.svelte';
import type { Pane } from './stores/pane.svelte';

const DRAG_THRESHOLD = 6;

/**
 * Maus-Bedienung eines Eintrags (Auswahl, Öffnen, Kontextmenü, Ziehen),
 * gemeinsam für Ansichten, die auf der Auswahl des `Pane` arbeiten.
 */
export function createItemActions(getPane: () => Pane, getItems: () => Entry[]) {
  let press: { x: number; y: number; index: number } | null = null;
  let suppressClickUntil = 0;

  function endPress() {
    press = null;
    window.removeEventListener('pointermove', onPressMove);
    window.removeEventListener('pointerup', endPress);
  }

  // Ein Zug über die Schwelle startet einen echten (OLE-)Drag.
  function onPressMove(event: PointerEvent) {
    if (!press) return;
    if ((event.buttons & 1) === 0) return endPress();
    if (Math.hypot(event.clientX - press.x, event.clientY - press.y) < DRAG_THRESHOLD) return;
    const { index } = press;
    endPress();
    const pane = getPane();
    const entry = getItems()[index];
    if (!entry) return;
    if (!pane.selected.has(entry.name)) pane.select(index);
    suppressClickUntil = performance.now() + 400;
    dnd.begin(ops.selectedPaths(pane), event);
  }

  return {
    pointerDown(event: PointerEvent, index: number) {
      if (event.button !== 0 || getPane().renaming) return;
      endPress();
      press = { x: event.clientX, y: event.clientY, index };
      window.addEventListener('pointermove', onPressMove);
      window.addEventListener('pointerup', endPress);
    },
    click(event: MouseEvent, index: number) {
      if (performance.now() < suppressClickUntil) return;
      getPane().select(index, { toggle: event.ctrlKey, range: event.shiftKey });
    },
    dblClick(event: MouseEvent, entry: Entry) {
      if (!event.ctrlKey && !event.shiftKey) void ops.open(getPane(), entry);
    },
    // Rechtsklick markiert den Eintrag (wie im Explorer) und zeigt das native Menü.
    context(event: MouseEvent, index: number) {
      event.preventDefault();
      event.stopPropagation();
      const pane = getPane();
      const entry = getItems()[index];
      if (entry && !pane.selected.has(entry.name)) pane.select(index);
      else pane.cursor = index;
      void ops.contextMenu(pane);
    },
    backgroundContext(event: MouseEvent) {
      event.preventDefault();
      const pane = getPane();
      // Menütaste / Umschalt+F10 (kein Rechtsklick): Menü der aktuellen Auswahl.
      if (event.button !== 2) return void ops.contextMenu(pane);
      pane.clearSelection();
      void ops.contextMenu(pane);
    },
    backgroundClick(event: MouseEvent) {
      if (event.target === event.currentTarget) getPane().clearSelection();
    },
  };
}
