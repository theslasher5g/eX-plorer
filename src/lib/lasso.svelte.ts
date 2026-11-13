import type { Pane } from './stores/pane.svelte';

/** Rechteck in Inhaltskoordinaten des scrollenden Bereichs. */
export interface Rect {
  left: number;
  top: number;
  right: number;
  bottom: number;
}

const THRESHOLD = 4;
/** Abstand zum Rand, ab dem beim Ziehen automatisch gescrollt wird. */
const EDGE = 28;

interface Host {
  container: () => HTMLElement | undefined;
  pane: () => Pane;
  /** Indizes (aufsteigend) der Einträge, die das Rechteck berühren. */
  hit: (rect: Rect) => number[];
}

/**
 * Auswahl per Gummiband: Maustaste auf freier Fläche drücken und ziehen.
 * Mit Strg oder Umschalt bleibt die bisherige Auswahl erhalten. Am oberen
 * und unteren Rand scrollt die Fläche mit.
 */
export class Lasso {
  /** Das sichtbare Rechteck (Inhaltskoordinaten); `null`, solange nicht gezogen wird. */
  rect = $state.raw<Rect | null>(null);

  readonly #host: Host;
  #start = { x: 0, y: 0 };
  #pointer = { x: 0, y: 0 };
  #base: ReadonlySet<string> | null = null;
  #active = false;
  #frame = 0;
  #signature = '';
  #suppressUntil = 0;

  constructor(host: Host) {
    this.#host = host;
  }

  /** Hat gerade ein Gummiband-Zug geendet? Dann soll der Klick danach nichts abwählen. */
  consumedClick(): boolean {
    return performance.now() < this.#suppressUntil;
  }

  #toContent(clientX: number, clientY: number): { x: number; y: number } {
    const el = this.#host.container();
    if (!el) return { x: 0, y: 0 };
    const box = el.getBoundingClientRect();
    const x = clientX - box.left + el.scrollLeft;
    const y = clientY - box.top + el.scrollTop;
    return {
      x: Math.max(0, Math.min(el.scrollWidth, x)),
      y: Math.max(0, Math.min(el.scrollHeight, y)),
    };
  }

  /** Räumt auf, wenn die Ansicht verschwindet (z. B. beim Wechsel mitten im Ziehen). */
  destroy(): void {
    this.#end();
  }

  pointerDown = (event: PointerEvent): void => {
    const el = this.#host.container();
    if (!el || event.button !== 0 || this.#active) return;
    // Nur freie Fläche: nicht auf Einträgen, Kopfzeilen, Knöpfen oder Eingabefeldern.
    if ((event.target as Element).closest('.row, .tile, .crow, .ghead, .lhead, button, input')) return;
    const box = el.getBoundingClientRect();
    // Die Bildlaufleiste gehört nicht zur Fläche.
    if (event.clientX >= box.left + el.clientWidth || event.clientY >= box.top + el.clientHeight) return;
    this.#start = this.#toContent(event.clientX, event.clientY);
    this.#pointer = { x: event.clientX, y: event.clientY };
    this.#base = event.ctrlKey || event.shiftKey ? new Set(this.#host.pane().selected) : null;
    this.#signature = '';
    window.addEventListener('pointermove', this.#move);
    window.addEventListener('pointerup', this.#end);
    window.addEventListener('pointercancel', this.#end);
  };

  #move = (event: PointerEvent): void => {
    if ((event.buttons & 1) === 0) return this.#end();
    this.#pointer = { x: event.clientX, y: event.clientY };
    if (!this.#active) {
      const here = this.#toContent(event.clientX, event.clientY);
      if (Math.hypot(here.x - this.#start.x, here.y - this.#start.y) < THRESHOLD) return;
      this.#active = true;
      this.#tick();
    }
    this.#update();
  };

  #update(): void {
    const here = this.#toContent(this.#pointer.x, this.#pointer.y);
    const rect: Rect = {
      left: Math.min(this.#start.x, here.x),
      top: Math.min(this.#start.y, here.y),
      right: Math.max(this.#start.x, here.x),
      bottom: Math.max(this.#start.y, here.y),
    };
    this.rect = rect;
    const indices = this.#host.hit(rect);
    const signature = `${indices.length}:${indices[0] ?? -1}:${indices.at(-1) ?? -1}`;
    if (signature === this.#signature) return;
    this.#signature = signature;
    this.#host.pane().setSelection(indices, this.#base);
  }

  /** Scrollt, solange der Zeiger am oberen oder unteren Rand steht. */
  #tick = (): void => {
    if (!this.#active) return;
    const el = this.#host.container();
    if (el) {
      const box = el.getBoundingClientRect();
      let dy = 0;
      if (this.#pointer.y < box.top + EDGE) dy = -Math.min(32, (box.top + EDGE - this.#pointer.y) / 2 + 4);
      else if (this.#pointer.y > box.bottom - EDGE) dy = Math.min(32, (this.#pointer.y - (box.bottom - EDGE)) / 2 + 4);
      if (dy !== 0) {
        el.scrollTop += dy;
        this.#update();
      }
    }
    this.#frame = requestAnimationFrame(this.#tick);
  };

  #end = (): void => {
    window.removeEventListener('pointermove', this.#move);
    window.removeEventListener('pointerup', this.#end);
    window.removeEventListener('pointercancel', this.#end);
    cancelAnimationFrame(this.#frame);
    if (this.#active) this.#suppressUntil = performance.now() + 300;
    this.#active = false;
    this.rect = null;
  };
}
