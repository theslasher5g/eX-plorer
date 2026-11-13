<script lang="ts">
  import { formatCount, formatSize } from '../format';
  import { t } from '../i18n';
  import type { Pane } from '../stores/pane.svelte';
  import { LIMITS, settings } from '../stores/settings.svelte';

  let { pane }: { pane: Pane } = $props();

  // In der Spaltenansicht gilt die Anzeige der aktiven Spalte.
  const target = $derived(pane.view === 'columns' ? pane.columns.active : pane);

  const text = $derived.by(() => {
    if (pane.isNetwork) return '';
    if (pane.error?.code === 'access_denied') return t('status.denied');
    if (pane.view === 'tree') return t('tree.rows', { count: formatCount(pane.tree.rows.length) });
    if (pane.search.running) return t('search.running', { count: formatCount(pane.search.hits.length) });
    const count = target.visible.length;
    const cols = pane.view === 'columns';
    const parts = [
      count === 1
        ? t(cols ? 'status.colItem' : 'status.item')
        : t(cols ? 'status.colItems' : 'status.items', { count: formatCount(count) }),
    ];
    if (target.selected.size > 0) {
      parts.push(t('status.selected', { count: formatCount(target.selected.size) }));
      if (target.selectedBytes > 0) parts.push(formatSize(target.selectedBytes));
    }
    const search = pane.search;
    if (search.unsupported) parts.push(t('search.networkOnly'));
    else if (search.failed) parts.push(t('search.failed'));
    else if (search.summary) {
      parts.push(t('search.found', { count: formatCount(search.hits.length) }));
      if (search.summary.truncated) parts.push(t('search.truncated'));
      if (search.summary.skippedCloud > 0) parts.push(t('search.cloud', { count: formatCount(search.summary.skippedCloud) }));
    }
    return parts.join(', ');
  });

  const tile = $derived(settings.value.tileSize);
</script>

<footer class="status">
  <span>{text}</span>
  <span class="sp"></span>
  {#if pane.view === 'tree'}
    <span>{t('tree.statusLimits', { depth: settings.value.treeDepth, perDir: formatCount(settings.value.treePerDir) })}</span>
  {:else if target.hiddenCount > 0}
    <span>{t('status.hidden', { count: formatCount(target.hiddenCount) })}</span>
  {/if}
  {#if pane.view === 'grid'}
    <span class="zoom">
      <svg viewBox="0 0 24 24" width="14" height="14" aria-hidden="true"><rect x="5" y="5" width="14" height="14" rx="3" /></svg>
      <input
        class="range"
        type="range"
        min={LIMITS.tileSize.min}
        max={LIMITS.tileSize.max}
        step={LIMITS.tileSize.step}
        value={tile}
        style:--p="{((tile - LIMITS.tileSize.min) / (LIMITS.tileSize.max - LIMITS.tileSize.min)) * 100}%"
        aria-label={t('view.tileSize')}
        oninput={(event) => settings.update({ tileSize: event.currentTarget.valueAsNumber })}
      />
      <svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true"><rect x="3" y="3" width="18" height="18" rx="4" /></svg>
    </span>
  {/if}
</footer>

<style>
  .status {
    height: 32px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 0 16px;
    border-top: 1px solid var(--border);
    font-size: 12.5px;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
    overflow: hidden;
  }

  .sp {
    flex: 1;
  }

  .zoom {
    flex: none;
    width: 170px;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .zoom svg {
    flex: none;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.7;
  }
</style>
