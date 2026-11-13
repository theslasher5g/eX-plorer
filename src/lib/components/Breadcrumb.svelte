<script lang="ts">
  import { tick } from 'svelte';
  import { ChevronRight } from '@lucide/svelte';
  import { t } from '../i18n';
  import { splitPath } from '../paths';
  import { dnd } from '../stores/dnd.svelte';

  let { path, onnavigate }: { path: string; onnavigate: (path: string) => void } = $props();

  let editing = $state(false);
  let draft = $state('');
  let input = $state<HTMLInputElement>();
  let box = $state<HTMLDivElement>();

  const crumbs = $derived(splitPath(path));

  // Bei langen Pfaden bleibt das Ende (der aktuelle Ordner) sichtbar.
  $effect(() => {
    void crumbs;
    if (box && !editing) box.scrollLeft = box.scrollWidth;
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

  // Klick auf die freie Fläche der Pfadleiste schaltet auf das Textfeld um.
  function onBoxClick(event: MouseEvent) {
    if (event.target === event.currentTarget) void edit();
  }
</script>

{#if editing}
  <div class="crumbbox editing">
    <input
      bind:this={input}
      bind:value={draft}
      class="pathinput"
      aria-label={t('nav.path')}
      spellcheck="false"
      autocomplete="off"
      onkeydown={onKeydown}
      onblur={() => (editing = false)}
    />
  </div>
{:else}
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="crumbbox" bind:this={box} title={t('nav.editPath')} onclick={onBoxClick}>
    {#each crumbs as crumb, i (crumb.path)}
      {#if i > 0}
        <span class="crumbsep"><ChevronRight size={14} strokeWidth={1.7} /></span>
      {/if}
      <button
        class="crumb"
        class:last={i === crumbs.length - 1}
        class:drop={dnd.isHover(crumb.path)}
        data-drop={crumb.path}
        title={crumb.path}
        onclick={() => onnavigate(crumb.path)}
      >
        {crumb.label}
      </button>
    {/each}
  </div>
{/if}

<style>
  .crumbbox {
    flex: 1;
    min-width: 90px;
    height: 34px;
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 0 6px;
    border-radius: var(--r-btn);
    background: var(--surface-2);
    overflow: hidden;
    cursor: text;
    transition:
      background var(--t-hover),
      box-shadow var(--t-hover);
  }

  .crumbbox.editing {
    background: var(--surface);
    box-shadow: 0 0 0 2px var(--accent);
  }

  .crumb {
    flex: none;
    max-width: 240px;
    height: 26px;
    padding: 0 8px;
    border-radius: var(--r-sm);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    transition: background var(--t-hover);
  }

  .crumb:hover {
    background: var(--hover);
  }

  .crumb.last {
    font-weight: 600;
  }

  .crumbsep {
    flex: none;
    display: flex;
    color: var(--muted);
    pointer-events: none;
  }

  .pathinput {
    flex: 1;
    min-width: 0;
    height: 28px;
    padding: 0 6px;
    border: 0;
    outline: none;
    background: transparent;
    user-select: text;
    -webkit-user-select: text;
  }
</style>
