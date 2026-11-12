<script lang="ts">
  import { Folder, Plus, X } from '@lucide/svelte';
  import { t } from '../i18n';
  import { dnd } from '../stores/dnd.svelte';
  import type { Tabs } from '../stores/tabs.svelte';
  import WindowControls from './WindowControls.svelte';

  let { tabs, onnewtab }: { tabs: Tabs; onnewtab: () => void } = $props();

  // Mittlere Maustaste schließt den Tab.
  function onAuxClick(event: MouseEvent, index: number) {
    if (event.button !== 1) return;
    event.preventDefault();
    tabs.close(index);
  }
</script>

<header class="titlebar" data-tauri-drag-region>
  <div class="appmark" aria-hidden="true" data-tauri-drag-region>
    <svg width="20" height="20" viewBox="0 0 24 24">
      <circle cx="12" cy="12" r="9" fill="none" stroke="var(--accent)" stroke-width="2.2" />
      <circle cx="12" cy="12" r="3.2" fill="var(--accent)" />
    </svg>
  </div>

  <div class="tabs" role="tablist" aria-label={t('tabs.label')} data-tauri-drag-region>
    {#each tabs.panes as pane, index (pane.id)}
      <div
        class="tab"
        class:on={index === tabs.active}
        class:drop={index !== tabs.active && dnd.isHover(pane.path)}
        data-drop={pane.path || undefined}
      >
        <button
          class="tab-main"
          role="tab"
          aria-selected={index === tabs.active}
          title={pane.path}
          onclick={() => tabs.activate(index)}
          onauxclick={(event) => onAuxClick(event, index)}
        >
          <Folder size={16} strokeWidth={1.7} />
          <span class="tab-title">{pane.title}</span>
        </button>
        <button
          class="tab-x"
          title={t('tabs.close')}
          aria-label={t('tabs.close')}
          disabled={tabs.panes.length === 1}
          onclick={() => tabs.close(index)}
        >
          <X size={16} strokeWidth={1.7} />
        </button>
      </div>
    {/each}
    <button class="icon-btn" title={t('tabs.new')} aria-label={t('tabs.new')} onclick={onnewtab}>
      <Plus size={18} strokeWidth={1.7} />
    </button>
  </div>

  <WindowControls />
</header>

<style>
  .titlebar {
    height: 44px;
    flex: none;
    display: flex;
    align-items: flex-end;
    padding-left: 12px;
    gap: 8px;
  }

  .appmark {
    width: 28px;
    height: 36px;
    flex: none;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .appmark svg {
    pointer-events: none;
  }

  .tabs {
    display: flex;
    align-items: flex-end;
    gap: 4px;
    flex: 1;
    min-width: 0;
    padding-bottom: 6px;
  }

  .tabs > .icon-btn {
    height: 32px;
  }

  .tab {
    display: flex;
    align-items: center;
    height: 32px;
    min-width: 120px;
    max-width: 200px;
    flex: 0 1 190px;
    border-radius: var(--r-btn);
    color: var(--muted);
    transition:
      background var(--t-select),
      box-shadow var(--t-select);
  }

  .tab:hover {
    background: var(--hover);
  }

  .tab.on {
    background: var(--surface);
    color: var(--text);
    box-shadow: var(--shadow-sm);
  }

  .tab-main {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 1;
    min-width: 0;
    height: 32px;
    padding: 0 4px 0 10px;
    border-radius: var(--r-btn);
  }

  .tab-main :global(svg) {
    flex: none;
  }

  .tab-title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 13px;
  }

  .tab.on .tab-title {
    font-weight: 600;
  }

  .tab-x {
    flex: none;
    width: 24px;
    height: 24px;
    margin-right: 4px;
    border-radius: var(--r-sm);
    display: flex;
    align-items: center;
    justify-content: center;
    opacity: 0.8;
  }

  .tab-x:hover {
    background: var(--press);
    opacity: 1;
  }

  .tab-x:disabled {
    opacity: 0.3;
    pointer-events: none;
  }
</style>
