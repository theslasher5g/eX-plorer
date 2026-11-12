<script lang="ts">
  import { ChevronDown, ChevronUp, CircleAlert, File, Folder, FolderSymlink } from '@lucide/svelte';
  import type { Entry } from '../api';
  import { formatDate, formatSize, typeLabel } from '../format';
  import { errorText, t, type MessageKey } from '../i18n';
  import type { Pane, SortKey } from '../stores/pane.svelte';

  let { pane }: { pane: Pane } = $props();

  // Feste Zeilenhöhe: Grundlage der Virtualisierung.
  const ROW = 32;
  // Höhe der Kopfzeile, die oben im Scrollbereich haftet.
  const HEAD = 30;
  const OVERSCAN = 10;

  const columns: { key: SortKey; label: MessageKey; numeric?: boolean }[] = [
    { key: 'name', label: 'col.name' },
    { key: 'modified', label: 'col.modified' },
    { key: 'type', label: 'col.type' },
    { key: 'size', label: 'col.size', numeric: true },
  ];

  let viewport = $state<HTMLDivElement>();
  let scrollTop = $state(0);
  let height = $state(0);

  const items = $derived(pane.visible);
  const start = $derived(Math.max(0, Math.floor(scrollTop / ROW) - OVERSCAN));
  const end = $derived(Math.min(items.length, Math.ceil((scrollTop + height) / ROW) + OVERSCAN));
  const rows = $derived(items.slice(start, end));

  // Neuer Ordner: wieder oben anfangen.
  $effect(() => {
    void pane.revision;
    if (viewport) viewport.scrollTop = 0;
    scrollTop = 0;
  });

  function reveal(index: number) {
    if (!viewport || index < 0) return;
    const top = index * ROW;
    if (top < viewport.scrollTop) viewport.scrollTop = top;
    else if (top + ROW > viewport.scrollTop + height - HEAD) {
      viewport.scrollTop = top + ROW - height + HEAD;
    }
  }

  function onRowClick(event: MouseEvent, index: number) {
    pane.select(index, { toggle: event.ctrlKey, range: event.shiftKey });
  }

  function onRowDblClick(event: MouseEvent, entry: Entry) {
    if (!event.ctrlKey && !event.shiftKey) pane.open(entry);
  }

  function onBackgroundClick(event: MouseEvent) {
    if (event.target === event.currentTarget) pane.clearSelection();
  }

  function onKeydown(event: KeyboardEvent) {
    const page = Math.max(1, Math.floor(height / ROW) - 1);
    const cursor = pane.cursor;
    let target: number | null = null;

    switch (event.key) {
      case 'ArrowDown':
        target = cursor + 1;
        break;
      case 'ArrowUp':
        target = cursor < 0 ? 0 : cursor - 1;
        break;
      case 'PageDown':
        target = cursor + page;
        break;
      case 'PageUp':
        target = cursor - page;
        break;
      case 'Home':
        target = 0;
        break;
      case 'End':
        target = items.length - 1;
        break;
      case 'Enter': {
        const entry = items[cursor];
        if (entry) pane.open(entry);
        break;
      }
      case 'Escape':
        pane.clearSelection();
        break;
      case 'a':
      case 'A':
        if (!event.ctrlKey) return;
        pane.selectAll();
        break;
      default:
        return;
    }

    event.preventDefault();
    if (target !== null && !event.altKey) {
      pane.moveCursor(target, event.shiftKey);
      reveal(pane.cursor);
    }
  }
</script>

<section class="list" aria-busy={pane.loading}>
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <div
    class="viewport"
    bind:this={viewport}
    bind:clientHeight={height}
    onscroll={() => (scrollTop = viewport?.scrollTop ?? 0)}
    onkeydown={onKeydown}
    tabindex="0"
    role="grid"
    aria-rowcount={items.length}
    aria-multiselectable="true"
  >
    <div class="head" role="row">
      {#each columns as column (column.key)}
        <button
          class="th"
          class:numeric={column.numeric}
          class:sorted={pane.sortKey === column.key}
          onclick={() => pane.setSort(column.key)}
        >
          <span>{t(column.label)}</span>
          {#if pane.sortKey === column.key}
            {#if pane.sortAsc}
              <ChevronUp size={12} strokeWidth={2} />
            {:else}
              <ChevronDown size={12} strokeWidth={2} />
            {/if}
          {/if}
        </button>
      {/each}
    </div>

    {#if pane.error}
      <div class="message">
        <CircleAlert size={30} strokeWidth={1.5} />
        <strong>{t('error.title')}</strong>
        <span>{errorText(pane.error.code)}</span>
        <button class="retry" onclick={() => pane.refresh()}>{t('error.retry')}</button>
      </div>
    {:else if items.length === 0}
      <div class="message">
        {#if pane.loading}
          <span>{t('list.loading')}</span>
        {:else if pane.filter.trim()}
          <span>{t('list.noMatch', { filter: pane.filter.trim() })}</span>
        {:else}
          <span>{t('list.empty')}</span>
        {/if}
      </div>
    {:else}
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <div
        class="spacer"
        style:height="{items.length * ROW}px"
        role="presentation"
        onclick={onBackgroundClick}
      >
        <div class="window" style:transform="translateY({start * ROW}px)" role="presentation">
          {#each rows as entry, i (entry.name)}
            {@const index = start + i}
            <div
              class="row"
              class:selected={pane.selected.has(entry.name)}
              class:cursor={pane.cursor === index}
              class:dim={entry.hidden}
              role="row"
              tabindex="-1"
              aria-rowindex={index + 1}
              aria-selected={pane.selected.has(entry.name)}
              onclick={(event) => onRowClick(event, index)}
              ondblclick={(event) => onRowDblClick(event, entry)}
            >
              <div class="cell name" role="gridcell">
                {#if entry.isDir && entry.isLink}
                  <span class="glyph folder"><FolderSymlink size={17} strokeWidth={1.6} /></span>
                {:else if entry.isDir}
                  <span class="glyph folder"><Folder size={17} strokeWidth={1.6} fill="currentColor" fill-opacity="0.22" /></span>
                {:else}
                  <span class="glyph"><File size={17} strokeWidth={1.4} /></span>
                {/if}
                <span class="text" title={entry.name}>{entry.name}</span>
              </div>
              <div class="cell" role="gridcell">{formatDate(entry.modifiedMs)}</div>
              <div class="cell" role="gridcell">{typeLabel(entry)}</div>
              <div class="cell numeric" role="gridcell">{entry.isDir ? '' : formatSize(entry.size)}</div>
            </div>
          {/each}
        </div>
      </div>
    {/if}
  </div>
</section>

<style>
  .list {
    --cols: minmax(180px, 1fr) 150px 150px 96px;
    display: grid;
    grid-template-rows: minmax(0, 1fr);
    min-height: 0;
  }

  .head,
  .row {
    display: grid;
    grid-template-columns: var(--cols);
    column-gap: 12px;
    align-items: center;
  }

  .head {
    position: sticky;
    top: 0;
    z-index: 1;
    height: 30px;
    padding: 0 20px;
    background: var(--surface);
  }

  .th {
    display: flex;
    align-items: center;
    gap: 4px;
    height: 100%;
    min-width: 0;
    font-size: 12px;
    color: var(--text-2);
    text-align: left;
    transition: color var(--fast) var(--ease);
  }

  .th:hover,
  .th.sorted {
    color: var(--text);
  }

  .th.numeric {
    justify-content: flex-end;
  }

  .viewport {
    overflow-y: scroll;
    overflow-x: hidden;
    outline: none;
    position: relative;
  }

  .spacer {
    position: relative;
    min-height: calc(100% - 30px);
  }

  .window {
    position: absolute;
    inset: 0 0 auto 0;
    padding: 0 8px;
    will-change: transform;
  }

  .row {
    height: 32px;
    padding: 0 12px;
    border-radius: var(--radius-s);
    outline: none;
  }

  .row:hover {
    background: var(--hover);
  }

  .row.selected {
    background: var(--accent-soft);
  }

  .row.selected:hover {
    background: var(--accent-soft-strong);
  }

  .viewport:focus-visible .row.cursor {
    box-shadow: inset 0 0 0 1.5px var(--accent);
  }

  .cell {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-2);
  }

  .cell.name {
    display: flex;
    align-items: center;
    gap: 9px;
    color: var(--text);
  }

  .cell.numeric {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  .text {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .glyph {
    flex: none;
    display: grid;
    color: var(--text-3);
  }

  .glyph.folder {
    color: var(--folder);
  }

  .row.dim .name,
  .row.dim .glyph {
    opacity: 0.55;
  }

  .message {
    position: absolute;
    inset: 30px 0 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 24px;
    text-align: center;
    color: var(--text-2);
  }

  .message strong {
    margin-top: 6px;
    font-family: var(--font-display);
    font-size: 15px;
    font-weight: 600;
    color: var(--text);
  }

  .retry {
    margin-top: 12px;
    padding: 6px 14px;
    border-radius: var(--radius-m);
    background: var(--field);
    color: var(--text);
    transition: background var(--fast) var(--ease);
  }

  .retry:hover {
    background: var(--active);
  }
</style>
