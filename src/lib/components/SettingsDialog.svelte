<script lang="ts">
  import { tick } from 'svelte';
  import { X } from '@lucide/svelte';
  import type { Settings, StartView, Theme } from '../api';
  import { formatCount } from '../format';
  import { t, type MessageKey } from '../i18n';
  import { onedrive } from '../stores/onedrive.svelte';
  import { LIMITS, settings, type SettingsSection } from '../stores/settings.svelte';

  type RangeKey = keyof typeof LIMITS;

  const sections: { id: SettingsSection; label: MessageKey }[] = [
    { id: 'general', label: 'settings.general' },
    { id: 'tree', label: 'settings.tree' },
    { id: 'onedrive', label: 'settings.onedrive' },
    { id: 'network', label: 'settings.network' },
  ];

  const themes: { id: Theme; label: MessageKey }[] = [
    { id: 'light', label: 'settings.theme.light' },
    { id: 'dark', label: 'settings.theme.dark' },
    { id: 'auto', label: 'settings.theme.auto' },
  ];

  const views: { id: StartView; label: MessageKey }[] = [
    { id: 'list', label: 'view.listShort' },
    { id: 'grid', label: 'view.gridShort' },
    { id: 'columns', label: 'view.columnsShort' },
  ];

  const ranges: Record<SettingsSection, { key: RangeKey; label: MessageKey; hint: MessageKey }[]> = {
    general: [{ key: 'tileSize', label: 'settings.tileSize', hint: 'settings.tileSizeHint' }],
    tree: [
      { key: 'treeDepth', label: 'settings.depth', hint: 'settings.depthHint' },
      { key: 'treeTotal', label: 'settings.total', hint: 'settings.totalHint' },
      { key: 'treePerDir', label: 'settings.perDir', hint: 'settings.perDirHint' },
    ],
    onedrive: [],
    network: [{ key: 'networkTimeoutSecs', label: 'settings.timeout', hint: 'settings.timeoutHint' }],
  };

  const section = $derived(settings.dialog);
  // Während des Ziehens zeigt der Regler den Entwurf; übernommen wird beim Loslassen.
  let draft = $state<Partial<Settings>>({});
  let card = $state<HTMLDivElement>();

  $effect(() => {
    if (!section) return;
    const previous = document.activeElement;
    void tick().then(() => card?.querySelector<HTMLElement>('.snav button.on')?.focus());
    return () => {
      draft = {};
      if (previous instanceof HTMLElement && previous.isConnected) previous.focus();
      else document.querySelector<HTMLElement>('[role="grid"], [role="tree"], [role="listbox"]')?.focus();
    };
  });

  function value(key: RangeKey): number {
    return draft[key] ?? settings.value[key];
  }

  function display(key: RangeKey): string {
    const current = value(key);
    if (key === 'tileSize') return t('settings.px', { value: current });
    if (key === 'networkTimeoutSecs') return t('settings.sec', { value: current });
    return formatCount(current);
  }

  function percent(key: RangeKey): string {
    const { min, max } = LIMITS[key];
    return `${((value(key) - min) / (max - min)) * 100}%`;
  }

  function onKeydown(event: KeyboardEvent) {
    event.stopPropagation();
    if (event.key === 'Escape') {
      event.preventDefault();
      settings.close();
    } else if (event.key === 'Tab' && card) {
      // Fokus bleibt im Dialog.
      const items = [...card.querySelectorAll<HTMLElement>('button, input')].filter((el) => !el.hasAttribute('disabled'));
      const first = items[0];
      const last = items.at(-1);
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last?.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first?.focus();
      }
    }
  }
</script>

{#snippet toggle(key: 'showExtensions' | 'showHidden' | 'showSyncStatus', label: MessageKey, hint: MessageKey)}
  <div class="srow">
    <div class="lbl">
      <b id="lbl-{key}">{t(label)}</b>
      <span>{t(hint)}</span>
    </div>
    <button
      class="sw"
      role="switch"
      aria-checked={settings.value[key]}
      aria-labelledby="lbl-{key}"
      onclick={() => settings.update({ [key]: !settings.value[key] })}
    ></button>
  </div>
{/snippet}

{#snippet choice<T extends string>(key: string, label: MessageKey, hint: MessageKey, options: { id: T; label: MessageKey }[], current: T, pick: (id: T) => void)}
  <div class="srow">
    <div class="lbl">
      <b id="lbl-{key}">{t(label)}</b>
      <span>{t(hint)}</span>
    </div>
    <div class="choice" role="radiogroup" aria-labelledby="lbl-{key}">
      {#each options as option (option.id)}
        <button role="radio" aria-checked={current === option.id} class:on={current === option.id} onclick={() => pick(option.id)}>
          {t(option.label)}
        </button>
      {/each}
    </div>
  </div>
{/snippet}

{#if section}
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="scrim" onclick={(event) => event.target === event.currentTarget && settings.close()}>
    <div
      class="card"
      bind:this={card}
      role="dialog"
      aria-modal="true"
      aria-labelledby="settings-title"
      tabindex="-1"
      onkeydown={onKeydown}
    >
      <nav class="snav" aria-label={t('settings.title')}>
        <h3>{t('settings.title')}</h3>
        {#each sections as item (item.id)}
          <button class:on={section === item.id} aria-current={section === item.id} onclick={() => settings.open(item.id)}>
            {t(item.label)}
          </button>
        {/each}
      </nav>

      <div class="spane">
        <header>
          <h2 id="settings-title">{t(sections.find((s) => s.id === section)?.label ?? 'settings.title')}</h2>
          <button class="icon-btn" title={t('settings.close')} aria-label={t('settings.close')} onclick={() => settings.close()}>
            <X size={18} strokeWidth={1.7} />
          </button>
        </header>

        <div class="sbody">
          {#if section === 'general'}
            {@render choice('theme', 'settings.theme', 'settings.themeHint', themes, settings.value.theme, (id) => settings.update({ theme: id }))}
            {@render toggle('showExtensions', 'settings.extensions', 'settings.extensionsHint')}
            {@render toggle('showHidden', 'settings.hidden', 'settings.hiddenHint')}
            {@render choice('startView', 'settings.startView', 'settings.startViewHint', views, settings.value.startView, (id) => settings.update({ startView: id }))}
          {:else if section === 'onedrive'}
            {@render toggle('showSyncStatus', 'settings.syncStatus', 'settings.syncStatusHint')}
            <div class="srow">
              <div class="lbl">
                <b>{t('settings.odAccounts')}</b>
                {#if onedrive.accounts.length === 0}
                  <span>{t('settings.odNone')}</span>
                {:else}
                  <ul class="accounts">
                    {#each onedrive.accounts as account (account.path)}
                      <li><b>{account.label}</b> <span>{account.email ?? ''}</span><span class="path">{account.path}</span></li>
                    {/each}
                  </ul>
                {/if}
              </div>
            </div>
            <p class="note">{t('settings.odNote')}</p>
          {:else if section === 'network'}
            <p class="note">{t('settings.netNote')}</p>
          {/if}

          {#each ranges[section] as item (item.key)}
            <div class="srow">
              <div class="lbl">
                <b id="lbl-{item.key}">{t(item.label)}</b>
                <span>{t(item.hint)}</span>
              </div>
              <div class="ctl">
                <input
                  class="range"
                  type="range"
                  min={LIMITS[item.key].min}
                  max={LIMITS[item.key].max}
                  step={LIMITS[item.key].step}
                  value={value(item.key)}
                  style:--p={percent(item.key)}
                  aria-labelledby="lbl-{item.key}"
                  oninput={(event) => (draft = { ...draft, [item.key]: event.currentTarget.valueAsNumber })}
                  onchange={(event) => {
                    settings.update({ [item.key]: event.currentTarget.valueAsNumber });
                    draft = {};
                  }}
                />
                <span class="val">{display(item.key)}</span>
              </div>
            </div>
          {/each}
        </div>

        <div class="sfoot">
          <button class="btn ghost" onclick={() => section && settings.reset(section)}>{t('settings.reset')}</button>
          <button class="btn pri" onclick={() => settings.close()}>{t('settings.done')}</button>
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    background: var(--scrim);
    backdrop-filter: blur(6px);
    animation: fade var(--t-overlay) ease-out;
  }

  .card {
    width: min(780px, 100%);
    height: min(540px, 100%);
    display: flex;
    overflow: hidden;
    border-radius: var(--r-dialog);
    background: var(--surface);
    box-shadow:
      var(--shadow-lg),
      0 0 0 1px var(--border-strong);
    animation: rise var(--t-overlay) ease-out;
    outline: none;
  }

  .snav {
    width: 200px;
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 18px 12px;
    background: var(--surface-2);
  }

  .snav h3 {
    margin: 0 8px 12px;
    font-size: 16px;
    font-weight: 600;
  }

  .snav button {
    height: 34px;
    padding: 0 10px;
    border-radius: var(--r-btn);
    transition: background var(--t-hover);
  }

  .snav button:hover {
    background: var(--hover);
  }

  .snav button.on {
    background: var(--surface);
    box-shadow: var(--shadow-sm);
    font-weight: 600;
  }

  .spane {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  header {
    display: flex;
    align-items: center;
    padding: 18px 20px 8px 28px;
  }

  header h2 {
    flex: 1;
    margin: 0;
    font-size: 20px;
    font-weight: 600;
  }

  .sbody {
    flex: 1;
    overflow: auto;
    padding: 4px 28px 16px;
  }

  .srow {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 10px 20px;
    padding: 14px 0;
    border-bottom: 1px solid var(--border);
  }

  .srow:last-child {
    border-bottom: 0;
  }

  .lbl {
    flex: 1;
    min-width: 200px;
  }

  .lbl b {
    display: block;
    font-weight: 600;
  }

  .lbl span {
    font-size: 12.5px;
    color: var(--muted);
  }

  .ctl {
    width: 260px;
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .val {
    min-width: 56px;
    text-align: right;
    font-size: 13px;
    font-variant-numeric: tabular-nums;
  }

  .choice {
    display: flex;
    padding: 3px;
    gap: 2px;
    border-radius: 10px;
    background: var(--glass);
    border: 1px solid var(--border);
  }

  .choice button {
    height: 26px;
    padding: 0 12px;
    border-radius: 7px;
    font-size: 13px;
    transition: background var(--t-hover);
  }

  .choice button:hover {
    background: var(--hover);
  }

  .choice button.on {
    background: var(--surface);
    box-shadow: var(--shadow-sm);
    font-weight: 600;
  }

  .accounts {
    margin: 6px 0 0;
    padding: 0;
    list-style: none;
    display: grid;
    gap: 8px;
  }

  .accounts li b {
    display: inline;
  }

  .accounts .path {
    display: block;
    font-family: var(--mono, ui-monospace, monospace);
    font-size: 11.5px;
  }

  .note {
    margin: 12px 0 0;
    font-size: 12.5px;
    color: var(--muted);
    line-height: 1.5;
  }

  .sfoot {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 14px 24px;
    border-top: 1px solid var(--border);
  }

  @keyframes fade {
    from {
      opacity: 0;
    }
  }

  @keyframes rise {
    from {
      opacity: 0;
      transform: scale(0.95) translateY(8px);
    }
  }

  @media (max-width: 640px) {
    .card {
      flex-direction: column;
    }

    .snav {
      width: auto;
      flex-direction: row;
      align-items: center;
      padding: 10px 12px;
    }

    .snav h3 {
      display: none;
    }
  }
</style>
