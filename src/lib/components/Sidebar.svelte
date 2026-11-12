<script lang="ts">
  import type { Component } from 'svelte';
  import { Cloud, Disc, Download, FileText, HardDrive, Image, Monitor, Network, Usb } from '@lucide/svelte';
  import type { Drive, DriveKind, PlaceKind, SidebarData } from '../api';
  import { formatSize } from '../format';
  import { t } from '../i18n';
  import { samePath } from '../paths';

  let {
    data,
    current,
    onnavigate,
  }: {
    data: SidebarData | null;
    current: string;
    onnavigate: (path: string) => void;
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

  function driveName(drive: Drive): string {
    return `${drive.label || t(`drive.${drive.kind}`)} (${drive.path.slice(0, 2)})`;
  }

  function driveTitle(drive: Drive): string {
    if (drive.totalBytes === null || drive.freeBytes === null) return drive.path;
    return t('drive.free', { free: formatSize(drive.freeBytes), total: formatSize(drive.totalBytes) });
  }
</script>

{#snippet item(Icon: Component, label: string, path: string, title: string)}
  <button class="nav" class:on={samePath(path, current)} {title} onclick={() => onnavigate(path)}>
    <Icon size={18} strokeWidth={1.7} />
    <span>{label}</span>
  </button>
{/snippet}

<aside class="side" aria-label={t('sidebar.label')}>
  {#if data}
    {#if data.favorites.length}
      <div class="gtitle">{t('sidebar.favorites')}</div>
      {#each data.favorites as place (place.path)}
        {@render item(placeIcons[place.kind], place.label || t(`place.${place.kind}`), place.path, place.path)}
      {/each}
    {/if}

    {#if data.onedrive.length}
      <div class="gtitle">{t('sidebar.onedrive')}</div>
      {#each data.onedrive as place (place.path)}
        {@render item(Cloud, place.label || t('place.onedrive'), place.path, place.path)}
      {/each}
    {/if}

    {#if data.drives.length}
      <div class="gtitle">{t('sidebar.drives')}</div>
      {#each data.drives as drive (drive.path)}
        {@render item(driveIcons[drive.kind], driveName(drive), drive.path, driveTitle(drive))}
      {/each}
    {/if}
  {/if}
</aside>

<style>
  .side {
    width: 220px;
    flex: none;
    padding: 6px 10px 12px;
    overflow: auto;
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
