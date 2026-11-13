<script lang="ts">
  import { displayName } from '../names';
  import type { Entry } from '../api';
  import { typeLabel } from '../filetypes';
  import { formatDateLong, formatSize } from '../format';
  import { t } from '../i18n';
  import { onedrive } from '../stores/onedrive.svelte';
  import { quicklook } from '../stores/quicklook.svelte';
  import type { Pane } from '../stores/pane.svelte';
  import SyncBadge from './SyncBadge.svelte';
  import Thumb from './Thumb.svelte';

  let { pane, entry }: { pane: Pane; entry: Entry } = $props();

  const sync = $derived(onedrive.stateOf(entry, pane.path));
</script>

<aside class="prev" aria-label={t('columns.preview')}>
  <div class="art"><Thumb {entry} dir={pane.path} /></div>
  <b class="name">{displayName(entry)}</b>
  <dl class="meta">
    <dt>{t('columns.kind')}</dt>
    <dd>{typeLabel(entry)}</dd>
    <dt>{t('col.size')}</dt>
    <dd>{formatSize(entry.size)}</dd>
    <dt>{t('columns.modified')}</dt>
    <dd>{formatDateLong(entry.modifiedMs)}</dd>
    {#if sync}
      <dt>{t('columns.status')}</dt>
      <dd><SyncBadge state={sync} /></dd>
    {/if}
  </dl>
  <button class="btn pri" onclick={() => quicklook.open(pane)}>{t('columns.quicklook')}</button>
</aside>

<style>
  .prev {
    flex: 1;
    min-width: 240px;
    padding: 28px 24px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 14px;
    overflow: auto;
  }

  .art {
    width: 160px;
    height: 120px;
    border-radius: 12px;
    background: var(--surface-2);
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: var(--shadow-sm);
    overflow: hidden;
  }

  .name {
    font-weight: 600;
    text-align: center;
    word-break: break-word;
  }

  .meta {
    width: 100%;
    margin: 0;
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 6px 16px;
    font-size: 13px;
  }

  .meta dt {
    color: var(--muted);
  }

  .meta dd {
    margin: 0;
    text-align: right;
  }
</style>
