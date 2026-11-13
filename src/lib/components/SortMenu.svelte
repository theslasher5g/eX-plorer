<script lang="ts">
  import { ArrowDownWideNarrow, Check } from '@lucide/svelte';
  import { t, type MessageKey } from '../i18n';
  import { GROUP_KEYS } from '../grouping';
  import type { Pane, SortKey } from '../stores/pane.svelte';

  let { pane }: { pane: Pane } = $props();

  let open = $state(false);

  const keys: { key: SortKey; label: MessageKey }[] = [
    { key: 'name', label: 'col.name' },
    { key: 'modified', label: 'col.modified' },
    { key: 'size', label: 'col.size' },
    { key: 'type', label: 'col.type' },
  ];

  function onKeydown(event: KeyboardEvent) {
    if (!open || event.key !== 'Escape') return;
    open = false;
    event.preventDefault();
    event.stopPropagation();
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div class="anchor">
  <button
    class="icon-btn"
    title={t('sort.title')}
    aria-label={t('sort.title')}
    aria-haspopup="menu"
    aria-expanded={open}
    onclick={() => (open = !open)}
  >
    <ArrowDownWideNarrow size={18} strokeWidth={1.7} />
  </button>

  {#if open}
    <button class="back" aria-label={t('sort.title')} tabindex="-1" onclick={() => (open = false)}></button>
    <div class="menu" role="menu">
      {#each keys as item (item.key)}
        <button
          class="mi"
          role="menuitemradio"
          aria-checked={pane.sortKey === item.key}
          onclick={() => {
            if (pane.sortKey !== item.key) pane.setSort(item.key);
            open = false;
          }}
        >
          <span class="check" class:off={pane.sortKey !== item.key}><Check size={16} strokeWidth={1.7} /></span>
          {t(item.label)}
        </button>
      {/each}
      <div class="msep"></div>
      {#each [true, false] as ascending (ascending)}
        <button
          class="mi"
          role="menuitemradio"
          aria-checked={pane.sortAsc === ascending}
          onclick={() => {
            pane.setSortDirection(ascending);
            open = false;
          }}
        >
          <span class="check" class:off={pane.sortAsc !== ascending}><Check size={16} strokeWidth={1.7} /></span>
          {t(ascending ? 'sort.ascending' : 'sort.descending')}
        </button>
      {/each}
      {#if pane.view === 'list' || pane.view === 'grid'}
        <div class="msep"></div>
        <div class="mh">{t('group.title')}</div>
        {#each GROUP_KEYS as key (key)}
          <button
            class="mi"
            role="menuitemradio"
            aria-checked={pane.groupKey === key}
            onclick={() => {
              pane.setGroup(key);
              open = false;
            }}
          >
            <span class="check" class:off={pane.groupKey !== key}><Check size={16} strokeWidth={1.7} /></span>
            {t(`group.${key}`)}
          </button>
        {/each}
        {#if pane.groupKey !== 'none'}
          {#each [true, false] as ascending (ascending)}
            <button
              class="mi"
              role="menuitemradio"
              aria-checked={pane.groupAsc === ascending}
              onclick={() => {
                pane.setGroupDirection(ascending);
                open = false;
              }}
            >
              <span class="check" class:off={pane.groupAsc !== ascending}><Check size={16} strokeWidth={1.7} /></span>
              {t(ascending ? 'group.ascending' : 'group.descending')}
            </button>
          {/each}
        {/if}
      {/if}
      <div class="msep"></div>
      <button
        class="mi"
        role="menuitemcheckbox"
        aria-checked={pane.showHidden}
        onclick={() => {
          pane.showHidden = !pane.showHidden;
          open = false;
        }}
      >
        <span class="check" class:off={!pane.showHidden}><Check size={16} strokeWidth={1.7} /></span>
        {t('sort.showHidden')}
      </button>
    </div>
  {/if}
</div>

<style>
  .anchor {
    position: relative;
    flex: none;
  }

  .back {
    position: fixed;
    inset: 0;
    z-index: 40;
    cursor: default;
  }

  .menu {
    position: absolute;
    z-index: 41;
    top: 40px;
    right: 0;
    min-width: 230px;
    padding: 6px;
    border-radius: var(--r-menu);
    background: var(--surface);
    box-shadow:
      var(--shadow-md),
      0 0 0 1px var(--border-strong);
    transform-origin: top right;
    animation: pop var(--t-hover) ease-out;
  }

  @keyframes pop {
    from {
      opacity: 0;
      transform: scale(0.96);
    }
  }

  .mi {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    height: 32px;
    padding: 0 10px;
    border-radius: var(--r-sm);
    font-size: 13.5px;
    white-space: nowrap;
  }

  .mi:hover {
    background: var(--hover);
  }

  .check {
    display: flex;
    color: var(--muted);
  }

  .check.off {
    visibility: hidden;
  }

  .mh {
    padding: 6px 10px 2px;
    font-size: 11.5px;
    font-weight: 600;
    color: var(--muted);
  }

  .msep {
    height: 1px;
    margin: 5px 6px;
    background: var(--border);
  }
</style>
