<script lang="ts">
  import { tick, type Component } from 'svelte';
  import { Cloud, Disc, Download, FileText, Folder, Globe, HardDrive, Image, Monitor, Network, SlidersHorizontal, Usb } from '@lucide/svelte';
  import type { Drive, DriveKind, PlaceKind, SidebarData } from '../api';
  import { formatSize } from '../format';
  import { t } from '../i18n';
  import { NETWORK_PATH, samePath } from '../paths';
  import { dnd } from '../stores/dnd.svelte';
  import { contextmenu } from '../stores/contextmenu.svelte';
  import { favoriteLabel, favorites } from '../stores/favorites.svelte';
  import { settings } from '../stores/settings.svelte';

  let {
    data,
    current,
    onnavigate,
    onnewtab,
  }: {
    data: SidebarData | null;
    current: string;
    onnavigate: (path: string) => void;
    onnewtab?: (path: string) => void;
  } = $props();

  const placeIcons: Record<PlaceKind, Component> = {
    desktop: Monitor,
    documents: FileText,
    downloads: Download,
    pictures: Image,
    onedrive: Cloud,
  };

  const driveIcons: Record<DriveKind, Component> = {
    fixed: HardDrive,
    ram: HardDrive,
    unknown: HardDrive,
    removable: Usb,
    remote: Network,
    cdrom: Disc,
  };

  /** Angeheftete Ordner, die nicht ohnehin schon zu den festen Favoriten gehören. */
  const pinned = $derived(
    favorites.list.filter((path) => !data?.favorites.some((place) => samePath(place.path, path))),
  );

  // Angeheftete Ordner per Ziehen umsortieren (eigene Zeiger-Ereignisse, kein OLE).
  const DRAG_START = 5;
  let press: { path: string; y: number } | null = null;
  /** `gap` = Einfügeposition 0…n zwischen den angehefteten Einträgen. */
  let reorder = $state<{ path: string; gap: number } | null>(null);
  let suppressClickUntil = 0;
  let pinList = $state<HTMLElement>();

  function gapAt(y: number): number {
    const rows = [...(pinList?.querySelectorAll<HTMLElement>('.pin') ?? [])];
    const index = rows.findIndex((row) => {
      const box = row.getBoundingClientRect();
      return y < box.top + box.height / 2;
    });
    return index < 0 ? rows.length : index;
  }

  function stopReorder() {
    press = null;
    reorder = null;
    window.removeEventListener('pointermove', onReorderMove);
    window.removeEventListener('pointerup', onReorderUp);
    window.removeEventListener('keydown', onReorderKey, true);
  }

  function onReorderMove(event: PointerEvent) {
    if (!press) return;
    if ((event.buttons & 1) === 0) return stopReorder();
    if (!reorder && Math.abs(event.clientY - press.y) < DRAG_START) return;
    reorder = { path: press.path, gap: gapAt(event.clientY) };
  }

  function onReorderUp() {
    const current = reorder;
    stopReorder();
    if (!current) return;
    suppressClickUntil = performance.now() + 300;
    favorites.reorder(current.path, pinned[current.gap] ?? null);
  }

  function onReorderKey(event: KeyboardEvent) {
    if (event.key !== 'Escape') return;
    event.preventDefault();
    event.stopPropagation();
    stopReorder();
  }

  function onPinDown(event: PointerEvent, path: string) {
    if (event.button !== 0 || (event.target as Element).closest('.unpin')) return;
    stopReorder();
    press = { path, y: event.clientY };
    window.addEventListener('pointermove', onReorderMove);
    window.addEventListener('pointerup', onReorderUp);
    window.addEventListener('keydown', onReorderKey, true);
  }

  /** Alt+Pfeil hoch/runter verschiebt den fokussierten Eintrag (Tastatur-Alternative). */
  function onPinKey(event: KeyboardEvent, path: string, index: number) {
    if (!event.altKey || (event.key !== 'ArrowUp' && event.key !== 'ArrowDown')) return;
    event.preventDefault();
    event.stopPropagation();
    const target = event.key === 'ArrowUp' ? index - 1 : index + 1;
    if (target < 0 || target >= pinned.length) return;
    if (event.key === 'ArrowUp') favorites.reorder(path, pinned[index - 1]);
    else favorites.reorder(path, pinned[index + 2] ?? null);
    // Der Eintrag wandert im DOM; der Fokus soll mitgehen.
    void tick().then(() => pinList?.querySelectorAll<HTMLElement>('.pin .nav')[target]?.focus());
  }

  const MENU_OPEN_TAB = 1;
  const MENU_UNPIN = 2;

  /** Rechtsklick auf einen angehefteten Ordner: lösen oder in neuem Tab öffnen. */
  async function onPinContext(event: MouseEvent, path: string) {
    event.preventDefault();
    event.stopPropagation();
    const choice = await contextmenu.showLocal([
      { id: MENU_OPEN_TAB, label: t('menu.newTab') },
      { id: MENU_UNPIN, label: t('favorites.unpin') },
    ]);
    if (choice === MENU_UNPIN) favorites.unpin(path);
    else if (choice === MENU_OPEN_TAB) onnewtab?.(path);
  }

  function driveName(drive: Drive): string {
    return `${drive.label || t(`drive.${drive.kind}`)} (${drive.path.slice(0, 2)})`;
  }

  function driveTitle(drive: Drive): string {
    if (drive.totalBytes === null || drive.freeBytes === null) return drive.path;
    return t('drive.free', { free: formatSize(drive.freeBytes), total: formatSize(drive.totalBytes) });
  }
</script>

{#snippet item(Icon: Component, label: string, path: string, title: string)}
  <button
    class="nav"
    class:on={samePath(path, current)}
    class:drop={dnd.isHover(path)}
    data-drop={path === NETWORK_PATH ? undefined : path}
    {title}
    onclick={() => onnavigate(path)}
  >
    <Icon size={18} strokeWidth={1.7} />
    <span>{label}</span>
  </button>
{/snippet}

<aside class="side" aria-label={t('sidebar.label')}>
  {#if data}
    {#if data.favorites.length || pinned.length}
      <div class="gtitle">{t('sidebar.favorites')}</div>
      {#each data.favorites as place (place.path)}
        {@render item(placeIcons[place.kind], place.label || t(`place.${place.kind}`), place.path, place.path)}
      {/each}
      <div class="pins" bind:this={pinList}>
      {#each pinned as path, index (path)}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="pin"
          class:dragging={reorder?.path === path}
          class:before={reorder !== null && reorder.gap === index && reorder.path !== path}
          class:after={reorder !== null && reorder.gap === pinned.length && index === pinned.length - 1 && reorder.path !== path}
          onpointerdown={(event) => onPinDown(event, path)}
          oncontextmenu={(event) => onPinContext(event, path)}
          onkeydowncapture={(event) => onPinKey(event, path, index)}
          onclickcapture={(event) => {
            if (performance.now() < suppressClickUntil) {
              event.stopPropagation();
              event.preventDefault();
            }
          }}
        >
          {@render item(Folder, favoriteLabel(path), path, path)}
        </div>
      {/each}
      </div>
    {/if}

    {#if data.onedrive.length}
      <div class="gtitle">{t('sidebar.onedrive')}</div>
      {#each data.onedrive as place (place.path)}
        {@render item(
          Cloud,
          place.account === 'personal' ? t('onedrive.personal') : place.label || t('place.onedrive'),
          place.path,
          place.email ? `${place.path} · ${place.email}` : place.path,
        )}
      {/each}
    {/if}

    {#if data.drives.length}
      <div class="gtitle">{t('sidebar.drives')}</div>
      {#each data.drives as drive (drive.path)}
        {@render item(driveIcons[drive.kind], driveName(drive), drive.path, driveTitle(drive))}
      {/each}
    {/if}
  {/if}
  <div class="gtitle">{t('sidebar.network')}</div>
  {@render item(Globe, t('place.network'), NETWORK_PATH, t('place.networkTitle'))}
  <div class="gap"></div>
  <button class="nav" title={t('settings.open')} onclick={() => settings.open()}>
    <SlidersHorizontal size={18} strokeWidth={1.7} />
    <span>{t('sidebar.settings')}</span>
  </button>
</aside>

<style>
  .side {
    width: 220px;
    flex: none;
    padding: 6px 10px 12px;
    overflow: auto;
  }

  .pin {
    position: relative;
    user-select: none;
  }

  .pin.dragging {
    opacity: 0.45;
  }

  .pin.before::before,
  .pin.after::after {
    content: '';
    position: absolute;
    left: 10px;
    right: 10px;
    height: 2px;
    border-radius: 1px;
    background: var(--accent);
    pointer-events: none;
  }

  .pin.before::before {
    top: -1px;
  }

  .pin.after::after {
    bottom: -1px;
  }

  .gap {
    height: 14px;
  }

  .gtitle {
    padding: 16px 10px 6px;
    font-size: 12px;
    font-weight: 600;
    color: var(--muted);
    letter-spacing: 0.01em;
  }

  .gtitle:first-child {
    padding-top: 6px;
  }

  .nav {
    position: relative;
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    height: 34px;
    padding: 0 10px;
    border-radius: var(--r-btn);
    transition: background var(--t-hover);
  }

  .nav:hover {
    background: var(--hover);
  }

  .nav.on {
    background: var(--nav-on);
    box-shadow: var(--shadow-sm);
    font-weight: 600;
  }

  .nav.on::before {
    content: '';
    position: absolute;
    left: 0;
    top: 9px;
    bottom: 9px;
    width: 3px;
    border-radius: 2px;
    background: var(--accent);
  }

  .nav :global(svg) {
    flex: none;
    color: var(--muted);
  }

  .nav.on :global(svg) {
    color: var(--accent);
  }

  .nav span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
