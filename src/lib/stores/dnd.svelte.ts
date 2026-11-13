import { getApi, toApiError, type DragDropEvent } from '../api';
import { errorText } from '../i18n';
import { parentPath, samePath, splitPath } from '../paths';
import { ops } from './ops.svelte';
import { toasts } from './toast.svelte';

/** Läuft gerade ein Zug innerhalb des Fensters? */
interface Drag {
  paths: string[];
  x: number;
  y: number;
  /** Was beim Loslassen geschähe; `null` über einem ungültigen Ziel. */
  effect: 'move' | 'copy' | null;
  target: string | null;
}

const DRAG_ROOT_LEAVE = 'dragging';

/**
 * Drag & Drop.
 *
 * *Innerhalb des Fensters* ziehen wir selbst (`begin`): Ein blockierender
 * Windows-Drag würde die Oberfläche anhalten, das Ziel ließe sich nicht live
 * markieren. Das Ziel ergibt sich aus dem Element unter dem Zeiger
 * (`data-drop`). Verlässt der Zeiger das Fenster, wechselt der Zug in einen
 * echten Windows-Drag (`start`), damit Dateien in andere Programme gelangen.
 *
 * *Eingehend von außen*: Tauri meldet Position und Pfade; das Ziel ergibt sich
 * ebenso aus `data-drop`.
 */
class Dnd {
  /** Pfad des Ziels unter dem Zeiger, solange gezogen wird. */
  hover = $state<string | null>(null);

  /** Der laufende Zug innerhalb des Fensters (für die Anzeige am Zeiger). */
  drag = $state.raw<Drag | null>(null);

  isHover(path: string): boolean {
    return this.hover !== null && samePath(this.hover, path);
  }

  #targetAt(position: { x: number; y: number }): string | null {
    const ratio = window.devicePixelRatio || 1;
    const element = document.elementFromPoint(position.x / ratio, position.y / ratio);
    return element?.closest<HTMLElement>('[data-drop]')?.dataset.drop ?? null;
  }

  #onEvent(event: DragDropEvent): void {
    if (event.type === 'leave') {
      this.hover = null;
      return;
    }
    const target = this.#targetAt(event.position);
    if (event.type !== 'drop') {
      this.hover = target;
      return;
    }
    this.hover = null;
    // Ein Ordner lässt sich nicht auf sich selbst ablegen: einfach ignorieren.
    if (!target || event.paths.length === 0 || event.paths.some((p) => samePath(p, target))) return;
    void ops.transfer(event.paths, target, 'auto');
  }

  /** Hängt sich an die Fenster-Ereignisse; liefert die Abmeldung. */
  async init(): Promise<() => void> {
    const api = await getApi();
    return api.onDragDrop((event) => this.#onEvent(event));
  }

  /** Ist `target` ein sinnvolles Ziel für diese Pfade? */
  #valid(paths: string[], target: string): boolean {
    for (const path of paths) {
      if (samePath(path, target)) return false;
      // Nicht in sich selbst oder einen Unterordner von sich.
      if (target.toLowerCase().startsWith(`${path.toLowerCase().replace(/\\$/, '')}\\`)) return false;
    }
    // Alles liegt schon dort: nichts zu tun, also auch nichts markieren.
    return !paths.every((path) => {
      const parent = parentPath(path);
      return parent !== null && samePath(parent, target);
    });
  }

  /** Verschieben oder Kopieren, wie es das Backend bei `auto` entscheidet. */
  #effect(paths: string[], target: string, event: { ctrlKey: boolean; shiftKey: boolean }): 'move' | 'copy' {
    if (event.ctrlKey) return 'copy';
    if (event.shiftKey) return 'move';
    const root = splitPath(target)[0]?.path.toLowerCase();
    return paths.every((p) => splitPath(p)[0]?.path.toLowerCase() === root) ? 'move' : 'copy';
  }

  #update(event: PointerEvent | MouseEvent | KeyboardEvent, x: number, y: number): void {
    const drag = this.drag;
    if (!drag) return;
    const found = document.elementFromPoint(x, y)?.closest<HTMLElement>('[data-drop]')?.dataset.drop ?? null;
    const target = found && this.#valid(drag.paths, found) ? found : null;
    this.hover = target;
    this.drag = { ...drag, x, y, target, effect: target ? this.#effect(drag.paths, target, event) : null };
  }

  #onMove = (event: PointerEvent): void => {
    if ((event.buttons & 1) === 0) return this.#cancel();
    this.#update(event, event.clientX, event.clientY);
  };

  #onUp = (event: PointerEvent): void => {
    this.#update(event, event.clientX, event.clientY);
    const drag = this.drag;
    this.#stop();
    if (!drag?.target) return;
    const mode = event.ctrlKey ? 'copy' : event.shiftKey ? 'move' : 'auto';
    void ops.transfer(drag.paths, drag.target, mode);
  };

  #onKey = (event: KeyboardEvent): void => {
    if (event.key === 'Escape') {
      event.preventDefault();
      event.stopPropagation();
      this.#cancel();
    } else if (event.key === 'Control' || event.key === 'Shift') {
      // Die Beschriftung folgt den Umschalttasten.
      const drag = this.drag;
      if (drag) this.#update(event, drag.x, drag.y);
    }
  };

  /** Verlässt der Zeiger das Fenster, geht der Zug in einen echten Windows-Drag über. */
  #onLeave = (event: MouseEvent): void => {
    if (event.relatedTarget !== null || (event.buttons & 1) === 0) return;
    const drag = this.drag;
    this.#stop();
    if (drag) void this.start(drag.paths);
  };

  #cancel = (): void => {
    this.#stop();
  };

  #stop(): void {
    window.removeEventListener('pointermove', this.#onMove, true);
    window.removeEventListener('pointerup', this.#onUp, true);
    window.removeEventListener('keydown', this.#onKey, true);
    window.removeEventListener('keyup', this.#onKey, true);
    window.removeEventListener('mouseout', this.#onLeave, true);
    window.removeEventListener('blur', this.#cancel);
    document.documentElement.classList.remove(DRAG_ROOT_LEAVE);
    this.drag = null;
    this.hover = null;
  }

  /** Beginnt einen Zug mit diesen Dateien; die Maustaste ist gedrückt. */
  begin(paths: string[], event: PointerEvent): void {
    if (this.drag || paths.length === 0) return;
    this.drag = { paths, x: event.clientX, y: event.clientY, effect: null, target: null };
    window.addEventListener('pointermove', this.#onMove, true);
    window.addEventListener('pointerup', this.#onUp, true);
    window.addEventListener('keydown', this.#onKey, true);
    window.addEventListener('keyup', this.#onKey, true);
    window.addEventListener('mouseout', this.#onLeave, true);
    window.addEventListener('blur', this.#cancel);
    document.documentElement.classList.add(DRAG_ROOT_LEAVE);
    this.#update(event, event.clientX, event.clientY);
  }

  /** Startet einen Drag in andere Programme. Kehrt nach dem Loslassen zurück. */
  async start(paths: string[]): Promise<void> {
    try {
      await (await getApi()).startDrag(paths);
    } catch (err) {
      toasts.show(errorText(toApiError(err).code), 'err');
    } finally {
      this.hover = null;
    }
  }
}

export const dnd = new Dnd();
