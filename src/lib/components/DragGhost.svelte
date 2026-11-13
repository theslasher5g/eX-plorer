<script lang="ts">
  import { Copy, FolderInput, Files } from '@lucide/svelte';
  import { formatCount } from '../format';
  import { itemsText, t } from '../i18n';
  import { splitPath } from '../paths';
  import { dnd } from '../stores/dnd.svelte';

  const drag = $derived(dnd.drag);
  const targetName = $derived(drag?.target ? (splitPath(drag.target).at(-1)?.label ?? drag.target) : '');
</script>

{#if drag}
  <div class="ghost" class:ok={drag.effect !== null} style:left="{drag.x + 16}px" style:top="{drag.y + 16}px" aria-hidden="true">
    {#if drag.effect === 'copy'}
      <Copy size={15} strokeWidth={1.9} />
    {:else if drag.effect === 'move'}
      <FolderInput size={15} strokeWidth={1.9} />
    {:else}
      <Files size={15} strokeWidth={1.9} />
    {/if}
    <span>
      {#if drag.effect}
        {t(drag.effect === 'copy' ? 'dnd.copyTo' : 'dnd.moveTo', { name: targetName })}
      {:else}
        {itemsText(drag.paths.length)}
      {/if}
    </span>
    {#if drag.effect && drag.paths.length > 1}<small>{formatCount(drag.paths.length)}</small>{/if}
  </div>
{/if}

<style>
  .ghost {
    position: fixed;
    z-index: 950;
    max-width: 320px;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    border-radius: 10px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    box-shadow: var(--shadow-md);
    font-size: 12.5px;
    pointer-events: none;
    white-space: nowrap;
  }

  .ghost.ok {
    background: var(--accent);
    border-color: transparent;
    color: var(--on-accent);
  }

  span {
    overflow: hidden;
    text-overflow: ellipsis;
  }

  small {
    padding: 0 6px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--on-accent) 22%, transparent);
    font-size: 11px;
  }
</style>
