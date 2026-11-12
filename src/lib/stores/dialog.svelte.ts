export interface ConfirmOptions {
  title: string;
  body: string;
  confirm: string;
  cancel: string;
  /** Destruktive Aktion: Bestätigen-Knopf in Warnfarbe. */
  danger?: boolean;
}

type Pending = ConfirmOptions & { resolve: (answer: boolean) => void };

class Dialogs {
  confirmState = $state.raw<Pending | null>(null);

  confirm(options: ConfirmOptions): Promise<boolean> {
    this.confirmState?.resolve(false);
    return new Promise((resolve) => {
      this.confirmState = { ...options, resolve };
    });
  }

  answer(value: boolean): void {
    const pending = this.confirmState;
    this.confirmState = null;
    pending?.resolve(value);
  }
}

export const dialogs = new Dialogs();
