<script lang="ts">
  import { onMount } from 'svelte';
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

<div class="caption">
  <button title={t('window.minimize')} aria-label={t('window.minimize')} onclick={() => run('minimize')}>
    <svg viewBox="0 0 14 14" aria-hidden="true"><path d="M2 7h10" /></svg>
  </button>
  <button
    title={maximized ? t('window.restore') : t('window.maximize')}
    aria-label={maximized ? t('window.restore') : t('window.maximize')}
    onclick={() => run('toggleMaximize')}
  >
    {#if maximized}
      <svg viewBox="0 0 14 14" aria-hidden="true">
        <rect x="2.5" y="4.5" width="7" height="7" rx="1.5" />
        <path d="M4.5 4.5V4a1.5 1.5 0 0 1 1.5-1.5h4A1.5 1.5 0 0 1 11.5 4v4A1.5 1.5 0 0 1 10 9.5h-.5" />
      </svg>
    {:else}
      <svg viewBox="0 0 14 14" aria-hidden="true"><rect x="2.5" y="2.5" width="9" height="9" rx="1.5" /></svg>
    {/if}
  </button>
  <button class="cl" title={t('window.close')} aria-label={t('window.close')} onclick={() => run('close')}>
    <svg viewBox="0 0 14 14" aria-hidden="true"><path d="M2.5 2.5l9 9M11.5 2.5l-9 9" /></svg>
  </button>
</div>

<style>
  .caption {
    display: flex;
    flex: none;
    height: 36px;
    align-self: flex-start;
  }

  button {
    width: 46px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 120ms;
  }

  button:hover {
    background: var(--hover);
  }

  button.cl:hover {
    background: #c42b1c;
    color: #fff;
  }

  button:focus-visible {
    outline-offset: -2px;
    border-radius: 0;
  }

  svg {
    width: 14px;
    height: 14px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.2;
  }
</style>
