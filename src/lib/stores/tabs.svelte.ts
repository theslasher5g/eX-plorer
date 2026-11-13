import { Pane } from './pane.svelte';
import { settings } from './settings.svelte';

/** Die offenen Tabs; jeder Tab ist eine eigenständige Ordneransicht. */
export class Tabs {
  panes = $state.raw<Pane[]>([new Pane()]);
  active = $state(0);

  readonly current = $derived(this.panes[this.active]);

  /** Jeder gerade angezeigte Ordner aller Tabs (bei Spaltenansicht mehrere je Tab). */
  get livePanes(): Pane[] {
    return this.panes.flatMap((p) => p.livePanes);
  }

  /** Der Ordner, auf dem Dateioperationen des aktiven Tabs arbeiten. */
  get target(): Pane {
    const pane = this.current;
    return pane.view === 'columns' ? pane.columns.active : pane;
  }

  /** Öffnet `path` in einem neuen Tab und aktiviert ihn. */
  open(path: string): void {
    const pane = new Pane();
    pane.view = settings.value.startView;
    this.panes = [...this.panes, pane];
    this.active = this.panes.length - 1;
    void pane.navigate(path);
  }

  /** Schließt einen Tab; der letzte Tab bleibt immer offen. */
  close(index: number): void {
    if (this.panes.length <= 1 || index < 0 || index >= this.panes.length) return;
    this.panes = this.panes.filter((_, i) => i !== index);
    if (index < this.active || this.active >= this.panes.length) this.active--;
  }

  activate(index: number): void {
    if (index >= 0 && index < this.panes.length) this.active = index;
  }

  cycle(delta: number): void {
    const count = this.panes.length;
    this.active = (this.active + delta + count) % count;
  }
}
