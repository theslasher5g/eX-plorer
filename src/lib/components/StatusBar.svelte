<script lang="ts">
  import { formatCount, formatSize } from '../format';
  import { t } from '../i18n';
  import type { Pane } from '../stores/pane.svelte';

  let { pane }: { pane: Pane } = $props();

  const text = $derived.by(() => {
    if (pane.error?.code === 'access_denied') return t('status.denied');
    const count = pane.visible.length;
    const parts = [count === 1 ? t('status.item') : t('status.items', { count: formatCount(count) })];
    if (pane.selected.size > 0) {
      parts.push(t('status.selected', { count: formatCount(pane.selected.size) }));
      if (pane.selectedBytes > 0) parts.push(formatSize(pane.selectedBytes));
    }
    return parts.join(', ');
  });
</script>

<footer class="status">
  <span>{text}</span>
  <span class="sp"></span>
  {#if pane.hiddenCount > 0}
    <span>{t('status.hidden', { count: formatCount(pane.hiddenCount) })}</span>
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
</style>
