<script lang="ts">
  import { ArrowLeft, ArrowRight, ArrowUp, FolderPlus, LayoutGrid, List, ListTree, Search, X } from '@lucide/svelte';
  import type { Component } from 'svelte';
  import { t } from '../i18n';
  import { ops } from '../stores/ops.svelte';
  import type { Pane, ViewMode } from '../stores/pane.svelte';
  import Breadcrumb from './Breadcrumb.svelte';
  import SortMenu from './SortMenu.svelte';

  let { pane }: { pane: Pane } = $props();

  const views: { id: ViewMode; icon: Component; label: 'view.list' | 'view.grid' | 'view.tree' }[] = [
    { id: 'list', icon: List, label: 'view.list' },
    { id: 'grid', icon: LayoutGrid, label: 'view.grid' },
    { id: 'tree', icon: ListTree, label: 'view.tree' },
  ];

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

  <div class="seg" role="group" aria-label={t('view.label')}>
    {#each views as view (view.id)}
      {@const Icon = view.icon}
      <button
        class:on={pane.view === view.id}
        title={t(view.label)}
        aria-label={t(view.label)}
        aria-pressed={pane.view === view.id}
        onclick={() => (pane.view = view.id)}
      >
        <Icon size={18} strokeWidth={1.7} />
      </button>
    {/each}
  </div>

  <button
    class="icon-btn"
    title={t('ops.newFolder')}
    aria-label={t('ops.newFolder')}
    disabled={!!pane.error || !pane.path}
    onclick={() => ops.newFolder()}
  >
    <FolderPlus size={18} strokeWidth={1.7} />
  </button>

  <SortMenu {pane} />

  <label class="search" title={t('nav.filterTitle')}>
    <Search size={16} strokeWidth={1.7} />
    <input
      bind:this={filterInput}
      bind:value={pane.filter}
      type="text"
      placeholder={t('nav.filter')}
      aria-label={t('nav.filter')}
      disabled={pane.view === 'tree'}
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

  .seg {
    flex: none;
    display: flex;
    gap: 2px;
    padding: 3px;
    border-radius: 9px;
    background: var(--surface-2);
  }

  .seg button {
    width: 34px;
    height: 28px;
    border-radius: 7px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--muted);
    transition:
      background var(--t-select),
      color var(--t-select),
      box-shadow var(--t-select);
  }

  .seg button:hover {
    color: var(--text);
  }

  .seg button.on {
    background: var(--surface);
    color: var(--accent);
    box-shadow: var(--shadow-sm);
  }

  .search:has(input:disabled) {
    opacity: 0.45;
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
