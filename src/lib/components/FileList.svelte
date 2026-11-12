<script lang="ts">
  import { ChevronDown, CircleAlert, Folder, Lock, Search } from '@lucide/svelte';
  import type { Entry } from '../api';
  import { typeLabel } from '../filetypes';
  import { formatDate, formatDateLong, formatSize } from '../format';
  import { errorText, t, type MessageKey } from '../i18n';
  import type { Pane, SortKey } from '../stores/pane.svelte';
  import { joinPath } from '../paths';
  import { dnd } from '../stores/dnd.svelte';
  import { ops } from '../stores/ops.svelte';
  import FileIcon from './FileIcon.svelte';
  import RenameBox from './RenameBox.svelte';

  let { pane }: { pane: Pane } = $props();

  // Feste Maße: Grundlage der Virtualisierung.
  const ROW = 34;
  const HEAD = 34;
  const PAD_TOP = 6;
  const PAD_BOTTOM = 12;
  const OVERSCAN = 10;

  const columns: { key: SortKey; label: MessageKey; right?: boolean }[] = [
    { key: 'name', label: 'col.name' },
    { key: 'modified', label: 'col.modified' },
    { key: 'size', label: 'col.size', right: true },
    { key: 'type', label: 'col.type' },
  ];

  let viewport = $state<HTMLDivElement>();
  let scrollTop = $state(0);
  let height = $state(0);

  const items = $derived(pane.visible);
  const start = $derived(Math.max(0, Math.floor((scrollTop - PAD_TOP) / ROW) - OVERSCAN));
  const end = $derived(Math.min(items.length, Math.ceil((scrollTop + height) / ROW) + OVERSCAN));
  const rows = $derived(items.slice(start, end));
  const filter = $derived(pane.filter.trim());

  const errorTitle = $derived.by(() => {
    const code = pane.error?.code;
    if (code === 'access_denied' || code === 'not_found') {
      return t(`error.title.${code}`, { name: pane.title || pane.path });
    }
    return t('error.title');
  });

  // Programmatisch gesetzte Auswahl (neuer Ordner, Einfügen, Umbenennen) ins Bild holen.
  $effect(() => {
    void pane.scrollSignal;
    if (pane.cursor >= 0) reveal(pane.cursor);
  });

  // Neuer Ordner: wieder oben anfangen.
  $effect(() => {
    void pane.revision;
    if (viewport) viewport.scrollTop = 0;
    scrollTop = 0;
  });

  function reveal(index: number) {
    if (!viewport || index < 0) return;
    const top = PAD_TOP + index * ROW;
    const visible = height - HEAD;
    if (top < viewport.scrollTop) viewport.scrollTop = Math.max(0, top - PAD_TOP);
    else if (top + ROW > viewport.scrollTop + visible) {
      // Beim letzten Eintrag den unteren Rand mit zeigen.
      const extra = index === items.length - 1 ? PAD_BOTTOM : 0;
      viewport.scrollTop = top + ROW - visible + extra;
    }
  }

  // Ein Zug mit gedrückter Maustaste über der Schwelle startet einen echten
  // Drag (OLE) in andere Programme oder auf Ordner, Tabs und Seitenleiste.
  const DRAG_THRESHOLD = 6;
  let press: { x: number; y: number; index: number } | null = null;
  let suppressClickUntil = 0;

  function endPress() {
    press = null;
    window.removeEventListener('pointermove', onPressMove);
    window.removeEventListener('pointerup', endPress);
  }

  function onPressMove(event: PointerEvent) {
    if (!press) return;
    if ((event.buttons & 1) === 0) return endPress();
    if (Math.hypot(event.clientX - press.x, event.clientY - press.y) < DRAG_THRESHOLD) return;
    const { index } = press;
    endPress();
    beginDrag(index);
  }

  function beginDrag(index: number) {
    const entry = items[index];
    if (!entry) return;
    if (!pane.selected.has(entry.name)) pane.select(index);
    suppressClickUntil = performance.now() + 400;
    void dnd.start(ops.selectedPaths(pane));
  }

  function onRowPointerDown(event: PointerEvent, index: number) {
    if (event.button !== 0 || pane.renaming) return;
    endPress();
    press = { x: event.clientX, y: event.clientY, index };
    window.addEventListener('pointermove', onPressMove);
    window.addEventListener('pointerup', endPress);
  }

  function onRowClick(event: MouseEvent, index: number) {
    if (performance.now() < suppressClickUntil) return;
    pane.select(index, { toggle: event.ctrlKey, range: event.shiftKey });
  }

  function onRowDblClick(event: MouseEvent, entry: Entry) {
    if (!event.ctrlKey && !event.shiftKey) void ops.open(pane, entry);
  }

  // Rechtsklick: Zeile markieren (wie im Explorer) und das native Menü zeigen.
  function onRowContext(event: MouseEvent, index: number) {
    event.preventDefault();
    const entry = items[index];
    if (entry && !pane.selected.has(entry.name)) pane.select(index);
    void ops.contextMenu(pane);
  }

  function onBackgroundContext(event: MouseEvent) {
    if ((event.target as Element).closest('.row')) return;
    event.preventDefault();
    pane.clearSelection();
    void ops.contextMenu(pane);
  }

  function onBackgroundClick(event: MouseEvent) {
    if (event.target === event.currentTarget) pane.clearSelection();
  }

  function onKeydown(event: KeyboardEvent) {
    const page = Math.max(1, Math.floor((height - HEAD) / ROW) - 1);
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
        if (entry) void ops.open(pane, entry);
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

<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div
  class="viewport"
  bind:this={viewport}
  bind:clientHeight={height}
  onscroll={() => (scrollTop = viewport?.scrollTop ?? 0)}
  onkeydown={onKeydown}
  oncontextmenu={onBackgroundContext}
  class:drop={dnd.isHover(pane.path) && !pane.error}
  data-drop={pane.error ? undefined : pane.path}
  tabindex="0"
  role="grid"
  aria-busy={pane.loading}
  aria-rowcount={items.length}
  aria-multiselectable="true"
>
  {#if pane.error}
    <div class="center">
      <div class="bigic err">
        {#if pane.error.code === 'access_denied'}
          <Lock size={38} strokeWidth={1.5} />
        {:else}
          <CircleAlert size={38} strokeWidth={1.5} />
        {/if}
      </div>
      <h2>{errorTitle}</h2>
      <p>{errorText(pane.error.code)}</p>
      <div class="actions">
        {#if pane.canBack}
          <button class="btn" onclick={() => pane.back()}>{t('error.back')}</button>
        {/if}
        <button class="btn pri" onclick={() => pane.refresh()}>{t('error.retry')}</button>
      </div>
    </div>
  {:else if items.length === 0}
    <div class="center">
      {#if pane.loading}
        <p>{t('list.loading')}</p>
      {:else if filter}
        <div class="bigic"><Search size={38} strokeWidth={1.5} /></div>
        <h2>{t('list.noMatch', { filter })}</h2>
        <p>{t('list.noMatchHint')}</p>
        <div class="actions">
          <button class="btn" onclick={() => (pane.filter = '')}>{t('list.resetFilter')}</button>
        </div>
      {:else}
        <div class="bigic"><Folder size={38} strokeWidth={1.5} /></div>
        <h2>{t('list.empty')}</h2>
        <p>{t('list.emptyHint')}</p>
      {/if}
    </div>
  {:else}
    <div class="lgrid lhead" role="row">
      {#each columns as column (column.key)}
        <button class="hc" class:r={column.right} role="columnheader" onclick={() => pane.setSort(column.key)}>
          {t(column.label)}
          {#if pane.sortKey === column.key}
            <span class="dir" class:up={pane.sortAsc}><ChevronDown size={16} strokeWidth={1.7} /></span>
          {/if}
        </button>
      {/each}
    </div>

    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div
      class="spacer"
      style:height="{items.length * ROW + PAD_TOP + PAD_BOTTOM}px"
      role="presentation"
      onclick={onBackgroundClick}
    >
      {#key pane.revision}
        <div class="window" style:transform="translateY({start * ROW}px)" role="presentation">
          {#each rows as entry, i (entry.name)}
            {@const index = start + i}
            <!-- Der Fokus liegt immer auf dem Raster, nie auf einzelnen Zeilen. -->
            <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions, a11y_interactive_supports_focus -->
            <div
              class="row lgrid"
              class:sel={pane.selected.has(entry.name)}
              class:cursor={pane.cursor === index}
              class:dim={entry.hidden || ops.isCut(joinPath(pane.path, entry.name))}
              class:drop={entry.isDir && dnd.isHover(joinPath(pane.path, entry.name))}
              data-drop={entry.isDir ? joinPath(pane.path, entry.name) : undefined}
              role="row"
              aria-rowindex={index + 1}
              aria-selected={pane.selected.has(entry.name)}
              onpointerdown={(event) => onRowPointerDown(event, index)}
              oncontextmenu={(event) => onRowContext(event, index)}
              onclick={(event) => onRowClick(event, index)}
              ondblclick={(event) => onRowDblClick(event, entry)}
            >
              <span class="c-name" role="gridcell">
                <FileIcon {entry} dir={pane.path} />
                {#if pane.renaming === entry.name}
                  <RenameBox
                    name={entry.name}
                    isDir={entry.isDir}
                    oncommit={(value) => ops.rename(pane, entry.name, value)}
                    oncancel={() => (pane.renaming = null)}
                  />
                {:else}
                  <span class="nm" title={entry.name}>{entry.name}</span>
                {/if}
              </span>
              <span class="c-meta" role="gridcell" title={formatDateLong(entry.modifiedMs)}>
                {formatDate(entry.modifiedMs)}
              </span>
              <span class="c-meta c-size" role="gridcell">{entry.isDir ? '—' : formatSize(entry.size)}</span>
              <span class="c-meta" role="gridcell">{typeLabel(entry)}</span>
            </div>
          {/each}
        </div>
      {/key}
    </div>
  {/if}
</div>

<style>
  .viewport {
    flex: 1;
    min-height: 0;
    position: relative;
    overflow-y: auto;
    overflow-x: hidden;
    outline: none;
  }

  .lgrid {
    display: grid;
    grid-template-columns: minmax(200px, 1fr) 160px 90px 210px;
    align-items: center;
    column-gap: 12px;
  }

  .lhead {
    position: sticky;
    top: 0;
    z-index: 1;
    height: 34px;
    padding: 0 24px;
    border-bottom: 1px solid var(--border);
    background: var(--surface);
    font-size: 12px;
  }

  .hc {
    justify-self: start;
    display: flex;
    align-items: center;
    gap: 4px;
    height: 26px;
    padding: 0 6px;
    margin-left: -6px;
    border-radius: var(--r-sm);
    font-weight: 600;
    color: var(--muted);
    white-space: nowrap;
    transition:
      background var(--t-hover),
      color var(--t-hover);
  }

  .hc:hover {
    background: var(--hover);
    color: var(--text);
  }

  .hc.r {
    justify-self: end;
    flex-direction: row-reverse;
    margin: 0 -6px 0 0;
  }

  .dir {
    display: flex;
    transition: transform var(--t-select);
  }

  .dir.up {
    transform: rotate(180deg);
  }

  .spacer {
    position: relative;
    min-height: calc(100% - 34px);
  }

  .window {
    position: absolute;
    inset: 6px 12px auto;
    will-change: transform;
    animation: swap var(--t-expand) ease-out;
  }

  @keyframes swap {
    from {
      opacity: 0;
      margin-top: 4px;
    }
  }

  .row {
    height: 34px;
    padding: 0 12px;
    border-radius: var(--r-btn);
    outline: none;
    transition: background 120ms;
  }

  .row:hover {
    background: var(--hover);
  }

  .row.sel {
    background: var(--sel);
  }

  .viewport:focus-visible .row.cursor {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }

  .c-name {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }

  .nm,
  .c-meta {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .c-meta {
    color: var(--muted);
    font-size: 13px;
  }

  .c-size {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  .row.dim .c-name {
    opacity: 0.55;
  }

  .row.drop {
    border-radius: var(--r-btn);
  }

  .viewport.drop {
    box-shadow: inset 0 0 0 2px var(--accent);
    background: var(--accent-soft);
  }

  .center {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    padding: 32px;
    text-align: center;
    animation: swap var(--t-expand) ease-out;
  }

  .center h2 {
    margin: 6px 0 0;
    font-size: 18px;
    font-weight: 600;
    overflow-wrap: anywhere;
  }

  .center p {
    margin: 0;
    max-width: 340px;
    color: var(--muted);
  }

  .actions {
    display: flex;
    gap: 8px;
    margin-top: 8px;
  }

  .bigic {
    width: 84px;
    height: 84px;
    border-radius: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--surface-2);
    color: var(--muted);
  }

  .bigic.err {
    background: var(--err-bg);
    color: var(--err);
  }
</style>
