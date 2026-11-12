<script lang="ts">
  import { onMount } from 'svelte';
  import { Copy, Minus, Square, X } from '@lucide/svelte';
  import { getApi } from '../api';
  import { t } from '../i18n';

  let maximized = $state(false);

  onMount(() => {
    let unlisten: (() => void) | undefined;
    let disposed = false;
    void (async () => {
      const { window } = await getApi();
      maximized = await window.isMaximized();
      const stop = await window.onResized(async () => {
        maximized = await window.isMaximized();
      });
      if (disposed) stop();
      else unlisten = stop;
    })();
    return () => {
      disposed = true;
      unlisten?.();
    };
  });

  async function run(action: 'minimize' | 'toggleMaximize' | 'close') {
    const { window } = await getApi();
    await window[action]();
  }
</script>

<div class="controls">
  <button title={t('window.minimize')} aria-label={t('window.minimize')} onclick={() => run('minimize')}>
    <Minus size={15} strokeWidth={1.5} />
  </button>
  <button
    title={maximized ? t('window.restore') : t('window.maximize')}
    aria-label={maximized ? t('window.restore') : t('window.maximize')}
    onclick={() => run('toggleMaximize')}
  >
    {#if maximized}
      <Copy size={13} strokeWidth={1.5} />
    {:else}
      <Square size={12} strokeWidth={1.5} />
    {/if}
  </button>
  <button class="close" title={t('window.close')} aria-label={t('window.close')} onclick={() => run('close')}>
    <X size={16} strokeWidth={1.5} />
  </button>
</div>

<style>
  .controls {
    display: flex;
    align-self: stretch;
    margin: -8px -8px -8px 4px;
  }

  button {
    display: grid;
    place-items: center;
    width: 46px;
    color: var(--text-2);
    transition:
      background var(--fast) var(--ease),
      color var(--fast) var(--ease);
  }

  button:hover {
    background: var(--hover);
    color: var(--text);
  }

  button.close:hover {
    background: #e81123;
    color: #fff;
  }

  button:focus-visible {
    outline-offset: -2px;
  }
</style>
