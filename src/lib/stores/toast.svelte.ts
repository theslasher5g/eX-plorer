export type ToastKind = 'ok' | 'err';

/** Kurze Rückmeldung unten in der Mitte. Es gibt immer höchstens eine. */
class Toasts {
  current = $state<{ id: number; text: string; kind: ToastKind } | null>(null);
  #timer: ReturnType<typeof setTimeout> | undefined;
  #next = 1;

  show(text: string, kind: ToastKind = 'ok'): void {
    clearTimeout(this.#timer);
    this.current = { id: this.#next++, text, kind };
    this.#timer = setTimeout(() => (this.current = null), kind === 'err' ? 5000 : 3000);
  }
}

export const toasts = new Toasts();
