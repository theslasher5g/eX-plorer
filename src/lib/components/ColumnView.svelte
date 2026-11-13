<script lang="ts">
  import { tick, untrack } from 'svelte';
  import { formatCount } from '../format';
  import { t } from '../i18n';
  import { ops } from '../stores/ops.svelte';
  import type { Pane } from '../stores/pane.svelte';
  import ColumnList from './ColumnList.svelte';
  import ColumnPreview from './ColumnPreview.svelte';
  import ListState from './ListState.svelte';

  let { pane }: { pane: Pane } = $props();

  const cols = $derived(pane.columns);
  const all = $derived(cols.all);
  let strip = $state<HTMLDivElement>();

  // Öffnet bzw. schließt Spalten, sobald sich eine Auswahl ändert.
  $effect(() => {
    void cols.watchKey();
    untrack(() => cols.reconcile());
  });

  // Nach „Rechts“ den ersten Eintrag der neuen Spalte markieren, sobald er da ist.
  $effect(() => {
    if (!cols.wantFirst) return;
    const column = cols.active;
    if (column.loading) return;
    untrack(() => {
      cols.wantFirst = false;
      if (column.visible.length > 0) column.select(0);
    });
  });

  // Neue Spalten rechts ins Bild holen.
  $effect(() => {
    void cols.rest.length;
    void tick().then(() => {
      if (strip) strip.scrollLeft = strip.scrollWidth;
    });
  });

  function onKey(column: Pane, event: KeyboardEvent, page: number) {
    if (event.altKey) return;
    const items = column.visible;
    const cursor = column.cursor;
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
      case 'ArrowRight':
        cols.enter();
        break;
      case 'ArrowLeft':
        cols.leave();
        break;
      case 'Enter': {
        const entry = items[cursor];
        if (entry) void ops.open(column, entry);
        break;
      }
      case 'Escape':
        column.clearSelection();
        break;
      case 'a':
      case 'A':
        if (!event.ctrlKey) return;
        column.selectAll();
        break;
      default:
        return;
    }
    event.preventDefault();
    if (target !== null) column.moveCursor(target, event.shiftKey);
  }

  /** Nach Pfeiltasten bleibt der Fokus in der (ggf. neuen) aktiven Spalte. */
  $effect(() => {
    const index = cols.activeIndex;
    const el = strip?.querySelectorAll<HTMLElement>('.col')[index];
    if (el && !el.contains(document.activeElement) && strip?.contains(document.activeElement)) el.focus();
  });
</script>

<div class="cols" bind:this={strip}>
  {#if pane.error}
    <ListState {pane} />
  {:else}
    {#each all as column, i (column.id)}
      <ColumnList
        pane={column}
        active={i === cols.activeIndex}
        onfocus={() => cols.activate(i)}
        onkeydown={(event, page) => onKey(column, event, page)}
      />
    {/each}
    {#if cols.preview}
      <ColumnPreview pane={cols.preview.column} entry={cols.preview.entry} />
    {:else if cols.multi}
      <p class="multi">{t('columns.multi', { count: formatCount(cols.multi.selected.size) })}</p>
    {/if}
  {/if}
</div>

<style>
  .cols {
    flex: 1;
    min-height: 0;
    display: flex;
    overflow-x: auto;
    overflow-y: hidden;
  }

  .multi {
    flex: 1;
    min-width: 240px;
    margin: 0;
    padding: 28px 24px;
    text-align: center;
    color: var(--muted);
    font-size: 13px;
  }
</style>
