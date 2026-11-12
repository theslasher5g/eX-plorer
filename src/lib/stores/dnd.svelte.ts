import { getApi, toApiError, type DragDropEvent } from '../api';
import { errorText } from '../i18n';
import { samePath } from '../paths';
import { ops } from './ops.svelte';
import { toasts } from './toast.svelte';

/**
 * Drag & Drop über OLE.
 *
 * *Eingehend* (auch aus der eigenen App): Tauri meldet Position und Pfade;
 * das Ziel ergibt sich aus dem Element unter dem Zeiger (`data-drop`).
 * *Ausgehend*: `start` lässt das Backend einen echten Shell-Drag starten,
 * damit Dateien in andere Programme gezogen werden können.
 */
class Dnd {
  /** Pfad des Ziels unter dem Zeiger, solange gezogen wird. */
  hover = $state<string | null>(null);

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
