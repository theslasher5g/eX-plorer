<script lang="ts">
  import { File } from '@lucide/svelte';
  import type { Entry, SyncState } from '../api';
  import { extensionBadge, fileCategory } from '../filetypes';
  import { isCloudPlaceholder, shellImageUrl } from '../images';
  import { joinPath } from '../paths';
  import SyncBadge from './SyncBadge.svelte';

  let { entry, dir, sync = null }: { entry: Entry; dir: string; sync?: SyncState | null } = $props();

  const badge = $derived(extensionBadge(entry.name));
  // Cloud-Platzhalter bekommen kein Thumbnail: das würde sie herunterladen.
  const url = $derived(
    entry.isDir || isCloudPlaceholder(entry.attributes) || !dir
      ? null
      : shellImageUrl(joinPath(dir, entry.name), 256, entry.modifiedMs),
  );

  let image = $state<HTMLImageElement>();
  let loadedUrl = $state<string | null>(null);
  const loaded = $derived(url !== null && loadedUrl === url);

  $effect(() => {
    if (image?.complete && image.naturalWidth > 0) loadedUrl = url;
  });
</script>

<span class="thumb" class:loaded>
  {#if entry.isDir}
    <svg class="folder" viewBox="0 0 40 32" aria-hidden="true">
      <path d="M2 6a3 3 0 0 1 3-3h9l4 4h17a3 3 0 0 1 3 3v18a3 3 0 0 1-3 3H5a3 3 0 0 1-3-3z" fill="var(--folder-back)" />
      <path d="M2 12a3 3 0 0 1 3-3h30a3 3 0 0 1 3 3v15a3 3 0 0 1-3 3H5a3 3 0 0 1-3-3z" fill="var(--folder-front)" />
    </svg>
  {:else}
    <span class="page">
      <i style:width="60%"></i><i></i><i></i><i style:width="80%"></i><i style:width="45%"></i>
      <span class="ft" style:--c="var(--t-{fileCategory(entry.name)})">
        {#if badge}{badge}{:else}<File size={12} strokeWidth={1.7} />{/if}
      </span>
    </span>
    {#if url}
      <img
        bind:this={image}
        src={url}
        alt=""
        loading="lazy"
        decoding="async"
        draggable="false"
        onload={() => (loadedUrl = url)}
        onerror={() => (loadedUrl = null)}
      />
    {/if}
  {/if}
  {#if sync}
    <span class="corner"><SyncBadge state={sync} compact /></span>
  {/if}
</span>

<style>
  .corner {
    position: absolute;
    left: 6px;
    bottom: 6px;
    display: flex;
  }

  .thumb {
    position: relative;
    width: 100%;
    aspect-ratio: 4 / 3;
    border-radius: 10px;
    background: var(--surface-2);
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
    transition: box-shadow var(--t-hover);
  }

  .folder {
    width: 56%;
  }

  .page {
    position: relative;
    width: 46%;
    aspect-ratio: 3 / 4;
    padding: 10px 8px;
    display: flex;
    flex-direction: column;
    gap: 5px;
    border-radius: 5px;
    background: var(--surface);
    box-shadow: var(--shadow-sm);
    transition: opacity var(--t-hover);
  }

  .page i {
    display: block;
    height: 3px;
    border-radius: 2px;
    background: var(--track-soft);
  }

  .ft {
    position: absolute;
    right: -8px;
    bottom: 8px;
    min-width: 30px;
    height: 20px;
    padding: 0 4px;
    border-radius: 6px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 9px;
    font-weight: 700;
    text-transform: uppercase;
    color: var(--c);
    background: var(--surface);
    box-shadow: var(--shadow-sm);
  }

  img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: contain;
    padding: 6px;
    opacity: 0;
    transition: opacity var(--t-hover);
    pointer-events: none;
  }

  .loaded img {
    opacity: 1;
  }

  .loaded .page {
    opacity: 0;
  }
</style>
