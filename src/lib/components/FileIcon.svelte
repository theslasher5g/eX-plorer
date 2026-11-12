<script lang="ts">
  import { CornerUpRight, File } from '@lucide/svelte';
  import type { Entry } from '../api';
  import { extensionBadge, fileCategory } from '../filetypes';

  let { entry }: { entry: Entry } = $props();

  const badge = $derived(entry.isDir ? '' : extensionBadge(entry.name));
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
{:else if badge}
  <span class="ft" style:--c="var(--t-{fileCategory(entry.name)})">{badge}</span>
{:else}
  <span class="ft plain"><File size={14} strokeWidth={1.7} /></span>
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
