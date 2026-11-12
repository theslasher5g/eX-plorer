<script lang="ts">
  import { CircleAlert, Folder, Lock, Search } from '@lucide/svelte';
  import { errorText, t } from '../i18n';
  import type { Pane } from '../stores/pane.svelte';

  // Leer-, Lade-, Fehler- und Kein-Treffer-Zustand einer Ordneransicht.
  let { pane }: { pane: Pane } = $props();

  const filter = $derived(pane.filter.trim());
  const errorTitle = $derived.by(() => {
    const code = pane.error?.code;
    if (code === 'access_denied' || code === 'not_found') {
      return t(`error.title.${code}`, { name: pane.title || pane.path });
    }
    return t('error.title');
  });
</script>

<div class="center">
  {#if pane.error}
    <div class="bigic err">
      {#if pane.error.code === 'access_denied'}
        <Lock size={38} strokeWidth={1.5} />
      {:else}
        <CircleAlert size={38} strokeWidth={1.5} />
      {/if}
    </div>
    <h2>{errorTitle}</h2>
    <p>{errorText(pane.error.code)}</p>
    <div class="actions">
      {#if pane.canBack}
        <button class="btn" onclick={() => pane.back()}>{t('error.back')}</button>
      {/if}
      <button class="btn pri" onclick={() => pane.refresh()}>{t('error.retry')}</button>
    </div>
  {:else if pane.loading}
    <p>{t('list.loading')}</p>
  {:else if filter}
    <div class="bigic"><Search size={38} strokeWidth={1.5} /></div>
    <h2>{t('list.noMatch', { filter })}</h2>
    <p>{t('list.noMatchHint')}</p>
    <div class="actions">
      <button class="btn" onclick={() => (pane.filter = '')}>{t('list.resetFilter')}</button>
    </div>
  {:else}
    <div class="bigic"><Folder size={38} strokeWidth={1.5} /></div>
    <h2>{t('list.empty')}</h2>
    <p>{t('list.emptyHint')}</p>
  {/if}
</div>

<style>
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
    animation: in var(--t-expand) ease-out;
  }

  @keyframes in {
    from {
      opacity: 0;
    }
  }

  h2 {
    margin: 6px 0 0;
    font-size: 18px;
    font-weight: 600;
    overflow-wrap: anywhere;
  }

  p {
    margin: 0;
    max-width: 340px;
    color: var(--muted);
  }

  .actions {
    display: flex;
    gap: 8px;
    margin-top: 8px;
  }

  .bigic {
    width: 84px;
    height: 84px;
    border-radius: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--surface-2);
    color: var(--muted);
  }

  .bigic.err {
    background: var(--err-bg);
    color: var(--err);
  }
</style>
