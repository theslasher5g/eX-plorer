<script lang="ts">
  import { ArrowLeft, ArrowRight, ArrowUp, Search, X } from '@lucide/svelte';
  import { t } from '../i18n';
  import type { Pane } from '../stores/pane.svelte';
  import Breadcrumb from './Breadcrumb.svelte';
  import SortMenu from './SortMenu.svelte';

  let { pane }: { pane: Pane } = $props();

  let breadcrumb = $state<ReturnType<typeof Breadcrumb>>();
  let filterInput = $state<HTMLInputElement>();

  export function editPath() {
    void breadcrumb?.edit();
  }

  export function focusFilter() {
    filterInput?.select();
  }

  function onFilterKeydown(event: KeyboardEvent) {
    if (event.key !== 'Escape') return;
    pane.filter = '';
    filterInput?.blur();
    event.preventDefault();
  }
</script>

<div class="toolbar">
  <div class="navpair">
    <button class="icon-btn" title={t('nav.back')} aria-label={t('nav.back')} disabled={!pane.canBack} onclick={() => pane.back()}>
      <ArrowLeft size={18} strokeWidth={1.7} />
    </button>
    <button class="icon-btn" title={t('nav.forward')} aria-label={t('nav.forward')} disabled={!pane.canForward} onclick={() => pane.forward()}>
      <ArrowRight size={18} strokeWidth={1.7} />
    </button>
    <button class="icon-btn" title={t('nav.up')} aria-label={t('nav.up')} disabled={!pane.canUp} onclick={() => pane.up()}>
      <ArrowUp size={18} strokeWidth={1.7} />
    </button>
  </div>

  <Breadcrumb bind:this={breadcrumb} path={pane.path} onnavigate={(path) => pane.navigate(path)} />

  <SortMenu {pane} />

  <label class="search" title={t('nav.filterTitle')}>
    <Search size={16} strokeWidth={1.7} />
    <input
      bind:this={filterInput}
      bind:value={pane.filter}
      type="text"
      placeholder={t('nav.filter')}
      aria-label={t('nav.filter')}
      spellcheck="false"
      autocomplete="off"
      onkeydown={onFilterKeydown}
    />
    {#if pane.filter}
      <button class="icon-btn clear" title={t('nav.clearFilter')} aria-label={t('nav.clearFilter')} onclick={() => (pane.filter = '')}>
        <X size={16} strokeWidth={1.7} />
      </button>
    {/if}
  </label>
</div>

<style>
  .toolbar {
    height: 56px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 14px;
    border-bottom: 1px solid var(--border);
  }

  .navpair {
    display: flex;
    gap: 2px;
  }

  .search {
    width: 190px;
    flex: none;
    height: 34px;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 8px 0 10px;
    border-radius: var(--r-btn);
    background: var(--surface-2);
    color: var(--muted);
    cursor: text;
    transition:
      box-shadow var(--t-hover),
      background var(--t-hover);
  }

  .search:focus-within {
    background: var(--surface);
    box-shadow: 0 0 0 2px var(--accent);
  }

  .search :global(svg) {
    flex: none;
  }

  .search input {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: none;
    background: transparent;
    color: var(--text);
    user-select: text;
    -webkit-user-select: text;
  }

  .search input::placeholder {
    color: var(--muted);
  }

  .clear {
    width: 24px;
    height: 24px;
  }
</style>
