<script lang="ts">
  import { formatCount } from '../format';
  import { t } from '../i18n';
  import type { Pane } from '../stores/pane.svelte';

  let { pane }: { pane: Pane } = $props();

  const count = $derived(pane.visible.length);
</script>

<footer>
  <span>{count === 1 ? t('status.item') : t('status.items', { count: formatCount(count) })}</span>
  {#if pane.selected.size > 0}
    <span>{t('status.selected', { count: formatCount(pane.selected.size) })}</span>
  {/if}
  {#if pane.hiddenCount > 0}
    <span>{t('status.hidden', { count: formatCount(pane.hiddenCount) })}</span>
  {/if}
</footer>

<style>
  footer {
    display: flex;
    gap: 16px;
    height: 28px;
    align-items: center;
    padding: 0 20px;
    border-top: 1px solid var(--border);
    font-size: 12px;
    color: var(--text-2);
    font-variant-numeric: tabular-nums;
  }
</style>
