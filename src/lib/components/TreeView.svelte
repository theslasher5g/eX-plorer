<script lang="ts">
  import { untrack } from 'svelte';
  import { ChevronRight, CircleAlert, Copy, Info, Lock, RefreshCw } from '@lucide/svelte';
  import { getApi, toApiError, type Entry } from '../api';
  import { formatCount, formatSize } from '../format';
  import { errorText, t } from '../i18n';
  import { ops } from '../stores/ops.svelte';
  import type { Pane } from '../stores/pane.svelte';
  import { settings } from '../stores/settings.svelte';
  import { toasts } from '../stores/toast.svelte';
  import { errorLabel, type TreeRow } from '../stores/tree.svelte';
  import FileIcon from './FileIcon.svelte';

  let { pane }: { pane: Pane } = $props();

  const ROW = 28;
  const PAD = 6;
  const OVERSCAN = 12;

  const tree = $derived(pane.tree);
  const rows = $derived(tree.rows);

  let viewport = $state<HTMLDivElement>();
  let scrollTop = $state(0);
  let height = $state(0);

  const start = $derived(Math.max(0, Math.floor((scrollTop - PAD) / ROW) - OVERSCAN));
  const end = $derived(Math.min(rows.length, Math.ceil((scrollTop + height) / ROW) + OVERSCAN));
  const shown = $derived(rows.slice(start, end));

  // Neu scannen, wenn sich Ordner, Limits oder die Anzeige versteckter Dateien ändern.
  $effect(() => {
    const { treeDepth, treeTotal, treePerDir, showHidden } = settings.value;
    void [treeDepth, treeTotal, treePerDir, showHidden, pane.revision];
    const path = pane.path;
    if (pane.error || !path) return;
    // Der Scan ändert eigenen Zustand; der darf diesen Effekt nicht erneut auslösen.
    untrack(() => {
      void pane.tree.start(path);
      if (viewport) viewport.scrollTop = 0;
    });
  });

  // Die Icon-Komponente erwartet einen Listeneintrag.
  function asEntry(row: Extract<TreeRow, { kind: 'entry' }>): Entry {
    const { name, isDir, isLink, size } = row.entry;
    return { name, isDir, isLink, size, modifiedMs: null, createdMs: null, attributes: 0, hidden: false, system: false };
  }

  function activate(row: TreeRow) {
    if (row.kind === 'more') tree.loadMore(row.dir);
    else if (row.kind === 'entry') {
      if (row.entry.isDir) tree.toggle(row.path);
      else void ops.openFile(row.path, row.entry.name);
    }
  }

  function reveal(index: number) {
    if (!viewport || index < 0) return;
    const top = PAD + index * ROW;
    if (top < viewport.scrollTop) viewport.scrollTop = Math.max(0, top - PAD);
    else if (top + ROW > viewport.scrollTop + height) viewport.scrollTop = top + ROW - height + PAD;
  }

  function select(index: number) {
    const row = rows[Math.max(0, Math.min(rows.length - 1, index))];
    if (!row) return;
    tree.selected = row.path;
    reveal(rows.indexOf(row));
  }

  function onKeydown(event: KeyboardEvent) {
    const index = rows.findIndex((row) => row.path === tree.selected);
    const row = rows[index];
    switch (event.key) {
      case 'ArrowDown':
        select(index + 1);
        break;
      case 'ArrowUp':
        select(index < 0 ? 0 : index - 1);
        break;
      case 'Home':
        select(0);
        break;
      case 'End':
        select(rows.length - 1);
        break;
      case 'ArrowRight':
        if (row?.kind === 'entry' && row.entry.isDir) {
          if (!row.expanded) tree.toggle(row.path);
          else select(index + 1);
        }
        break;
      case 'ArrowLeft':
        if (row?.kind === 'entry' && row.entry.isDir && row.expanded) tree.toggle(row.path);
        else if (row && row.kind !== 'root') select(rows.findIndex((r) => r.path === row.dir));
        break;
      case 'Enter':
        if (row) activate(row);
        break;
      default:
        return;
    }
    event.preventDefault();
  }

  function onContext(event: MouseEvent, row: TreeRow) {
    event.preventDefault();
    if (row.kind !== 'entry') return;
    tree.selected = row.path;
    void ops.treeContextMenu(pane, row.path, row.entry.name, row.entry.isDir);
  }

  async function copyText() {
    try {
      await (await getApi()).clipboardSetText(tree.toText());
      toasts.show(t('tree.copied'));
    } catch (err) {
      toasts.show(errorText(toApiError(err).code), 'err');
    }
  }

  const limitText = $derived.by(() => {
    const depth = settings.value.treeDepth;
    const total = formatCount(settings.value.treeTotal);
    if (tree.cancelled) return t('tree.cancelled', { count: formatCount(tree.scanned) });
    if (tree.remoteLimited) return t('tree.remote');
    return t(tree.truncated ? 'tree.limitedHit' : 'tree.limited', { depth, total });
  });
</script>

<div class="tree">
  {#if !pane.error && !tree.error}
    <div class="banner" class:warn={tree.truncated && !tree.scanning} role="status">
      <div class="grow">
        {#if tree.scanning}
          <span class="spin"><RefreshCw size={16} strokeWidth={1.7} /></span>
          <span>{t('tree.scanning', { count: formatCount(tree.scanned) })}</span>
        {:else}
          <Info size={16} strokeWidth={1.7} />
          <span>
            {limitText} ·
            <button class="link" onclick={() => settings.open('tree')}>{t('tree.changeLimits')}</button>
          </span>
        {/if}
      </div>
      {#if tree.scanning}
        <button class="btn" onclick={() => tree.cancel()}>{t('tree.cancel')}</button>
      {:else if tree.cancelled}
        <button class="btn" onclick={() => tree.start(pane.path)}>{t('tree.rescan')}</button>
      {/if}
      <button class="btn" disabled={rows.length === 0} onclick={copyText}>
        <Copy size={16} strokeWidth={1.7} />
        {t('tree.copy')}
      </button>
    </div>
  {/if}

  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <div
    class="viewport"
    bind:this={viewport}
    bind:clientHeight={height}
    onscroll={() => (scrollTop = viewport?.scrollTop ?? 0)}
    onkeydown={onKeydown}
    tabindex="0"
    role="tree"
    aria-busy={tree.scanning}
  >
    {#if pane.error || tree.error}
      {@const code = (pane.error ?? tree.error)?.code ?? 'io'}
      <div class="center">
        <div class="bigic">
          {#if code === 'access_denied'}<Lock size={38} strokeWidth={1.5} />{:else}<CircleAlert size={38} strokeWidth={1.5} />{/if}
        </div>
        <h2>{t('error.title')}</h2>
        <p>{errorText(code)}</p>
        <button class="btn pri" onclick={() => pane.refresh()}>{t('error.retry')}</button>
      </div>
    {:else}
      <div class="spacer" style:height="{rows.length * ROW + 2 * PAD}px" role="presentation">
        <div class="window" style:transform="translateY({start * ROW}px)" role="presentation">
          {#each shown as row (row.path)}
            <!-- svelte-ignore a11y_click_events_have_key_events, a11y_interactive_supports_focus -->
            <div
              class="tr"
              class:sel={tree.selected === row.path}
              role="treeitem"
              aria-selected={tree.selected === row.path}
              aria-expanded={row.kind === 'entry' && row.entry.isDir ? row.expanded : undefined}
              onclick={() => (tree.selected = row.path)}
              ondblclick={() => activate(row)}
              oncontextmenu={(event) => onContext(event, row)}
            >
              {#if row.kind === 'root'}
                <span class="tlabel root">
                  <FileIcon entry={{ name: row.name, isDir: true, isLink: false, size: 0, modifiedMs: null, createdMs: null, attributes: 0, hidden: false, system: false }} dir="" />
                  <span class="nm" title={row.path}>{row.name}</span>
                </span>
              {:else}
                {#each row.guides as open, i (i)}
                  <span class="tc">{#if open}<span class="vl"></span>{/if}</span>
                {/each}
                <span class="tc el" class:last={row.kind === 'more' || row.last} class:mid={row.kind === 'entry' && !row.last}></span>
                {#if row.kind === 'more'}
                  <span class="tchev"></span>
                  <button class="tmore" disabled={row.loading} onclick={() => tree.loadMore(row.dir)}>
                    {row.loading ? t('tree.loading') : t('tree.more', { count: formatCount(row.count) })}
                  </button>
                {:else}
                  {#if row.entry.isDir}
                    <button
                      class="tchev"
                      class:open={row.expanded}
                      tabindex="-1"
                      aria-label={t(row.expanded ? 'tree.collapse' : 'tree.expand')}
                      onclick={(event) => {
                        event.stopPropagation();
                        tree.selected = row.path;
                        tree.toggle(row.path);
                      }}
                    >
                      <ChevronRight size={14} strokeWidth={1.7} />
                    </button>
                  {:else}
                    <span class="tchev"></span>
                  {/if}
                  <span class="tlabel">
                    <FileIcon entry={asEntry(row)} dir={row.dir} />
                    <span class="nm" title={row.entry.name}>{row.entry.name}</span>
                    {#if row.error}
                      <span class="tag err">{errorLabel(row.error)}</span>
                    {:else if row.loading}
                      <span class="tag">{t('tree.loading')}</span>
                    {:else if row.entry.isLink}
                      <span class="tag">{t('tree.link')}</span>
                    {:else if row.entry.depthLimited && !row.expanded}
                      <span class="tag warn">{t('tree.depthLimit')}</span>
                    {/if}
                  </span>
                  {#if !row.entry.isDir}
                    <span class="tmeta">{formatSize(row.entry.size)}</span>
                  {/if}
                {/if}
              {/if}
            </div>
          {/each}
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .tree {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .banner {
    flex: none;
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px 12px;
    margin: 12px 16px 4px;
    padding: 8px 8px 8px 14px;
    border-radius: var(--r-menu);
    background: var(--accent-soft);
    font-size: 13px;
  }

  .banner.warn {
    background: var(--warn-bg);
  }

  .grow {
    flex: 1;
    min-width: 200px;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .grow :global(svg) {
    flex: none;
    color: var(--accent);
  }

  .banner.warn .grow :global(svg) {
    color: var(--warn);
  }

  .spin {
    display: flex;
    animation: spin 1s linear infinite;
  }

  .link {
    color: var(--accent);
    text-decoration: underline;
    text-underline-offset: 2px;
    border-radius: 4px;
  }

  .viewport {
    flex: 1;
    min-height: 0;
    position: relative;
    overflow: auto;
    outline: none;
  }

  .spacer {
    position: relative;
    min-height: 100%;
  }

  .window {
    position: absolute;
    inset: 6px 12px auto;
    will-change: transform;
  }

  .tr {
    display: flex;
    align-items: center;
    height: 28px;
    padding-right: 12px;
    border-radius: 7px;
    transition: background 120ms;
  }

  .tr:hover {
    background: var(--hover);
  }

  .tr.sel {
    background: var(--sel);
  }

  .viewport:focus-visible .tr.sel {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }

  /* Verbindungslinien wie beim tree-Befehl */
  .tc {
    position: relative;
    flex: none;
    width: 22px;
    height: 28px;
  }

  .vl {
    position: absolute;
    left: 10px;
    top: 0;
    bottom: 0;
    width: 1px;
    background: var(--border-strong);
  }

  .el::before {
    content: '';
    position: absolute;
    left: 10px;
    top: 0;
    width: 1px;
    background: var(--border-strong);
  }

  .el.mid::before {
    bottom: 0;
  }

  .el.last::before {
    height: 14px;
  }

  .el::after {
    content: '';
    position: absolute;
    left: 10px;
    top: 14px;
    width: 11px;
    height: 1px;
    background: var(--border-strong);
  }

  .tchev {
    flex: none;
    width: 18px;
    height: 18px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 5px;
    color: var(--muted);
    transition: transform var(--t-select);
  }

  button.tchev:hover {
    background: var(--press);
  }

  .tchev.open {
    transform: rotate(90deg);
  }

  .tlabel {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    margin-left: 2px;
  }

  .tlabel.root {
    margin-left: 8px;
    font-weight: 600;
  }

  .tlabel :global(.folder) {
    width: 20px;
    height: 16px;
  }

  .tlabel :global(.fi) {
    transform: scale(0.85);
  }

  .nm {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tag {
    flex: none;
    padding: 1px 7px;
    border-radius: 9px;
    font-size: 11px;
    white-space: nowrap;
    background: var(--surface-2);
    color: var(--muted);
  }

  .tag.warn {
    background: var(--warn-bg);
    color: var(--warn);
  }

  .tag.err {
    background: var(--err-bg);
    color: var(--err);
  }

  .tmore {
    margin-left: 2px;
    padding: 2px 6px;
    border-radius: var(--r-sm);
    color: var(--accent);
    font-weight: 600;
    text-decoration: underline;
    text-underline-offset: 2px;
  }

  .tmore:disabled {
    color: var(--muted);
    text-decoration: none;
  }

  .tmeta {
    flex: none;
    margin-left: auto;
    padding-left: 12px;
    font-size: 12px;
    color: var(--muted);
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
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
  }

  .center h2 {
    margin: 6px 0 0;
    font-size: 18px;
    font-weight: 600;
  }

  .center p {
    margin: 0 0 8px;
    max-width: 340px;
    color: var(--muted);
  }

  .bigic {
    width: 84px;
    height: 84px;
    border-radius: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--err-bg);
    color: var(--err);
  }
</style>
