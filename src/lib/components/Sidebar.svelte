<script lang="ts">
  import type { Component } from 'svelte';
  import {
    Cloud,
    Disc,
    Download,
    FileText,
    HardDrive,
    Image,
    Monitor,
    Network,
    Usb,
  } from '@lucide/svelte';
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
    const letter = drive.path.slice(0, 2);
    return `${drive.label || t(`drive.${drive.kind}`)} (${letter})`;
  }

  function driveUsage(drive: Drive): { text: string; used: number } | null {
    if (drive.totalBytes === null || drive.freeBytes === null || drive.totalBytes <= 0) return null;
    return {
      text: t('drive.free', {
        free: formatSize(drive.freeBytes),
        total: formatSize(drive.totalBytes),
      }),
      used: 1 - drive.freeBytes / drive.totalBytes,
    };
  }
</script>

{#snippet item(Icon: Component, label: string, path: string, title: string)}
  <li>
    <button
      class="item"
      class:active={samePath(path, current)}
      {title}
      onclick={() => onnavigate(path)}
    >
      <Icon size={16} strokeWidth={1.75} />
      <span class="label">{label}</span>
    </button>
  </li>
{/snippet}

<nav class="sidebar" aria-label={t('app.title')}>
  <div class="drag" data-tauri-drag-region>
    <span class="brand" data-tauri-drag-region>{t('app.title')}</span>
  </div>

  <div class="scroll">
    {#if data}
      {#if data.favorites.length}
        <section>
          <h2>{t('sidebar.favorites')}</h2>
          <ul>
            {#each data.favorites as place (place.path)}
              {@render item(placeIcons[place.kind], place.label || t(`place.${place.kind}`), place.path, place.path)}
            {/each}
          </ul>
        </section>
      {/if}

      {#if data.onedrive.length}
        <section>
          <h2>{t('sidebar.onedrive')}</h2>
          <ul>
            {#each data.onedrive as place (place.path)}
              {@render item(Cloud, place.label || t('place.onedrive'), place.path, place.path)}
            {/each}
          </ul>
        </section>
      {/if}

      {#if data.drives.length}
        <section>
          <h2>{t('sidebar.drives')}</h2>
          <ul>
            {#each data.drives as drive (drive.path)}
              {@const usage = driveUsage(drive)}
              {@const Icon = driveIcons[drive.kind]}
              <li>
                <button
                  class="item"
                  class:active={samePath(drive.path, current)}
                  title={usage?.text ?? drive.path}
                  onclick={() => onnavigate(drive.path)}
                >
                  <Icon size={16} strokeWidth={1.75} />
                  <span class="label">
                    {driveName(drive)}
                    {#if usage}
                      <span class="meter" class:full={usage.used > 0.9}>
                        <span style:width="{Math.round(usage.used * 100)}%"></span>
                      </span>
                    {/if}
                  </span>
                </button>
              </li>
            {/each}
          </ul>
        </section>
      {/if}
    {/if}
  </div>
</nav>

<style>
  .sidebar {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    min-width: 0;
    background: var(--sidebar-tint);
  }

  .drag {
    height: 48px;
    display: flex;
    align-items: center;
    padding: 0 18px;
  }

  .brand {
    font-family: var(--font-display);
    font-size: 13px;
    font-weight: 600;
    color: var(--text-2);
    letter-spacing: 0.01em;
  }

  .scroll {
    overflow-y: auto;
    padding: 0 10px 12px;
  }

  section + section {
    margin-top: 18px;
  }

  h2 {
    margin: 0 0 4px;
    padding: 0 8px;
    font-size: 11px;
    font-weight: 600;
    color: var(--text-3);
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 1px;
  }

  .item {
    display: flex;
    align-items: center;
    gap: 9px;
    width: 100%;
    min-height: 30px;
    padding: 4px 8px;
    border-radius: var(--radius-s);
    text-align: left;
    transition: background var(--fast) var(--ease);
  }

  .item :global(svg) {
    flex: none;
    color: var(--accent);
  }

  .item:hover {
    background: var(--hover);
  }

  .item.active {
    background: var(--active);
  }

  .label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .meter {
    display: block;
    height: 3px;
    margin-top: 4px;
    border-radius: 2px;
    background: var(--active);
    overflow: hidden;
  }

  .meter > span {
    display: block;
    height: 100%;
    border-radius: 2px;
    background: var(--accent);
  }

  .meter.full > span {
    background: var(--danger);
  }
</style>
