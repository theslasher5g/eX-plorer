<script lang="ts">
  import { ArrowUp, ChevronLeft, ChevronRight, Eye, EyeOff, RotateCw, Search } from '@lucide/svelte';
  import { t } from '../i18n';
  import type { Pane } from '../stores/pane.svelte';
  import Breadcrumb from './Breadcrumb.svelte';
  import WindowControls from './WindowControls.svelte';

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

<header class="toolbar" data-tauri-drag-region>
  <div class="group">
    <button class="icon" title={t('nav.back')} aria-label={t('nav.back')} disabled={!pane.canBack} onclick={() => pane.back()}>
      <ChevronLeft size={18} strokeWidth={1.75} />
    </button>
    <button class="icon" title={t('nav.forward')} aria-label={t('nav.forward')} disabled={!pane.canForward} onclick={() => pane.forward()}>
      <ChevronRight size={18} strokeWidth={1.75} />
    </button>
    <button class="icon" title={t('nav.up')} aria-label={t('nav.up')} disabled={!pane.canUp} onclick={() => pane.up()}>
      <ArrowUp size={17} strokeWidth={1.75} />
    </button>
    <button class="icon" title={t('nav.refresh')} aria-label={t('nav.refresh')} onclick={() => pane.refresh()}>
      <span class="spin" class:spinning={pane.loading}><RotateCw size={15} strokeWidth={1.75} /></span>
    </button>
  </div>

  <Breadcrumb bind:this={breadcrumb} path={pane.path} onnavigate={(path) => pane.navigate(path)} />

  <label class="filter">
    <Search size={14} strokeWidth={1.75} />
    <input
      bind:this={filterInput}
      bind:value={pane.filter}
      placeholder={t('nav.filter')}
      aria-label={t('nav.filter')}
      spellcheck="false"
      autocomplete="off"
      onkeydown={onFilterKeydown}
    />
  </label>

  <button
    class="icon"
    class:on={pane.showHidden}
    title={pane.showHidden ? t('nav.hideHidden') : t('nav.showHidden')}
    aria-label={pane.showHidden ? t('nav.hideHidden') : t('nav.showHidden')}
    aria-pressed={pane.showHidden}
    onclick={() => (pane.showHidden = !pane.showHidden)}
  >
    {#if pane.showHidden}
      <Eye size={16} strokeWidth={1.75} />
    {:else}
      <EyeOff size={16} strokeWidth={1.75} />
    {/if}
  </button>

  <WindowControls />
</header>

<style>
  .toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 48px;
    padding: 8px 8px 8px 12px;
    border-bottom: 1px solid var(--border);
  }

  .group {
    display: flex;
    gap: 2px;
  }

  .icon {
    flex: none;
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    border-radius: var(--radius-m);
    color: var(--text-2);
    transition:
      background var(--fast) var(--ease),
      color var(--fast) var(--ease),
      opacity var(--fast) var(--ease);
  }

  .icon:hover:not(:disabled) {
    background: var(--hover);
    color: var(--text);
  }

  .icon:active:not(:disabled) {
    background: var(--active);
  }

  .icon:disabled {
    opacity: 0.35;
  }

  .icon.on {
    color: var(--accent);
    background: var(--accent-soft);
  }

  .spin {
    display: grid;
  }

  .spin.spinning {
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .filter {
    flex: 0 1 220px;
    min-width: 120px;
    height: 32px;
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 0 10px;
    border-radius: var(--radius-m);
    background: var(--field);
    color: var(--text-3);
    cursor: text;
    transition:
      background var(--fast) var(--ease),
      box-shadow var(--fast) var(--ease);
  }

  .filter:focus-within {
    background: var(--field-focus);
    box-shadow: 0 0 0 2px var(--accent);
  }

  .filter input {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: 0;
    background: transparent;
    color: var(--text);
    user-select: text;
    -webkit-user-select: text;
  }

  .filter input::placeholder {
    color: var(--text-3);
  }
</style>
