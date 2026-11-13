<script lang="ts">
  import { formatSize } from '../format';
  import { createItemActions } from '../itemActions';
  import { joinPath } from '../paths';
  import { dnd } from '../stores/dnd.svelte';
  import { onedrive } from '../stores/onedrive.svelte';
  import { ops } from '../stores/ops.svelte';
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
  const rowCount = $derived(Math.ceil(items.length / cols));
  const firstRow = $derived(Math.max(0, Math.floor((scrollTop - PAD_TOP) / rowH) - OVERSCAN));
  const lastRow = $derived(Math.min(rowCount, Math.ceil((scrollTop + height) / rowH) + OVERSCAN));
  const start = $derived(firstRow * cols);
  const shown = $derived(items.slice(start, lastRow * cols));

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
    const top = PAD_TOP + Math.floor(index / cols) * rowH;
    if (top < viewport.scrollTop) viewport.scrollTop = Math.max(0, top - PAD_TOP);
    else if (top + rowH > viewport.scrollTop + height) viewport.scrollTop = top + rowH - height;
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
        target = cursor < 0 ? 0 : Math.min(items.length - 1, cursor + cols);
        break;
      case 'ArrowUp':
        target = cursor < 0 ? 0 : cursor - cols < 0 ? cursor : cursor - cols;
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
      style:height="{rowCount * rowH + PAD_TOP + PAD_BOTTOM - GAP_Y}px"
      role="presentation"
      onclick={actions.backgroundClick}
    >
      {#key pane.revision}
        <div
          class="window"
          style:transform="translateY({firstRow * rowH}px)"
          style:grid-template-columns="repeat({cols}, minmax(0, 1fr))"
          style:grid-auto-rows="{rowH - GAP_Y}px"
          role="presentation"
        >
          {#each shown as entry, i (entry.name)}
            {@const index = start + i}
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
                  name={entry.name}
                  isDir={entry.isDir}
                  oncommit={(value) => ops.rename(pane, entry.name, value)}
                  oncancel={() => (pane.renaming = null)}
                />
              {:else}
                <span class="tname" title={entry.name}>{entry.name}</span>
              {/if}
              <span class="tmeta">{entry.isDir ? '' : formatSize(entry.size)}</span>
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

  .viewport.drop {
    box-shadow: inset 0 0 0 2px var(--accent);
    background: var(--accent-soft);
  }

  .spacer {
    position: relative;
    min-height: 100%;
  }

  .window {
    position: absolute;
    inset: 20px 24px auto;
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
