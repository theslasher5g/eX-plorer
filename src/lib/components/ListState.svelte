<script lang="ts">
  import { CircleAlert, Folder, Lock, Search, WifiOff } from '@lucide/svelte';
  import { errorText, t } from '../i18n';
  import { ops } from '../stores/ops.svelte';
  import type { Pane } from '../stores/pane.svelte';

  // Leer-, Lade-, Fehler- und Kein-Treffer-Zustand einer Ordneransicht.
  let { pane }: { pane: Pane } = $props();

  const filter = $derived(pane.filter.trim());
  // Solange die Suche in Unterordnern noch läuft oder gleich startet, ist „keine Treffer“ verfrüht.
  const searching = $derived(
    !pane.isNetwork &&
      pane.view !== 'tree' &&
      (pane.search.running ||
        (pane.search.summary === null && !pane.search.unsupported && !pane.search.failed)),
  );
  const errorTitle = $derived.by(() => {
    const code = pane.error?.code;
    if (
      code === 'access_denied' ||
      code === 'not_found' ||
      code === 'offline' ||
      code === 'timeout' ||
      code === 'auth_required'
    ) {
      return t(`error.title.${code}`, { name: pane.title || pane.path });
    }
    return t('error.title');
  });
</script>

<div class="center">
  {#if pane.error}
    <div class="bigic err">
      {#if pane.error.code === 'access_denied' || pane.error.code === 'auth_required'}
        <Lock size={38} strokeWidth={1.5} />
      {:else if pane.error.code === 'offline' || pane.error.code === 'timeout'}
        <WifiOff size={38} strokeWidth={1.5} />
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
      {#if pane.error.code === 'auth_required'}
        <button class="btn pri" onclick={() => ops.reconnect(pane)}>{t('error.login')}</button>
      {:else}
        <button class="btn pri" onclick={() => pane.refresh()}>{t('error.retry')}</button>
      {/if}
    </div>
  {:else if pane.loading}
    <p>{t('list.loading')}</p>
  {:else if filter && searching}
    <div class="bigic"><Search size={38} strokeWidth={1.5} /></div>
    <h2>{t('list.searching')}</h2>
    <p>{t('list.searchingHint', { filter })}</p>
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
