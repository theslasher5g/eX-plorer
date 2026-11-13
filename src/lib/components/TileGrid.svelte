<script lang="ts">
  import { displayName, leafName } from '../names';
  import { formatSize } from '../format';
  import { Lasso } from '../lasso.svelte';
  import { t } from '../i18n';
  import { createItemActions } from '../itemActions';
  import { joinPath } from '../paths';
  import { dnd } from '../stores/dnd.svelte';
  import { onedrive } from '../stores/onedrive.svelte';
  import { ops } from '../stores/ops.svelte';
  import type { Entry } from '../api';
  import type { Group } from '../grouping';
  import type { Pane } from '../stores/pane.svelte';
  import { settings } from '../stores/settings.svelte';
  import ListState from './ListState.svelte';
  import RenameBox from './RenameBox.svelte';
  import Thumb from './Thumb.svelte';

  let { pane }: { pane: Pane } = $props();

  // Maße der virtualisierten Kachelfläche.
  const PAD_X = 24;
  const PAD_TOP = 20;
  const PAD_BOTTOM = 24;
  const GAP_X = 14;
  const GAP_Y = 18;
  /** Höhe unter dem Bild: Abstand, zwei Namenszeilen, Größe, Innenabstand. */
  const TEXT_H = 72;
  const OVERSCAN = 2;
  /** Höhe der Gruppenkopfzeile und Abstand zur nächsten Gruppe. */
  const HEAD_H = 40;
  const GROUP_GAP = 8;

  let viewport = $state<HTMLDivElement>();
  let scrollTop = $state(0);
  let height = $state(0);
  let width = $state(0);

  const items = $derived(pane.visible);
  const actions = createItemActions(
    () => pane,
    () => items,
  );

  const tile = $derived(settings.value.tileSize);
  const cols = $derived(Math.max(1, Math.floor((width - 2 * PAD_X + GAP_X) / (tile + GAP_X))));
  const tileW = $derived(Math.max(60, (width - 2 * PAD_X - (cols - 1) * GAP_X) / cols));
  const rowH = $derived(Math.round((tileW - 12) * 0.75) + TEXT_H + GAP_Y);
  const groups = $derived(pane.groups);
  const grouped = $derived(groups.length > 0);

  // Lage der Gruppen (ohne Gruppierung eine unsichtbare): Kopfzeile, dann Kachelzeilen.
  const layout = $derived.by(() => {
    const segments = grouped ? groups : [{ id: '', label: '', start: 0, count: items.length }];
    const head = grouped ? HEAD_H : 0;
    let y = PAD_TOP;
    const out = segments.map((group) => {
      const rows = Math.ceil(group.count / cols);
      const itemsTop = y + head;
      const bottom = itemsTop + rows * rowH;
      const segment = { group, top: y, itemsTop, rows, bottom };
      y = bottom + (grouped ? GROUP_GAP : 0);
      return segment;
    });
    return { segments: out, height: Math.max(0, y - (grouped ? GROUP_GAP : 0) - GAP_Y) + PAD_BOTTOM };
  });

  /** Die sichtbaren Gruppen mit den Kacheln, die gezeichnet werden. */
  const visibleSegments = $derived.by(() => {
    const margin = OVERSCAN * rowH;
    const out: {
      group: Group;
      top: number;
      head: boolean;
      gridTop: number;
      start: number;
      shown: Entry[];
    }[] = [];
    for (const seg of layout.segments) {
      if (seg.bottom < scrollTop - margin || seg.top > scrollTop + height + margin) continue;
      const first = Math.max(0, Math.floor((scrollTop - seg.itemsTop) / rowH) - OVERSCAN);
      const last = Math.min(seg.rows, Math.ceil((scrollTop + height - seg.itemsTop) / rowH) + OVERSCAN);
      const start = seg.group.start + first * cols;
      const end = Math.min(seg.group.start + seg.group.count, seg.group.start + last * cols);
      out.push({
        group: seg.group,
        top: seg.top,
        head: grouped,
        gridTop: seg.itemsTop + first * rowH,
        start,
        shown: end > start ? items.slice(start, end) : [],
      });
    }
    return out;
  });

  /** Gruppe und Lage eines Eintrags. */
  function locate(index: number) {
    for (const seg of layout.segments) {
      if (index >= seg.group.start && index < seg.group.start + seg.group.count) {
        const local = index - seg.group.start;
        return { seg, row: Math.floor(local / cols), col: local % cols };
      }
    }
    return null;
  }

  const lasso = new Lasso({
    container: () => viewport,
    pane: () => pane,
    hit(rect) {
      const out: number[] = [];
      const colFrom = Math.max(0, Math.floor((rect.left - PAD_X + GAP_X) / (tileW + GAP_X)));
      const colTo = Math.min(cols - 1, Math.floor((rect.right - PAD_X) / (tileW + GAP_X)));
      for (const seg of layout.segments) {
        if (seg.bottom < rect.top || seg.itemsTop > rect.bottom) continue;
        const rowFrom = Math.max(0, Math.floor((rect.top - seg.itemsTop) / rowH));
        const rowTo = Math.min(seg.rows - 1, Math.floor((rect.bottom - seg.itemsTop) / rowH));
        for (let row = rowFrom; row <= rowTo; row++) {
          // Der Zwischenraum unter einer Kachelzeile zählt nicht mit.
          const y0 = seg.itemsTop + row * rowH;
          if (rect.top > y0 + rowH - GAP_Y || rect.bottom < y0) continue;
          for (let col = colFrom; col <= colTo; col++) {
            const x0 = PAD_X + col * (tileW + GAP_X);
            if (rect.left > x0 + tileW || rect.right < x0) continue;
            const index = seg.group.start + row * cols + col;
            if (index < seg.group.start + seg.group.count) out.push(index);
          }
        }
      }
      return out;
    },
  });

  $effect(() => {
    void pane.revision;
    if (viewport) viewport.scrollTop = 0;
    scrollTop = 0;
  });

  $effect(() => {
    void pane.scrollSignal;
    if (pane.cursor >= 0) reveal(pane.cursor);
  });

  function reveal(index: number) {
    if (!viewport || index < 0) return;
    const at = locate(index);
    if (!at) return;
    const top = at.seg.itemsTop + at.row * rowH;
    if (top < viewport.scrollTop) viewport.scrollTop = Math.max(0, top - (grouped ? HEAD_H : PAD_TOP));
    else if (top + rowH > viewport.scrollTop + height) viewport.scrollTop = top + rowH - height;
  }

  /** Eine Zeile hoch/runter in derselben Spalte, auch über Gruppengrenzen. */
  function vertical(cursor: number, dir: 1 | -1): number {
    const at = locate(cursor);
    if (!at) return cursor;
    const { seg, row, col } = at;
    const g = seg.group;
    const index = layout.segments.indexOf(seg);
    if (dir === 1) {
      if ((row + 1) * cols < g.count) return g.start + Math.min(g.count - 1, (row + 1) * cols + col);
      const next = layout.segments[index + 1];
      return next ? next.group.start + Math.min(col, next.group.count - 1) : cursor;
    }
    if (row > 0) return g.start + (row - 1) * cols + col;
    const prev = layout.segments[index - 1];
    if (!prev) return cursor;
    const lastRow = Math.floor((prev.group.count - 1) / cols);
    return prev.group.start + Math.min(prev.group.count - 1, lastRow * cols + col);
  }

  function onKeydown(event: KeyboardEvent) {
    const cursor = pane.cursor;
    const page = Math.max(1, Math.floor(height / rowH)) * cols;
    let target: number | null = null;
    switch (event.key) {
      case 'ArrowRight':
        target = cursor + 1;
        break;
      case 'ArrowLeft':
        target = cursor < 0 ? 0 : cursor - 1;
        break;
      case 'ArrowDown':
        target = cursor < 0 ? 0 : vertical(cursor, 1);
        break;
      case 'ArrowUp':
        target = cursor < 0 ? 0 : vertical(cursor, -1);
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
  class:drop={dnd.isHover(pane.path) && !pane.error}
  data-drop={pane.error ? undefined : pane.path}
  bind:this={viewport}
  bind:clientHeight={height}
  bind:clientWidth={width}
  onscroll={() => (scrollTop = viewport?.scrollTop ?? 0)}
  onkeydown={onKeydown}
  onpointerdown={lasso.pointerDown}
  oncontextmenu={actions.backgroundContext}
  tabindex="0"
  role="listbox"
  aria-multiselectable="true"
  aria-busy={pane.loading}
>
  {#if pane.error || items.length === 0}
    <ListState {pane} />
  {:else}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div
      class="spacer"
      style:height="{layout.height}px"
      role="presentation"
      onclick={(event) => lasso.consumedClick() || actions.backgroundClick(event)}
    >
      {#key pane.revision}
        {#each visibleSegments as seg (seg.group.id)}
          {#if seg.head}
            <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
            <div
              class="ghead"
              role="presentation"
              style:top="{seg.top}px"
              style:height="{HEAD_H}px"
              title={t('group.selectAll', { count: seg.group.count })}
              onclick={(event) => pane.selectGroup(seg.group, event.ctrlKey)}
            >
              <b>{seg.group.label}</b>
              <span>{seg.group.count}</span>
            </div>
          {/if}
          <div
            class="window"
            style:transform="translateY({seg.gridTop}px)"
            style:grid-template-columns="repeat({cols}, minmax(0, 1fr))"
            style:grid-auto-rows="{rowH - GAP_Y}px"
            role="presentation"
          >
          {#each seg.shown as entry, i (entry.name)}
            {@const index = seg.start + i}
            {@const path = joinPath(pane.path, entry.name)}
            <!-- svelte-ignore a11y_click_events_have_key_events, a11y_interactive_supports_focus -->
            <div
              class="tile"
              class:sel={pane.selected.has(entry.name)}
              class:cursor={pane.cursor === index}
              class:dim={entry.hidden || ops.isCut(path)}
              class:drop={entry.isDir && dnd.isHover(path)}
              data-drop={entry.isDir ? path : undefined}
              role="option"
              aria-selected={pane.selected.has(entry.name)}
              onpointerdown={(event) => actions.pointerDown(event, index)}
              onclick={(event) => actions.click(event, index)}
              ondblclick={(event) => actions.dblClick(event, entry)}
              oncontextmenu={(event) => actions.context(event, index)}
            >
              <Thumb {entry} dir={pane.path} sync={onedrive.stateOf(entry, pane.path)} />
              {#if pane.renaming === entry.name}
                <RenameBox
                  name={leafName(entry)}
                  isDir={entry.isDir}
                  oncommit={(value) => ops.rename(pane, entry.name, value)}
                  oncancel={() => (pane.renaming = null)}
                />
              {:else}
                <span class="tname" title={entry.name}>{displayName(entry)}</span>
              {/if}
              <span class="tmeta">{entry.isDir ? '' : formatSize(entry.size)}</span>
            </div>
          {/each}
          </div>
        {/each}
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

  .viewport.drop {
    box-shadow: inset 0 0 0 2px var(--accent);
    background: var(--accent-soft);
  }

  .spacer {
    position: relative;
    min-height: 100%;
  }

  .ghead {
    position: absolute;
    left: 24px;
    right: 24px;
    display: flex;
    align-items: flex-end;
    gap: 10px;
    padding: 0 6px 6px;
    border-bottom: 1px solid var(--border);
    font-size: 13px;
    user-select: none;
  }

  .ghead b {
    font-weight: 600;
    color: var(--accent);
  }

  .ghead span {
    color: var(--muted);
    font-size: 12px;
  }

  .window {
    position: absolute;
    inset: 0 24px auto;
    display: grid;
    gap: 18px 14px;
    will-change: transform;
    animation: swap var(--t-expand) ease-out;
  }

  @keyframes swap {
    from {
      opacity: 0;
      margin-top: 4px;
    }
  }

  .tile {
    min-width: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 6px;
    border-radius: 12px;
    transition: background var(--t-hover);
  }

  .tile:hover {
    background: var(--hover);
  }

  .tile.sel :global(.thumb) {
    box-shadow: 0 0 0 2.5px var(--accent);
  }

  .tile.sel .tname {
    background: var(--accent);
    color: var(--on-accent);
  }

  .viewport:focus-visible .tile.cursor {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }

  .tile.dim :global(.thumb),
  .tile.dim .tname {
    opacity: 0.55;
  }

  .tname {
    max-width: 100%;
    padding: 2px 8px;
    border-radius: var(--r-sm);
    text-align: center;
    font-size: 13px;
    line-height: 1.25;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    overflow-wrap: anywhere;
  }

  .tmeta {
    min-height: 16px;
    margin-top: -4px;
    font-size: 12px;
    color: var(--muted);
  }

  .tile :global(.rename) {
    flex: none;
    width: 100%;
    text-align: center;
  }
</style>
