import type { Entry } from '../api';
import type { Pane } from './pane.svelte';

/** Schnellvorschau (Leertaste) für den markierten Eintrag der aktiven Ansicht. */
class QuickLook {
  pane = $state.raw<Pane | null>(null);
  index = $state(0);

  get isOpen(): boolean {
    return this.pane !== null;
  }

  get entries(): Entry[] {
    return this.pane?.visible ?? [];
  }

  get entry(): Entry | undefined {
    return this.entries[this.index];
  }

  /** Öffnet die Vorschau für den Fokus-Eintrag bzw. den ersten markierten. */
  open(pane: Pane): void {
    const items = pane.visible;
    let index = pane.cursor;
    if (index < 0 || !items[index] || !pane.selected.has(items[index].name)) {
      index = items.findIndex((e) => pane.selected.has(e.name));
    }
    if (index < 0) return;
    this.index = index;
    this.pane = pane;
  }

  toggle(pane: Pane): void {
    if (this.isOpen) this.close();
    else this.open(pane);
  }

  close(): void {
    this.pane = null;
  }

  /** Wechselt zum Nachbarn und zieht die Auswahl der Liste mit. */
  move(delta: number): void {
    const pane = this.pane;
    if (!pane) return;
    const index = Math.max(0, Math.min(pane.visible.length - 1, this.index + delta));
    if (index === this.index) return;
    this.index = index;
    pane.select(index);
    pane.scrollSignal++;
  }

  goto(index: number): void {
    this.move(index - this.index);
  }
}

export const quicklook = new QuickLook();
