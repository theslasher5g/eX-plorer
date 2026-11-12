<script lang="ts">
  import { tick } from 'svelte';
  import { ChevronRight } from '@lucide/svelte';
  import { t } from '../i18n';
  import { splitPath } from '../paths';

  let { path, onnavigate }: { path: string; onnavigate: (path: string) => void } = $props();

  let editing = $state(false);
  let draft = $state('');
  let input = $state<HTMLInputElement>();
  let trail = $state<HTMLDivElement>();

  const crumbs = $derived(splitPath(path));

  // Bei langen Pfaden bleibt das Ende (der aktuelle Ordner) sichtbar.
  $effect(() => {
    void crumbs;
    if (trail) trail.scrollLeft = trail.scrollWidth;
  });

  export async function edit() {
    draft = path;
    editing = true;
    await tick();
    input?.select();
  }

  function commit() {
    const target = draft.trim();
    editing = false;
    if (target) onnavigate(target);
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter') commit();
    else if (event.key === 'Escape') editing = false;
    else return;
    event.preventDefault();
    event.stopPropagation();
  }
</script>

<div class="field" class:editing>
  {#if editing}
    <input
      bind:this={input}
      bind:value={draft}
      aria-label={t('nav.path')}
      spellcheck="false"
      autocomplete="off"
      onkeydown={onKeydown}
      onblur={() => (editing = false)}
    />
  {:else}
    <div class="trail" bind:this={trail}>
      {#each crumbs as crumb, i (crumb.path)}
        {#if i > 0}
          <span class="sep"><ChevronRight size={12} strokeWidth={2} /></span>
        {/if}
        <button
          class="crumb"
          class:last={i === crumbs.length - 1}
          onclick={() => onnavigate(crumb.path)}
        >
          {crumb.label}
        </button>
      {/each}
    </div>
    <button class="rest" title={t('nav.editPath')} aria-label={t('nav.editPath')} onclick={edit}></button>
  {/if}
</div>

<style>
  .field {
    flex: 1;
    min-width: 0;
    height: 32px;
    display: flex;
    align-items: center;
    border-radius: var(--radius-m);
    background: var(--field);
    transition:
      background var(--fast) var(--ease),
      box-shadow var(--fast) var(--ease);
  }

  .field.editing {
    background: var(--field-focus);
    box-shadow: 0 0 0 2px var(--accent);
  }

  input {
    width: 100%;
    height: 100%;
    padding: 0 10px;
    border: 0;
    outline: 0;
    background: transparent;
    user-select: text;
    -webkit-user-select: text;
  }

  .trail {
    display: flex;
    align-items: center;
    min-width: 0;
    padding: 0 4px;
    overflow: hidden;
  }

  .crumb {
    flex: none;
    max-width: 220px;
    padding: 3px 6px;
    border-radius: var(--radius-s);
    color: var(--text-2);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    transition:
      background var(--fast) var(--ease),
      color var(--fast) var(--ease);
  }

  .crumb:hover {
    background: var(--hover);
    color: var(--text);
  }

  .crumb.last {
    color: var(--text);
    font-weight: 600;
  }

  .sep {
    flex: none;
    display: grid;
    color: var(--text-3);
  }

  .rest {
    flex: 1;
    min-width: 24px;
    align-self: stretch;
    cursor: text;
    border-radius: var(--radius-m);
  }
</style>
