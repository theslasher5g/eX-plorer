<script lang="ts">
  import { ChevronRight } from '@lucide/svelte';
  import { errorText, t } from '../i18n';
  import { joinPath } from '../paths';
  import { createItemActions } from '../itemActions';
  import { Lasso } from '../lasso.svelte';
  import { dnd } from '../stores/dnd.svelte';
  import { onedrive } from '../stores/onedrive.svelte';
  import { ops } from '../stores/ops.svelte';
  import type { Pane } from '../stores/pane.svelte';
  import FileIcon from './FileIcon.svelte';
  import RenameBox from './RenameBox.svelte';
  import SyncBadge from './SyncBadge.svelte';

  let {
    pane,
    active,
    onfocus,
    onkeydown,
  }: {
    pane: Pane;
    /** Diese Spalte bekommt die Tastatur. */
    active: boolean;
    onfocus: () => void;
    onkeydown: (event: KeyboardEvent, page: number) => void;
  } = $props();

  // Feste Maße: Grundlage der Virtualisierung.
  const ROW = 32;
  const PAD = 8;
  const OVERSCAN = 10;

  let viewport = $state<HTMLDivElement>();
  let scrollTop = $state(0);
  let height = $state(0);

  const items = $derived(pane.visible);
  const start = $derived(Math.max(0, Math.floor((scrollTop - PAD) / ROW) - OVERSCAN));
  const end = $derived(Math.min(items.length, Math.ceil((scrollTop + height) / ROW) + OVERSCAN));
  const rows = $derived(items.slice(start, end));

  const actions = createItemActions(
    () => pane,
    () => items,
  );

  const lasso = new Lasso({
    container: () => viewport,
    pane: () => pane,
    hit(rect) {
      const first = Math.max(0, Math.floor((rect.top - PAD) / ROW));
      const last = Math.min(items.length - 1, Math.floor((rect.bottom - PAD) / ROW));
      const out: number[] = [];
      for (let i = first; i <= last; i++) out.push(i);
      return out;
    },
  });

  $effect(() => {
    void pane.scrollSignal;
    if (pane.cursor >= 0) reveal(pane.cursor);
  });

  $effect(() => {
    void pane.revision;
    if (viewport) viewport.scrollTop = 0;
    scrollTop = 0;
  });

  function reveal(index: number) {
    if (!viewport || index < 0) return;
    const top = PAD + index * ROW;
    if (top < viewport.scrollTop) viewport.scrollTop = Math.max(0, top - PAD);
    else if (top + ROW > viewport.scrollTop + height) viewport.scrollTop = top + ROW - height + PAD;
  }

  function handleKey(event: KeyboardEvent) {
    onkeydown(event, Math.max(1, Math.floor(height / ROW) - 1));
    if (event.defaultPrevented) queueMicrotask(() => reveal(pane.cursor));
  }

  function onBackgroundContext(event: MouseEvent) {
    if ((event.target as Element).closest('.crow')) return;
    actions.backgroundContext(event);
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div
  class="col"
  class:active
  class:drop={dnd.isHover(pane.path) && !pane.error}
  data-drop={pane.error ? undefined : pane.path}
  bind:this={viewport}
  bind:clientHeight={height}
  onscroll={() => (scrollTop = viewport?.scrollTop ?? 0)}
  onfocusin={onfocus}
  onpointerdown={(event) => {
    onfocus();
    lasso.pointerDown(event);
  }}
  onkeydown={handleKey}
  oncontextmenu={onBackgroundContext}
  tabindex="0"
  role="listbox"
  aria-label={pane.title}
  aria-busy={pane.loading}
  aria-multiselectable="true"
>
  {#if pane.error}
    <p class="note" role="alert">{errorText(pane.error.code)}</p>
  {:else if items.length === 0 && !pane.loading}
    <p class="note">{t('columns.empty')}</p>
  {:else}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div
      class="spacer"
      style:height="{items.length * ROW + PAD * 2}px"
      role="presentation"
      onclick={(event) => lasso.consumedClick() || actions.backgroundClick(event)}
    >
      {#key pane.revision}
        <div class="window" style:transform="translateY({PAD + start * ROW}px)" role="presentation">
          {#each rows as entry, i (entry.name)}
            {@const index = start + i}
            {@const sync = onedrive.stateOf(entry, pane.path)}
            {@const selected = pane.selected.has(entry.name)}
            <!-- svelte-ignore a11y_click_events_have_key_events, a11y_interactive_supports_focus -->
            <div
              class="crow"
              class:sel={selected && !active}
              class:cur={selected && active}
              class:dim={entry.hidden || ops.isCut(joinPath(pane.path, entry.name))}
              class:drop={entry.isDir && dnd.isHover(joinPath(pane.path, entry.name))}
              data-drop={entry.isDir ? joinPath(pane.path, entry.name) : undefined}
              role="option"
              aria-selected={selected}
              onpointerdown={(event) => actions.pointerDown(event, index)}
              onclick={(event) => actions.click(event, index)}
              ondblclick={(event) => actions.dblClick(event, entry)}
              oncontextmenu={(event) => actions.context(event, index)}
            >
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
              {#if sync}<SyncBadge state={sync} compact />{/if}
              {#if entry.isDir}<ChevronRight class="chev" size={14} strokeWidth={1.9} />{/if}
            </div>
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
  .col {
    user-select: none;
    width: 232px;
    flex: none;
    position: relative;
    overflow-y: auto;
    overflow-x: hidden;
    border-right: 1px solid var(--border);
    outline: none;
  }

  .spacer {
    position: relative;
  }

  .window {
    position: absolute;
    inset: 0 8px auto 8px;
  }

  .crow {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 32px;
    padding: 0 8px;
    border-radius: 7px;
    transition: background var(--t-hover);
    user-select: none;
  }

  .crow:hover {
    background: var(--hover);
  }

  .crow.sel {
    background: var(--sel);
  }

  .crow.cur {
    background: var(--accent);
    color: var(--on-accent);
  }

  .crow.dim {
    opacity: 0.55;
  }

  .crow.drop {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }

  .nm {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 13px;
  }

  .crow :global(.chev) {
    flex: none;
    color: var(--muted);
  }

  .crow.cur :global(.chev) {
    color: var(--on-accent);
  }

  .note {
    margin: 0;
    padding: 16px;
    font-size: 12.5px;
    color: var(--muted);
  }
</style>
