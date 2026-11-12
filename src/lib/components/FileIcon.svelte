<script lang="ts">
  import { CornerUpRight, File } from '@lucide/svelte';
  import type { Entry } from '../api';
  import { extensionBadge, fileCategory } from '../filetypes';
  import { isCloudPlaceholder, shellImageUrl } from '../images';
  import { joinPath } from '../paths';

  let { entry, dir }: { entry: Entry; dir: string } = $props();

  const badge = $derived(entry.isDir ? '' : extensionBadge(entry.name));

  // Windows-Icon bzw. Thumbnail der Datei. Cloud-Platzhalter bleiben beim
  // gezeichneten Symbol, damit nichts heruntergeladen wird.
  const imageUrl = $derived(
    entry.isDir || isCloudPlaceholder(entry.attributes) || !dir
      ? null
      : shellImageUrl(joinPath(dir, entry.name), 32, entry.modifiedMs),
  );

  let image = $state<HTMLImageElement>();
  let loadedUrl = $state<string | null>(null);
  const loaded = $derived(imageUrl !== null && loadedUrl === imageUrl);

  // Aus dem Cache kann das Bild schon fertig sein, bevor onload feuert.
  $effect(() => {
    if (image?.complete && image.naturalWidth > 0) loadedUrl = imageUrl;
  });
</script>

{#if entry.isDir}
  <span class="folder">
    <svg viewBox="0 0 40 32" aria-hidden="true">
      <path d="M2 6a3 3 0 0 1 3-3h9l4 4h17a3 3 0 0 1 3 3v18a3 3 0 0 1-3 3H5a3 3 0 0 1-3-3z" fill="var(--folder-back)" />
      <path d="M2 12a3 3 0 0 1 3-3h30a3 3 0 0 1 3 3v15a3 3 0 0 1-3 3H5a3 3 0 0 1-3-3z" fill="var(--folder-front)" />
    </svg>
    {#if entry.isLink}
      <span class="link"><CornerUpRight size={9} strokeWidth={2.6} /></span>
    {/if}
  </span>
{:else}
  <span class="fi" class:loaded>
    {#if badge}
      <span class="ft" style:--c="var(--t-{fileCategory(entry.name)})">{badge}</span>
    {:else}
      <span class="ft plain"><File size={14} strokeWidth={1.7} /></span>
    {/if}
    {#if imageUrl}
      <img
        bind:this={image}
        src={imageUrl}
        alt=""
        decoding="async"
        draggable="false"
        onload={() => (loadedUrl = imageUrl)}
        onerror={() => (loadedUrl = null)}
      />
    {/if}
  </span>
{/if}

<style>
  .folder {
    position: relative;
    flex: none;
    width: 24px;
    height: 19px;
    display: block;
  }

  .folder svg {
    display: block;
    width: 100%;
    height: 100%;
  }

  .link {
    position: absolute;
    left: -3px;
    bottom: -3px;
    width: 13px;
    height: 13px;
    border-radius: 4px;
    display: grid;
    place-items: center;
    background: var(--surface);
    color: var(--accent);
    box-shadow: var(--shadow-sm);
  }

  .fi {
    position: relative;
    flex: none;
    width: 30px;
    height: 22px;
    margin: 0 -3px;
    display: block;
  }

  .fi .ft {
    margin: 0;
    transition: opacity var(--t-hover);
  }

  .fi.loaded .ft {
    opacity: 0;
  }

  .fi img {
    position: absolute;
    left: 3px;
    top: -1px;
    width: 24px;
    height: 24px;
    object-fit: contain;
    opacity: 0;
    transition: opacity var(--t-hover);
    pointer-events: none;
  }

  .fi.loaded img {
    opacity: 1;
  }

  .ft {
    --c: var(--t-txt);
    flex: none;
    width: 30px;
    height: 22px;
    margin: 0 -3px;
    border-radius: 6px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.02em;
    text-transform: uppercase;
    color: var(--c);
    background: color-mix(in srgb, var(--c) 14%, transparent);
  }
</style>
