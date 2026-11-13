<script lang="ts">
  import { displayName, leafName, locationOf } from '../names';
  import { ChevronDown } from '@lucide/svelte';
  import { onDestroy } from 'svelte';
  import { Lasso } from '../lasso.svelte';
  import { handleTypeAhead } from '../typeahead';
  import type { Entry } from '../api';
  import { typeLabel } from '../filetypes';
  import { formatDate, formatDateLong, formatSize } from '../format';
  import { t, type MessageKey } from '../i18n';
  import type { Pane, SortKey } from '../stores/pane.svelte';
  import { joinPath } from '../paths';
  import { dnd } from '../stores/dnd.svelte';
  import { ops } from '../stores/ops.svelte';
  import { onedrive } from '../stores/onedrive.svelte';
  import FileIcon from './FileIcon.svelte';
  import SyncBadge from './SyncBadge.svelte';
  import ListState from './ListState.svelte';
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
  const groups = $derived(pane.groups);

  // Zeilenmodell: ohne Gruppen ist Zeile = Eintrag; mit Gruppen steht vor jeder
  // Gruppe eine Kopfzeile (negativer Wert -(Gruppe+1)), sonst der Eintragsindex.
  const model = $derived.by(() => {
    if (groups.length === 0) return null;
    const rows = new Int32Array(items.length + groups.length);
    let r = 0;
    groups.forEach((group, g) => {
      rows[r++] = -(g + 1);
      for (let i = 0; i < group.count; i++) rows[r++] = group.start + i;
    });
    return rows;
  });
  const rowCount = $derived(model ? model.length : items.length);

  /** Zeile eines Eintrags (Kopfzeilen davor mitgezählt). */
  function rowOf(index: number): number {
    if (groups.length === 0) return index;
    let low = 0;
    let high = groups.length - 1;
    while (low < high) {
      const mid = (low + high + 1) >> 1;
      if (groups[mid].start <= index) low = mid;
      else high = mid - 1;
    }
    return index + low + 1;
  }

  const start = $derived(Math.max(0, Math.floor((scrollTop - PAD_TOP) / ROW) - OVERSCAN));
  const end = $derived(Math.min(rowCount, Math.ceil((scrollTop + height) / ROW) + OVERSCAN));
  const slice = $derived.by(() => {
    const out: { key: string; index: number; group: number }[] = [];
    for (let r = start; r < end; r++) {
      const v = model ? model[r] : r;
      if (v < 0) out.push({ key: `g:${groups[-v - 1].id}`, index: -1, group: -v - 1 });
      else out.push({ key: `e:${items[v].name}`, index: v, group: -1 });
    }
    return out;
  });

  const lasso = new Lasso({
    container: () => viewport,
    pane: () => pane,
    hit(rect) {
      const first = Math.max(0, Math.floor((rect.top - HEAD - PAD_TOP) / ROW));
      const last = Math.min(rowCount - 1, Math.floor((rect.bottom - HEAD - PAD_TOP) / ROW));
      const out: number[] = [];
      for (let r = first; r <= last; r++) {
        const v = model ? model[r] : r;
        if (v >= 0) out.push(v);
      }
      return out;
    },
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
    const top = PAD_TOP + rowOf(index) * ROW;
    const visible = height - HEAD;
    if (top < viewport.scrollTop) viewport.scrollTop = Math.max(0, top - PAD_TOP);
    else if (top + ROW > viewport.scrollTop + visible) {
      // Beim letzten Eintrag den unteren Rand mit zeigen.
      const extra = index === items.length - 1 ? PAD_BOTTOM : 0;
      viewport.scrollTop = top + ROW - visible + extra;
    }
  }

  // Beim Verschwinden der Ansicht (Ansichtswechsel mitten im Klicken/Ziehen) alles aufräumen.
  onDestroy(() => {
    endPress();
    lasso.destroy();
  });

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
    beginDrag(index, event);
  }

  function beginDrag(index: number, event: PointerEvent) {
    const entry = items[index];
    if (!entry) return;
    if (!pane.selected.has(entry.name)) pane.select(index);
    suppressClickUntil = performance.now() + 400;
    dnd.begin(ops.selectedPaths(pane), event);
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
    else pane.cursor = index;
    void ops.contextMenu(pane);
  }

  function onBackgroundContext(event: MouseEvent) {
    if ((event.target as Element).closest('.row')) return;
    event.preventDefault();
    // Menütaste / Umschalt+F10 (kein Rechtsklick): Menü der aktuellen Auswahl.
    if (event.button === 2) pane.clearSelection();
    void ops.contextMenu(pane);
  }

  function onBackgroundClick(event: MouseEvent) {
    if (event.target === event.currentTarget) pane.clearSelection();
  }

  function onKeydown(event: KeyboardEvent) {
    const typed = handleTypeAhead(event, pane);
    if (typed !== null) {
      if (typed >= 0) reveal(pane.cursor);
      return;
    }
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
  onpointerdown={lasso.pointerDown}
  oncontextmenu={onBackgroundContext}
  class:drop={dnd.isHover(pane.path) && !pane.error}
  data-drop={pane.error ? undefined : pane.path}
  tabindex="0"
  role="grid"
  aria-busy={pane.loading}
  aria-rowcount={items.length}
  aria-multiselectable="true"
>
  {#if pane.error || items.length === 0}
    <ListState {pane} />
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
      style:height="{rowCount * ROW + PAD_TOP + PAD_BOTTOM}px"
      role="presentation"
      onclick={(event) => lasso.consumedClick() || onBackgroundClick(event)}
    >
      {#key pane.revision}
        <div class="window" style:transform="translateY({start * ROW}px)" role="presentation">
          {#each slice as r (r.key)}
            {#if r.group >= 0}
              {@const group = groups[r.group]}
              <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
              <div
                class="ghead"
                role="presentation"
                title={t('group.selectAll', { count: group.count })}
                onclick={(event) => pane.selectGroup(group, event.ctrlKey)}
              >
                <b>{group.label}</b>
                <span>{group.count}</span>
              </div>
            {:else}
            {@const index = r.index}
            {@const entry = items[index]}
            {@const sync = onedrive.stateOf(entry, pane.path)}
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
                    name={leafName(entry)}
                    isDir={entry.isDir}
                    oncommit={(value) => ops.rename(pane, entry.name, value)}
                    oncancel={() => (pane.renaming = null)}
                  />
                {:else}
                  <span class="nm" title={entry.name}>{displayName(entry)}</span>
                  {#if locationOf(entry)}<span class="where" title={locationOf(entry)}>{locationOf(entry)}</span>{/if}
                {/if}
              </span>
              <span class="c-meta" role="gridcell" title={formatDateLong(entry.modifiedMs)}>
                {formatDate(entry.modifiedMs)}
              </span>
              <span class="c-meta c-size" role="gridcell">{entry.isDir ? '—' : formatSize(entry.size)}</span>
              <span class="c-meta" role="gridcell">
                {#if sync}
                  <SyncBadge state={sync} />
                {:else}
                  {typeLabel(entry)}
                {/if}
              </span>
            </div>
            {/if}
          {/each}
        </div>
      {/key}
    </div>
    {#if lasso.rect}
      <div
        class="lasso"
        style:left="{lasso.rect.left}px"
        style:top="{lasso.rect.top}px"
        style:width="{lasso.rect.right - lasso.rect.left}px"
        style:height="{lasso.rect.bottom - lasso.rect.top}px"
      ></div>
    {/if}
  {/if}
</div>

<style>
  .viewport {
    user-select: none;
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

  .ghead {
    height: 34px;
    padding: 0 12px;
    display: flex;
    align-items: center;
    gap: 10px;
    border-bottom: 1px solid var(--border);
    font-size: 13px;
    user-select: none;
    cursor: default;
  }

  .ghead b {
    font-weight: 600;
    color: var(--accent);
  }

  .ghead span {
    color: var(--muted);
    font-size: 12px;
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

  .where {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12px;
    color: var(--muted);
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

</style>
