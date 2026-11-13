import { t } from '../i18n';
import { samePath, splitPath } from '../paths';
import { settings } from './settings.svelte';
import { toasts } from './toast.svelte';

/** Höchstzahl angehefteter Ordner; das Backend erzwingt sie noch einmal. */
export const MAX_FAVORITES = 40;

/** Name, unter dem ein angehefteter Ordner in der Seitenleiste steht. */
export function favoriteLabel(path: string): string {
  return splitPath(path).at(-1)?.label ?? path;
}

/**
 * An die Seitenleiste angeheftete Ordner („Schnellzugriff“). Sie liegen in den
 * Einstellungen und überleben so einen Neustart.
 */
class Favorites {
  get list(): string[] {
    return settings.value.favorites;
  }

  isPinned(path: string): boolean {
    return this.list.some((p) => samePath(p, path));
  }

  pin(path: string): void {
    if (this.isPinned(path)) {
      toasts.show(t('favorites.already', { name: favoriteLabel(path) }));
      return;
    }
    if (this.list.length >= MAX_FAVORITES) {
      toasts.show(t('favorites.full'), 'err');
      return;
    }
    settings.update({ favorites: [...this.list, path] });
    toasts.show(t('favorites.pinned', { name: favoriteLabel(path) }));
  }

  unpin(path: string): void {
    if (!this.isPinned(path)) return;
    settings.update({ favorites: this.list.filter((p) => !samePath(p, path)) });
    toasts.show(t('favorites.unpinned', { name: favoriteLabel(path) }));
  }

  /** Stellt `path` vor `before` (ans Ende, wenn `before` fehlt). */
  reorder(path: string, before: string | null): void {
    if (!this.isPinned(path) || (before !== null && samePath(before, path))) return;
    const rest = this.list.filter((p) => !samePath(p, path));
    const index = before === null ? -1 : rest.findIndex((p) => samePath(p, before));
    const own = this.list.find((p) => samePath(p, path)) ?? path;
    if (index < 0) rest.push(own);
    else rest.splice(index, 0, own);
    if (rest.every((p, i) => p === this.list[i])) return;
    settings.update({ favorites: rest });
  }

  toggle(path: string): void {
    if (this.isPinned(path)) this.unpin(path);
    else this.pin(path);
  }
}

export const favorites = new Favorites();
